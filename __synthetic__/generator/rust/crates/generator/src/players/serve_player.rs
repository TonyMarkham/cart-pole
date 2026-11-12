use crate::{
    config::Config,
    error::{app_error::AppError, app_result::AppResult},
    players::{
        check_done::check_done,
        connection::{PlayerSocket, connect},
        launch::launch,
        placement::player_port,
        player_grid::PlayerGrid,
        run_result::RunResult,
    },
    pool::pool_entry::PoolEntry,
    proto::{GenerationDone, GenerationJob},
};

use futures_util::{SinkExt, StreamExt};
use prost::Message as _;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::process::Child;
use tokio_tungstenite::tungstenite::Message;
use tracing::{info, warn};

/// The players' address: each listens on this machine only (StateServer's
/// default host, 127.0.0.1).
const PLAYER_HOST: &str = "127.0.0.1";

/// A plain WebSocket URL's scheme: the StateServer speaks unencrypted
/// WebSocket (its Url, `ws://<address>:<port>/`).
const WEBSOCKET_SCHEME: &str = "ws://";

/// The path the players' WebSocket listens on: the StateServer accepts any
/// path, and its own Url names the root.
const WEBSOCKET_PATH: &str = "/";

/// The sets not yet handed to a player, shared by every player's task.
pub(crate) type SetQueue = Arc<Mutex<VecDeque<PoolEntry>>>;

/// Player `k`'s whole life: launch it, connect, then until the queue is empty
/// take the next set, send it as a `GenerationJob` (one binary message) and
/// wait for its `GenerationDone`. A player runs one job at a time, so the
/// next job goes only after the last one's reply. When the queue is empty the
/// connection closes and the player is killed: it is idle, and every run it
/// made is already on disk. The first failure stops this player and is
/// returned; the player is killed when its `Child` is dropped.
pub(crate) async fn serve_player(
    config: Arc<Config>,
    grid: PlayerGrid,
    k: u16,
    queue: SetQueue,
) -> AppResult<Vec<RunResult>> {
    let port = player_port(config.first_port, k)?;
    let url = format!("{WEBSOCKET_SCHEME}{PLAYER_HOST}:{port}{WEBSOCKET_PATH}");
    let output_directory = config
        .output_dir
        .to_str()
        .ok_or_else(|| {
            AppError::settings(format!(
                "output_dir {} is not valid UTF-8, which GenerationJob.output_directory needs",
                config.output_dir.display()
            ))
        })?
        .to_owned();
    let mut child = launch(&config, grid, k, port)?;
    let mut socket = connect(
        &url,
        &mut child,
        Duration::from_secs(config.player_start_timeout_secs.get()),
        Duration::from_millis(config.connect_retry_millis.get()),
    )
    .await?;
    info!(player = k, %url, "connected");

    let mut results = Vec::new();
    while let Some(entry) = next_set(&queue)? {
        let job = GenerationJob {
            set: Some(entry.set.clone()),
            output_directory: output_directory.clone(),
        };
        info!(player = k, set = %entry.set.name, states = entry.set.states, "job sent");
        socket
            .send(Message::Binary(job.encode_to_vec().into()))
            .await
            .map_err(|error| {
                AppError::player(format!(
                    "sending set '{}' to player {k} ({url}): {error}",
                    entry.set.name
                ))
            })?;
        let done = receive_done(&mut socket, &mut child, k, &url).await?;
        let result = check_done(&entry, &done)?;
        info!(
            player = k,
            set = %result.set_name,
            frames = result.frames,
            run = %result.run_directory.display(),
            "run done"
        );
        results.push(result);
    }

    if let Err(error) = socket.close(None).await {
        warn!(player = k, %error, "closing the connection");
    }
    child
        .kill()
        .await
        .map_err(|error| AppError::player(format!("stopping player {k}: {error}")))?;
    info!(player = k, "stopped");
    Ok(results)
}

/// The next set to hand out, `None` once every set has been taken.
fn next_set(queue: &SetQueue) -> AppResult<Option<PoolEntry>> {
    let mut sets = queue
        .lock()
        .map_err(|_| AppError::player("a player's task failed while taking a set"))?;
    Ok(sets.pop_front())
}

/// The player's reply to the job just sent: the next binary message, decoded
/// as a `GenerationDone`. Text, ping and pong messages are skipped. Fails when
/// the connection ends or the player exits first: a set can run for many
/// minutes, so there is no time limit, but a dead player never answers.
async fn receive_done(
    socket: &mut PlayerSocket,
    child: &mut Child,
    k: u16,
    url: &str,
) -> AppResult<GenerationDone> {
    loop {
        tokio::select! {
            message = socket.next() => match message {
                Some(Ok(Message::Binary(bytes))) => {
                    return GenerationDone::decode(bytes.as_ref()).map_err(|error| {
                        AppError::player(format!(
                            "player {k} ({url}) sent a reply that is not a GenerationDone: {error}"
                        ))
                    });
                }
                Some(Ok(Message::Close(_))) | None => {
                    return Err(AppError::player(format!(
                        "player {k} ({url}) closed the connection before replying"
                    )));
                }
                Some(Ok(_)) => {}
                Some(Err(error)) => {
                    return Err(AppError::player(format!(
                        "reading player {k}'s reply ({url}): {error}"
                    )));
                }
            },
            status = child.wait() => {
                let status = status.map_err(|error| {
                    AppError::player(format!("waiting on player {k}: {error}"))
                })?;
                return Err(AppError::player(format!(
                    "player {k} ({url}) exited before replying: {status}"
                )));
            }
        }
    }
}
