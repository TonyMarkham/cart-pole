use serde::Deserialize;

/// Calibrating Orin → orchestrator (C9): whether every point lay within the
/// limit, in which case the matrix is written; the worst point, in pixels; and
/// the lens number.
#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct FittedReply {
    #[serde(rename = "type")]
    pub kind: String,
    pub ok: bool,
    pub worst_px: f64,
    pub lens_k: f64,
}
