use serde::Deserialize;

/// The part of the board's answer to `state` this page uses: its state's name
/// and its own words. In FAULT the answer is the FAULT STATE reply, whose
/// `result` is `"fault"` too.
#[derive(Debug, Deserialize)]
pub(crate) struct BoardAnswer {
    pub(crate) result: String,
    #[serde(default)]
    pub(crate) message: String,
}
