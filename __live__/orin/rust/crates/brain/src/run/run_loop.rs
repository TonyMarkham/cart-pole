use crate::{
    arduino::Link,
    camera::Camera,
    command::Command,
    constants::{END_CLASS, FRAME_HEIGHT, FRAME_WIDTH, NANOS_PER_MILLI, PIVOT_CLASS},
    error::app_result::AppResult,
    geometry::{homography::Homography, point::Point},
    history::History,
    inputs::Inputs,
    log::{frame_line::FrameLine, run_log::RunLog},
    pool::Pool,
    run::{brain::Brain, run_state::RunState},
    worker::{Worker, detection::Detection},
};

use std::{
    thread,
    time::{Duration, Instant},
};

/// One run, from the camera opening to the reason it ended: STOP, the
/// board's FAULT STATE or UNCALIBRATED reply, or a failure. The sequence is
/// `real-loop-dia.md`'s (*The sequence*, the loop), one command per pass:
///
/// 1. the camera's next frame after the wait, with its stamp;
/// 2. the worker finds the markers;
/// 3. Pivot and End to metres: pixels, the bend undone, the matrix;
/// 4. the pool: the new entry at age 0, the older ones aged;
/// 5. the 36 inputs, the worker's a, the command c;
/// 6. the datagram, then the log line;
/// 7. the board's replies, if any; then the wait.
///
/// The brain is marked idle the moment the passes end, before the camera is
/// closed: `idle` is answered then, so STOP's zeros follow the last command
/// at once, and the camera's closing, which can take seconds, comes after.
pub(crate) fn run(brain: &Brain, homography: Homography, mut log: RunLog) -> AppResult<String> {
    let settings = &brain.settings;
    let frame_timeout = Duration::from_millis(settings.brain.frame_timeout_ms);

    let mut worker = brain.worker()?;
    let link = Link::connect(&settings.arduino)?;
    let camera = Camera::open(&settings.camera)?;
    camera.settle(settings.camera.settle_frames, frame_timeout)?;
    println!(
        "run started: policy {}, top rate {} pulses/s, log {}",
        worker.values.source,
        worker.values.top_pulse_rate,
        log.dir.display()
    );

    let ended = passes(brain, &camera, &mut worker, &link, &homography, &mut log);
    brain.mark_idle(match &ended {
        Ok(reason) => reason.clone(),
        Err(error) => error.message().to_string(),
    });
    drop(camera);
    ended
}

/// The passes of the loop, until STOP, a reply that ends the run, or a
/// failure.
fn passes(
    brain: &Brain,
    camera: &Camera,
    worker: &mut Worker,
    link: &Link,
    homography: &Homography,
    log: &mut RunLog,
) -> AppResult<String> {
    let settings = &brain.settings;
    let pressed = Instant::now();
    let frame_timeout = Duration::from_millis(settings.brain.frame_timeout_ms);
    let delay = Duration::from_millis(settings.brain.delay_ms);
    let values = worker.values.clone();
    let mut pool = Pool::pre_warmed(settings.brain.pre_warm_gap_ms);
    let mut history = History::new(values.past_commands);

    loop {
        if stop_asked(brain) {
            return Ok("stopped".to_string());
        }
        // The frame after the wait: whatever arrived during it is dropped unread.
        let frame = camera.next_after(camera.seq(), frame_timeout)?;
        let arrived = ms_since(pressed, frame.arrived);

        let detections = worker.detect(&frame.pixels)?;
        let pivot = to_metres(homography, detections[PIVOT_CLASS]);
        let end = to_metres(homography, detections[END_CLASS]);
        pool.push(frame.stamp_ns, pivot, end);

        let inputs = Inputs::build(&pool, &history, values.age_unit_ms);
        let a = worker.act(inputs.values())?;
        let command = Command::from_action(a, values.top_pulse_rate);
        link.send(command)?;
        let sent = ms_since(pressed, Instant::now());
        history.push(a);

        log.write(&FrameLine {
            boxes: detections.map(Detection::as_array),
            points: [pivot.as_array(), end.as_array()],
            inputs: inputs.values(),
            stamp: frame.stamp_ns,
            a,
            c: command.rate,
            arrived,
            sent,
        })?;
        if let Ok(mut shared) = brain.shared.lock() {
            shared.frames += 1;
        }

        for reply in link.replies() {
            if let Some(reason) = reply.ends_run() {
                return Ok(reason);
            }
        }
        if !delay.is_zero() {
            thread::sleep(delay);
        }
    }
}

/// Whether `idle` has been asked for. A poisoned lock stops the run too.
fn stop_asked(brain: &Brain) -> bool {
    brain
        .shared
        .lock()
        .map(|shared| shared.stop || shared.state != RunState::Run)
        .unwrap_or(true)
}

/// A detection's centre, normalised, to metres in the pole's plane: scaled to
/// the frame's pixels, then through the lens and the matrix. A marker not
/// found stays (−1, −1): there is no pixel to map.
fn to_metres(homography: &Homography, detection: Detection) -> Point {
    if !detection.found() {
        return Point::NOT_DETECTED;
    }
    homography.to_metres(
        detection.x * FRAME_WIDTH as f64,
        detection.y * FRAME_HEIGHT as f64,
    )
}

/// Milliseconds from `start` to `moment`, on the monotonic clock.
fn ms_since(start: Instant, moment: Instant) -> f64 {
    moment.saturating_duration_since(start).as_nanos() as f64 / NANOS_PER_MILLI
}
