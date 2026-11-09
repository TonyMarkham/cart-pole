//! The camera exactly as the brain opens it (the brain's `camera/mod.rs`):
//! `gst-launch-1.0` running NVIDIA's camera software, its frames read off a
//! pipe with their GStreamer stamps. Here every frame is passed on in order,
//! since the test needs each one, not only the newest.

pub(crate) mod frame;
pub(crate) mod gdp_header;
pub(crate) mod pipeline;

// ---------------------------------------------------------------------------------------------- //

use crate::{
    camera::{frame::Frame, gdp_header::GdpHeader},
    constants::{
        CAMERA_STOP_TIMEOUT_S, FRAME_BYTES, GDP_HEADER_LENGTH, GST_LAUNCH, KILL, KILL_INTERRUPT,
        STOP_POLL_MS,
    },
    settings::camera_settings::CameraSettings,
};

use anyhow::{Context, Result, anyhow};
use std::{
    io::{self, Read},
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        mpsc::{self, Receiver, RecvTimeoutError, Sender},
    },
    thread,
    time::{Duration, Instant},
};

/// The open camera. Dropping it asks the camera software to stop and waits
/// for it, as the brain's does.
pub(crate) struct Camera {
    process: Child,
    frames: Receiver<Result<Frame, String>>,
}

impl Camera {
    /// Starts the camera software and the thread that reads its frames.
    pub(crate) fn open(camera: &CameraSettings) -> Result<Camera> {
        let mut process = Command::new(GST_LAUNCH)
            .args(pipeline::arguments(camera))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .with_context(|| format!("{GST_LAUNCH} did not start"))?;
        let output = process
            .stdout
            .take()
            .ok_or_else(|| anyhow!("{GST_LAUNCH} gave no output pipe"))?;
        let (sender, frames) = mpsc::channel();
        thread::Builder::new()
            .name("camera".to_string())
            .spawn(move || {
                let reason = match read_frames(output, &sender) {
                    Ok(()) => "the camera closed its pipe".to_string(),
                    Err(error) => error,
                };
                let _ended = sender.send(Err(reason));
            })
            .context("the camera's reader thread did not start")?;
        Ok(Camera { process, frames })
    }

    /// The next frame, in order, within `timeout`.
    pub(crate) fn next(&self, timeout: Duration) -> Result<Frame> {
        match self.frames.recv_timeout(timeout) {
            Ok(Ok(frame)) => Ok(frame),
            Ok(Err(reason)) => Err(anyhow!("the camera stopped: {reason}")),
            Err(RecvTimeoutError::Timeout) => {
                Err(anyhow!("no frame within {} ms", timeout.as_millis()))
            }
            Err(RecvTimeoutError::Disconnected) => Err(anyhow!("the camera's reader ended")),
        }
    }
}

impl Drop for Camera {
    /// Asks the camera software to stop: an interrupt, then up to
    /// `CAMERA_STOP_TIMEOUT_S`, looked at every `STOP_POLL_MS`, then killed.
    fn drop(&mut self) {
        if matches!(self.process.try_wait(), Ok(Some(_))) {
            return;
        }
        let interrupted = Command::new(KILL)
            .arg(KILL_INTERRUPT)
            .arg(self.process.id().to_string())
            .status()
            .map(|status| status.success())
            .unwrap_or(false);
        let deadline = Instant::now() + Duration::from_secs(CAMERA_STOP_TIMEOUT_S);
        while interrupted && Instant::now() < deadline {
            if matches!(self.process.try_wait(), Ok(Some(_))) {
                return;
            }
            thread::sleep(Duration::from_millis(STOP_POLL_MS));
        }
        let _killed = self.process.kill();
        let _waited = self.process.wait();
    }
}

/// Reads GDP packets until the pipe ends or a packet is wrong, as the brain's
/// `read_frames` does, and sends each frame as it is whole. `arrived` is
/// stamped the moment its last byte has been read.
fn read_frames(
    mut output: impl Read,
    sender: &Sender<Result<Frame, String>>,
) -> Result<(), String> {
    let mut header = [0u8; GDP_HEADER_LENGTH];
    let mut seq = 0u64;
    loop {
        match output.read_exact(&mut header) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => return Ok(()),
            Err(error) => return Err(format!("reading the camera's pipe: {error}")),
        }
        let packet = GdpHeader::parse(&header)?;
        let length = packet.payload_length as usize;
        if !packet.is_buffer() {
            io::copy(&mut output.by_ref().take(length as u64), &mut io::sink())
                .map_err(|error| format!("skipping a GDP packet: {error}"))?;
            continue;
        }
        if length != FRAME_BYTES {
            return Err(format!(
                "a frame of {length} bytes, not {FRAME_BYTES}: the pipeline is not making 728 x 544 \
                 GRAY8 frames"
            ));
        }
        let mut pixels = vec![0u8; FRAME_BYTES];
        output
            .read_exact(&mut pixels)
            .map_err(|error| format!("reading a frame: {error}"))?;
        let arrived = Instant::now();
        seq += 1;
        let frame = Frame {
            seq,
            stamp_ns: packet.timestamp_ns,
            arrived,
            pixels: Arc::from(pixels),
        };
        if sender.send(Ok(frame)).is_err() {
            return Ok(());
        }
    }
}
