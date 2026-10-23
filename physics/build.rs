//! Links MuJoCo from the release tree `physics.toml` names, and tells the
//! crate where that file is so the run reads the same one.
//!
//! `mujoco.dll` and `mujoco.lib` are copied into `OUT_DIR`, which becomes
//! the link search path. Cargo puts link-search paths inside `target/` on
//! `PATH` for `cargo run` and `cargo test`, so the DLL is found at run
//! time without installing it anywhere.

use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow};

fn main() -> Result<()> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let root = manifest_dir
        .parent()
        .ok_or_else(|| anyhow!("{} has no parent", manifest_dir.display()))?;
    let config = root.join("physics.toml");
    println!("cargo:rerun-if-changed={}", config.display());
    println!("cargo:rustc-env=PHYSICS_TOML={}", config.display());

    // Only mujoco_dir is needed here; the full file is validated by
    // PhysicsConfig at run time.
    let text = fs::read_to_string(&config).with_context(|| format!("reading {}", config.display()))?;
    let table: toml::Table = text.parse().with_context(|| format!("parsing {}", config.display()))?;
    let mujoco_dir = table
        .get("mujoco_dir")
        .and_then(|value| value.as_str())
        .ok_or_else(|| anyhow!("{}: mujoco_dir (a path string) is missing", config.display()))?;
    let mujoco_dir = root.join(mujoco_dir);

    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    copy(&mujoco_dir.join("bin").join("mujoco.dll"), &out_dir)?;
    copy(&mujoco_dir.join("lib").join("mujoco.lib"), &out_dir)?;
    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=dylib=mujoco");
    Ok(())
}

fn copy(source: &Path, out_dir: &Path) -> Result<()> {
    println!("cargo:rerun-if-changed={}", source.display());
    let name = source
        .file_name()
        .ok_or_else(|| anyhow!("{} has no file name", source.display()))?;
    fs::copy(source, out_dir.join(name)).with_context(|| {
        format!(
            "copying {} (physics.toml's mujoco_dir must be a MuJoCo 3.15.0 release tree)",
            source.display()
        )
    })?;
    Ok(())
}
