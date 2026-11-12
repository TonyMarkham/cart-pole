//! protoc for this machine, from NuGet's Google.Protobuf.Tools at the release
//! protobuf-tools.toml pins (`ToolsPin`). The first run downloads that
//! release's .nupkg, checks its SHA-256 against the pin, and unpacks only
//! this machine's protoc into the cache; later runs reuse it.

use crate::{
    error::{app_error::AppError, app_result::AppResult},
    tools_pin::ToolsPin,
};

use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{self, Cursor},
    path::{Path, PathBuf},
};
use zip::ZipArchive;

/// NuGet's id for the package, lowercase as its flat-container URLs and file
/// names use it.
const PACKAGE_ID: &str = "google.protobuf.tools";

/// NuGet's flat container: a release's .nupkg is at
/// `<NUGET_FLAT_CONTAINER>/<id>/<version>/<id>.<version>.nupkg`
/// (NuGet's package-content API, "Download package content (.nupkg)").
const NUGET_FLAT_CONTAINER: &str = "https://api.nuget.org/v3-flatcontainer";

/// The most the download may be: the 3.36.2 .nupkg is 20,508,720 bytes, and
/// ureq refuses larger bodies than its limit.
const MAX_NUPKG_BYTES: u64 = 64 * 1024 * 1024;

/// The extension protoc is written under before being renamed into place.
const PARTIAL_EXTENSION: &str = "partial";

/// Unix permission bits for the unpacked protoc: rwxr-xr-x, so it can run.
#[cfg(unix)]
const EXECUTABLE_MODE: u32 = 0o755;

// protoc's path inside the package for each machine: the package's tools/
// folder layout (listed from the 3.36.2 .nupkg). Apple silicon has no arm64
// build in the package; it runs the x64 one under Rosetta.
/// Windows on x86-64.
const ENTRY_WINDOWS_X64: &str = "tools/windows_x64/protoc.exe";
/// Windows on 32-bit x86.
const ENTRY_WINDOWS_X86: &str = "tools/windows_x86/protoc.exe";
/// macOS, Intel and (under Rosetta) Apple silicon.
const ENTRY_MACOS_X64: &str = "tools/macosx_x64/protoc";
/// Linux on x86-64.
const ENTRY_LINUX_X64: &str = "tools/linux_x64/protoc";
/// Linux on 64-bit ARM.
const ENTRY_LINUX_ARM64: &str = "tools/linux_aarch64/protoc";
/// Linux on 32-bit x86.
const ENTRY_LINUX_X86: &str = "tools/linux_x86/protoc";

fn protoc_entry(pin: &ToolsPin) -> AppResult<&'static str> {
    if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        Ok(ENTRY_WINDOWS_X64)
    } else if cfg!(all(target_os = "windows", target_arch = "x86")) {
        Ok(ENTRY_WINDOWS_X86)
    } else if cfg!(target_os = "macos") {
        Ok(ENTRY_MACOS_X64)
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        Ok(ENTRY_LINUX_X64)
    } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
        Ok(ENTRY_LINUX_ARM64)
    } else if cfg!(all(target_os = "linux", target_arch = "x86")) {
        Ok(ENTRY_LINUX_X86)
    } else {
        Err(AppError::tools(format!(
            "Google.Protobuf.Tools {} has no protoc for this machine",
            pin.version
        )))
    }
}

/// This machine's protoc, at `<tools_dir>/<id>.<version>/<entry>`, e.g.
/// target/protoc/google.protobuf.tools.3.36.2/tools/windows_x64/protoc.exe:
/// unpacked there on the first run, reused after.
pub(crate) fn ensure_protoc(tools_dir: &Path, pin: &ToolsPin) -> AppResult<PathBuf> {
    let entry = protoc_entry(pin)?;
    let release = format!("{PACKAGE_ID}.{}", pin.version);
    let protoc = tools_dir.join(&release).join(entry);
    if protoc.is_file() {
        return Ok(protoc);
    }

    let url = format!(
        "{NUGET_FLAT_CONTAINER}/{PACKAGE_ID}/{}/{release}.nupkg",
        pin.version
    );
    tracing::info!(version = %pin.version, url = %url, "downloading Google.Protobuf.Tools");
    let nupkg = ureq::get(&url)
        .call()
        .map_err(|error| AppError::tools(format!("downloading {url}: {error}")))?
        .into_body()
        .with_config()
        .limit(MAX_NUPKG_BYTES)
        .read_to_vec()
        .map_err(|error| AppError::tools(format!("reading {url}: {error}")))?;

    let actual: String = Sha256::digest(&nupkg)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    if actual != pin.sha256 {
        return Err(AppError::tools(format!(
            "{url} has SHA-256 {actual}, expected {} (protobuf-tools.toml)",
            pin.sha256
        )));
    }

    // A .nupkg is a zip; only this machine's protoc is taken out of it.
    let mut archive = ZipArchive::new(Cursor::new(nupkg))
        .map_err(|error| AppError::tools(format!("opening {url} as a zip: {error}")))?;
    let mut packed = archive
        .by_name(entry)
        .map_err(|error| AppError::tools(format!("finding {entry} in {url}: {error}")))?;
    let folder = protoc.parent().unwrap_or(tools_dir);
    fs::create_dir_all(folder)
        .map_err(|error| AppError::io(format!("creating {}: {error}", folder.display())))?;
    // Written beside the final name and renamed into place, so an
    // interrupted run never leaves a half-written protoc that the next run
    // would trust.
    let partial = protoc.with_extension(PARTIAL_EXTENSION);
    let mut out = File::create(&partial)
        .map_err(|error| AppError::io(format!("creating {}: {error}", partial.display())))?;
    io::copy(&mut packed, &mut out)
        .map_err(|error| AppError::io(format!("writing {}: {error}", partial.display())))?;
    drop(out);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&partial, fs::Permissions::from_mode(EXECUTABLE_MODE)).map_err(
            |error| AppError::io(format!("making {} executable: {error}", partial.display())),
        )?;
    }
    fs::rename(&partial, &protoc).map_err(|error| {
        AppError::io(format!(
            "renaming {} to {}: {error}",
            partial.display(),
            protoc.display()
        ))
    })?;
    Ok(protoc)
}
