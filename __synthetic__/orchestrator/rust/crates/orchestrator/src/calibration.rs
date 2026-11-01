use std::f64::consts::{FRAC_PI_2, PI};

use anyhow::{Result, bail};
use physics::{Physics, Pose};

/// One pose the calibration measures (synthetic-loop-dia.md, Calibration): the
/// two joints, and where MuJoCo says Pivot and End then are, each `(x, z)` in
/// metres in the pole's plane.
#[derive(Debug)]
pub struct CalibrationPose {
    pub name: &'static str,
    pub cart: f64,
    pub pole: f64,
    pub pivot: [f64; 2],
    pub end: [f64; 2],
}

/// The cart's three stops, `(left, centre, right)` in `qpos`, m. `left_room`
/// and `right_room` are the cart's clearance to each base with it centred, so
/// the end stops stand `stop_gap` short of them. The centre is halfway between
/// the two, as on the rig, and need not be `qpos` 0: the rail's two rooms
/// differ.
pub fn stops(left_room: f64, right_room: f64, stop_gap: f64) -> Result<(f64, f64, f64)> {
    if !(0.0..left_room.min(right_room)).contains(&stop_gap) {
        bail!(
            "--calibration-stop-gap {stop_gap} m does not fit the rail: the cart has {left_room} m to the left base and {right_room} m to the right"
        );
    }
    let (left, right) = (stop_gap - left_room, right_room - stop_gap);
    Ok((left, (left + right) / 2.0, right))
}

/// The seven poses, with their true points read from MuJoCo. It only sets
/// each pose and reads it: nothing steps, so the pole stays where it is put.
/// Leaves MuJoCo in the last pose.
pub fn calibration_poses(physics: &mut Physics, stop_gap: f64) -> Result<Vec<CalibrationPose>> {
    physics.reset(Pose {
        cart: 0.0,
        pole: 0.0,
    })?;
    let sites = physics.sites();
    let left_room = sites.left_cart[0] - sites.left_rail[0];
    let right_room = sites.right_rail[0] - sites.right_cart[0];
    let (left, centre, right) = stops(left_room, right_room, stop_gap)?;

    // Which way a quarter turn swings End is the joint's axis's to say.
    physics.reset(Pose {
        cart: 0.0,
        pole: FRAC_PI_2,
    })?;
    let sites = physics.sites();
    let towards_right = if sites.end[0] > sites.pivot[0] {
        FRAC_PI_2
    } else {
        -FRAC_PI_2
    };

    let layout = [
        ("left hanging", left, 0.0),
        ("centre hanging", centre, 0.0),
        ("right hanging", right, 0.0),
        ("left horizontal", left, -towards_right),
        ("right horizontal", right, towards_right),
        ("left up", left, PI),
        ("right up", right, PI),
    ];
    let mut poses = Vec::with_capacity(layout.len());
    for (name, cart, pole) in layout {
        physics.reset(Pose { cart, pole })?;
        let sites = physics.sites();
        poses.push(CalibrationPose {
            name,
            cart,
            pole,
            pivot: [sites.pivot[0], sites.pivot[2]],
            end: [sites.end[0], sites.end[2]],
        });
    }
    Ok(poses)
}
