use crate::camera::xyz::Xyz;

use serde::Serialize;
use std::{fmt, str::FromStr};

/// The Unity camera's pose for a run (__documentation__/synthetic-loop-dia.md,
/// Contracts, the camera pose): world position in metres and Euler rotation
/// in degrees, in Unity's frame. Written `px,py,pz,rx,ry,rz` on a command
/// line, e.g. `0,1.1,-2.5,0,0,0`; sent as
/// `{"position": {"x", "y", "z"}, "eulerRotation": {"x", "y", "z"}}`.
#[derive(Serialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CameraTransform {
    pub position: Xyz,
    pub euler_rotation: Xyz,
}

/// `px,py,pz,rx,ry,rz`: three for the position, three for the rotation.
const PARTS: usize = 6;

impl FromStr for CameraTransform {
    type Err = String;

    fn from_str(text: &str) -> Result<CameraTransform, String> {
        let values = text
            .split(',')
            .map(|part| {
                part.trim()
                    .parse::<f64>()
                    .ok()
                    .filter(|value| value.is_finite())
                    .ok_or_else(|| format!("{part:?} is not a number"))
            })
            .collect::<Result<Vec<f64>, String>>()?;
        let [px, py, pz, rx, ry, rz] = <[f64; PARTS]>::try_from(values).map_err(|values| {
            format!(
                "{text:?}: expected {PARTS} numbers px,py,pz,rx,ry,rz (metres, then degrees), found {}",
                values.len()
            )
        })?;
        Ok(CameraTransform {
            position: Xyz {
                x: px,
                y: py,
                z: pz,
            },
            euler_rotation: Xyz {
                x: rx,
                y: ry,
                z: rz,
            },
        })
    }
}

/// The command-line form, `px,py,pz,rx,ry,rz`, which `from_str` reads back.
impl fmt::Display for CameraTransform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (p, r) = (self.position, self.euler_rotation);
        write!(f, "{},{},{},{},{},{}", p.x, p.y, p.z, r.x, r.y, r.z)
    }
}
