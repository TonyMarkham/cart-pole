/// How a state is coloured. The words carry the meaning, so nothing depends on
/// seeing the colour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Tone {
    Ok,
    Warn,
    Info,
    None,
}

impl Tone {
    pub(crate) fn class(self) -> &'static str {
        match self {
            Tone::Ok => "ok",
            Tone::Warn => "warn",
            Tone::Info => "info",
            Tone::None => "none",
        }
    }
}
