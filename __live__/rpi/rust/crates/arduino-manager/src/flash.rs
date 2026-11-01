//! `POST /flash`: the PC builds the firmware and posts its `.bin`; the
//! manager uploads it to the board with `arduino-cli`. The upload resets the
//! board, so the serial line is closed first and reopened after. The
//! upload's output goes into the log file of the session it ends; the
//! reopening starts a new file.

use std::{path::Path, time::Duration};

use crate::station::{Action, Station};

/// The largest image accepted, bytes: the RA4M1's 256 KB of code flash
/// (Renesas RA4M1 datasheet, R01DS0355), which the bootloader shares.
pub const MAX_BIN: usize = 256 * 1024;

/// Flashes `image`. Returns whether it worked and what `arduino-cli`
/// printed.
pub async fn flash(station: &Station, image: &[u8]) -> (bool, String) {
    if image.is_empty() {
        return (false, "the request has no .bin in its body".to_owned());
    }
    if !station.claim(Action::Flash) {
        return (false, "busy: wait for the running action to end, or press STOP".to_owned());
    }
    let (ok, output) = upload(station, image).await;
    station.release(Action::Flash, if ok { "uploaded".to_owned() } else { "failed: see the log".to_owned() });
    (ok, output)
}

async fn upload(station: &Station, image: &[u8]) -> (bool, String) {
    let settings = &station.settings.flash;
    let port = &station.settings.serial.port;
    let file = settings.work_dir.join("upload.bin");
    if let Err(error) = std::fs::create_dir_all(&settings.work_dir).and_then(|()| std::fs::write(&file, image)) {
        return (false, format!("cannot write {}: {error}", file.display()));
    }
    station.note(&format!("flashing {} bytes to {port}", image.len()));
    station.link.pause().await;

    let run = tokio::process::Command::new(&settings.arduino_cli)
        .args(["upload", "-p", port, "--fqbn", &settings.fqbn, "--input-file"])
        .arg(&file)
        .output()
        .await;
    let (ok, output) = match run {
        Ok(output) => (
            output.status.success(),
            format!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr)),
        ),
        Err(error) => (false, format!("cannot run {}: {error}", settings.arduino_cli)),
    };

    // The board leaves its bootloader and enumerates again; wait for its
    // port before reopening, so the first attempt does not fail.
    let deadline = tokio::time::Instant::now() + Duration::from_millis(settings.port_wait_ms);
    while !Path::new(port).exists() && tokio::time::Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    for line in output.lines().filter(|line| !line.trim().is_empty()) {
        station.note(&format!("arduino-cli: {line}"));
    }
    station.note(if ok { "Flash: uploaded" } else { "Flash: failed" });
    station.link.resume().await;
    (ok, output)
}
