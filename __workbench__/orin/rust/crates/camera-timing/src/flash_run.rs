use crate::{
    calibration::Calibration,
    camera::{Camera, frame::Frame},
    constants::{NANOS_PER_MICRO, NANOS_PER_SECOND},
    flash::Flash,
    frame_record::FrameRecord,
    led::Led,
    random::Random,
    report::Report,
    run_folder,
    settings::timing_settings::TimingSettings,
};

use anyhow::{Result, bail};
use std::{
    fs,
    path::Path,
    thread,
    time::{Duration, Instant},
};

/// The test's name: its run folders are `camera_<seconds>`.
const TEST_NAME: &str = "camera";

/// The files written into each run's folder.
const SUMMARY_FILE: &str = "summary.txt";
const FLASHES_FILE: &str = "flashes.csv";
const FRAMES_FILE: &str = "frames.csv";

/// The random delay from a frame's arrival to its flash, ns: from the frame's
/// arrival itself, 0, up to the last whole ns before the next frame is due,
/// one ns short of the period, so every flash lands within one frame period.
const DELAY_FIRST_NS: i64 = 0;
const ONE_NS_SHORT: i64 = 1;

/// The labels of the lines printed as the run goes.
const SAY_FOLDER: &str = "run folder:";
const SAY_LED_FOUND: &str = "LED found:";
const SAY_FLASHES: &str = "flashes,";
const SAY_SEEN: &str = "seen";

/// The run: the camera settles, the calibration finds the LED, then each
/// flash is placed at a random moment within one frame period after a
/// frame's arrival, lasts `pulse_us`, and is followed by
/// `pulse_spacing_frames` frames, each read and checked for the LED. Every
/// time is read on one clock, the Orin's monotonic clock (`Instant`), as ns
/// since the run started.
pub(crate) fn run(settings: &TimingSettings, settings_path: &Path) -> Result<()> {
    let test = &settings.test;
    if settings.camera.framerate == 0 || test.pulse_spacing_frames == 0 {
        bail!("[camera] framerate and [test] pulse_spacing_frames must be above 0");
    }
    let folder = run_folder::create(settings_path, &test.output_dir, TEST_NAME)?;
    println!("{SAY_FOLDER} {}", folder.display());
    let timeout = Duration::from_millis(test.frame_timeout_ms);
    let period_ns = NANOS_PER_SECOND / u64::from(settings.camera.framerate);

    let led = Led::open(&settings.led.line_name)?;
    let camera = Camera::open(&settings.camera)?;
    let start = Instant::now();
    let since = |moment: Instant| moment.duration_since(start).as_nanos() as f64;

    for _ in 0..settings.camera.settle_frames {
        camera.next(timeout)?;
    }
    let calibration = Calibration::take(
        &camera,
        &led,
        test.calibration_frames,
        test.patch_half,
        test.min_contrast,
        timeout,
    )?;
    let threshold = calibration.threshold();
    println!(
        "{SAY_LED_FOUND} columns {}..{}, rows {}..{}; dark {:.1}, lit {:.1}, threshold {threshold:.1}",
        calibration.patch.left,
        calibration.patch.right,
        calibration.patch.top,
        calibration.patch.bottom,
        calibration.off_level,
        calibration.on_level
    );

    let record = |frame: &Frame| FrameRecord {
        seq: frame.seq,
        arrived_ns: since(frame.arrived),
        stamp_ns: frame.stamp_ns,
        level: calibration.patch.level(&frame.pixels),
    };
    let mut random = Random::new(test.seed);
    let mut frames = Vec::new();
    let mut flashes = Vec::new();
    for n in 0..test.pulses {
        let anchor = camera.next(timeout)?;
        frames.push(record(&anchor));
        let delay = random.between(
            DELAY_FIRST_NS,
            i64::try_from(period_ns).unwrap_or(i64::MAX) - ONE_NS_SHORT,
        );
        let target = anchor.arrived + Duration::from_nanos(delay.unsigned_abs());
        if let Some(wait) = target.checked_duration_since(Instant::now()) {
            thread::sleep(wait);
        }

        let on_before = Instant::now();
        led.on()?;
        let on_after = Instant::now();
        let lit_until = on_after + Duration::from_nanos(test.pulse_us * NANOS_PER_MICRO);
        // A spin, not a sleep: a sleep this short overshoots by tens of µs.
        while Instant::now() < lit_until {
            std::hint::spin_loop();
        }
        let off_before = Instant::now();
        led.off()?;
        let off_after = Instant::now();

        let mut flash = Flash {
            on_before_ns: since(on_before),
            on_after_ns: since(on_after),
            off_before_ns: since(off_before),
            off_after_ns: since(off_after),
            caught_by: None,
            lit_frames: 0,
        };
        for _ in 0..test.pulse_spacing_frames {
            let frame = camera.next(timeout)?;
            let seen = record(&frame);
            frames.push(seen);
            // Frames that arrived before the LED lit can't show it.
            if frame.arrived >= on_before && seen.level > threshold {
                flash.lit_frames += 1;
                if flash.caught_by.is_none() {
                    flash.caught_by = Some(seen);
                }
            }
        }
        flashes.push(flash);
        let done = n + 1;
        if test.progress_every > 0 && done.is_multiple_of(test.progress_every) {
            let caught = flashes
                .iter()
                .filter(|flash| flash.caught_by.is_some())
                .count();
            println!("{done} {SAY_FLASHES} {caught} {SAY_SEEN}");
        }
    }
    led.off()?;
    drop(camera);

    let report = Report {
        calibration,
        exposure_ns: settings.camera.exposure_ns as f64,
        period_ns: period_ns as f64,
        pulse_ns: (test.pulse_us * NANOS_PER_MICRO) as f64,
        flashes,
        frames,
    };
    let summary = report.summary();
    let files = [
        (SUMMARY_FILE, summary.clone()),
        (FLASHES_FILE, report.flashes_csv()),
        (FRAMES_FILE, report.frames_csv()),
    ];
    for (name, text) in files {
        let path = folder.join(name);
        fs::write(&path, text)
            .map_err(|error| anyhow::anyhow!("writing {}: {error}", path.display()))?;
    }
    print!("{summary}");
    Ok(())
}
