//! Links MuJoCo from the release tree `orchestrator.toml` names. That file
//! is read only here: the run takes the MJCF and its names from the
//! orchestrator's command line.
//!
//! `mujoco.dll` and `mujoco.lib` are copied into `OUT_DIR`, which becomes
//! the link search path. Cargo puts link-search paths inside `target/` on
//! `PATH` for `cargo run` and `cargo test`, so the DLL is found at run
//! time without installing it anywhere.
//!
//! The trainer launches the built binary from `target/<profile>/` without
//! cargo, so `mujoco.dll` is also copied there: Windows finds a DLL beside
//! the executable first.

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
    let config = root.join("orchestrator.toml");
    println!("cargo:rerun-if-changed={}", config.display());

    let text = fs::read_to_string(&config).with_context(|| format!("reading {}", config.display()))?;
    let table: toml::Table = text.parse().with_context(|| format!("parsing {}", config.display()))?;
    // mujoco_dir is the only key: a leftover key (model, [names]) would
    // look like it configures the run, and nothing reads it.
    if let Some(key) = table.keys().find(|key| *key != "mujoco_dir") {
        return Err(anyhow!(
            "{}: unknown key `{key}`; only mujoco_dir belongs here (the MJCF and its names are the trainer's settings.toml [mjcf])",
            config.display()
        ));
    }
    let mujoco_dir = table
        .get("mujoco_dir")
        .and_then(|value| value.as_str())
        .ok_or_else(|| anyhow!("{}: mujoco_dir (a path string) is missing", config.display()))?;
    let mujoco_dir = root.join(mujoco_dir);

    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    copy(&mujoco_dir.join("bin").join("mujoco.dll"), &out_dir)?;
    copy(&mujoco_dir.join("lib").join("mujoco.lib"), &out_dir)?;
    // OUT_DIR is target/<profile>/build/physics-<hash>/out: three folders up
    // is target/<profile>/, where the binaries are.
    let profile_dir = out_dir
        .ancestors()
        .nth(3)
        .ok_or_else(|| anyhow!("{} is not under target/<profile>/build", out_dir.display()))?;
    copy(&mujoco_dir.join("bin").join("mujoco.dll"), profile_dir)?;
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
            "copying {} (orchestrator.toml's mujoco_dir must be a MuJoCo 3.15.0 release tree)",
            source.display()
        )
    })?;
    Ok(())
}
