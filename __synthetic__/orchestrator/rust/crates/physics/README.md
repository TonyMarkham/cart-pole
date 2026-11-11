# physics — MuJoCo for the synthetic loop

The balance policy has to train against real physics, not a scripted
sweep, and the orchestrator is the only participant that drives it
(`synthetic-loop-dia.md`). MuJoCo is a C library, so this crate is the
bridge: it links `mujoco.dll` into the orchestrator's process and gives
it a small, safe Rust type, `Physics`, that loads the cart-pole's MJCF,
sets the cart's speed, steps time forward and reads back the two joints,
the twelve sites, and the pole's speed, centre of mass, mass and inertia.
The plan behind it, with what was decided and measured, was
`mujoco-plan.md` (removed 2026-11-11; in git at commit `ecced2cc`).

It is not a general MuJoCo binding. It declares only the eighteen C
functions the cart-pole needs, and it refuses any model that is not one
cart slide, one pole hinge and one actuator.

## The two layers

- **`src/ffi.rs` — the raw C API.** Hand-written `extern "C"`
  declarations, each copied from the MuJoCo **3.15.0** headers, with the
  header line in a comment so it can be checked by hand. No bindgen.
  State goes in and out through `mj_getState` / `mj_setState`. Site
  positions are not state (MuJoCo derives them from `qpos`), so they are
  read from `mjData` itself, and the actuator's `ctrlrange` from
  `mjModel`: `mjModel` and `mjData` mirror only their first fields, up
  to the last one read (`actuator_ctrlrange`, `site_xpos`), and `load`
  proves that mirror against the DLL before anything reads through it
  (check 5 below), so a mismatch fails at load instead of misreading a
  field.
- **`src/lib.rs` — the safe wrapper.** `Physics` owns the model and the
  data, frees both on drop, and checks everything MuJoCo could get wrong
  at load. All of the workspace's `unsafe` code lives in this crate, each
  block with a `SAFETY:` comment; callers write only safe Rust.

## What a caller sees

| Item | What it does |
|---|---|
| `PhysicsConfig { model, names }` | What to load: the MJCF path, and the names in it of the cart joint, pole joint, actuator and the twelve sites (the four markers, Target, Dynamic Target, Cart, Cart Target, and the four strike faces). The caller fills it; the orchestrator takes all fifteen from its command line, which the trainer fills from its `settings.toml` `[mjcf]`. |
| `Physics::load(&config)` | Loads the MJCF, runs the load checks (below), measures the timestep, prints one summary line. |
| `reset(Pose)` | Back to t = 0 at the given pose, at rest (`qvel` and `ctrl` zeroed), derived quantities recomputed. |
| `set_ctrl(f64)` | The cart actuator's `ctrl`. For the cart-pole's `<velocity>` actuator this is the target cart speed in m/s, sign = direction. MuJoCo clamps it to the MJCF's `ctrlrange`. Rejects NaN and infinity. |
| `step(n)` | Advances simulated time by `n` physics steps. |
| `time()` | Simulated time in seconds (`data.time`). |
| `pose()` | The two joints' `qpos` as a `Pose`. |
| `sites()` | The twelve named sites as `Sites { left, right, pivot, end, target, dynamic_target, cart, cart_target, left_rail, left_cart, right_rail, right_cart }`, each its world position for the current `qpos` in MuJoCo's frame (meters, z up; Unity's (x, y, z) is MuJoCo's (x, z, y)). Computed on the call (`mj_kinematics`), because after `mj_step` MuJoCo's own copy still describes the state before the step; this changes nothing in the simulation. |
| `pole()` | The pole's state as `PoleState { speed, angle, com, hinge }`: the pole joint's `qvel` in rad/s and `qpos` in rad (0 hanging, π upright, counting on past a full turn), the centre of mass of everything that swings on the joint, MuJoCo's `subtree_com`, and the joint's anchor, MuJoCo's `xanchor`, both world points like the sites. Computed on the call (`mj_kinematics`, `mj_comPos`) for the same reason as `sites()`. |
| `pole_body()` | What the pole's energy needs and the pose never changes, as `PoleBody { mass, com_distance, inertia, gravity }`: kg, the centre's distance from the joint's axis in m, the inertia about that axis in kg·m², and gravity's size in m/s². Read and proved at load (check 7). |
| `timestep()` | The physics step in seconds, as measured at load. |
| `max_speed()` | The cart top speed in m/s: the cart actuator's `ctrlrange` upper bound, read from the loaded model. `load` refuses an actuator that is not `ctrllimited` or whose `ctrlrange` is not symmetric (lo = −hi), so ± this bounds every command. |
| `warnings()` | MuJoCo warnings since the process started. Any warning means the run's states can't be trusted, so the orchestrator fails the run on a nonzero count. |

**`Pose`** is `{ cart, pole }`: the cart's position along the slide in
meters, and the pole's hinge angle in radians, where **0 is hanging and
π is upright**.

