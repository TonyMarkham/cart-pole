mod args;
mod config;
mod dataset;
mod error;
mod players;
mod pool;
mod proto;
#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------------------------- //

use crate::{
    args::Args,
    config::Config,
    dataset::write_data_yaml,
    error::{app_error::AppError, app_result::AppResult},
    players::run_players,
    pool::{load_pool, read_set},
};

use clap::Parser;
use std::{fs, io, process::ExitCode, sync::Arc};
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::EnvFilter;

// What is logged when RUST_LOG is not set: the generator's progress (info)
// and anything worse. RUST_LOG=debug, for example, overrides it.
const DEFAULT_LOG_LEVEL: LevelFilter = LevelFilter::INFO;

// The dataset generator's orchestrator. The detector's training frames are
// made by Unity players running in generator mode; this program loads the
// pool (generator.toml's master, with each set in sets_dir laid over it) and
// checks it, launches the players, gives each one generation set at a time
// until the pool is empty, and writes data.yaml in output_dir from the runs
// they made.
//
// An error ends the run with exit code 1 and its message (which names the
// file or player and where in the code it was raised) on stderr; a run that
// stops part way writes no data.yaml.
#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> AppResult<()> {
    // A RUST_LOG that does not parse is an error, not a silent fallback.
    let filter = EnvFilter::builder()
        .with_default_directive(DEFAULT_LOG_LEVEL.into())
        .from_env()
        .map_err(|error| AppError::settings(format!("parsing RUST_LOG: {error}")))?;
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(io::stderr)
        .init();

    let args = Args::parse();
    let config = Config::load(&args.config)?;
    let master = read_set(&config.master)?;
    let pool = load_pool(&config.sets_dir, &master, &config.val)?;
    info!(
        sets = pool.len(),
        master = %config.master.display(),
        sets_dir = %config.sets_dir.display(),
        output_dir = %config.output_dir.display(),
        "pool loaded"
    );
    for entry in &pool {
        let set = &entry.set;
        info!(
            name = %set.name,
            is_val = entry.is_val,
            states = set.states,
            seed = set.seed,
            "generation set"
        );
    }

    fs::create_dir_all(&config.log_dir)
        .map_err(|error| AppError::io(format!("creating {}: {error}", config.log_dir.display())))?;
    let config = Arc::new(config);
    let runs = run_players(Arc::clone(&config), pool).await?;
    let data_yaml = write_data_yaml(&config.output_dir, &runs)?;
    info!(runs = runs.len(), data_yaml = %data_yaml.display(), "dataset written");
    Ok(())
}
