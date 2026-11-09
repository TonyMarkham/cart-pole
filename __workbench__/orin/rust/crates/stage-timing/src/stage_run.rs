use crate::{
    camera::Camera,
    command::Command,
    constants::{END_CLASS, FRAME_HEIGHT, FRAME_WIDTH, PIVOT_CLASS},
    geometry::{homography::Homography, point::Point},
    history::History,
    home::expand_home,
    inputs::Inputs,
    pool::Pool,
    report::Report,
    run_folder,
    sender::Sender,
    settings::stage_settings::StageSettings,
    stage_record::StageRecord,
    worker::{Worker, detection::Detection},
};

use anyhow::{Context, Result};
use std::{
    fs,
    path::Path,
    thread,
    time::{Duration, Instant},
};

/// Each run's folder is named `stage_<seconds since 1970>`.
const TEST_NAME: &str = "stage";

/// The files each run writes into its folder.
const SUMMARY_FILE: &str = "summary.txt";
const FRAMES_FILE: &str = "frames.csv";

/// What the screen says as the run goes.
const SAY_FOLDER: &str = "run folder:";
const SAY_WORKER: &str = "worker ready: policy";
const SAY_PROGRESS: &str = "frames";

/// The run: the brain's loop (its `run/run_loop.rs`, `passes`), pass for
/// pass, with every stage's end stamped on the Orin's monotonic clock:
///
/// 1. the camera's next frame after the last pass: `arrived`, when the
///    reader had it whole;
/// 2. the frame into the worker's pipe: `frame_sent`;
/// 3. the boxes back: `boxes`;
/// 4. Pivot and End to metres, the pool, the 36 inputs: `inputs`;
/// 5. a back from the worker: `answered`;
/// 6. the command's datagram handed to the system: `sent`.
///
/// What the brain does after `sent`, its log line and reading the board's
/// replies, is left out: it is outside gaps 2–6. Every time is ns since the
/// camera settled.
pub(crate) fn run(settings: &StageSettings, settings_path: &Path) -> Result<()> {
    let folder = run_folder::create(settings_path, &settings.test.output_dir, TEST_NAME)?;
    println!("{SAY_FOLDER} {}", folder.display());
    let timeout = Duration::from_millis(settings.run.frame_timeout_ms);
    let delay = Duration::from_millis(settings.run.delay_ms);

    let homography = Homography::load(&expand_home(&settings.worker.homography))?;
    let mut worker = Worker::start(&settings.worker)?;
    let values = worker.values.clone();
    println!(
        "{SAY_WORKER} {}, top rate {} pulses/s",
        values.source, values.top_pulse_rate
    );
    let sender = Sender::connect(&settings.send)?;
    let camera = Camera::open(&settings.camera)?;
    camera.settle(settings.camera.settle_frames, timeout)?;

    let start = Instant::now();
    let since = |moment: Instant| moment.saturating_duration_since(start).as_nanos() as f64;
    let mut pool = Pool::pre_warmed(settings.run.pre_warm_gap_ms);
    let mut history = History::new(values.past_commands);
    let mut records = Vec::with_capacity(settings.test.frames as usize);

    for n in 1..=settings.test.frames {
        let frame = camera.next_after(camera.seq(), timeout)?;
        worker.send_frame(&frame.pixels)?;
        let frame_sent = Instant::now();
        let detections = worker.read_detections()?;
        let boxes = Instant::now();
        let pivot = to_metres(&homography, detections[PIVOT_CLASS]);
        let end = to_metres(&homography, detections[END_CLASS]);
        pool.push(frame.stamp_ns, pivot, end);
        let inputs = Inputs::build(&pool, &history, values.age_unit_ms);
        let inputs_built = Instant::now();
        let a = worker.act(inputs.values())?;
        let answered = Instant::now();
        let command = Command::from_action(a, values.top_pulse_rate);
        sender.send(command)?;
        let sent = Instant::now();
        history.push(a);

        records.push(StageRecord {
            seq: frame.seq,
            stamp_ns: frame.stamp_ns,
            arrived_ns: since(frame.arrived),
            frame_sent_ns: since(frame_sent),
            boxes_ns: since(boxes),
            inputs_ns: since(inputs_built),
            answered_ns: since(answered),
            sent_ns: since(sent),
            found: detections
                .iter()
                .filter(|detection| detection.found())
                .count(),
            a,
            c: command.rate,
        });
        let every = settings.test.progress_every;
        if every > 0 && n % every == 0 {
            println!("{n} {SAY_PROGRESS}");
        }
        if !delay.is_zero() {
            thread::sleep(delay);
        }
    }
    drop(camera);
    drop(worker);

    let report = Report { records };
    let summary = report.summary();
    print!("{summary}");
    fs::write(folder.join(SUMMARY_FILE), &summary)
        .with_context(|| format!("writing {SUMMARY_FILE}"))?;
    fs::write(folder.join(FRAMES_FILE), report.frames_csv())
        .with_context(|| format!("writing {FRAMES_FILE}"))?;
    Ok(())
}

/// A detection's centre, normalised, to metres in the pole's plane, as the
/// brain's `to_metres` does: a marker not found stays (−1, −1).
fn to_metres(homography: &Homography, detection: Detection) -> Point {
    if !detection.found() {
        return Point::NOT_DETECTED;
    }
    homography.to_metres(
        detection.x * FRAME_WIDTH as f64,
        detection.y * FRAME_HEIGHT as f64,
    )
}
