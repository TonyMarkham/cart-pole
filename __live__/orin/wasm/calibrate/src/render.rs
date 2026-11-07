use crate::{
    constants::{CART_AT_CENTRE_X, CART_AT_IDLER_X, CART_AT_MOTOR_X, CART_WIDTH, MARKERS},
    measure::{
        fit_report::FitReport, frame_boxes::FrameBoxes, live_status::LiveStatus,
        marker_box::MarkerBox, marker_summary::MarkerSummary, measure_summary::MeasureSummary,
    },
    number::grouped,
    session::{Session, entry::Entry, place::Place},
    tone::Tone,
};

use std::fmt::Write;

/// The rail as SVG, as the camera sees it: the idler on the left and the motor on
/// the right (measured on the rig, 2026-11-01). Nothing from outside this crate is written here, so none of it needs
/// escaping. The drawing's only moving part is the belt, because the cart's
/// position during a move is not known.
pub(crate) fn rail(session: &Session) -> Result<String, std::fmt::Error> {
    let place = session.place();
    let mut out = String::new();
    write!(
        out,
        "<svg viewBox=\"0 76 860 156\" role=\"img\" aria-label=\"Drawing of the rail as the \
         camera sees it, with the idler pulley on the left and the motor on the right. The \
         sentence below says the same.\">\
         <rect class=\"steel\" x=\"50\" y=\"136\" width=\"750\" height=\"8\"/>"
    )?;
    let heading = session.heading().map_or("", |heading| heading.class());
    write!(
        out,
        "<g class=\"{heading}\">\
         <line class=\"belt top\" x1=\"50\" y1=\"118\" x2=\"800\" y2=\"118\"/>\
         <line class=\"belt bottom\" x1=\"50\" y1=\"162\" x2=\"800\" y2=\"162\"/>\
         </g>\
         <circle class=\"steel\" cx=\"50\" cy=\"140\" r=\"22\"/>\
         <text class=\"block-label\" x=\"50\" y=\"198\">Idler</text>\
         <rect class=\"steel\" x=\"780\" y=\"104\" width=\"60\" height=\"72\" rx=\"6\"/>\
         <text class=\"block-label\" x=\"810\" y=\"198\">Motor</text>"
    )?;
    let (motor, idler) = match session.switches() {
        Some(switches) => (
            switch_state(switches.motor_pressed(), place),
            switch_state(switches.idler_pressed(), place),
        ),
        None => (("no reading", Tone::None), ("no reading", Tone::None)),
    };
    switch(&mut out, "D3", idler, 92, 92, "start")?;
    switch(&mut out, "D2", motor, 752, 768, "end")?;
    cart(&mut out, place)?;
    write!(out, "</svg>")?;
    Ok(out)
}

/// A switch's word and colour. Pressed is expected at an end, so only both
/// pressed at once is a warning.
fn switch_state(pressed: bool, place: Place) -> (&'static str, Tone) {
    match (pressed, place) {
        (true, Place::BothPressed) => ("pressed", Tone::Warn),
        (true, _) => ("pressed", Tone::Info),
        (false, _) => ("released", Tone::Ok),
    }
}

/// One switch on the drawing, with its name and what it reads.
fn switch(
    out: &mut String,
    name: &str,
    (state, tone): (&str, Tone),
    x: i32,
    label_x: i32,
    anchor: &str,
) -> std::fmt::Result {
    let class = tone.class();
    write!(
        out,
        "<rect class=\"{class}\" x=\"{x}\" y=\"122\" width=\"16\" height=\"14\"/>\
         <text class=\"switch-label {class}\" x=\"{label_x}\" y=\"224\" \
         text-anchor=\"{anchor}\">{name}: {state}</text>"
    )
}

/// The cart at an end or the centre when the page knows it is there. Otherwise
/// it is drawn unsure in the middle, labelled `Where?` or `Moving`.
fn cart(out: &mut String, place: Place) -> std::fmt::Result {
    let (x, label, sure) = match place {
        Place::Motor => (CART_AT_MOTOR_X, "Cart", true),
        Place::Idler => (CART_AT_IDLER_X, "Cart", true),
        Place::Centre => (CART_AT_CENTRE_X, "Cart", true),
        Place::Unknown | Place::BothPressed => (CART_AT_CENTRE_X, "Where?", false),
        Place::Moving => (CART_AT_CENTRE_X, "Moving", false),
    };
    let (box_class, text_class) = if sure {
        ("cart", "cart-text")
    } else {
        ("cart-unsure", "cart-unsure-text")
    };
    write!(
        out,
        "<rect class=\"{box_class}\" x=\"{x}\" y=\"88\" width=\"{CART_WIDTH}\" height=\"48\" \
         rx=\"6\"/>\
         <text class=\"{text_class}\" x=\"{}\" y=\"117\">{label}</text>",
        x + CART_WIDTH / 2,
    )
}

