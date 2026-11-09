use serde::Deserialize;

/// `?speed=5000` on a move to the centre. It stays text, because the firmware
/// alone decides what a good speed is, and answers `bad_speed` to anything else.
#[derive(Deserialize)]
pub(crate) struct CentreQuery {
    pub(crate) speed: Option<String>,
}
