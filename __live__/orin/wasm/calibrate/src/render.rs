use crate::{
    constants::{CART_AT_CENTRE_X, CART_AT_IDLER_X, CART_AT_MOTOR_X, CART_WIDTH},
    entry::Entry,
    fit_report::FitReport,
    frame_boxes::FrameBoxes,
    marker_box::MarkerBox,
    marker_summary::MarkerSummary,
    measure_summary::MeasureSummary,
    place::Place,
    session::{Session, grouped},
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

fn switch_state(pressed: bool, place: Place) -> (&'static str, Tone) {
    match (pressed, place) {
        (true, Place::BothPressed) => ("pressed", Tone::Warn),
        (true, _) => ("pressed", Tone::Info),
        (false, _) => ("released", Tone::Ok),
    }
}

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

/// One stop's result. Only numbers are written here.
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
    let (class, words) = if report.ok {
        ("good", "The fit passed.")
    } else {
        (
            "bad",
            "The fit failed: nothing was stored. Measure the stops again.",
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
        "<p>Lens correction: {:+.3}, found from these points. Without it the worst point would be \
         {:.3} px off.</p>",
        report.lens_k, report.worst_uncorrected_px
    )?;
    write!(out, "<p>The stops were measured:")?;
    for (stop, minutes) in &report.kept_minutes_ago {
        write!(out, " {} {minutes} min ago.", escaped(stop))?;
    }
    write!(out, "</p>")?;
    if let Some(saved_to) = &report.saved_to {
        write!(out, "<p>Stored in <code>{}</code>.</p>", escaped(saved_to))?;
    }
    Ok(out)
}

fn escaped(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{answer::Answer, reading::Reading, travel::Travel};

    #[test]
    fn the_drawing_says_when_the_cart_is_not_known() {
        let session = Session::new();
        let svg = rail(&session).unwrap_or_default();
        assert!(svg.starts_with("<svg") && svg.ends_with("</svg>"));
        assert!(svg.contains("Where?") && svg.contains("no reading"));
    }

    #[test]
    fn a_failed_reading_leaves_no_switch_state() {
        let mut session = Session::new();
        session.read(Reading::Failed("lost".to_string()), "now".to_string());
        let svg = rail(&session).unwrap_or_default();
        assert!(!svg.contains("pressed") && !svg.contains("released"));
    }

    #[test]
    fn the_idler_is_drawn_left_of_the_motor() {
        let svg = rail(&Session::new()).unwrap_or_default();
        let (idler, motor) = (svg.find(">Idler<"), svg.find(">Motor<"));
        assert!(idler < motor);
        assert!(
            svg.contains("x=\"50\" y=\"198\">Idler") && svg.contains("x=\"810\" y=\"198\">Motor")
        );
    }

    #[test]
    fn a_missed_marker_draws_no_box() {
        let found = MarkerBox {
            cx: 100.0,
            cy: 50.0,
            w: 20.0,
            h: 10.0,
        };
        let frame = FrameBoxes {
            pivot: Some(found),
            end: None,
        };
        let drawn = boxes(&frame, 728, 544).unwrap_or_default();
        assert!(drawn.contains("box-pivot") && !drawn.contains("box-end"));
        assert!(drawn.contains("x=\"90.0\" y=\"45.0\" width=\"20.0\" height=\"10.0\""));
    }

    #[test]
    fn a_failed_fit_says_nothing_was_stored() {
        let report = FitReport {
            ok: false,
            residuals: Vec::new(),
            worst_px: 8.0,
            limit_px: 1.0,
            lens_k: 0.44,
            worst_uncorrected_px: 1.6,
            kept_minutes_ago: [("motor".to_string(), 3)].into(),
            gaps_mm: [("motor".to_string(), 5.0), ("idler".to_string(), 7.5)].into(),
            span_pulses: 59_822,
            span_m: 0.89733,
            cad_length_m: 0.873,
            difference_m: 0.02433,
            saved_to: None,
        };
        let text = fit(&report).unwrap_or_default();
        assert!(text.contains("nothing was stored") && !text.contains("Stored in"));
        assert!(text.contains("897.3 mm") && text.contains("873.0 mm"));
        assert!(text.contains("-24.3 mm"));
        assert!(text.contains("5 mm at the motor") && text.contains("7.5 mm at the idler"));
        assert!(text.contains("+0.440") && text.contains("1.600 px"));
        assert!(text.contains("motor 3 min ago"));
    }

    #[test]
    fn what_came_from_outside_is_escaped() {
        let entry = Entry {
            clock: "now".to_string(),
            travel: Travel::Motor,
            answer: Answer::lost("<b>&\"".to_string()),
            took_s: 1.0,
        };
        let table = rows(&[entry]).unwrap_or_default();
        assert!(table.contains("&lt;b&gt;&amp;&quot;"));
        assert!(!table.contains("<b>"));
    }

    #[test]
    fn the_newest_move_comes_first() {
        let entry = |clock: &str| Entry {
            clock: clock.to_string(),
            travel: Travel::Motor,
            answer: Answer::lost("said".to_string()),
            took_s: 1.0,
        };
        let table = rows(&[entry("first"), entry("second")]).unwrap_or_default();
        let (first, second) = (table.find("first"), table.find("second"));
        assert!(second < first);
    }
}
