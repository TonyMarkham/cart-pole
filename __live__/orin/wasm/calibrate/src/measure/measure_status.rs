use crate::measure::{frame_boxes::FrameBoxes, measure_summary::MeasureSummary};

use serde::Deserialize;
use std::collections::BTreeMap;

/// `/calibrate/measure/status`: `Service.status` in `calibrate_service.py`.
#[derive(Debug, Deserialize)]
pub(crate) struct MeasureStatus {
    /// `idle`, `settling`, `measuring`, `done` or `failed`.
    pub(crate) state: String,
    pub(crate) target: u32,
    pub(crate) count: u32,
    pub(crate) message: String,
    pub(crate) summary: Option<MeasureSummary>,
    /// The latest frame as a JPEG in base64, while measuring.
    pub(crate) frame: Option<String>,
    pub(crate) boxes: Option<FrameBoxes>,
    pub(crate) width: u32,
    pub(crate) height: u32,
    /// Every pose the fit needs, by `Pole::pose_key`.
    pub(crate) poses: Vec<String>,
    /// The poses with a kept measurement.
    pub(crate) measured: Vec<String>,
    pub(crate) min_images: u32,
    pub(crate) max_images: u32,
    /// The hand-measured gap kept for each end, by stop.
    pub(crate) gaps_mm: BTreeMap<String, f64>,
    pub(crate) max_gap_mm: f64,
}
