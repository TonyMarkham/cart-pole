//! The set the tests share, and the helpers built on it.

use crate::pool::{parse_set::parse_set, pool_entry::PoolEntry};

use std::path::PathBuf;

/// How far apart two `f32`s may be and still count as equal: the JSON
/// values below are exact in `f32` or within one rounding of it, far inside
/// this.
pub(crate) const TOLERANCE: f32 = 1e-6;

/// A set in the JSON form the sets folder holds: lowerCamelCase names,
/// ranges as {min, max}.
pub(crate) const SET: &str = r#"{
    "name": "Set-01",
    "seed": 1,
    "states": 1000,
    "fixedPoses": [
        {
            "name": "Idler",
            "joints": [
                {"joint": "02-Cart.Slider.joint", "value": -0.445},
                {"joint": "04-Pole.Hinge.joint", "value": -180}
            ],
            "props": ["T-Square (Idler)"]
        }
    ],
    "fixedPoseFrames": 100,
    "cameraImageWidth": 1456,
    "cameraImageHeight": 1088,
    "cameraYaw": {"min": -5, "max": 5},
    "cameraDistance": {"min": -0.1, "max": 0.1},
    "desks": ["Classic", "Standard"],
    "unlimitedHinge": {"min": -3.1415927, "max": 3.1415927}
}"#;

/// Whether two `f32`s are equal within `TOLERANCE`.
pub(crate) fn approx(actual: f32, expected: f32) -> bool {
    (actual - expected).abs() < TOLERANCE
}

/// `SET` renamed, as the pool entry read from `<name>.json`, val or train.
pub(crate) fn entry(name: &str, is_val: bool) -> PoolEntry {
    let mut set = parse_set(SET).unwrap();
    set.name = name.to_owned();
    PoolEntry {
        path: PathBuf::from(format!("{name}.json")),
        set,
        is_val,
    }
}
