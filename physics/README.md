# physics — MuJoCo for the synthetic loop

The balance policy has to train against real physics, not a scripted
sweep, and the orchestrator is the only participant that drives it
(`synthetic-loop-dia.md`). MuJoCo is a C library, so this crate is the
bridge: it links `mujoco.dll` into the orchestrator's process and gives
it a small, safe Rust type, `Physics`, that loads the cart-pole's MJCF,
sets the cart's speed, steps time forward and reads the two joints back.
The plan behind it, with what was decided and measured, is
`mujoco-plan.md`.

It is not a general MuJoCo binding. It declares only the dozen C
functions the cart-pole needs, and it refuses any model that is not one
cart slide, one pole hinge and one actuator.

## The two layers

- **`src/ffi.rs` — the raw C API.** Hand-written `extern "C"`
  declarations, each copied from the MuJoCo **3.15.0** headers, with the
  header line in a comment so it can be checked by hand. No bindgen.
  `mjModel` and `mjData` are opaque pointers: no Rust copy of their
  layout exists, so a MuJoCo upgrade cannot silently misread a field.
  State goes in and out only through `mj_getState` / `mj_setState`.
- **`src/lib.rs` — the safe wrapper.** `Physics` owns the model and the
  data, frees both on drop, and checks everything MuJoCo could get wrong
  at load. All of the workspace's `unsafe` code lives in this crate, each
  block with a `SAFETY:` comment; callers write only safe Rust.

## What a caller sees

| Item | What it does |
|---|---|
| `PhysicsConfig { model, names }` | What to load: the MJCF path, and the names of the cart joint, pole joint and actuator in it. The caller fills it; the orchestrator takes all four from its command line, which the trainer fills from its `settings.toml` `[mjcf]`. |
| `Physics::load(&config)` | Loads the MJCF, runs the load checks (below), measures the timestep, prints one summary line. |
| `reset(Pose)` | Back to t = 0 at the given pose, at rest (`qvel` and `ctrl` zeroed), derived quantities recomputed. |
| `set_ctrl(f64)` | The cart actuator's `ctrl`. For the cart-pole's `<velocity>` actuator this is the target cart speed in m/s, sign = direction. MuJoCo clamps it to the MJCF's `ctrlrange` (±0.85 m/s). Rejects NaN and infinity. |
| `step(n)` | Advances simulated time by `n` physics steps. |
| `time()` | Simulated time in seconds (`data.time`). |
| `pose()` | The two joints' `qpos` as a `Pose`. |
| `timestep()` | The physics step in seconds, as measured at load. |
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
    },
};
let mut physics = Physics::load(&config)?;
physics.reset(Pose { cart: 0.0, pole: 0.0 })?; // cart centred, pole hanging
physics.set_ctrl(0.1)?;                        // 0.1 m/s to the + side
physics.step(5);                               // 5 × 2 ms = 10 ms
println!("t {} s, {:?}", physics.time(), physics.pose());
```

## The checks at load

Each one fails `load` with a message naming what was found, instead of
letting a wrong model run:

1. **Library version.** `mj_version()` must be 3.15.0, the version the
   declarations in `ffi.rs` were copied from. A different `mujoco.dll`
   could have different signatures, so it is refused.
2. **The names exist.** Each of the three names must be in the model:
   `the model has no joint named "…"`.
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
  `ffi.rs` against the new headers, update `MJ_VERSION`, and point
  `mujoco_dir` at the new release tree.
