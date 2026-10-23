//! Does the crate compute what Ultralytics' own predictor computes on the
//! same model and frames? (detector-plan.md, Done-when 2.)
//!
//! Reads a reference CSV written by the training env's predictor run (one
//! row per frame: `frame`, then per class `<Name>_conf` — empty when not
//! detected — and `<Name>_xy`, the normalized center "x y"), runs the
//! crate on each frame, and checks every slot: the same detected /
//! not-detected decision, centers within 1 px, confidences within 0.01.
//! A decision may differ when the confidence lies within 0.01 of the
//! threshold (the reference's, or the crate's when the reference has
//! none); such slots are listed, not failed.
//!
//! usage: cargo run --release -p object-detection --example reference_check --
//!        <ObjectDetection.toml> <reference.csv> <frames_dir>

use std::{path::PathBuf, time::Instant};

use anyhow::{Context, Result, anyhow, bail};
use object_detection::{Detector, DetectorConfig, SLOTS};

// Done-when 2's tolerances.
const CENTER_PX: f64 = 1.0;
const CONFIDENCE: f32 = 0.01;

// The loop's frame contract (loop-plan.md).
const WIDTH: usize = 728;
const HEIGHT: usize = 544;

// One class's reference: confidence and normalized center, or None when
// the reference did not detect it.
type Reference = Option<(f32, f64, f64)>;

fn main() -> Result<()> {
    let args: Vec<PathBuf> = std::env::args_os().skip(1).map(PathBuf::from).collect();
    let [config, csv, frames] = args.as_slice() else {
        bail!("usage: reference_check <ObjectDetection.toml> <reference.csv> <frames_dir>");
    };
    let detector = Detector::load(DetectorConfig::load(config)?)?;
    let threshold = detector.config().confidence;
    let classes = detector.config().classes.clone();

    let text = std::fs::read_to_string(csv).with_context(|| format!("reading {}", csv.display()))?;
    let mut lines = text.lines();
    let header: Vec<&str> = lines.next().ok_or_else(|| anyhow!("{} is empty", csv.display()))?.split(',').collect();
    let column = |name: &str| {
        header
            .iter()
            .position(|h| *h == name)
            .ok_or_else(|| anyhow!("{} has no column {name}", csv.display()))
    };
    let frame_column = column("frame")?;
    let class_columns = classes
        .iter()
        .map(|name| Ok((column(&format!("{name}_conf"))?, column(&format!("{name}_xy"))?)))
        .collect::<Result<Vec<_>>>()?;

    let (mut frames_seen, mut failures, mut borderline) = (0usize, 0usize, 0usize);
    let (mut max_dx, mut max_dy, mut max_dconf) = (0f64, 0f64, 0f32);
    let mut detect_seconds = 0f64;
    for (row_index, line) in lines.enumerate() {
        let fields: Vec<&str> = line.split(',').collect();
        let field = |i: usize| fields.get(i).copied().ok_or_else(|| anyhow!("row {}: too few fields", row_index + 1));
        let name = field(frame_column)?;
        let path = frames.join(name);
        let jpeg = std::fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
        let started = Instant::now();
        let marker_frame = detector.detect(&jpeg, WIDTH, HEIGHT).with_context(|| name.to_string())?;
        detect_seconds += started.elapsed().as_secs_f64();
        frames_seen += 1;

        for (slot, &(conf_column, xy_column)) in class_columns.iter().enumerate().take(SLOTS) {
            let reference = parse_reference(field(conf_column)?, field(xy_column)?)
                .with_context(|| format!("{name} {}", classes[slot]))?;
            let ours = marker_frame.confidences[slot];
            let label = format!("{name} {:5}", classes[slot]);
            match (reference, marker_frame.detected(slot)) {
                (Some((conf, x, y)), true) => {
                    let (ox, oy) = marker_frame.slots[slot];
                    let dx = (f64::from(ox) - x).abs() * WIDTH as f64;
                    let dy = (f64::from(oy) - y).abs() * HEIGHT as f64;
                    let dconf = (ours - conf).abs();
                    (max_dx, max_dy, max_dconf) = (max_dx.max(dx), max_dy.max(dy), max_dconf.max(dconf));
                    if dx > CENTER_PX || dy > CENTER_PX || dconf > CONFIDENCE {
                        failures += 1;
                        println!(
                            "FAIL {label}: center ({ox:.4}, {oy:.4}) vs ({x:.4}, {y:.4}) = {dx:.2}, {dy:.2} px; conf {ours:.3} vs {conf:.3}"
                        );
                    }
                }
                (None, false) => {}
                (reference, _) => {
                    // The decisions differ: allowed only near the threshold.
                    let near = reference.map_or(ours, |(conf, _, _)| conf);
                    let what = match reference {
                        Some((conf, _, _)) => format!("reference detected at {conf:.3}, crate not (conf {ours:.3})"),
                        None => format!("crate detected at {ours:.3}, reference not"),
                    };
                    if (near - threshold).abs() <= CONFIDENCE {
                        borderline += 1;
                        println!("near threshold {label}: {what}");
                    } else {
                        failures += 1;
                        println!("FAIL {label}: {what}");
                    }
                }
            }
        }
    }

    println!(
        "{}: {frames_seen} frames, {} slots; max center difference {max_dx:.3} px x, {max_dy:.3} px y; max confidence difference {max_dconf:.4}; {borderline} near threshold; mean detect {:.0} ms",
        csv.display(),
        frames_seen * SLOTS,
        detect_seconds * 1000.0 / frames_seen.max(1) as f64
    );
    if failures > 0 {
        bail!("{failures} slot(s) outside the tolerances");
    }
    println!("all slots within tolerance");
    Ok(())
}

// "0.956" and "0.2359 0.4759", or "" and "-1 -1" when not detected.
fn parse_reference(conf: &str, xy: &str) -> Result<Reference> {
    if conf.is_empty() {
        return Ok(None);
    }
    let conf: f32 = conf.parse().with_context(|| format!("confidence {conf:?}"))?;
    let mut parts = xy.split(' ').map(str::parse::<f64>);
    match (parts.next(), parts.next(), parts.next()) {
        (Some(Ok(x)), Some(Ok(y)), None) => Ok(Some((conf, x, y))),
        _ => bail!("center {xy:?} is not two numbers"),
    }
}
