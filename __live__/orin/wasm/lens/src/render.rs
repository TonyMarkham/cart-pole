use crate::{lens_result::LensResult, lens_status::LensStatus, meter::Meter};

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lens_rect::LensRect;

    fn status(tilt: &str, in_box: bool, turned: bool) -> LensStatus {
        LensStatus {
            on: true,
            message: String::new(),
            per_s: 40.0,
            frame: None,
            markers: vec![vec![[10.0, 20.0], [30.0, 20.0], [30.0, 40.0], [10.0, 40.0]]],
            outline: Some(vec![[5.0, 5.0], [300.0, 5.0], [300.0, 200.0], [5.0, 200.0]]),
            corners: 13,
            all_corners: 15,
            min_corners: 11,
            in_box,
            turn_deg: None,
            ready: in_box && turned,
            width: 1456,
            height: 1088,
            distance_mm: 1000.0,
            tilt_deg: 30.0,
            min_tilt_deg: 20.0,
            rect: Some(LensRect {
                x: 0.0,
                y: 0.0,
                w: 355.0,
                h: 237.0,
                tilt: tilt.to_owned(),
                shape: vec![[29.5, 19.8], [325.5, 19.8], [325.5, 217.2], [29.5, 217.2]],
            }),
            position: 0,
            positions: 45,
            flat_positions: 25,
            held: 3,
            hold_frames: 10,
            kept: 2,
            min_views: 10,
            result: None,
        }
    }

    #[test]
    fn the_box_is_red_until_a_frame_counts() {
        let red = overlay(&status("", false, true)).unwrap_or_default();
        assert!(red.contains("class=\"target\" x=\"0\" y=\"0\" width=\"355\" height=\"237\""));
        assert!(
            red.contains("class=\"shape\" points=\"29.5,19.8 325.5,19.8 325.5,217.2 29.5,217.2\"")
        );
        let green = overlay(&status("", true, true)).unwrap_or_default();
        assert!(green.contains("class=\"target held\""));
        assert!(green.contains("class=\"board\""));
        assert!(green.contains("points=\"10.0,20.0 30.0,20.0 30.0,40.0 10.0,40.0\""));
    }

    #[test]
    fn the_line_says_what_to_do() {
        assert!(
            line(&status("", false, true))
                .starts_with("Flat position 1 of 25: hold the board flat")
        );
        let mut turned = status("left", false, false);
        turned.position = 25;
        assert!(line(&turned).starts_with(
            "Turned position 1 of 20: hold the board with its left side turned about 30 degrees away"
        ));
        assert!(
            line(&status("left", true, false))
                .starts_with("Inside the box: now turn the board's left side")
        );
        assert!(line(&status("", true, true)).starts_with("Hold it still: 3 of 10"));
    }

    #[test]
    fn the_turn_bar_fills_at_the_aim_and_counts_from_the_least() {
        assert_eq!(turn(&status("", true, true)), None);
        let mut turned = status("left", true, false);
        turned.turn_deg = Some(15.0);
        let short = turn(&turned);
        assert_eq!(short.as_ref().map(|meter| meter.fraction), Some(0.5));
        assert_eq!(short.as_ref().map(|meter| meter.good), Some(false));
        assert_eq!(short.and_then(|meter| meter.mark), Some(20.0 / 30.0));
        turned.turn_deg = Some(24.0);
        assert_eq!(turn(&turned).map(|meter| meter.good), Some(true));
        turned.turn_deg = Some(-12.0);
        assert!(
            turn(&turned).is_some_and(|meter| meter.label.contains("12 degrees the wrong way"))
        );
    }

    #[test]
    fn the_progress_bar_counts_the_positions_behind() {
        let mut status = status("left", false, false);
        status.position = 27;
        let meter = progress(&status);
        assert_eq!(meter.fraction, 27.0 / 45.0);
        assert!(
            meter
                .label
                .starts_with("27 of 45 positions done, 2 stills kept")
        );
        assert_eq!(hold(&status).fraction, 0.3);
    }
}
