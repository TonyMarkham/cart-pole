# Orchestrator Plan — the stand-in for the physical world

The policy has to train against the world as the robot will meet it —
physics that keeps moving while the robot's computer is still thinking —
so the loop needs one participant that owns that world and its clock.
The **orchestrator** is that participant: MuJoCo and the synthetic
Arduino run inside it, it is the only participant that sends MuJoCo
anything, and it turns the Orin's processing time into simulated time.
It takes the trainer's start, command and continue/stop; it gives the
synthetic Orin the pose to render, the trainer the marker pool, and
MuJoCo the cart's speed; and it writes the truth log.

Status: **plan** (2026-10-23), written from `synthetic-loop-dia.md`
alone, the source of truth. Element numbers below (1–26) are the
diagram's messages.

Progress (2026-10-24, run on this PC; *Steps* has each check): steps
1–7 are done. The
code is `orchestrator/src/`; the trainer launches it with
`uv run --project trainer python trainer/src/naive_trainer.py`.

## Done when

1. **The order holds.** A run's log shows the diagram's messages in its
   order: the start (1–10), every observation (11–21), and the shutdown
   (22–26).
2. **The clock holds.** Every capture time lies on the 2 ms grid, every
   gap between two captures is 10, 12, 14 or 16 ms, and each new cart
   speed first acts from its arrival step on.
3. **The freeze holds.** MuJoCo is never stepped past an arrival step
   before the Orin's reply for that observation is in.
