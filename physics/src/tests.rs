use crate::{Names, Physics, PhysicsConfig, Pose};

use std::{f64::consts::PI, fs, path::PathBuf};

use anyhow::{Result, anyhow};

// One micrometer. Every expected position is MuJoCo's own, read at
// another pose (MuJoCo is the source of truth: no position is chained by
// hand from the XML), so the two agree far inside it.
const TOLERANCE: f64 = 1e-6;

// The run's model and names come from the trainer's settings.toml, the
// one place they live: [mjcf] model is relative to [orchestrator] cwd,
// which is relative to the trainer folder.
fn config() -> Result<PhysicsConfig> {
    let trainer = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../trainer");
    let settings: toml::Table = fs::read_to_string(trainer.join("settings.toml"))?.parse()?;
    let text = |table: &str, key: &str| {
        settings
            .get(table)
            .and_then(|t| t.get(key))
            .and_then(|v| v.as_str())
            .map(str::to_owned)
            .ok_or_else(|| anyhow!("settings.toml [{table}] {key} is missing"))
    };
    let cwd = trainer.join(text("orchestrator", "cwd")?);
    Ok(PhysicsConfig {
        model: cwd.join(text("mjcf", "model")?),
        names: Names {
            cart_joint: text("mjcf", "cart_joint")?,
            pole_joint: text("mjcf", "pole_joint")?,
            actuator: text("mjcf", "actuator")?,
            left_site: text("mjcf", "left_site")?,
            right_site: text("mjcf", "right_site")?,
            pivot_site: text("mjcf", "pivot_site")?,
            end_site: text("mjcf", "end_site")?,
            target_site: text("mjcf", "target_site")?,
            dynamic_target_site: text("mjcf", "dynamic_target_site")?,
            cart_site: text("mjcf", "cart_site")?,
            cart_target_site: text("mjcf", "cart_target_site")?,
            left_rail_site: text("mjcf", "left_rail_site")?,
            left_cart_site: text("mjcf", "left_cart_site")?,
            right_rail_site: text("mjcf", "right_rail_site")?,
            right_cart_site: text("mjcf", "right_cart_site")?,
        },
    })
}

fn assert_near(what: &str, got: [f64; 3], want: [f64; 3]) {
    let apart = (0..3).map(|i| (got[i] - want[i]).abs()).fold(0.0, f64::max);
    assert!(
        apart < TOLERANCE,
        "{what}: MuJoCo {got:?}, expected {want:?}, {apart} m apart"
    );
}

