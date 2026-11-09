//! The brain's Python worker: a child process holding the TensorRT engine and
//! `policy.npz`, which finds the markers in each frame and answers the
//! policy's inputs, over its own pipes (`real-loop-dia.md`, *The run and idle
//! states*, how the Rust brain uses Python).

pub(crate) mod detection;
pub(crate) mod policy_values;
pub(crate) mod ready_line;

// ---------------------------------------------------------------------------------------------- //

use crate::{
    constants::{
        CLASSES, DETECTION_FLOATS, DETECTIONS_BYTES, F64_BYTES, FRAME_BYTES, STOP_POLL_MS,
        WORKER_FRAME_REQUEST, WORKER_POLICY_REQUEST, WORKER_STOP_TIMEOUT_MS,
    },
    error::{app_error::AppError, app_result::AppResult},
    settings::{brain_settings::BrainSettings, home::expand_home},
    worker::{detection::Detection, policy_values::PolicyValues, ready_line::ReadyLine},
};

use std::{
    io::{BufRead, BufReader, Read, Write},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

/// The running worker. It lives as long as the brain: dropping it closes its
/// input, which ends it.
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
    pub(crate) fn start(brain: &BrainSettings) -> AppResult<Worker> {
        let (program, arguments) = brain
            .worker
            .split_first()
            .ok_or_else(|| AppError::settings("[brain] worker names no program"))?;
        let mut process = Command::new(program)
            .args(arguments)
            .current_dir(expand_home(&brain.worker_dir))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|error| {
                AppError::worker(format!(
                    "{} did not start in {}: {error}",
                    brain.worker.join(" "),
                    brain.worker_dir
                ))
            })?;
        let input = process
            .stdin
            .take()
            .ok_or_else(|| AppError::worker("the worker gave no input pipe"))?;
        let output = process
            .stdout
            .take()
            .ok_or_else(|| AppError::worker("the worker gave no output pipe"))?;

        // The ready line is read on a thread of its own, so a worker that hangs
        // while loading is given up on after the timeout rather than waited for
        // forever.
        let (sender, receiver) = mpsc::channel();
        thread::Builder::new()
            .name("worker-ready".to_string())
            .spawn(move || {
                let mut output = BufReader::new(output);
                let mut line = String::new();
                let read = output.read_line(&mut line);
                let _sent = sender.send((read, line, output));
            })
            .map_err(|error| {
                AppError::worker(format!("the ready thread did not start: {error}"))
            })?;
        let timeout = Duration::from_millis(brain.worker_start_timeout_ms);
        let (read, line, output) = match receiver.recv_timeout(timeout) {
            Ok(received) => received,
            Err(_) => {
                let _killed = process.kill();
                return Err(AppError::worker(format!(
                    "the worker said nothing within {} ms",
                    brain.worker_start_timeout_ms
                )));
            }
        };
        match read {
            Ok(0) => {
                return Err(AppError::worker(
                    "the worker ended before its ready line: its own message is above",
                ));
            }
            Ok(_) => {}
            Err(error) => {
                return Err(AppError::worker(format!(
                    "reading the worker's ready line: {error}"
                )));
            }
        }
        let values = ReadyLine::parse(&line)?;
        Ok(Worker {
            process,
            input: Some(input),
            output,
            values,
        })
    }

    /// The four classes' boxes for one frame, 728 × 544 bytes.
    pub(crate) fn detect(&mut self, pixels: &[u8]) -> AppResult<[Detection; CLASSES]> {
        if pixels.len() != FRAME_BYTES {
            return Err(AppError::worker(format!(
                "a frame of {} bytes, not {FRAME_BYTES}",
                pixels.len()
            )));
        }
        let input = self.input()?;
        input.write_all(&[WORKER_FRAME_REQUEST])?;
        input.write_all(pixels)?;
        input.flush()?;
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
            .zip(answer.as_chunks::<{ DETECTION_FLOATS * 4 }>().0)
        {
            *detection = Detection::from_bytes(bytes);
        }
        Ok(detections)
    }

    /// The policy's answer, a, for the 36 inputs.
    pub(crate) fn act(&mut self, inputs: &[f64]) -> AppResult<f64> {
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
    fn input(&mut self) -> AppResult<&mut ChildStdin> {
        self.input
            .as_mut()
            .ok_or_else(|| AppError::worker("the worker's input pipe is closed"))
    }

    /// Fills `answer` from the worker, or says the worker has gone.
    fn read_answer(&mut self, answer: &mut [u8]) -> AppResult<()> {
        self.output.read_exact(answer).map_err(|error| {
            AppError::worker(format!(
                "the worker gave no answer: {error}: it has ended, and its own message is above"
            ))
        })
    }
}

impl Drop for Worker {
    /// Closes the worker's input, which ends it: its next read returns nothing
    /// and it exits. It is given `WORKER_STOP_TIMEOUT_MS` to do so, looked at
    /// every `STOP_POLL_MS`, and killed if it has not.
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