4. **Deterministic physics.** The same seed and the same trainer
   commands give the same ids, capture times, latencies and true cart
   and pole, bit for bit (the diagram's *Determinism*: the observed
   values jitter with Unity's SSAO noise, by decision).
5. **The truth log joins.** `states.csv` has one row per observation and
   joins `observations.jsonl` on the id.
6. **Both endings work.** The trainer's stop shuts the run down and the
   trainer is told it closed (26); a reply carrying `failed` shuts it
   down and the trainer is told the run failed and training ends (26).

## Requirements (from the diagram)

- **O1 — the world in one process.** MuJoCo and the synthetic Arduino
  run in-process; the orchestrator is the only participant that sends
  MuJoCo anything.
- **O2 — the start (1–10).** The trainer starts the run by launching the
  orchestrator, with the command in `trainer/settings.toml` (1). Then:
  load the MJCF the command names, with its joint and actuator names
  (the trainer's `[mjcf]`), and reset to `--start` (the pole's angle
  alone: the cart has no start value), at rest, with the
  cart at 50% of the rail and
  t = 0 (2); create the synthetic Arduino with `RIG` (3); launch the
  synthetic Orin with `--orchestrator ws://… --frames-listen ADDR --out
  DIR --seed S` (4) and wait for its control connection (5); launch the
  Unity player with `-orin ws://ADDR` (6); wait for the Orin's "Unity
  has registered" (8), the trigger to start; read MuJoCo's time and pose
  (9–10), the first capture state.
- **O3 — the clock.** MuJoCo runs at a fixed deltatime, its 2 ms
  timestep. At each capture state the orchestrator draws the Orin's
  latency, a seeded RND(MIN, MAX) ms (`--rnd-delay-min`,
  `--rnd-delay-max`), and caches the arrival time, the capture time plus
  the latency. Captures sit on the 2 ms grid, so the arrival step rounds
  the latency up: at 8–15 ms, gaps of 10, 12, 14 or 16 ms.
- **O4 — observe while physics steps (11–16).** The orchestrator sends
  the Orin `{"id": n, "time": t, "joints": [cart, pole]}` (11). While
  the Orin works, MuJoCo steps every 2 ms on the old command (15–16)
  until a step's time is later than the arrival time — the arrival step
  — and is frozen there until the Orin's reply (14) is in. Only one
  message to another process is in flight at any moment.
- **O5 — a failed run.** A reply carrying `failed` (3 failed frames
  among the pool's 6 entries) skips the rest and goes to the shutdown
  (22); the run fails and training ends.
- **O6 — the truth log.** One `states.csv` row per observation: id n,
  capture time t, the true cart (m) and pole (rad) at capture, and the
  4 observed `(x, y)` of the pool's newest entry, under the diagram's
  header (*Truth log*). The trainer never sees it. With `--no-render`
  there is no observation, so the eight observed fields are empty.
- **O7 — the trainer gets the pool, and nothing else (17).** It answers
  with the command (an `i32`, sign = direction, magnitude = pulses/s)
  and continue/stop (18). The link is the orchestrator's stdin and
  stdout (the diagram's *Trainer link*): one stdout line
  `{"id": n, "pool": [6 rows]}` out, one stdin line
  `{"command": c, "continue": bool}` in, with MuJoCo frozen at the
  arrival step until it is in. stdout carries the link alone: the
  orchestrator's own output, and its children's, goes to stderr. A
  command that does not fit an `i32`, or a line that is not the answer,
  fails the run.
- **O8 — continue (19–21).** The orchestrator hands the command to the
  synthetic Arduino (19), the only thing it is sent, and gets back the
  DRV8825 signals, DIR and STEP rate (20). Cart speed = STEP rate ×
  15 µm, in m/s, signed by DIR; it is set as MuJoCo's ctrl at the
  arrival step (21). The arrival step is also the next capture: the loop
  goes round.
- **O9 — stop and shutdown (22–26).** On the trainer's stop, or a
  failed run: close the control socket (22), the Orin closes the frame
  socket (23), the player and the Orin exit (24–25), and the trainer is
  told the run closed, or that it failed and training ends (26): the
  orchestrator exits, closing stdout, with code 0 (closed) or 1
  (failed).
- **O10 — determinism.** The latency draws come from `--seed`, the same
  seed the Orin's pre-warm gaps come from; the freeze keeps the
  workstation's speed out of simulated time. The physics repeats bit for
  bit; the observation jitters with Unity's SSAO noise (the diagram's
  *Determinism*).

## Configuration (user, 2026-10-23)

- **`orchestrator.toml` holds build inputs only.** Today that is
  `mujoco_dir`, the MuJoCo release tree the physics crate links; it is
  read by `physics/build.rs` and never at run time, and any other key
  fails the build. The MuJoCo a run uses is the one its binary was built
  with (`mujoco.dll` sits beside the binary).
- **Everything about a run comes from the command line**, which the
  trainer builds from its `trainer/settings.toml`: the MJCF and its
  names (`--model`, `--cart-joint`, `--pole-joint`, `--actuator`, from
  `[mjcf]`) and the rest of the command (`[orchestrator]`). The
  trainer launches the built binary, `target/release/orchestrator.exe`,
  never `cargo run`.
- **Each socket has its own port, on 127.0.0.1**, set in the
  trainer's `[orchestrator]` as `orin_port` and `unity_port` and passed
  as `--orin-port` (the control socket, 4–5: the orchestrator listens,
  the Orin connects) and `--unity-port` (the frame socket, 6–7: Unity
  connects; the Orin serves it, `--frames-listen 127.0.0.1:<port>`).
  The two must differ.
- **The run's seed** is the trainer's `[orchestrator]` `seed` (user,
  2026-10-23), passed as `--seed` and on to the Orin as `--seed` (4): it
  seeds the Orin's pre-warm gaps and the orchestrator's latency draws.
- **The latency band** is the trainer's `[orchestrator]`
  `rnd_delay_min` and `rnd_delay_max`, in ms (user, 2026-10-24; 8 and
  15 today), passed as `--rnd-delay-min` and `--rnd-delay-max` and on to
  the Orin under the same names (4): the band of the orchestrator's
  latency draws and of the Orin's pre-warm gaps. 0 ≤ min ≤ max.
- **The latency generator is SplitMix64, written in the orchestrator**
  (user, 2026-10-24): no dependency, a published algorithm with known
  test outputs, and a stream that never changes unless the code does.
  Seeded with `--seed`; each draw is one 64-bit output, its top 53 bits
  as u ∈ [0, 1), latency = min + (max − min)·u ms. The Orin's pre-warm generator is
  its own (Python's `random`), seeded the same way: each must repeat
  itself, not match the other.
- **A value given on the command line wins.** Where a setting has both a
  command-line argument and a config value, the config's value is
  ignored.
- **`--no-render` keeps its own stand-ins.** Without the Orin there is
  no pool, so no trainer link: `--frames` (how many states) and
  `--control` (a scripted speed) drive a physics-only run, and are
  refused without `--no-render`, where the trainer's answer drives the
  run.
- **The timestep and the integrator belong to the MJCF**, as its
  `<option>`, written by the Unity exporter from the rig's StepJointSet.
  The orchestrator neither sets nor overrides them.

## Not decided by the diagram

These are needed to build the orchestrator; per `AGENTS.md`, each lands
in the diagram first:

- **Where `--start` comes from** for a run: the trainer, or the
  orchestrator's own command line. (`--seed` is settled: the trainer's
  `settings.toml`, *Configuration*.)
- **Timeouts** — for the Orin's and the player's connections and for
  the Orin's reply.

## Steps

1. **Start** (O1, O2): the start sequence up to the first capture state.
   *Check:* Done-when 1, start part. **Done (2026-10-24):** a trainer
   run prints 1–10 in order — the control socket, the Orin launched with
   `--frames-listen` and `--seed`, its pre-warm and frame socket, its
   connection, the player launched with `-orin`, "Unity has registered",
   then the first capture at t = 0.
2. **Clock and observation** (O3, O4): latency draw, arrival step,
   stepping on the old command, the freeze. *Check:* Done-when 2–3.
   **Done (2026-10-24):** 500 observations give gaps of 10, 12, 14 and
   16 ms only (146 / 154 / 132 / 67, against 143 / 143 / 143 / 71 for
   the band), every capture on the 2 ms grid. The freeze holds by
   construction: `step_to` stops at the arrival step and the reply is
   awaited before anything steps again. The latency generator
   (SplitMix64, *Configuration*) matches the algorithm's published
   outputs for seed 0 (`orchestrator/src/tests.rs`). The band comes
   from `--rnd-delay-min` and `--rnd-delay-max` (2026-10-24): at the
   trainer's 8 and 15, a seed-0 run's 250 latencies and the Orin's
   pre-warm ages match the run before the change exactly; a 2–4 ms band
   gives 500 gaps, all 4 ms (every latency in [2, 4) arrives at the next
   step but one); an inverted band is refused.
3. **Truth log** (O6). *Check:* Done-when 5. **Done (2026-10-24):** 60
   rows join `observations.jsonl` on the id, the capture time and the
   newest entry bit for bit — after turning on `serde_json`'s
   `float_roundtrip`, whose default parse was 1 ulp off in 41 of 60.
4. **Trainer link** (O7). *Check:* each pool reaches the trainer and
   its answer drives the run. **Done (2026-10-24):** `trainer_link.rs`
   writes each pool as one stdout line and reads the answer from stdin,
   blocking in place while MuJoCo is frozen at the arrival step; every
   other line the orchestrator prints is on stderr, and the Orin's and
   the player's stdout go there too (their stdin is closed). A trainer
   run with the naive trainer's command 0: all 250 pools answered, the
   stop on the 250th, the shutdown in order, exit 0; the trainer's turn
   takes about 0.2 ms. With command 6,667 pulses/s over 60 observations,
   ctrl is +0.100 m/s and the cart moves at 0.0995 m/s over the last 10.
   `--frames` and `--control` are refused without `--no-render`
   (*Configuration*), whose physics-only runs leave stdout empty.
5. **Command path** (O8). *Check:* a constant command moves the cart at
   STEP rate × 15 µm. **Done (2026-10-24):** at the arrival step the
   command goes to the synthetic Arduino once; its DIR and STEP rate
   become the cart velocity, set as MuJoCo's ctrl there and left
   untouched while MuJoCo steps to the next arrival step (the reset
   zeroes ctrl, so the cart is at rest until the first arrival step).
   `--no-render` runs with `--control speed:±0.4`: the cart does not
   move before t = 0.016 s, the first arrival step; ctrl is ±0.400 m/s,
   and over the last 10 of 60 observations the cart moves at
   ±0.400349 m/s (the velocity servo, `kv` 100, against the swinging
   pole). `speed:5` is clamped to 45,333 pulses/s, ctrl 0.680 m/s. Two
   seed-3 runs of a sine command give byte-identical `states.csv`. Since
   the trainer link (step 4), the trainer's command drives the cart;
   the scripted `--control` remains only for `--no-render`.
6. **Endings** (O5, O9). *Check:* Done-when 6. **Done (2026-10-24)**,
   each ending checked once: the trainer's stop — the run shuts down in
   order and the orchestrator exits 0, which the trainer reports as "the
   run closed" (step 4's run); a failed run — with a stand-in camera
   sending broken frames, the third failed frame in the pool gives
   `failed`, the shutdown runs in order and the orchestrator exits 1
   (checked before the link; the trainer reading that 1 as "the run
   failed" has not been run).
7. **Determinism** (O10). *Check:* Done-when 4. **Done (2026-10-24):**
   two trainer runs with seed 0 give identical ids, capture times, true
   cart and pole, and `failed`. The frames differ (53 of 60), by
   decision: the cause is URP's SSAO in Blue Noise mode, whose noise
   index advances on every render (`SSAOUtils.AdvanceBlueNoiseIndex`),
   while the loop camera also renders the player's window between
   triggers. The detections differ by at most 0.04 px, and every marker
   is found in every frame; the user accepted that jitter (2026-10-24).

## Placeholders

- **The Orin's latency** — RND(8, 15) ms today (the trainer's
  `rnd_delay_min`, `rnd_delay_max`): the detection time measured on the
  Orin over 250 frames (2026-10-23, `detector-plan.md` *Results*: FP16
  8.4–10.4 ms, the Python path 11.9–14.5 ms). It stands for D =
  detection + policy + WiFi; the policy's inference and the WiFi hop to
  the Arduino are still to be measured.

## Not in this plan

More than one episode per run, an early stop on a truth-versus-
observation threshold, pooling (*Later — scaling*), and the trainer
itself (`trainer-plan.md`).

## Later — scaling (only after the simple loop works)

The orchestrator would hold a pool of Unity players and synthetic Orins
and cycle through them for throughput. To leave room for that, the
simple loop reaches every part by an address it is given, never by
assuming a single instance. Known consequence: the player keeps no
state between frames and pools freely; MuJoCo and the synthetic Arduino
hold one run's state; the synthetic Orin keeps the 6-entry pool, so a
loop must stay pinned to one synthetic Orin, or the pool must move with
the loop — decided when pooling arrives.

## References

- `synthetic-loop-dia.md` — the source of truth for this plan
- `mujoco-plan.md`, `synthetic-arduino-plan.md`,
  `synthetic-orin-plan.md`, `unity-player-plan.md`, `trainer-plan.md` —
  the other participants
