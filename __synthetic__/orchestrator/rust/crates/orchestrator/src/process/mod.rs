use crate::{
    args::Args,
    window::{Pixels, grid_position},
};

use anyhow::{Context, Result, anyhow};
use std::{
    net::{Ipv4Addr, SocketAddr},
    path::Path,
    process::Stdio,
    time::Duration,
};
use tokio::{
    net::{TcpListener, TcpStream},
    process::{Child, Command},
};

pub(crate) const CLIENT_EXIT_GRACE: Duration = Duration::from_secs(5);

// The player's command line (PlayerController.cs and the StateServer in
// com.greyhound.mujoco.runtime): the job it runs as, always the camera here,
// so the prefab's default job never decides it; and the port its StateServer
// listens on.
const PLAYER_MODE_ARGUMENT: &str = "-player-mode";
const CAMERA_MODE: &str = "SyntheticCamera";
const STATE_SERVER_ARGUMENT: &str = "-state-server";

// The frame socket's address as the synthetic Orin connects to it: the
// player's StateServer on 127.0.0.1, the loop being on this workstation.
pub(crate) fn frames_url(port: u16) -> String {
    format!("ws://{}/", SocketAddr::from((Ipv4Addr::LOCALHOST, port)))
}

pub(crate) fn start_player(
    args: &Args,
    k: usize,
    port: u16,
    size: Option<Pixels>,
) -> Result<Option<Child>> {
    match (&args.player, args.no_launch) {
        (Some(player), false) => {
            let window = size.map(|size| {
                (
                    grid_position(k, args.instance_columns, size, args.window_origin),
                    size,
                )
            });
            Ok(Some(launch(k, player, port, window)?))
        }
        _ => {
            eprintln!(
                "[{k}] --no-launch: enter Play mode in the editor; the synthetic Orin connects to its StateServer on port {port}"
            );
            Ok(None)
        }
    }
}

pub(crate) fn launch(
    k: usize,
    player: &Path,
    port: u16,
    window: Option<(Pixels, Pixels)>,
) -> Result<Child> {
    let mut command = Command::new(player);
    command
        .arg(PLAYER_MODE_ARGUMENT)
        .arg(CAMERA_MODE)
        .arg(STATE_SERVER_ARGUMENT)
        .arg(port.to_string());
    let mut shown = format!(
        "{} {PLAYER_MODE_ARGUMENT} {CAMERA_MODE} {STATE_SERVER_ARGUMENT} {port}",
        player.display()
    );
    if let Some((
        Pixels { x, y },
        Pixels {
            x: width,
            y: height,
        },
    )) = window
    {
        command
            .arg("-popupwindow")
            .arg("-window-position")
            .arg(format!("{x},{y}"))
            .arg("-screen-width")
            .arg(width.to_string())
            .arg("-screen-height")
            .arg(height.to_string());
        shown.push_str(&format!(
            " -popupwindow -window-position {x},{y} -screen-width {width} -screen-height {height}"
        ));
    }
    eprintln!("[{k}] launching {shown}");
    command
        .stdin(Stdio::null())
        .stdout(child_stdout())
        .kill_on_drop(true)
        .spawn()
        .with_context(|| format!("launching {}", player.display()))
}

pub(crate) fn child_stdout() -> Stdio {
    Stdio::from(std::io::stderr())
}

pub(crate) async fn bind(port: u16, what: &str) -> Result<(TcpListener, String)> {
    let address = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    let listener = TcpListener::bind(address)
        .await
        .with_context(|| format!("binding the {what} on {address}"))?;
    let url = format!("ws://{address}");
    eprintln!("{what}: listening on {url}");
    Ok((listener, url))
}

pub(crate) async fn accept(
    listener: &TcpListener,
    child: &mut Child,
    who: &str,
    timeout: Duration,
) -> Result<TcpStream> {
    let wait = async {
        tokio::select! {
            accepted = listener.accept() => Ok(accepted?.0),
            status = child.wait() => Err(anyhow!("{who} exited before connecting: {status:?}")),
        }
    };
    tokio::time::timeout(timeout, wait).await.map_err(|_| {
        anyhow!(
            "no connection from the {who} within {} s",
            timeout.as_secs()
        )
    })?
}

pub(crate) async fn stop(mut child: Child, who: &str) -> Result<()> {
    match tokio::time::timeout(CLIENT_EXIT_GRACE, child.wait()).await {
        Ok(status) => eprintln!("{who} exited: {}", status?),
        Err(_) => {
            child
                .kill()
                .await
                .with_context(|| format!("killing the {who}"))?;
            eprintln!("{who} killed after {} s", CLIENT_EXIT_GRACE.as_secs());
        }
    }
    Ok(())
}
