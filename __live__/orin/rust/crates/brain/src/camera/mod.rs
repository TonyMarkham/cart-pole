//! The camera as the brain sees it: `gst-launch-1.0` running NVIDIA's camera
//! software, its frames read off a pipe with their GStreamer stamps, and the
//! newest kept for the loop.

pub(crate) mod frame;
pub(crate) mod gdp_header;
pub(crate) mod latest;
pub(crate) mod latest_state;
pub(crate) mod pipeline;

// ---------------------------------------------------------------------------------------------- //

use crate::{
    camera::{frame::Frame, gdp_header::GdpHeader, latest::Latest},
    constants::{
        CAMERA_STOP_TIMEOUT_S, FRAME_BYTES, GDP_HEADER_LENGTH, GST_LAUNCH, KILL, KILL_INTERRUPT,
        STOP_POLL_MS,
    },
    error::{app_error::AppError, app_result::AppResult},
    settings::camera_settings::CameraSettings,
};

use std::{
    io::{self, Read},
    process::{Child, Command, Stdio},
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

/// The open camera. Opening it starts the camera software; dropping it asks the
/// software to stop and waits for it. Use it for the length of a run only, so
/// `calibrate_service.py` can have the camera between runs.
pub(crate) struct Camera {
    process: Child,
    latest: Arc<Latest>,
}

impl Camera {
    /// Starts the camera software and the thread that reads its frames.
    pub(crate) fn open(camera: &CameraSettings) -> AppResult<Camera> {
        let mut process = Command::new(GST_LAUNCH)
            .args(pipeline::arguments(camera))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|error| AppError::camera(format!("{GST_LAUNCH} did not start: {error}")))?;
        let output = process
            .stdout
            .take()
            .ok_or_else(|| AppError::camera(format!("{GST_LAUNCH} gave no output pipe")))?;
        let latest = Arc::new(Latest::default());
        let reader_latest = latest.clone();
        thread::Builder::new()
            .name("camera".to_string())
            .spawn(move || {
                let reason = match read_frames(output, &reader_latest) {
                    Ok(()) => "the camera closed its pipe".to_string(),
                    Err(error) => error,
                };
                reader_latest.end(reason);
            })
            .map_err(|error| {
                AppError::camera(format!("the camera's reader thread did not start: {error}"))
            })?;
        Ok(Camera { process, latest })
    }

    /// How many frames the camera has made since it opened.
    pub(crate) fn seq(&self) -> u64 {
        self.latest.seq()
    }

    /// The first frame numbered after `seq`, within `timeout`.
    pub(crate) fn next_after(&self, seq: u64, timeout: Duration) -> AppResult<Frame> {
        self.latest.next_after(seq, timeout)
    }

    /// Waits until the camera has made `count` frames: the settling frames,
    /// dropped unread. Each is given `timeout` to arrive.
    pub(crate) fn settle(&self, count: u64, timeout: Duration) -> AppResult<()> {
        while self.seq() < count {
            let seq = self.seq();
            self.next_after(seq, timeout)?;
        }
        Ok(())
    }
}

impl Drop for Camera {
    /// Asks the camera software to stop, as the Python does: an interrupt, then
    /// up to `CAMERA_STOP_TIMEOUT_S`, looked at every `STOP_POLL_MS`, then
    /// killed.
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

/// Reads GDP packets until the pipe ends or a packet is wrong: a frame of the
/// wrong size, or not GDP at all. Each frame goes to `latest` as it is whole.
pub(crate) fn read_frames(mut output: impl Read, latest: &Latest) -> Result<(), String> {
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
            // GStreamer's own messages: the stream's start, its caps, its
            // segment. Read past them.
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
        seq += 1;
        latest.put(Frame {
            seq,
            stamp_ns: packet.timestamp_ns,
            arrived: Instant::now(),
            pixels: Arc::from(pixels),
        });
    }
}
