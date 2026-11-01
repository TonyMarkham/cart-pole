use crate::switch::Switch;

use core::fmt::{self, Write};

/// A move's answer as JSON, the same five keys every time, whether it worked
/// (`Moved`) or not (`AppError`). `SEEK_ANSWER_MAX` must hold the longest. The
/// messages hold no `"` or `\`, so they need no escaping.
pub fn write_answer(
    out: &mut impl Write,
    status: u16,
    result: &str,
    message: &str,
    pin: Option<Switch>,
    pulses: u32,
) -> fmt::Result {
    write!(
        out,
        "{{\"status\":{status},\"result\":\"{result}\",\"message\":\"{message}\","
    )?;
    match pin {
        Some(pin) => write!(out, "\"pin\":\"{}\",", pin.name())?,
        None => write!(out, "\"pin\":null,")?,
    }
    write!(out, "\"pulses\":{pulses}}}")
}
