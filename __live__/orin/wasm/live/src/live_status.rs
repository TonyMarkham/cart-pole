use crate::marker_box::MarkerBox;

use serde::Deserialize;
use std::collections::BTreeMap;

/// `/live/status`: `Service.live_status` in `calibrate_service.py`.
#[derive(Debug, Deserialize)]
pub(crate) struct LiveStatus {
    pub(crate) on: bool,
    /// Why the view stopped by itself, if it did.
    pub(crate) message: String,
    pub(crate) frames: u64,
    /// Frames detected per second, over the last `window` frames.
    pub(crate) per_s: f64,
    pub(crate) window: u32,
    /// In how many of the last `window` frames each marker was found, by name.
    pub(crate) found: BTreeMap<String, u32>,
    /// The latest frame as a JPEG in base64.
    pub(crate) frame: Option<String>,
    /// The latest frame's box for each marker, by name; None where it was missed.
    pub(crate) boxes: Option<BTreeMap<String, Option<MarkerBox>>>,
    pub(crate) width: u32,
    pub(crate) height: u32,
}
