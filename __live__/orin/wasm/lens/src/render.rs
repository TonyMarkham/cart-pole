use crate::{
    bar::meter::Meter,
    lens_status::{LensStatus, lens_result::LensResult},
};

use std::fmt::Write;

/// The box to hold the board in, red until a frame counts and green then; the
/// board's shape to match, dashed, inside it; the board's edge where the camera
/// finds it, orange; and the outline of every marker found, yellow. Only numbers
/// are written here.
pub(crate) fn overlay(status: &LensStatus) -> Result<String, std::fmt::Error> {
    let mut out = String::new();
    write!(
        out,
        "<svg viewBox=\"0 0 {} {}\" role=\"img\" aria-label=\"Where to hold the board\">",
        status.width, status.height
    )?;
    if let Some(rect) = &status.rect {
        let class = if status.ready {
            "target held"
        } else {
            "target"
        };
        write!(
            out,
            "<rect class=\"{class}\" x=\"{:.0}\" y=\"{:.0}\" width=\"{:.0}\" height=\"{:.0}\"/>",
            rect.x, rect.y, rect.w, rect.h
        )?;
        polygon(&mut out, "shape", &rect.shape)?;
    }
    if let Some(edge) = &status.outline {
        polygon(&mut out, "board", edge)?;
    }
    for marker in &status.markers {
        polygon(&mut out, "marker", marker)?;
    }
    write!(out, "</svg>")?;
    Ok(out)
}

/// One outline through `corners`, in full-sensor pixels.
fn polygon(out: &mut String, class: &str, corners: &[[f64; 2]]) -> std::fmt::Result {
    let points: Vec<String> = corners
        .iter()
        .map(|[u, v]| format!("{u:.1},{v:.1}"))
        .collect();
    write!(
        out,
        "<polygon class=\"{class}\" points=\"{}\"/>",
        points.join(" ")
    )
}

/// The line above the picture: what to do now.
pub(crate) fn line(status: &LensStatus) -> String {
    if !status.on {
        let why = if status.message.is_empty() {
            String::new()
        } else {
            format!(" ({})", status.message)
        };
        return format!(
            "Off{why}. {} stills kept. Press Start to carry on.",
            status.kept
        );
    }
    let what = match &status.rect {
        None => format!(
            "Every position is done. Press Solve (it needs at least {}).",
            status.min_views
        ),
        Some(_) if status.ready => format!(
            "Hold it still: {} of {} frames.",
            status.held, status.hold_frames
        ),
        Some(rect) if status.in_box => format!(
            "Inside the box: now turn the board's {} side farther from the camera, to at least {:.0} degrees.",
            rect.tilt, status.min_tilt_deg
        ),
        Some(rect) => {
            // The flat positions come first, then the turned ones, each counted
            // from 1 on its own.
            let flat = status.flat_positions;
            let (place, how) = if rect.tilt.is_empty() {
                (
                    format!("Flat position {} of {flat}", status.position + 1),
                    "flat, facing the camera,".to_owned(),
                )
            } else {
                (
                    format!(
                        "Turned position {} of {}",
                        status.position.saturating_sub(flat) + 1,
                        status.positions.saturating_sub(flat)
                    ),
                    format!(
                        "with its {} side turned about {:.0} degrees away from the camera,",
                        rect.tilt, status.tilt_deg
                    ),
                )
            };
            format!(
                "{place}: hold the board {how} about {:.1} m away, over the dashed shape in the red box. {} of {} corners found, {} needed.",
                status.distance_mm / 1000.0,
                status.corners,
                status.all_corners,
                status.min_corners
            )
        }
    };
    format!(
        "{what} {} stills kept, {:.0} frames a second.",
        status.kept, status.per_s
    )
}

/// How many positions are behind: kept or skipped, since the box moves through
/// them in order.
pub(crate) fn progress(status: &LensStatus) -> Meter {
    let done = if status.rect.is_none() {
        status.positions
    } else {
        status.position
    };
    Meter {
        fraction: f64::from(done) / f64::from(status.positions.max(1)),
        label: format!(
            "{done} of {} positions done, {} stills kept ({} are needed to solve)",
            status.positions, status.kept, status.min_views
        ),
        good: status.rect.is_none(),
        mark: None,
    }
}

/// How far the board is turned, full at the aim, with a tick at the least that
/// counts. None when the board is to be held flat or the view is off.
pub(crate) fn turn(status: &LensStatus) -> Option<Meter> {
    let rect = status
        .rect
        .as_ref()
        .filter(|rect| status.on && !rect.tilt.is_empty())?;
    let aim = status.tilt_deg.max(1.0);
    let mark = Some(status.min_tilt_deg / aim);
    Some(match status.turn_deg {
        None => Meter {
            fraction: 0.0,
            label: "Turn: not measured, too little of the board is found".to_owned(),
            good: false,
            mark,
        },
        Some(degrees) if degrees < 0.0 => Meter {
            fraction: 0.0,
            label: format!(
                "Turn: {:.0} degrees the wrong way, the {} side is nearer the camera",
                -degrees, rect.tilt
            ),
            good: false,
            mark,
        },
        Some(degrees) => Meter {
            fraction: degrees / aim,
            label: format!(
                "Turn: {degrees:.0} degrees, {:.0} counts, {:.0} is the aim",
                status.min_tilt_deg, status.tilt_deg
            ),
            good: degrees >= status.min_tilt_deg,
            mark,
        },
    })
}

/// How long the board has been held where it counts.
pub(crate) fn hold(status: &LensStatus) -> Meter {
    Meter {
        fraction: f64::from(status.held) / f64::from(status.hold_frames.max(1)),
        label: format!("Held for {} of {} frames", status.held, status.hold_frames),
        good: status.ready,
        mark: None,
    }
}

/// A solve, in words.
pub(crate) fn result(result: &LensResult) -> String {
    let at = |row: usize, col: usize| {
        result
            .camera_matrix
            .get(row)
            .and_then(|r| r.get(col))
            .copied()
            .unwrap_or(f64::NAN)
    };
    let dist: Vec<String> = result.dist.iter().map(|k| format!("{k:+.4}")).collect();
    let bend = if result.monotonic {
        "The bend grows steadily out to the corners, as a real lens's does."
    } else {
        "The bend does not grow steadily out to the corners: OpenCV counts that as a failed calibration."
    };
    format!(
        "Solved from {} stills: the corners are {:.3} px from where the result puts them, on average. \
         Focal length {:.1} x {:.1} px; lens centre {:.1}, {:.1} px; bend k1, k2, p1, p2, k3: {}. {bend} \
         Stored in {}.",
        result.views,
        result.rms_px,
        at(0, 0),
        at(1, 1),
        at(0, 2),
        at(1, 2),
        dist.join(", "),
        result.saved_to
    )
}
