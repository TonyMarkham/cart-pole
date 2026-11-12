use clap::Parser;
use std::path::PathBuf;

/// The dataset generator's orchestrator: reads the pool of generation sets
/// (__synthetic__/generator/sets/*.json) that the Unity generator players
/// will each take one of at a time.
#[derive(Parser, Debug)]
#[command(version, about)]
pub(crate) struct Args {
    /// The generator's settings, generator.toml (__synthetic__/generator/rust/).
    #[arg(long)]
    pub(crate) config: PathBuf,
}
