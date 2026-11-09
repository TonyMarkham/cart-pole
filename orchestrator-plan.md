# Orchestrator Plan — the stand-in for the physical world

The policy has to train against the world as the robot will meet it —
physics that keeps moving while the robot's computer is still thinking —
so the loop needs one participant that owns that world and its clock.
The **orchestrator** is that participant: MuJoCo and the synthetic
Arduino run inside it, it is the only participant that sends MuJoCo
anything, and it turns the Orin's processing time into simulated time.
It takes the trainer's start, command and continue, reset or stop; it gives the
synthetic Orin the pose to render, the trainer the marker pool, and
MuJoCo the cart's speed; and it writes the truth log.

Status: **plan** (2026-10-23), written from `synthetic-loop-dia.md`
alone, the source of truth. Element numbers below (1–26) are the
diagram's messages.

Progress (2026-10-24, run on this PC; *Steps* has each check): steps
1–7 are done. Steps 11–12 (the rig's timings, 2026-11-08) are built and
checked without the Orin, the player or the trainer. The
code is `__synthetic__/orchestrator/rust/crates/orchestrator/src/`; the trainer launches it (`train.py`,
`env_check.py`).

## Done when

1. **The order holds.** A run's log shows the diagram's messages in its
   order: the start (1–10), every observation (11–21), and the shutdown
   (22–26).
2. **The schedule holds** (the diagram's *Timing*, user 2026-11-08). Made
   at each reset from the episode's seed: steps −5 to
   `--max-episode-steps` + 14, five picks a step in the diagram's order,
   every time rounded to the nearest timestep, halfway going later. Every
   capture, send and motor change happens on its scheduled timestep, and
   a worked example checks by hand (the diagram's: picks 45.37, 23.88,
   51.27, 8.35 and 45.40 ms give a send at 70 ms, a motor change at 120
   ms and step 1's capture at 32 ms).
3. **The waits hold.** MuJoCo never moves on from a capture before the
   Orin's reply for it is in, nor from a send before the trainer's answer
   is in.
4. **Commands land or are gobbled.** A command takes effect on its motor
   change's timestep, the cart moving on it from the next timestep on,
   unless it is lost (no motor change) or gobbled (a newer step's command
   landed first); events on one timestep go in the diagram's order.
4a. **FAULT ends the episode.** A strike face within
   `--calibration-stop-gap` of its base after a timestep puts the
   synthetic Arduino in FAULT on that timestep's tick; the line owed to
   the trainer goes out at once with `"fault": true`, and only a reset or
   a stop may answer it.
4b. **Deterministic physics.** The same seed and the same trainer
   commands give the same ids, schedule, landed and gobbled commands and
   true cart and pole, bit for bit (the diagram's *Determinism*: the
   observed values jitter with Unity's SSAO noise, by decision).
5. **The truth log joins.** `states.csv` has one row per capture and
   joins `observations.jsonl` on the id.
6. **Both endings work.** The trainer's stop shuts the run down and the
   trainer is told it closed (26); a reply carrying `failed` shuts it
   down and the trainer is told the run failed and training ends (26).
7. **A reset is a fresh start in place.** After a reset with seed S, the
   episode's schedule, landed commands and true cart and pole match a
   fresh run with seed S bit for bit, its ids offset by the captures
   before it; the Orin and the player are launched once for the whole
   run.

## Requirements (from the diagram)

- **O1 — the world in one process.** MuJoCo and the synthetic Arduino
  run in-process; the orchestrator is the only participant that sends
  MuJoCo anything.
- **O2a — the calibration (C1–C10, user 2026-11-01).** Before anything in O2, one Orin
  (`--calibrate FILE`) and one player, on instance 0's ports, measure the seven poses
  the orchestrator reads from MuJoCo, and exit. `FILE` is `homography.json` in `--out`.
  A failed fit fails the run. `src/calibration/` holds the poses, and `calibrate` in
  `src/main.rs` the sequence.
- **O2 — the start (1–10).** The trainer starts the run by launching the
  orchestrator, with the command in `__synthetic__/trainer/python/settings.toml` (1). Then:
  load the MJCF the command names, with its joint and actuator names
  (the trainer's `[mjcf]`), and reset to the start: at rest, the pole
  hanging, θ = 0 (user, 2026-10-24; `--start`'s default, which the
  trainer does not pass), the cart at 50% of the rail and
  t = 0 (2); create the synthetic Arduino with its `Config` (3): the
  drive figures from `--full-steps-per-rev`, `--microsteps` and
  `--belt-um-per-rev`, the jump limit m from `--jump-limit` (the
  trainer's `[drive]`) and the cart top speed,
  the MJCF actuator's `ctrlrange` read from the loaded model; launch the
  synthetic Orin with `--orchestrator ws://… --frames-listen ADDR --out
  DIR --pool-gaps d₁,…,d₆ --homography FILE` (4), the gaps from the
  first episode's schedule (O3, *The initial pool*), and wait for its control connection (5); launch the
  Unity player with `-orin ws://ADDR` (6); wait for the Orin's "Unity
  has registered" (8), the trigger to start; read MuJoCo's time and pose
  (9–10), step 0's capture state at t = 0.
- **O3 — the schedule** (the diagram's *Timing*, user 2026-11-08). MuJoCo
  runs at a fixed deltatime, its 2 ms timestep, and the orchestrator alone
  decides when anything happens. At each reset (and before the run's first
  episode, from `--seed`) it makes the episode's schedule from the
  episode's seed: steps −5 to `--max-episode-steps` + 14 (520 at 500),
  and for each in turn, from −5 up, five picks from the one generator:
  stretch 1 RND(`--frame-ready-min`, `--frame-ready-max`), stretch 2
  RND(`--to-uno-min`, `--to-uno-max`), stretch 3
  RND(`--change-motor-min`, `--change-motor-max`), the wait
  RND(`--next-frame-ready-min`, `--next-frame-ready-max`), all ms, and the
  drop, a whole number from 1 to `--drop-one-message-in`, lost when it
  comes out 1. Each step k's times, as a running total of the exact picks
  in ms from step 0's capture: picture ready, then capture = picture ready
  − stretch 1, send = picture ready + stretch 2, motor change = send +
  stretch 3 (none if lost), and step k + 1's picture ready = send + the
  wait; step 0's capture is 0. Each entry is rounded to the nearest whole
  timestep as it is written, halfway going later. Nothing is picked once
  the episode runs. The bands must keep the diagram's orders, or the run
  is refused at load: each stretch's min at least one timestep (a step's
  own timings never share a timestep), `frame_ready_min − next_frame_ready_max`
  at least one timestep (the next capture comes before this step's send),
  and `frame_ready_min + to_uno_min + next_frame_ready_min −
  frame_ready_max` at least one timestep (captures stay in order).
- **O4 — the walk (11–21; the diagram's *Events on a shared timestep*).**
  MuJoCo moves through the episode one timestep at a time. On t = 0, step
  0's capture. Then on each timestep, in this order: MuJoCo moves on
  (15–16) and the four strike faces are read; the motor changes due on it,
  in step order, each landing or gobbled (O8); the send due on it, if any
  (17–18, O7); the synthetic Arduino's tick (19–21, O8); the capture due
  on it, if any. At a capture the orchestrator reads the capture state
  (as 9–10), sends the Orin `{"id": n, "time": t, "joints": [cart,
  pole]}` (11), waits on that timestep for its reply (14), logs the truth
  row (O6), and keeps the capture state and pool until the step's send.
  Within an instance only one message to another process is in flight at
  any moment.
- **O5 — a failed run.** A reply carrying `failed` (3 failed frames
  among the pool's 6 entries) skips the rest and goes to the shutdown
  (22); the run fails and training ends.
- **O6 — the truth log.** One `states.csv` row per observation: the
  episode (0, one more at each reset), id n, capture time t (from 0 in
  each episode), the true cart (m) and pole (rad) at capture, and the
  4 observed `(x, y)` of the pool's newest entry, under the diagram's
  header (*Truth log*). The trainer never sees it. With `--no-render`
  there is no observation, so the eight observed fields are empty.
- **O7 — the trainer gets the pool, and the capture's sites for the
  reward (17).** Each pool entry carries the twelve site positions read at
  that capture's step with its pose, and held under its id while MuJoCo
  steps on (`synthetic-loop-dia.md`, *Contracts*, the sites), the
  capture's MuJoCo time, and the pole's state from the same step: its
  speed, angle, centre of mass and hinge point (*Contracts*, the pole,
  user 2026-10-28). The run's values carry the pole's body once: its
  mass, centre-of-mass distance from the hinge, inertia about the hinge,
  and gravity (*Contracts*, the run's values). `--run-values` loads the
  model, prints that same run's-values line on stdout and exits, with no
  Orin and no player: MuJoCo's values for a test that needs them without
  the loop (user, 2026-10-28: MuJoCo is the source of truth). The line
  goes out at the step's send, and carries `"fault": false` (O8a). It answers
  with the command (an `i32`, sign = direction, magnitude = pulses/s)
  and continue, reset or stop (18). The link is the orchestrator's stdin and
  stdout (the diagram's *Trainer link*): one stdout line
  `{"id": n, "pool": [6 rows]}` out, one stdin line
  `{"command": c, "next": "continue" | "reset" | "stop"}` in, with
  `"seed": S` on a reset only, with MuJoCo frozen at the
  step's send until it is in. stdout carries the link alone: the
  orchestrator's own output, and its children's, goes to stderr. A
  command that does not fit an `i32`, or a line that is not the answer,
  fails the run.
- **O8 — continue: commands in flight, the gobble rule and the tick
  (19–21).** On continue the step's command is in flight until its motor
  change, from the schedule; a lost one has none. The orchestrator keeps
  the last landed step, none at a reset. When a motor change comes due:
  if its step is later than the last landed step it lands, and becomes
  the last landed step; otherwise it is gobbled. Every timestep the
  orchestrator gives the synthetic Arduino one tick (19): the command that
  landed on that timestep, if one did (the last to land, when several
  share it), and the two limit switches, each closed when its strike-face
  gap (Left Cart.x − Left Rail.x, Right Rail.x − Right Cart.x) is at most
  `--calibration-stop-gap`. It gets back the DM542 signals, DIR and STEP
  rate after the tick's ramp, and whether it is in FAULT (20). Cart speed =
  STEP rate × metres per pulse (`[drive]`), in m/s, signed by DIR, set as
  MuJoCo's ctrl (21), so MuJoCo moves on it from that timestep to the
  next. Nothing happens between the schedule's events: no render, no
  detection and no message.
- **O8a — FAULT ends the episode** (the diagram's *The rail's ends*). When
  the tick answers FAULT, the walk stops on that timestep and the line
  owed to the trainer goes out at once (17): the pool and capture state of
  the step after the last one sent, already captured, with `"fault":
  true`. A continue in answer to it fails the run; a reset or a stop is
  handled as usual.
- **O9 — stop and shutdown (22–26).** On the trainer's stop, or a
  failed run: close the control socket (22) with the close handshake
  (send the close frame, read until the Orin's close frame answers it,
  then drop the connection), the Orin closes the frame
  socket (23), the player and the Orin exit (24–25), and the trainer is
  told the run closed, or that it failed and training ends (26): the
  orchestrator exits, closing stdout, with code 0 (closed) or 1
  (failed).
- **O10 — determinism.** Every random timing comes from one generator
  seeded with the episode's seed, all of it at the reset: the schedule's
  2,600 picks (at 500 steps). The waits at captures and sends keep the
  workstation's speed out of simulated time. The physics repeats bit for
  bit; the observation jitters with Unity's SSAO noise (the diagram's
  *Determinism*).
- **O10a — the pool's pre-warm** (the diagram's *The initial pool*). The
  gaps the Orin is given, at launch (4) and at each reset (R5), are d₁ =
  0 − C₋₁, d₂ = C₋₁ − C₋₂, …, d₅ = C₋₄ − C₋₅ from the schedule's
  rounded captures Cₖ, in ms, and d₆ = d₅, which never reaches the policy.
- **O11 — reset in place (R1–R7)** (the diagram's *Reset in place*;
  user, 2026-10-24). On the trainer's reset with seed S: count one more
  episode (R1); reset MuJoCo to the start pose, t = 0, ctrl 0 (R2); the
  synthetic Arduino goes back to rest, sent rate and command 0, out of
  FAULT (R3); make the episode's schedule from S, set the last landed step
  to none and drop every command in flight and every capture not yet sent
  (R4); send the Orin `{"type": "reset", "gaps": [d₁, …, d₆]}` (R5, O10a)
  and wait for its `{"type": "reset"}` (R7); then the walk starts again
  from t = 0, step 0's capture. Ids keep counting across the run; nothing
  is launched or closed.

## Configuration (user, 2026-10-23)

- **`orchestrator.toml` holds build inputs only.** Today that is
  `mujoco_dir`, the MuJoCo release tree the physics crate links; it is
  read by `__synthetic__/orchestrator/rust/crates/physics/build.rs` and never at run time, and any other key
  fails the build. The MuJoCo a run uses is the one its binary was built
  with (`mujoco.dll` sits beside the binary).
- **Everything about a run comes from the command line**, which the
  trainer builds from its `__synthetic__/trainer/python/settings.toml`: the MJCF and its
  names (`--model`, `--cart-joint`, `--pole-joint`, `--actuator`, and
  the twelve sites' `--left-site`, `--right-site`, `--pivot-site`,
  `--end-site`, `--target-site`, `--dynamic-target-site`, `--cart-site`,
  `--cart-target-site`, `--left-rail-site`, `--left-cart-site`,
  `--right-rail-site`, `--right-cart-site`, from `[mjcf]`) and the rest of the
  command (`[orchestrator]`). The
  trainer launches the built binary, `target/release/orchestrator.exe`,
  never `cargo run`.
- **Each socket has its own port, on 127.0.0.1**, set in the
  trainer's `[orchestrator]` as `orin_port` and `unity_port` and passed
  as `--orin-port` (the control socket, 4–5: the orchestrator listens,
  the Orin connects) and `--unity-port` (the frame socket, 6–7: Unity
  connects; the Orin serves it, `--frames-listen 127.0.0.1:<port>`).
  The two must differ.
- **The run's seed** is the trainer's `[orchestrator]` `seed` (user,
  2026-10-23), passed as `--seed`: it seeds the first episode's schedule,
  and each reset brings the next episode's seed. The Orin is not told it
  (user, 2026-10-30): it picks nothing.
- **The random timings** (user, 2026-11-08) are the trainer's
  `[orchestrator]` `frame_ready_min`/`_max` (stretch 1, 43 and 48 ms),
  `to_uno_min`/`_max` (stretch 2, 20.5 and 27.8 ms),
  `change_motor_min`/`_max` (stretch 3, 3.6 and 123.3 ms),
  `next_frame_ready_min`/`_max` (the wait, 0 and 16.7 ms) and
  `drop_one_message_in` (3,000), each passed as a flag of the same name
  with dashes (`--frame-ready-min`, …). Each band 0 ≤ min ≤ max, finite;
  `--drop-one-message-in` a whole number at least 1; and the orders of
  O3. They replace `--rnd-delay-min`, `--rnd-delay-max`,
  `--command-rate-min` and `--command-rate-max`.
- **The schedule's length** is the gym file's `[env] max_episode_steps`,
  passed as `--max-episode-steps` (at least 1): 5 + it + 15 steps. The 5
  pre-warm steps are the pool's 6 entries less step 0's own, and the 15
  spare steps the user's choice (2026-11-08): fixed facts, not settings.
  An episode that would need a step past the schedule fails the run.
- **The switches' trip gap** is `--calibration-stop-gap`, the same
  setting as the calibration's end stops (the diagram's *The rail's
  ends*).
- **The timing generator is SplitMix64, written in the orchestrator**
  (user, 2026-10-24): no dependency, a published algorithm with known
  test outputs, and a stream that never changes unless the code does.
  Seeded with the episode's seed; each pick is one 64-bit output, its top
  53 bits as u ∈ [0, 1), a value = min + (max − min)·u of its band, and
  the drop 1 + ⌊N·u⌋, lost when it is 1 (u < 1 ÷ N). One generator serves
  every pick, in the schedule's order.
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
   **Done (2026-10-24, before the command rate; on the rate, 2026-10-30:
   2,000 observations at 20–30 Hz give gaps of 34 to 50 ms only, mean 41.57
   ms, and two runs with one seed write identical `states.csv` files):**
   500 observations give gaps of 10, 12, 14 and
   16 ms only (146 / 154 / 132 / 67, against 143 / 143 / 143 / 71 for
   the band), every capture on the 2 ms grid. The freeze holds by
   construction: `step_to` stops at the arrival step and the reply is
   awaited before anything steps again. The latency generator
   (SplitMix64, *Configuration*) matches the algorithm's published
   outputs for seed 0 (`__synthetic__/orchestrator/rust/crates/orchestrator/src/tests.rs`). The band comes
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
   STEP rate × metres per pulse. The ramp (added 2026-10-28, the
   diagram's *The acceleration ceiling*): a jump in the command reaches
   ctrl at no more than m × 2 ms a step, and a reset leaves ctrl and the
   ramp at 0; the 2026-10-24 record below predates it. **Done (2026-10-24):** at the arrival step the
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
8. **Reset in place** (O11, O6, O7). *Check:* Done-when 7: a run of
   two episodes, the second after a reset with seed S, against a fresh
   run with seed S. **Done (2026-10-24):** the answer is `{"command",
   "next", "seed"}` (`trainer_link.rs`, tests
   `the_answer_is_continue_reset_or_stop` and `a_bad_answer_is_refused`);
   a reset resets MuJoCo to the start, reseeds the latency draws, sends
   the Orin the reset (then its seed, now its gaps) and waits for its reply, and
   `states.csv` has the `episode` column. A run of 150 steps on seed 5
   then a reset to seed 7 and 150 more, against a fresh seed-7 run with
   the same commands: the reset episode's 151 rows match the fresh run's
   time, cart and pole exactly, starting at t = 0, cart 0, pole 0, with
   ids 151–301 against 0–150; one launch of the Orin and the player.

9. **The windows' grid** (the diagram's *Training instances*, *The
   windows*; user, 2026-10-24; sized to fill the screen, user, 2026-10-31).
   With `--instance-rows R`, `--instance-columns C` (each default 1, R × C
   at most 12), `--screen-width W`, `--screen-height H` (required together)
   and `--window-origin x,y` (default 0,0), a window is ⌊W ÷ C⌋ wide and
   ⌊H ÷ R⌋ tall, and each player is launched borderless (`-popupwindow`;
   user, 2026-10-31) with that size as `-screen-width` and `-screen-height`,
   and `-window-position x,y`, x = origin x + (k mod C) × width,
   y = origin y + ⌊k ÷ C⌋ × height. A window under 1 pixel is refused
   before anything launches. Without `--screen-width`, none of the four is
   passed. *Check:* unit tests of the sizes and the positions, and a run
   whose windows fill the screen (the latter needs the player's
   `PlayerController`, `unity-player-plan.md` step 10).
10. **The close handshake** (O9; the diagram's note at 22; user,
    2026-10-24, after run 1's 400,000–600,000 steps ended with all six
    Orins exiting 1, `trainer-observations.md`). The shutdown sends every
    Orin its close frame first, then reads each control socket until the
    Orin's close frame comes back, or the socket ends, or the grace
    period (`CLIENT_EXIT_GRACE`, 5 s) passes; only then is the socket
    dropped. Anything else read meanwhile, such as a keepalive ping, is
    passed over. *Check:* a test with a client that pings just before the
    close: it must receive the close frame and answer it; then a run's
    Orins exiting 0. **Done (2026-10-24):** `await_close` in `main.rs`;
    the test `the_close_frame_reaches_a_client_whose_ping_is_unread`
    passes, and run without `await_close` it fails 3 of 3 with os error
    10054, the Orins' reset; run 1's 600,000–1,100,000 session ended
    with all six Orins exiting 0.
11. **The schedule** (O3, O10, O10a; user, 2026-11-08). The five picks a
    step in order, the running totals, the rounding, the pre-warm gaps,
    the bands' checks; the new flags in place of the latency and the
    command rate. *Check:* unit tests against values worked out by hand:
    the diagram's worked example; halfway times going later, negative
    ones included; the pick order; a lost command; the pre-warm gaps; each
    refused band. Steps 2, 5 and 7's records above are of the old clock,
    the latency and the command rate, which this step replaces.
12. **The walk** (O4, O8, O8a, O11). The per-timestep order, the waits,
    commands in flight, the gobble rule, the tick with the switches, the
    FAULT line and the reset. *Check:* unit tests on a stand-in world,
    no MuJoCo, against a hand-made schedule: the events' order on a shared
    timestep, a landed and a gobbled command, a lost one, FAULT's line and
    its refused continue, and a run past the schedule refused. Then a run
    through the trainer (not done by the agent: Python and Unity are the
    user's to run).
    **Built (2026-11-08), steps 11 and 12:** `src/timing/`, `src/schedule/`,
    `src/walk/`, `src/simulation/` and `src/instance/`, the crate laid out
    one type a file, in folders, after `__live__/arduino/rust/crates/controller`.
    The 38 tests in `src/tests.rs` pass, among them the hand-worked
    schedule and walk tests above, and `cargo clippy --all-targets --no-deps`
    is clean. Two `--no-render` runs on seed 0 with the settings' bands: 500
    steps at rest gave 501 captures, every gap a whole even number of ms from
    18 to 46 ms, mean 32.25 ms, sends 20 to 44 ms apart, the last send at
    16.162 s, and of the 500 commands 359 landed, 139 were gobbled and none
    lost; driven right at 0.6 m/s the cart tripped the switch at 0.872 s,
    after 25 steps. Not run: the loop through the trainer, with the Orin and
    the player.

## Placeholders

None for the timings since 2026-11-08: the bands are the rig's measured
gaps (`real-timings.md`, `real-timing-conclusion.md`). The Orin's latency,
RND(8, 15) ms, the detection time alone, was the placeholder until then.

## Not in this plan

More than one loop at a time, an early stop on a truth-versus-
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
*Superseded (the diagram's *Training instances*, user, 2026-10-24):* no
cycling. The orchestrator owns a fixed group of training instances, each
a bundle of one MuJoCo, one synthetic Arduino, one synthetic Orin and one
Unity player that stays together, so the Orin's pool never leaves its
loop. `[orchestrator] instance_rows` × `instance_columns`, at most 12, sets
the group's size per run (`instances` until 2026-10-31, user);
instance k uses ports `orin_port + 2k` and `unity_port + 2k`.

**Built (2026-10-24, step 9).** `--instances N` (1 to 12 since 2026-10-24, first 1 to 6; default 1; since 2026-10-31 `--instance-rows` × `--instance-columns`);
`main.rs` holds one `Instance` per bundle (its `Simulation`, Orin, player,
current observation) and runs the diagram's rounds: every advanced
instance's pose goes out first, then each steps to its arrival step and
collects its reply; the pools go out in one line and the answers name
which instances advance next (`trainer_link.rs`, tests
`an_answer_is_continue_reset_or_stop`,
`a_line_may_answer_some_pending_instances`, `a_bad_answer_is_refused`).
Instance k's seed is `--seed + k`, its folder `--out/instance_k/`; every
port of the group must be its own. `--no-render` runs one instance.
Since step 12 (2026-11-08) a round walks every advanced instance at once,
each to its next send or a FAULT, each waiting at its own captures for its
own Orin (the diagram's *Training instances*, *Rounds*); the record below
is of the old rounds. Checked with the user's rebuilt player (Force Single Instance off): 4
instances, instance k driven by a sine of amplitude (k + 1) × 0.2, against
four 1-instance runs on the same seeds and commands: every instance's 151
rows match its 1-instance run exactly on id, time, cart and pole, the
carts reaching 0.067, 0.135, 0.199 and 0.276 m; one launch, 4 players
registered. 150 steps took 55.8 ms a step for 4 instances (72 samples/s)
against 23.2 ms for one (43 samples/s). Found on the way: when the
orchestrator is killed, or fails before every Orin is connected, the
Orin's `uv run` is killed but its Python child is not, and it keeps its
frame port (three were left holding 9001, 9005 and 9007).

## References

- `synthetic-loop-dia.md` — the source of truth for this plan
- `mujoco-plan.md`, `synthetic-arduino-plan.md`,
  `synthetic-orin-plan.md`, `unity-player-plan.md`, `trainer-plan.md` —
  the other participants