/// The table's rows, newest first. What the board or the webserver said is
/// escaped, because it comes from outside this crate.
pub(crate) fn rows(entries: &[Entry]) -> Result<String, std::fmt::Error> {
    let mut out = String::new();
    if entries.is_empty() {
        write!(out, "<tr><td colspan=\"8\">No moves yet.</td></tr>")?;
    }
    for entry in entries.iter().rev() {
        let answer = &entry.answer;
        let class = if answer.succeeded() { "good" } else { "bad" };
        write!(
            out,
            "<tr><td>{}</td><td>{}</td><td class=\"{class}\">{}</td><td class=\"{class}\">{}</td>\
             <td>{}</td><td>{}</td><td>{:.1} s</td><td>{}</td></tr>",
            escaped(&entry.clock),
            entry.travel.name(),
            answer.status,
            escaped(&answer.result),
            escaped(answer.pin.as_deref().unwrap_or("-")),
            grouped(answer.pulses),
            entry.took_s,
            escaped(&answer.message),
        )?;
    }
    Ok(out)
}

/// The slide show's boxes, drawn over the frame: Pivot's in green and End's in blue,
/// the colours coming from `index.html`. Only numbers are written here.
pub(crate) fn boxes(
    boxes: &FrameBoxes,
    width: u32,
    height: u32,
) -> Result<String, std::fmt::Error> {
    let mut out = String::new();
    write!(
        out,
        "<svg viewBox=\"0 0 {width} {height}\" role=\"img\" \
         aria-label=\"The boxes the detector found\">"
    )?;
    if let Some(marker) = boxes.pivot {
        one_box(&mut out, "box-pivot", marker)?;
    }
    if let Some(marker) = boxes.end {
        one_box(&mut out, "box-end", marker)?;
    }
    write!(out, "</svg>")?;
    Ok(out)
}

/// One marker's box, centred on the detector's point.
fn one_box(out: &mut String, class: &str, marker: MarkerBox) -> std::fmt::Result {
    write!(
        out,
        "<rect class=\"{class}\" x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\"/>",
        marker.cx - marker.w / 2.0,
        marker.cy - marker.h / 2.0,
        marker.w,
        marker.h
    )
}

/// The live view's boxes: every found marker, named above its box, so the person can
/// see the whole rig is in the picture. Only this crate's names and numbers are
/// written here.
pub(crate) fn live_boxes(status: &LiveStatus) -> Result<String, std::fmt::Error> {
    let mut out = String::new();
    write!(
        out,
        "<svg viewBox=\"0 0 {} {}\" role=\"img\" \
         aria-label=\"The boxes the detector found\">",
        status.width, status.height
    )?;
    for (name, class) in MARKERS {
        if let Some(marker) = live_box(status, name) {
            one_box(&mut out, class, marker)?;
            write!(
                out,
                "<text class=\"{class}\" x=\"{:.1}\" y=\"{:.1}\">{name}</text>",
                marker.cx - marker.w / 2.0,
                marker.cy - marker.h / 2.0 - 4.0
            )?;
        }
    }
    write!(out, "</svg>")?;
    Ok(out)
}

/// The line under the live view: how often each marker was found lately.
pub(crate) fn live_line(status: &LiveStatus) -> String {
    let found: Vec<String> = MARKERS
        .iter()
        .map(|(name, _)| format!("{name} {}", status.found.get(*name).copied().unwrap_or(0)))
        .collect();
    format!(
        "Live, {:.0} frames a second. Found in the last {} frames: {}.",
        status.per_s,
        status.window,
        found.join(", ")
    )
}

/// The box the live view found for the marker `name`, if any.
fn live_box(status: &LiveStatus, name: &str) -> Option<MarkerBox> {
    status.boxes.as_ref()?.get(name).copied().flatten()
}

