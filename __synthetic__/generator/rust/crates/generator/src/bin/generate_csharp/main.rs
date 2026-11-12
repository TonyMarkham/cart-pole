mod error;
mod layout;
mod lifecycle;
mod protoc;
#[cfg(test)]
mod tests;
mod tools_pin;

// ---------------------------------------------------------------------------------------------- //

use crate::{
    error::{app_error::AppError, app_result::AppResult},
    layout::{
        GENERATED_FILE, LIFECYCLE_FILE, PROTO_FILE, out_dir, proto_dir, tools_dir, tools_pin_file,
    },
    lifecycle::lifecycle_source,
    protoc::ensure_protoc,
    tools_pin::ToolsPin,
};

use std::{
    fs, io,
    process::{Command, ExitCode},
};
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::EnvFilter;

// What is logged when RUST_LOG is not set: progress (info) and anything
// worse. RUST_LOG=debug, for example, overrides it.
const DEFAULT_LOG_LEVEL: LevelFilter = LevelFilter::INFO;

// protoc's flags (protoc --help): the folder imports resolve against, and the
// folder the C# generator writes into.
const PROTO_PATH_FLAG: &str = "--proto_path=";
const CSHARP_OUT_FLAG: &str = "--csharp_out=";

// Rewrites the Unity package's C# classes for generation_set.proto: run it
// after every change to the .proto, then commit both files it writes.
//
//   cargo run -p generator --bin generate-csharp
//
// Reads  __synthetic__/generator/proto/generation_set.proto, and writes into
// unity/packages/simulation.cart-pole.synthetic-data-gen/Runtime/Proto/:
// - GenerationSet.cs: protoc's classes, untouched;
// - GenerationSet.Lifecycle.cs: [NoAutoStaticsCleanup] on each of them, for
//   Unity 6000.5+ (lifecycle.rs).
// protoc comes from protoc.rs, at the release protobuf-tools.toml pins. Nothing here runs when the generator or Unity
// run: only when the .proto changes. An error ends the run with exit code 1
// and its message on stderr.
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> AppResult<()> {
    // A RUST_LOG that does not parse is an error, not a silent fallback.
    let filter = EnvFilter::builder()
        .with_default_directive(DEFAULT_LOG_LEVEL.into())
        .from_env()
        .map_err(|error| AppError::io(format!("parsing RUST_LOG: {error}")))?;
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(io::stderr)
        .init();

    let pin = ToolsPin::load(&tools_pin_file())?;
    let protoc = ensure_protoc(&tools_dir(), &pin)?;
    let proto_dir = proto_dir();
    let out_dir = out_dir();
    fs::create_dir_all(&out_dir)
        .map_err(|error| AppError::io(format!("creating {}: {error}", out_dir.display())))?;

    let status = Command::new(&protoc)
        .arg(format!("{PROTO_PATH_FLAG}{}", proto_dir.display()))
        .arg(format!("{CSHARP_OUT_FLAG}{}", out_dir.display()))
        .arg(proto_dir.join(PROTO_FILE))
        .status()
        .map_err(|error| AppError::protoc(format!("starting {}: {error}", protoc.display())))?;
    if !status.success() {
        return Err(AppError::protoc(format!(
            "{} ended with {status}",
            protoc.display()
        )));
    }
    let generated_path = out_dir.join(GENERATED_FILE);
    info!(file = %generated_path.display(), "wrote");

    let generated = fs::read_to_string(&generated_path)
        .map_err(|error| AppError::io(format!("reading {}: {error}", generated_path.display())))?;
    let lifecycle_path = out_dir.join(LIFECYCLE_FILE);
    fs::write(&lifecycle_path, lifecycle_source(&generated)?)
        .map_err(|error| AppError::io(format!("writing {}: {error}", lifecycle_path.display())))?;
    info!(file = %lifecycle_path.display(), "wrote");
    Ok(())
}
