# Hard-coded in error — values the loop's code holds that it should read

Changing the rig, for example from the 30 V supply to 12 V, should mean
changing a value in the MJCF, `trainer/settings.toml` or the gym file,
never editing code (user, 2026-10-25: "Hard-Coded constants are pretty
much universally a bad design"). Today the stepper's top speed alone sits
in three places that must be kept equal by hand. This file lists every
value the loop's code holds that breaks that rule, so the fix can be done
later in one pass. It is a record of findings, not a plan: nothing here
has been changed.

## The rule

A constant in code is in error when either:

- its value is also held somewhere else: the MJCF, a toml, or another
  process's code (a copy, which can drift from the original); or
- it is a choice someone might change between runs (a setting kept in
  code).

It is not in error when a datatype, an outside API or algorithm, or plain
arithmetic fixes it (see *Not in error*). A value in a toml is a setting,
never hard-coded.

## Scope

Audited 2026-10-25 by extracting, with comments stripped, every line
holding a number of 2 or more or a decimal, then reading each line found
in full:

- Rust: `orchestrator/src`, `arduino/src`, `physics/src`;
- Python: `trainer/src`, `orin/src`;
- Unity: the `simulation.cart-pole.sim-loop` package, and
  `GreyscaleEncoder` in `simulation.cart-pole.synthetic-data-gen`, which
  the loop calls.

Not audited: the tests; the rest of the synthetic-data-gen package;
`unity/projects/cart-pole/Assets` scripts; `training/`; `vision/`;
`freecad/`; the greyhound repo; and the literals 0 and 1, which the
extraction skipped.

## The values

### A value the MJCF already holds

| # | Hard-coded value | Where | The MJCF's value |
|---|---|---|---|
| 1 | top speed 850,000 µm/s | `arduino/src/config.rs:36` (`RIG.max_speed_um_per_s`) | the actuator's `ctrlrange` ±0.85 m/s |
| 2 | travel 450,000 µm | `arduino/src/config.rs:35` (`RIG.travel_um`) | the slide joint's `range` ±0.45 m |
| 3 | `MAX_RATE = 56_666` pulses/s | `trainer/src/contracts.py:171` | `ctrlrange` ÷ metres per pulse |
| 4 | `STEP_MS = 2.0` | `trainer/src/endings/strike_gap.py:21` | `<option timestep>`, which `physics/src/lib.rs:158` already reads from the model |
| 5 | `GRID_MS = 2.0` | `trainer/src/observations/pool_and_commands.py:23` | the same timestep |
| 6 | the fake rig's timestep and geometry (`STEP_MS`, `RAIL_X`, `HINGE_Y/Z`, `POLE`, the travels) | `trainer/src/link/fake.py:23, 26-30` | the timestep and the sites; used by the tests only |

### Drive figures with no file to live in

| # | Hard-coded value | Where |
|---|---|---|
| 7 | 200 full steps/rev (the motor's 1.8°), 32 microsteps, 96,000 µm per pulley turn | `arduino/src/config.rs:32-34` (`RIG`) |
| 8 | `METRES_PER_PULSE = 15e-6`, derived from #7 by hand | `trainer/src/endings/strike_gap.py:20`, `trainer/src/link/fake.py:24` |

The motor's figures themselves are in `motor-spec.md`.

### A value held in another file or process

| # | Hard-coded value | Copies |
|---|---|---|
| 9 | pool of 6 entries | `trainer/src/contracts.py:23` (`ENTRIES`), `orchestrator/src/orin_reply.rs:4` (`POOL_ENTRIES`), `orin/src/synthetic_orin.py:96` (`WINDOW`) |
| 10 | 4 markers, rows of 9 floats | `trainer/src/contracts.py:24`; `orchestrator/src/orin_reply.rs:9` (`[f64; 9]`); `orchestrator/src/main.rs:349` (`",".repeat(7)`) and `:418-425` (`n[1]` … `n[8]`); `orin/src/synthetic_orin.py:124, 234, 236` (`* 4`); `orin/src/detector_config.py:36` (`!= 4`). `orin/settings.toml`'s `classes` already lists the four. |
| 11 | frame 728×544 | `unity/projects/packages/simulation.cart-pole.sim-loop/Runtime/SimulationLoopController.cs:53-54`, `orin/src/synthetic_orin.py:93`, `orin/src/reference_check.py:35` |
| 12 | not detected, (−1, −1) | `trainer/src/contracts.py:31`, `orin/src/detector.py:34`, `orin/src/synthetic_orin.py:99` |
| 13 | 3 failed frames fail the run | `orin/src/synthetic_orin.py:103`; restated in the orchestrator's error text, `orchestrator/src/main.rs:759` |
| 14 | at most 12 instances | `orchestrator/src/args.rs:74` (`1..=12`, on each of rows and columns since 2026-10-31), `orchestrator/src/main.rs` (`INSTANCES_MAX`, rows × columns), `trainer/src/settings.py:27` (`INSTANCES_MAX`) |
| 15 | 2 ports per instance | `orchestrator/src/main.rs:176`, `trainer/src/settings.py:102` |
| 16 | the model's shape: 12 sites, 2 joints, 1 actuator | `physics/src/lib.rs:32-37` (`QPOS_LEN`, `CTRL_LEN`, `SITES`) |
| 17 | depth buffer 24 bits | `SimulationLoopController.cs:55` (`DepthBits`), `unity/projects/packages/simulation.cart-pole.synthetic-data-gen/Runtime/GenerationSetRunner.cs:83` (a bare `24`) |
| 18 | capture-record column `p9`, the second pool row's age | `trainer/src/measures/common.py:35`, which assumes rows of 9 |

### A choice kept in code

| # | Hard-coded value | Where | Note |
|---|---|---|---|
| 19 | connect timeout 120 s, reply timeout 10 s | `orchestrator/src/args.rs:139, 143` | argument defaults; the trainer never passes them |
| 20 | exit grace 5 s | `orchestrator/src/main.rs:49` (`CLIENT_EXIT_GRACE`) | |
| 21 | pole start angle 0.0 rad | `orchestrator/src/args.rs:161` (`--start`) | argument default; the trainer never passes it |
| 22 | 1 instance, ports 9000 and 9001 | `orchestrator/src/args.rs:74, 80, 86` | argument defaults duplicating `settings.toml`, which the trainer always passes |
| 23 | `--frames` 60, `--control` zero, `--out target/orchestrator-frames` | `orchestrator/src/args.rs:148, 155, 187` | argument defaults: the no-render mode and the output folder |
| 24 | idle wait 100 ms | `SimulationLoopController.cs:65` (`IdleWaitMilliseconds`) | |
| 25 | JPEG quality 90 | `unity/projects/packages/simulation.cart-pole.synthetic-data-gen/Runtime/GreyscaleEncoder.cs:21` | shared by the loop and the dataset generator; must match what the detector trained on |
| 26 | reward floors 0.8 and 0.5, and the 0.1 inside `K` | `trainer/src/rewards/site_product.py:29, 33-34` (file deleted 2026-10-28 with its reward) | the root `reward-plan.md` of the time made them constants of the calculation |
| 27 | 2 episodes | `trainer/src/env_check.py:20` | command-line default |
| 28 | tolerances 1.0 px and 0.01; 20 warm-up frames | `orin/src/reference_check.py:31-32, 47` | |

## Not in error

Each is fixed by something outside the code's choices:

- **Datatypes:** `SEED_LIMIT = 2**64` (`trainer/src/settings.py:26`,
  `trainer/src/link/answer.py:14`, `orin/src/synthetic_orin.py:133`), the
  u64 seed; `seed % 2**32` (`trainer/src/train.py:179`), the 32-bit seed
  SB3 and NumPy take; the 1–65535 port checks, a u16 port;
  `COMMAND_LEN = 4` (`arduino/src/command.rs:2`), an i32's bytes.
- **Outside APIs:** the `mj*` numbers in `physics/src/ffi.rs`, MuJoCo
  3.1.5's ABI; `4 + len(classes)` (`orin/src/detector.py`,
  `orin/src/build_engine.py`), YOLO's output layout; the letterbox
  `round(d/2 ∓ 0.1)` and `/ 255` (`orin/src/detector.py`), Ultralytics'
  preprocessing, which must match the model's training;
  `MjcfToUnityHingeSign = -1` (`SimulationLoopController.cs:48`),
  greyhound's exporter swapping Y and Z.
- **An algorithm's own constants:** `TWO_POW_53` and the SplitMix64
  multipliers (`orchestrator/src/latency.rs`); `sqrt(-2 ln 0.1)` given
  #26's 0.1; the (0, 1] bounds in `trainer/src/learning.py`.
- **Units:** `MICROS_PER_SECOND` (`arduino/src/stepper.rs:4`) and the
  ×1000 ms↔s factors.
- **Internal to the code:** `LOAD_ERROR_LEN = 1000`
  (`physics/src/lib.rs:40`), the error buffer handed to MuJoCo; the
  4096-byte read buffer (`SimulationLoopController.cs:462`); the
  self-test probe values 1.5 and [0.25, −0.5] (`physics/src/lib.rs:284-285`);
  exit code 130 for Ctrl+C (`trainer/src/train.py:195`).

## What depends on it

The tests assert today's values and change with them:
`arduino/src/tests.rs` (56,666 in five places) and
`orchestrator/src/tests.rs:132`.