/// One pose's result. Only numbers are written here.
pub(crate) fn summary(summary: &MeasureSummary) -> Result<String, std::fmt::Error> {
    let mut out = String::new();
    write!(
        out,
        "<p>{} images counted, of {} looked at, in {:.1} s.{}</p>\
         <div class=\"table-wrap\"><table class=\"small\"><thead><tr><th>Marker</th>\
         <th>Across (px from the left edge)</th><th>Down (px from the top edge)</th>\
         <th>Spread (px)</th><th>Dropped</th></tr></thead>\
         <tbody>",
        summary.images,
        summary.seen,
        summary.took_s,
        summary
            .gap_mm
            .map_or(String::new(), |gap| format!(" Gap: {gap} mm.")),
    )?;
    marker_row(&mut out, "Pivot", &summary.pivot)?;
    marker_row(&mut out, "End", &summary.end)?;
    write!(out, "</tbody></table></div>")?;
    Ok(out)
}

/// One marker's row in a stop's summary.
fn marker_row(out: &mut String, name: &str, marker: &MarkerSummary) -> std::fmt::Result {
    write!(
        out,
        "<tr><td>{name}</td><td>{:.2}</td><td>{:.2}</td><td>{:.3}</td><td>{} of {}</td></tr>",
        marker.u, marker.v, marker.spread, marker.dropped, marker.count
    )
}

/// The fit. What came from outside this crate is escaped.
pub(crate) fn fit(report: &FitReport) -> Result<String, std::fmt::Error> {
    let mut out = String::new();
    // Over the limit is not a failure: whether the matrix is good enough is settled
    // by the robot balancing with it (user, 2026-11-02), so the person can accept it.
    let (class, words) = if report.within_limit {
        ("good", "Every point is within the limit.")
    } else if report.accepted {
        (
            "good",
            "A point is over the limit, and you accepted this fit, so it is the one to use.",
        )
    } else {
        (
            "bad",
            "A point is over the limit. That is not a failure: whether the matrix is good enough is \
             decided by the robot balancing with it. The fit is stored either way; press Accept this \
             fit to mark it as the one to use, or measure poses again.",
        )
    };
    write!(
        out,
        "<p class=\"{class}\">{words} Its worst point is {:.3} px off, and the limit is {:.3} px.</p>\
         <div class=\"table-wrap\"><table class=\"small\"><thead><tr><th>Point</th>\
         <th>Off by (px)</th></tr></thead><tbody>",
        report.worst_px, report.limit_px
    )?;
    for residual in &report.residuals {
        write!(
            out,
            "<tr><td>{}</td><td>{:.3}</td></tr>",
            escaped(&residual.point),
            residual.px
        )?;
    }
    write!(
        out,
        "</tbody></table></div><p>Span: {} pulses, {:.1} mm, which is what was used. The CAD and the \
         gaps say {:.1} mm, {:+.1} mm from that.</p>",
        grouped(report.span_pulses),
        report.span_m * 1_000.0,
        report.cad_length_m * 1_000.0,
        report.difference_m * -1_000.0
    )?;
    if let (Some(motor), Some(idler)) = (report.gaps_mm.get("motor"), report.gaps_mm.get("idler")) {
        write!(
            out,
            "<p>Gaps you measured: {motor} mm at the motor's end, {idler} mm at the idler's end.</p>"
        )?;
    }
    write!(
        out,
        "<p>Lens: every point's bend was undone first with the lens calibration of {} \
         (average error {:.2} px, from the /lens page). Without it the worst point would be \
         {:.3} px off.</p>",
        escaped(&report.lens.date),
        report.lens.rms_px,
        report.worst_uncorrected_px
    )?;
    write!(out, "<p>The poses were measured:")?;
    for (pose, minutes) in &report.kept_minutes_ago {
        write!(out, " {} {minutes} min ago.", escaped(pose))?;
    }
    write!(out, "</p>")?;
    if let Some(saved_to) = &report.saved_to {
        write!(out, "<p>Stored in <code>{}</code>", escaped(saved_to))?;
        if let Some(dated) = &report.dated_copy {
            write!(
                out,
                ", and kept for good in <code>{}</code>",
                escaped(dated)
            )?;
        }
        write!(out, ".</p>")?;
    }
    Ok(out)
}

/// Text from outside this crate, made safe to write into the page's HTML.
fn escaped(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
