use crate::marker_box::MarkerBox;

use serde::Deserialize;

/// The two markers that count in a frame; None where the detector missed one.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
pub(crate) struct FrameBoxes {
    #[serde(rename = "Pivot")]
    pub(crate) pivot: Option<MarkerBox>,
    #[serde(rename = "End")]
    pub(crate) end: Option<MarkerBox>,
}
