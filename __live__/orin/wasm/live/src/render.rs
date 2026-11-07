use crate::{
    constants::MARKERS,
    live_status::{LiveStatus, marker_box::MarkerBox},
};

use std::fmt::Write;

/// Every found marker's box over the frame, with its name above it. Only this
/// crate's own names and numbers are written here, so nothing needs escaping.
pub(crate) fn boxes(status: &LiveStatus) -> Result<String, std::fmt::Error> {
    let mut out = String::new();
    write!(
        out,
        "<svg viewBox=\"0 0 {} {}\" role=\"img\" aria-label=\"The boxes the detector found\">",
        status.width, status.height
    )?;
    for (name, class) in MARKERS {
        if let Some(marker) = found_box(status, name) {
            write!(
                out,
                "<rect class=\"{class}\" x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\"/>\
                 <text class=\"{class}\" x=\"{:.1}\" y=\"{:.1}\">{name}</text>",
                marker.cx - marker.w / 2.0,
                marker.cy - marker.h / 2.0,
                marker.w,
                marker.h,
                marker.cx - marker.w / 2.0,
                marker.cy - marker.h / 2.0 - 4.0,
            )?;
        }
    }
    write!(out, "</svg>")?;
    Ok(out)
}

/// One row per marker: how often it was found lately, and where it is now.
pub(crate) fn rows(status: &LiveStatus) -> Result<String, std::fmt::Error> {
    let mut out = String::new();
    for (name, class) in MARKERS {
        let found = status.found.get(name).copied().unwrap_or(0);
        let tone = if found == status.window {
            "good"
        } else {
            "bad"
        };
        let place = found_box(status, name).map_or_else(
            || "not in this frame".to_string(),
            |marker| format!("{:.1}, {:.1}", marker.cx, marker.cy),
        );
        write!(
            out,
            "<tr><td><span class=\"swatch {class}\"></span>{name}</td>\
             <td class=\"{tone}\">{found} of {}</td><td>{place}</td></tr>",
            status.window
        )?;
    }
    Ok(out)
}

/// The box found for the marker `name` in the latest frame, if any.
fn found_box(status: &LiveStatus, name: &str) -> Option<MarkerBox> {
    status.boxes.as_ref()?.get(name).copied().flatten()
}
