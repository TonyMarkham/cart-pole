//! The brain's Python worker, unchanged (`__live__/orin/python/src/brain_worker.py`),
//! started and spoken to as the brain does it (the brain's `worker/mod.rs`).
//! The one difference: a frame's request is split into sending the frame and
//! reading the boxes back, so the moment between the two can be stamped.

pub(crate) mod detection;
pub(crate) mod policy_values;
pub(crate) mod ready_line;

// ---------------------------------------------------------------------------------------------- //

use crate::{
    constants::{
        CLASSES, DETECTION_BYTES, DETECTIONS_BYTES, F64_BYTES, FRAME_BYTES, STOP_POLL_MS,
        WORKER_FRAME_REQUEST, WORKER_POLICY_REQUEST, WORKER_READY_THREAD, WORKER_STOP_TIMEOUT_MS,
    },
    home::expand_home,
    settings::worker_settings::WorkerSettings,
    worker::{detection::Detection, policy_values::PolicyValues, ready_line::ReadyLine},
};

use anyhow::{Context, Result, anyhow, bail};
use std::{
    io::{BufRead, BufReader, Read, Write},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

/// The running worker. Dropping it closes its input, which ends it.
pub(crate) struct Worker {
    process: Child,
    /// The worker's input pipe. An `Option` only so that `Drop` can close it
    /// first and then wait for the worker to end.
    input: Option<ChildStdin>,
    output: BufReader<ChildStdout>,
    pub(crate) values: PolicyValues,
}

impl Worker {
    /// Starts the worker in its folder and waits for its ready line, which
    /// comes once the engine and the policy are loaded.
    pub(crate) fn start(settings: &WorkerSettings) -> Result<Worker> {
        let (program, arguments) = settings
            .worker
            .split_first()
            .ok_or_else(|| anyhow!("[worker] worker names no program"))?;
        let mut process = Command::new(program)
            .args(arguments)
            .current_dir(expand_home(&settings.worker_dir))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .with_context(|| {
                format!(
                    "{} did not start in {}",
                    settings.worker.join(" "),
                    settings.worker_dir
                )
            })?;
        let input = process
            .stdin
            .take()
            .ok_or_else(|| anyhow!("the worker gave no input pipe"))?;
        let output = process
            .stdout
            .take()
            .ok_or_else(|| anyhow!("the worker gave no output pipe"))?;

        // The ready line is read on a thread of its own, so a worker that hangs
        // while loading is given up on after the timeout.
        let (sender, receiver) = mpsc::channel();
        thread::Builder::new()
            .name(WORKER_READY_THREAD.to_string())
            .spawn(move || {
                let mut output = BufReader::new(output);
                let mut line = String::new();
                let read = output.read_line(&mut line);
                let _sent = sender.send((read, line, output));
            })
            .context("the ready thread did not start")?;
        let timeout = Duration::from_millis(settings.worker_start_timeout_ms);
        let Ok((read, line, output)) = receiver.recv_timeout(timeout) else {
            let _killed = process.kill();
            bail!(
                "the worker said nothing within {} ms",
                settings.worker_start_timeout_ms
            );
        };
        if read.context("reading the worker's ready line")? == 0 {
            bail!("the worker ended before its ready line: its own message is above");
        }
        let values = ReadyLine::parse(&line)?;
        Ok(Worker {
            process,
            input: Some(input),
            output,
            values,
        })
    }

    /// Sends one frame, 728 × 544 bytes, for detection. It returns once the
    /// last byte is in the pipe.
    pub(crate) fn send_frame(&mut self, pixels: &[u8]) -> Result<()> {
        if pixels.len() != FRAME_BYTES {
            bail!("a frame of {} bytes, not {FRAME_BYTES}", pixels.len());
        }
        let input = self.input()?;
        input.write_all(&[WORKER_FRAME_REQUEST])?;
        input.write_all(pixels)?;
        input.flush()?;
        Ok(())
    }

    /// The four classes' boxes for the frame just sent.
    pub(crate) fn read_detections(&mut self) -> Result<[Detection; CLASSES]> {
        let mut answer = [0u8; DETECTIONS_BYTES];
        self.read_answer(&mut answer)?;
        let mut detections = [Detection {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
            confidence: 0.0,
        }; CLASSES];
        for (detection, bytes) in detections
            .iter_mut()
            .zip(answer.as_chunks::<DETECTION_BYTES>().0)
        {
            *detection = Detection::from_bytes(bytes);
        }
        Ok(detections)
    }

    /// The policy's answer, a, for the 36 inputs.
    pub(crate) fn act(&mut self, inputs: &[f64]) -> Result<f64> {
        let mut bytes = Vec::with_capacity(inputs.len() * F64_BYTES);
        for value in inputs {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        let input = self.input()?;
        input.write_all(&[WORKER_POLICY_REQUEST])?;
        input.write_all(&bytes)?;
        input.flush()?;
        let mut answer = [0u8; F64_BYTES];
        self.read_answer(&mut answer)?;
        Ok(f64::from_le_bytes(answer))
    }

    /// The input pipe, which is only ever missing while the worker is being
    /// dropped.
    fn input(&mut self) -> Result<&mut ChildStdin> {
        self.input
            .as_mut()
            .ok_or_else(|| anyhow!("the worker's input pipe is closed"))
    }

    /// Fills `answer` from the worker, or says the worker has gone.
    fn read_answer(&mut self, answer: &mut [u8]) -> Result<()> {
        self.output
            .read_exact(answer)
            .context("the worker gave no answer: it has ended, and its own message is above")
    }
}

impl Drop for Worker {
    /// Closes the worker's input, which ends it. It is given
    /// `WORKER_STOP_TIMEOUT_MS` to do so, looked at every `STOP_POLL_MS`, and
    /// killed if it has not.
    fn drop(&mut self) {
        drop(self.input.take());
        let deadline = Instant::now() + Duration::from_millis(WORKER_STOP_TIMEOUT_MS);
        while Instant::now() < deadline {
            if matches!(self.process.try_wait(), Ok(Some(_))) {
                return;
            }
            thread::sleep(Duration::from_millis(STOP_POLL_MS));
        }
        let _killed = self.process.kill();
        let _waited = self.process.wait();
    }
}
