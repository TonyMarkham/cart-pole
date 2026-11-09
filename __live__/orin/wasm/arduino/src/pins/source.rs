use crate::pins::meaning::Meaning;

/// Whether the firmware reports a pin. `key` names it in the Arduino's answer.
pub(crate) enum Source {
    Live {
        key: &'static str,
        high: Meaning,
        low: Meaning,
    },
    NotRead,
}
