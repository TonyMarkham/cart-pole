use crate::test::Test;

use clap::Parser;

/// The bench's PC side: runs one test on the bench's two boards.
#[derive(Debug, Parser)]
#[command(version, about)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) test: Test,
}
