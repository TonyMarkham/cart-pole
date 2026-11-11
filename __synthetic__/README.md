# \_\_synthetic\_\_ — code for the simulated robot

The policy learns in a simulation of the rig, so the repo holds two robots: the
real one in [\_\_live\_\_](../__live__), and the simulated one here. They share no
code (user, 2026-11-01): what both need is copied, so a change to one can never
break the other.

```text
__synthetic__/
├── orchestrator/rust/        stands in for the physical world
│   ├── orchestrator.toml     where the MuJoCo physics library is, read when building
│   └── crates/
│       ├── orchestrator/     drives the loop
│       ├── physics/          MuJoCo, the physics simulator, inside the orchestrator
│       └── arduino/          the synthetic Arduino, also inside the orchestrator
├── orin/python/              the synthetic Orin: finds the markers in each frame
└── trainer/python/           trains the policy, and starts every run
```

Each folder is named for what it stands in for, then the language it is written
in. A **crate** is one Rust package, and a **`uv` project** is a Python package
that keeps its dependencies in its own `.venv` folder.

The crates belong to the workspace in the repo root's `Cargo.toml`, so the built
program is `target/release/orchestrator.exe`. Every command runs from the repo root.

**To build the orchestrator:**

```bash
cargo build --release -p orchestrator
```

**To check the loop runs,** with one simulated robot, for one episode:

```bash
uv run --project __synthetic__/trainer/python python __synthetic__/trainer/python/src/env_check.py 1
```

**The calibration.** The detector gives pixels and the policy needs metres, so every
launch first brings up one Orin and one player to find the matrix between the two, and
writes it to `homography.json` in the run's `--out` folder. A launch whose fit fails
stops there. [The diagram](../synthetic-loop-dia.md) has the sequence, under
*Calibration*, and each point's error is printed as it runs. Its settings are
`[calibration]` in `orin/python/settings.toml` and `calibration_stop_gap` in
`trainer/python/settings.toml`.

**To check the calibration's arithmetic,** with no player or GPU:

```bash
uv run --project __synthetic__/orin/python python __synthetic__/orin/python/src/test_calibration.py
```

**To write a trained policy for the real robot,** as one numpy file the Orin runs
without PyTorch (`policy.npz` in the run's folder; the live Orin's README, *The
policy*, says what is in it and where it goes):

```bash
uv run --project __synthetic__/trainer/python python __synthetic__/trainer/python/src/export_policy.py __synthetic__/trainer/python/runs/20261101_163458
```

These stay outside this folder: the Unity project and its sim-loop package
(`unity/`, which dataset generation shares), the detector's training and its
datasets (`__object_detection__/`), and the plans at the repo's root. `synthetic-loop-dia.md` is the
design this folder's code follows.
