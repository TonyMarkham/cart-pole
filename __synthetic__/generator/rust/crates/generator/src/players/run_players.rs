use crate::{
    config::Config,
    error::{app_error::AppError, app_result::AppResult},
    players::{
        run_result::RunResult,
        serve_player::{SetQueue, serve_player},
    },
    pool::pool_entry::PoolEntry,
};

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};
use tokio::task::JoinSet;
use tracing::info;

/// Runs every set in `pool` on the grid's players (instance_rows x
/// instance_columns) at once, each player taking the next set as it finishes
/// one, and returns every run in set-name order. No more players start than
/// there are sets; they fill the grid from the top-left cell. The first
/// player to fail stops the whole run: the other players' tasks are dropped,
/// which kills their processes, and no data.yaml is written from a partial
/// dataset.
pub(crate) async fn run_players(
    config: Arc<Config>,
    pool: Vec<PoolEntry>,
) -> AppResult<Vec<RunResult>> {
    let grid = config.grid()?;
    let sets = u16::try_from(pool.len()).unwrap_or(u16::MAX);
    let players = grid.players.min(sets);
    info!(
        players,
        sets = pool.len(),
        window = %format!("{}x{}", grid.window.width, grid.window.height),
        "starting players"
    );
    let queue: SetQueue = Arc::new(Mutex::new(VecDeque::from(pool)));
    let mut tasks = JoinSet::new();
    for k in 0..players {
        tasks.spawn(serve_player(
            Arc::clone(&config),
            grid,
            k,
            Arc::clone(&queue),
        ));
    }

    let mut results = Vec::new();
    while let Some(joined) = tasks.join_next().await {
        let player_results = joined
            .map_err(|error| AppError::player(format!("a player's task ended: {error}")))??;
        results.extend(player_results);
    }
    results.sort_by(|a, b| a.set_name.cmp(&b.set_name));
    Ok(results)
}
