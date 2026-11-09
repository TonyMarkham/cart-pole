pub(crate) mod lens_rect;
pub(crate) mod lens_result;

// ---------------------------------------------------------------------------------------------- //

use crate::lens_status::{lens_rect::LensRect, lens_result::LensResult};

use serde::Deserialize;

/// `/lens/status`: `Service.lens_status` in `calibrate_service.py`.
#[derive(Debug, Deserialize)]
pub(crate) struct LensStatus {
    pub(crate) on: bool,
    /// Why the view stopped by itself, if it did.
    pub(crate) message: String,
    pub(crate) per_s: f64,
    /// The latest frame as a JPEG in base64, at half the sensor's size.
    pub(crate) frame: Option<String>,
    /// Each board marker found in the latest frame: its four corners.
    pub(crate) markers: Vec<Vec<[f64; 2]>>,
    /// The board's outer edge where the corners found put it, if 4 or more were.
    pub(crate) outline: Option<Vec<[f64; 2]>>,
    /// The board's corners found in the latest frame, of how many, and how many a
    /// frame needs to count.
    pub(crate) corners: u32,
    pub(crate) all_corners: u32,
    pub(crate) min_corners: u32,
    /// Enough corners found, and everything found inside the box.
    pub(crate) in_box: bool,
    /// At a tilted position, how far the named side is turned away from the
    /// camera, degrees, negative when towards it; None when flat or too little of
    /// the board is found to tell.
    pub(crate) turn_deg: Option<f64>,
    /// That, and at a tilted position the named side turned at least
    /// `min_tilt_deg` away: the frame counts towards keeping a still.
    pub(crate) ready: bool,
    /// The sensor's full size, which every coordinate here is in.
    pub(crate) width: u32,
    pub(crate) height: u32,
    /// How far from the camera to hold the board, how far to aim to turn it, and
    /// the least turn that counts.
    pub(crate) distance_mm: f64,
    pub(crate) tilt_deg: f64,
    pub(crate) min_tilt_deg: f64,
    /// None once every position is done.
    pub(crate) rect: Option<LensRect>,
    pub(crate) position: u32,
    pub(crate) positions: u32,
    /// The first this many positions are flat; the rest are tilted.
    pub(crate) flat_positions: u32,
    pub(crate) held: u32,
    pub(crate) hold_frames: u32,
    pub(crate) kept: u32,
    pub(crate) min_views: u32,
    pub(crate) result: Option<LensResult>,
}