// One test, so the model loads once at a time (the log handler and its
// warning count are process-wide).
#[test]
fn every_site_reaches_the_caller_at_its_world_position() -> Result<()> {
    let config = config()?;
    let mut physics = Physics::load(&config)?;

    // MuJoCo is the source of the pole's body; load proved it against
    // MuJoCo's other figures (check 7). Here MuJoCo's per-capture values
    // must agree with each other: the angle is the pose set, the centre of
    // mass sits straight below MuJoCo's hinge point by MuJoCo's com
    // distance when hanging and that far above it upright, the pole is
    // still after a reset and turns once let go from 1 rad. Gravity is
    // MuJoCo's default, the model leaves it unset.
    let body = physics.pole_body();
    let near = |what: &str, got: f64, want: f64| {
        assert!(
            (got - want).abs() < TOLERANCE,
            "{what}: {got}, expected {want}"
        );
    };
    near("gravity, m/s²", body.gravity, 9.81);
    physics.reset(Pose {
        cart: 0.0,
        pole: 0.0,
    })?;
    let pole = physics.pole();
    assert_eq!((pole.speed, pole.angle), (0.0, 0.0));
    near("hanging: com x - hinge x", pole.com[0] - pole.hinge[0], 0.0);
    near(
        "hanging: hinge z - com z",
        pole.hinge[2] - pole.com[2],
        body.com_distance,
    );
    physics.reset(Pose {
        cart: 0.0,
        pole: PI,
    })?;
    let pole = physics.pole();
    assert_eq!(pole.angle, PI);
    near(
        "upright: com z - hinge z",
        pole.com[2] - pole.hinge[2],
        body.com_distance,
    );
    physics.reset(Pose {
        cart: 0.0,
        pole: 1.0,
    })?;
    physics.step(10);
    assert!(
        physics.pole().speed != 0.0,
        "the pole let go from 1 rad is still after 10 steps"
    );

    // The cart top speed is the actuator's ctrlrange: commanded at it, and
    // at twice it (MuJoCo clamps ctrl to the range), the cart settles at
    // that speed. Settled after 0.2 s, then timed over 0.1 s; the slide's
    // friction leaves it short by frictionloss / kv, well inside 1%.
    let top = physics.max_speed();
    assert!(top > 0.0, "cart top speed {top} m/s");
    for ctrl in [top, 2.0 * top] {
        physics.reset(Pose {
            cart: 0.0,
            pole: 0.0,
        })?;
        physics.set_ctrl(ctrl)?;
        physics.step(100);
        let from = physics.pose().cart;
        physics.step(50);
        let speed = (physics.pose().cart - from) / (50.0 * physics.timestep());
        assert!(
            (speed - top).abs() < 0.01 * top,
            "ctrl {ctrl}: the cart settled at {speed} m/s, top {top} m/s"
        );
    }

    // Centred: Cart sits on Cart Target. Upright: End sits on Target and on
    // Dynamic Target. Hanging and upright, End is the same distance below
    // and above Pivot, so Pivot is halfway between the two.
    physics.reset(Pose {
        cart: 0.0,
        pole: 0.0,
    })?;
    let hanging = physics.sites();
    physics.reset(Pose {
        cart: 0.0,
        pole: PI,
    })?;
    let upright = physics.sites();
    assert_near("Cart, centred", hanging.cart, hanging.cart_target);
    assert_near("End, upright", upright.end, upright.target);
    assert_near(
        "End on Dynamic Target, upright",
        upright.end,
        upright.dynamic_target,
    );
    let halfway = [0, 1, 2].map(|i| (hanging.end[i] + upright.end[i]) / 2.0);
    assert_near(
        "Pivot, halfway from End hanging to End upright",
        hanging.pivot,
        halfway,
    );

    // Pivot sits on the hinge axis (y): its x and z are MuJoCo's hinge
    // point's.
    let hinge = physics.pole().hinge;
    assert_near(
        "Pivot on the hinge axis",
        [upright.pivot[0], 0.0, upright.pivot[2]],
        [hinge[0], 0.0, hinge[2]],
    );

    // Cart moved by 0.3 m: the sites on the cart (and, at the same pole
    // angle, on the pole) move by exactly that along the rail; the rest
    // stay where they are. Upright, End stays on Dynamic Target.
    physics.reset(Pose {
        cart: 0.3,
        pole: PI,
    })?;
    let moved = physics.sites();
    let shifted = |p: [f64; 3]| [p[0] + 0.3, p[1], p[2]];
    assert_near("Pivot, cart 0.3", moved.pivot, shifted(upright.pivot));
    assert_near("End, cart 0.3", moved.end, shifted(upright.end));
    assert_near(
        "Dynamic Target, cart 0.3",
        moved.dynamic_target,
        shifted(upright.dynamic_target),
    );
    assert_near("Cart, cart 0.3", moved.cart, shifted(upright.cart));
    assert_near(
        "Left Cart, cart 0.3",
        moved.left_cart,
        shifted(upright.left_cart),
    );
    assert_near(
        "Right Cart, cart 0.3",
        moved.right_cart,
        shifted(upright.right_cart),
    );
    assert_near("Left, cart 0.3", moved.left, upright.left);
    assert_near("Right, cart 0.3", moved.right, upright.right);
    assert_near("Target, cart 0.3", moved.target, upright.target);
    assert_near(
        "Cart Target, cart 0.3",
        moved.cart_target,
        upright.cart_target,
    );
    assert_near("Left Rail, cart 0.3", moved.left_rail, upright.left_rail);
    assert_near("Right Rail, cart 0.3", moved.right_rail, upright.right_rail);
    assert_near(
        "End on Dynamic Target, cart 0.3",
        moved.end,
        moved.dynamic_target,
    );

    // The strike faces meet where the sites put them, not at the slide's
    // range: each gap closes when the cart has moved by its centred width.
    let left_gap = hanging.left_cart[0] - hanging.left_rail[0];
    let right_gap = hanging.right_rail[0] - hanging.right_cart[0];
    assert!(
        left_gap > 0.0 && right_gap > 0.0,
        "centred gaps {left_gap} m and {right_gap} m"
    );
    physics.reset(Pose {
        cart: -left_gap,
        pole: 0.0,
    })?;
    let sites = physics.sites();
    let closed = sites.left_cart[0] - sites.left_rail[0];
    assert!(
        closed.abs() < TOLERANCE,
        "left gap {closed} m at cart {} m",
        -left_gap
    );
    physics.reset(Pose {
        cart: right_gap,
        pole: 0.0,
    })?;
    let sites = physics.sites();
    let closed = sites.right_rail[0] - sites.right_cart[0];
    assert!(
        closed.abs() < TOLERANCE,
        "right gap {closed} m at cart {right_gap} m"
    );

    // After stepping, the sites describe the state after the last step,
    // not before it: at 0.5 m/s one 2 ms step moves the cart 1 mm, 1000
    // times the tolerance. Pivot moves with the cart, so its x is the
    // centred Pivot's plus the cart's position.
    physics.reset(Pose {
        cart: 0.0,
        pole: 0.0,
    })?;
    physics.set_ctrl(0.5)?;
    physics.step(50);
    let cart = physics.pose().cart;
    let pivot = physics.sites().pivot;
    assert!(
        (pivot[0] - (hanging.pivot[0] + cart)).abs() < TOLERANCE,
        "Pivot x {} after stepping, cart at {cart}",
        pivot[0]
    );
    assert_eq!(physics.warnings(), 0);
    drop(physics);

    // A site name the model lacks fails the load, as a joint name does.
    let mut misnamed = config;
    misnamed.names.end_site = "no-such-site".into();
    let error = Physics::load(&misnamed).err().map(|e| e.to_string());
    assert_eq!(
        error.as_deref(),
        Some(r#"the model has no site named "no-such-site""#)
    );
    Ok(())
}
