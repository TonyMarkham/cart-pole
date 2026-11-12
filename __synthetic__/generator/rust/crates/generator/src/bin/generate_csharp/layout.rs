//! Where generate-csharp reads and writes, all fixed by the repo's layout and
//! found from the generator crate's own folder, so it works from any working
//! directory.

use std::path::PathBuf;

/// The generator crate's folder (__synthetic__/generator/rust/crates/generator),
/// set by cargo at build time.
const CRATE_DIR: &str = env!("CARGO_MANIFEST_DIR");

/// From the crate's folder to the repo root: crates/generator -> crates ->
/// rust -> generator -> __synthetic__ -> the root.
const CRATE_TO_REPO_ROOT: &str = "../../../../..";

/// From the crate's folder to generation_set.proto's folder (the same path
/// build.rs uses): crates/generator -> rust -> generator, then proto/.
const CRATE_TO_PROTO_DIR: &str = "../../../proto";

/// From the crate's folder to protobuf-tools.toml, beside generator.toml in
/// __synthetic__/generator/rust/.
const CRATE_TO_TOOLS_PIN: &str = "../../protobuf-tools.toml";

/// The .proto the C# classes are generated from.
pub(crate) const PROTO_FILE: &str = "generation_set.proto";

/// The file protoc's C# generator writes for PROTO_FILE. Its rule: the
/// .proto's base name in PascalCase (underscores dropped, each word
/// capitalised) plus .cs, so generation_set.proto -> GenerationSet.cs.
/// Renaming the .proto renames this file.
pub(crate) const GENERATED_FILE: &str = "GenerationSet.cs";

/// The file generate-csharp writes beside it (lifecycle.rs).
pub(crate) const LIFECYCLE_FILE: &str = "GenerationSet.Lifecycle.cs";

/// From the repo root to where the C# lands: the Proto/ folder of the
/// runtime assembly of the Unity package simulation.cart-pole.synthetic-data-gen
/// (its folder unity/packages/simulation.cart-pole.synthetic-data-gen/, whose
/// Runtime/*.asmdef compiles everything under Runtime/). Moving or renaming
/// that package means changing this path.
const REPO_TO_OUT_DIR: &str =
    "unity/packages/simulation.cart-pole.synthetic-data-gen/Runtime/Proto";

/// From the repo root to protoc's cache: under target/, which git ignores.
const REPO_TO_TOOLS_DIR: &str = "target/protoc";

fn repo_root() -> PathBuf {
    PathBuf::from(CRATE_DIR).join(CRATE_TO_REPO_ROOT)
}

pub(crate) fn proto_dir() -> PathBuf {
    PathBuf::from(CRATE_DIR).join(CRATE_TO_PROTO_DIR)
}

pub(crate) fn tools_pin_file() -> PathBuf {
    PathBuf::from(CRATE_DIR).join(CRATE_TO_TOOLS_PIN)
}

pub(crate) fn out_dir() -> PathBuf {
    repo_root().join(REPO_TO_OUT_DIR)
}

pub(crate) fn tools_dir() -> PathBuf {
    repo_root().join(REPO_TO_TOOLS_DIR)
}