```rust
use physics::{Names, Physics, PhysicsConfig, Pose};

let config = PhysicsConfig {
    model: "path/to/asy-cart-pole.xml".into(),
    names: Names {
        cart_joint: "02-Cart.Slider.joint".into(),
        pole_joint: "04-Pole.Hinge.joint".into(),
        actuator: "03-Cart.Slider.Actuator".into(),
        left_site: "05-Left.Site".into(),
        right_site: "06-Right.Site".into(),
        pivot_site: "07-Pole.Pivot.Site".into(),
        end_site: "08-Pole.End.Site".into(),
        target_site: "09-Pole.End.Target.Site".into(),
        dynamic_target_site: "14-Dynamic.End.Target.Site".into(),
        cart_site: "16-Cart.Site".into(),
        cart_target_site: "15-Cart.Target.Site".into(),
        left_rail_site: "10-Left-Rail.Site".into(),
        left_cart_site: "11-Left-Cart.Site".into(),
        right_rail_site: "12-Right-Rail.Site".into(),
        right_cart_site: "13-Right-Cart.Site".into(),
    },
};
let mut physics = Physics::load(&config)?;
physics.reset(Pose { cart: 0.0, pole: 0.0 })?; // cart centred, pole hanging
physics.set_ctrl(0.1)?;                        // 0.1 m/s to the + side
physics.step(5);                               // 5 × 2 ms = 10 ms
println!("t {} s, {:?}", physics.time(), physics.pose());
println!("End marker at {:?}", physics.sites().end); // world, meters
```

## The checks at load

Each one fails `load` with a message naming what was found, instead of
letting a wrong model run:

1. **Library version.** `mj_version()` must be 3.15.0, the version the
   declarations in `ffi.rs` were copied from. A different `mujoco.dll`
   could have different signatures, so it is refused.
2. **The names exist.** Each of the twelve names must be in the model:
   `the model has no joint named "…"`, or `site`, or `actuator`.
3. **The model's shape.** Exactly two one-degree-of-freedom joints
   (`qpos` length 2, ids 0 and 1) and exactly one actuator (`ctrl`
   length 1, id 0). With exactly that shape, a joint's `qpos` index is
   its id, which is how `pose()` knows which number is the cart.
4. **The timestep.** Read by stepping once from reset and taking the
   elapsed time, which avoids reading a field of `mjModel`. It must be
   positive. The crate never sets the timestep or the integrator: both
   come from the MJCF's `<option>`, which the Unity exporter writes
   from the rig's StepJointSet. For the cart-pole that is
   `<option timestep="0.002" integrator="implicitfast"/>`: 2 ms, the
   loop's fixed deltatime.
5. **The mirrored layout.** Each field read through `ffi.rs`'s copy of
   `mjModel` / `mjData` is compared with the same quantity reached
   another way: `nq` and `nu` with `mj_stateSize`; `time` and `qpos`
   with a probe state written by `mj_setState`; the `qpos` and
   `site_xpos` pointers with MuJoCo's buffer bounds; and every site's
   position with `mj_insideSite`, which must place it inside that site
   and a point 1 m away outside it; `opt.timestep` with the measured
   timestep; every `mjModel` pointer from `buffer` to
   `actuator_ctrlrange` with MuJoCo's buffer (each inside it, in header
   order); `qpos0` with the qpos `mj_resetData` copies from it; and the
   `ctrlrange` read with MuJoCo's own clamp (one step from rest at
   `ctrl` hi equals one at hi + 1, and differs from one at hi / 2).
6. **The cart top speed.** The actuator must be `ctrllimited` with a
   symmetric `ctrlrange` (lo = −hi > 0); `max_speed()` is hi. Tried by deleting one mirrored
   field at a time (`arena`, `energy`, `xaxis`): each fails `load`.
7. **The pole's body.** `body_mass` must add up to `mj_getTotalmass`;
   `body_parentid` must list every body after its parent; and at a probe
   pose every body's `subtree_com` and `body_subtreemass` must equal the
   mass-weighted centre and the mass of its subtree, summed by the crate
   from `xipos` and `body_mass`, so a misread array is refused. Then
   `pole_body()` is read: the mass, `body_subtreemass` of the pole
   joint's body; the centre's distance from the joint's axis, from
   `xanchor` and `xaxis` (the axis must be a unit vector and the centre
   off it); the inertia about the axis, the joint's diagonal entry of
   MuJoCo's inertia matrix through `mj_mulM`, at least mass × distance²
   (the parallel-axis floor); and gravity, which must point straight
   down, −z.

## MuJoCo's log

The crate installs a log handler for the whole process. It prints every
MuJoCo message to stderr as `mujoco info|warning|error: …` and counts
warnings for `warnings()`. It never panics into C. After an error
message, MuJoCo itself ends the process.

## Building

- **Which MuJoCo** comes from `mujoco_dir` in the repo-root
  `orchestrator.toml`, a MuJoCo 3.15.0 release tree (here, the sibling
  greyhound checkout). That file is read only by `build.rs`, and any
  other key in it is an error, so a leftover setting can't look as if
  it configures a run.
- `build.rs` copies `mujoco.lib` and `mujoco.dll` into the build's
  `OUT_DIR` and links from there, and copies `mujoco.dll` beside the
  binaries in `target/<profile>/`. Windows finds a DLL beside the
  executable first, so `target/release/orchestrator.exe` runs without
  cargo; `cargo run` and `cargo test` also find it on their `PATH`.
- Editing `orchestrator.toml`, or replacing the MuJoCo files, rebuilds
  and relinks on the next cargo build.
- Because the DLL comes from beside the binary, `mujoco_dir` is a build
  input only: a run always uses the MuJoCo its binary was built with.

## Limits

- `Physics` holds raw pointers, so it is neither `Send` nor `Sync`: it
  lives on the thread that loaded it.
- The log handler and warning count are process-wide, so two `Physics`
  in one process share one count.
- **To move to another MuJoCo version**, re-check every declaration in
  `ffi.rs` against the new headers, including every field of the
  mirrored `mjModel` / `mjData` prefixes (fields are added between
  versions), update `MJ_VERSION`, and point `mujoco_dir` at the new
  release tree.
