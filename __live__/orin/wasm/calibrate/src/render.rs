use crate::{
    constants::{CART_AT_CENTRE_X, CART_AT_IDLER_X, CART_AT_MOTOR_X, CART_WIDTH},
    entry::Entry,
    place::Place,
    session::{Session, grouped},
    tone::Tone,
};

use std::fmt::Write;

/// The rail as SVG, with the motor on the left, where `control-design.md` puts
/// D2. Nothing from outside this crate is written here, so none of it needs
/// escaping. The drawing's only moving part is the belt, because the cart's
/// position during a move is not known.
pub(crate) fn rail(session: &Session) -> Result<String, std::fmt::Error> {
    let place = session.place();
    let mut out = String::new();
    write!(
        out,
        "<svg viewBox=\"0 76 860 156\" role=\"img\" aria-label=\"Drawing of the rail, with the \
         motor on the left and the idler pulley on the right. The sentence below says the same.\">\
         <rect class=\"steel\" x=\"60\" y=\"136\" width=\"750\" height=\"8\"/>"
    )?;
    let heading = session.heading().map_or("", |heading| heading.class());
    write!(
        out,
        "<g class=\"{heading}\">\
         <line class=\"belt top\" x1=\"60\" y1=\"118\" x2=\"810\" y2=\"118\"/>\
         <line class=\"belt bottom\" x1=\"60\" y1=\"162\" x2=\"810\" y2=\"162\"/>\
         </g>\
         <rect class=\"steel\" x=\"20\" y=\"104\" width=\"60\" height=\"72\" rx=\"6\"/>\
         <text class=\"block-label\" x=\"50\" y=\"198\">Motor</text>\
         <circle class=\"steel\" cx=\"810\" cy=\"140\" r=\"22\"/>\
         <text class=\"block-label\" x=\"810\" y=\"198\">Idler</text>"
    )?;
    let (motor, idler) = match session.switches() {
        Some(switches) => (
            switch_state(switches.motor_pressed(), place),
            switch_state(switches.idler_pressed(), place),
        ),
        None => (("no reading", Tone::None), ("no reading", Tone::None)),
    };
    switch(&mut out, "D2", motor, 92, 92, "start")?;
    switch(&mut out, "D3", idler, 752, 768, "end")?;
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
