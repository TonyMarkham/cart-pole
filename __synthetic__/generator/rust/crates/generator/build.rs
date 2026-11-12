// Generates the Rust types for the generator's messages from the single
// definition, __synthetic__/generator/proto/generation_set.proto: prost's
// structs (generator.rs) and pbjson's serde code for the standard protobuf
// JSON form the sets are written in (generator.serde.rs), both into OUT_DIR,
// where src/proto.rs includes them.
//
// protox compiles the .proto in Rust, so no protoc has to be installed to
// build this crate.

use prost::Message;
use std::{env, error::Error, path::PathBuf};

// The proto folder, from this crate's folder (where cargo runs build
// scripts): crates/generator -> rust -> generator.
const PROTO_DIR: &str = "../../../proto";
const PROTO_FILE: &str = "generation_set.proto";
// The .proto's `package generator;`, as pbjson names it: generate serde code
// for every type in it.
const PROTO_PACKAGE: &str = ".generator";

fn main() -> Result<(), Box<dyn Error>> {
    let proto_dir = PathBuf::from(PROTO_DIR);
    println!(
        "cargo:rerun-if-changed={}",
        proto_dir.join(PROTO_FILE).display()
    );

    let descriptors = protox::compile([PROTO_FILE], [&proto_dir])?;
    let descriptor_bytes = descriptors.encode_to_vec();

    prost_build::Config::new().compile_fds(descriptors)?;

    pbjson_build::Builder::new()
        .out_dir(PathBuf::from(env::var("OUT_DIR")?))
        .register_descriptors(&descriptor_bytes)?
        .build(&[PROTO_PACKAGE])?;
    Ok(())
}
