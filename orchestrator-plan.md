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

## Done when

1. **The order holds.** A run's log shows the diagram's messages in its
   order: the start (1–10), every observation (11–21), and the shutdown
   (22–26).
2. **The clock holds.** Every capture time lies on the 2 ms grid, every
   gap between two captures is 10, 12, 14 or 16 ms, and each new cart
   speed first acts from its arrival step on.
3. **The freeze holds.** MuJoCo is never stepped past an arrival step
   before the Orin's reply for that observation is in.
4. **Deterministic.** The same seed and the same trainer commands give
   byte-identical `states.csv` and `observations.jsonl`.
5. **The truth log joins.** `states.csv` has one row per observation and
   joins `observations.jsonl` on the id.
6. **Both endings work.** The trainer's stop shuts the run down and the
   trainer is told it closed (26); a reply carrying `failed` shuts it
   down and the trainer is told the run failed and training ends (26).

## Requirements (from the diagram)

- **O1 — the world in one process.** MuJoCo and the synthetic Arduino
  run in-process; the orchestrator is the only participant that sends
  MuJoCo anything.
- **O2 — the start (1–10).** On the trainer's start (1): load the MJCF
  and reset to `--start`, at rest, with the cart at 50% of the rail and
  t = 0 (2); create the synthetic Arduino with `RIG` (3); launch the
  synthetic Orin with `--orchestrator ws://… --frames-listen ADDR --out
  DIR --seed S` (4) and wait for its control connection (5); launch the
  Unity player with `-orin ws://ADDR` (6); wait for the Orin's "Unity
  has registered" (8), the trigger to start; read MuJoCo's time and pose
  (9–10), the first capture state.
- **O3 — the clock.** MuJoCo runs at a fixed deltatime, its 2 ms
  timestep. At each capture state the orchestrator draws the Orin's
  latency, a seeded RND(8, 15) ms, and caches the arrival time, the
  capture time plus the latency. Captures sit on the 2 ms grid, so the
  arrival step rounds the latency up: gaps of 10, 12, 14 or 16 ms.
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
  4 observed `(x, y)` of the pool's newest entry. The trainer never sees
  it.
- **O7 — the trainer gets the pool, and nothing else (17).** It answers
  with the command (an `i32`, sign = direction, magnitude = pulses/s)
  and continue/stop (18).
- **O8 — continue (19–21).** The orchestrator hands the command to the
  synthetic Arduino (19), the only thing it is sent, and gets back the
  DRV8825 signals, DIR and STEP rate (20). Cart speed = STEP rate ×
  15 µm, in m/s, signed by DIR; it is set as MuJoCo's ctrl at the
  arrival step (21). The arrival step is also the next capture: the loop
  goes round.
- **O9 — stop and shutdown (22–26).** On the trainer's stop, or a
  failed run: close the control socket (22), the Orin closes the frame
  socket (23), the player and the Orin exit (24–25), and the trainer is
  told the run closed, or that it failed and training ends (26).
- **O10 — determinism.** The latency draws come from `--seed`, the same
  seed the Orin's pre-warm gaps come from; the freeze keeps the
  workstation's speed out of simulated time.

## Not decided by the diagram

These are needed to build the orchestrator; per `AGENTS.md`, each lands
in the diagram first:

- **The trainer's link** (1, 17, 18, 26): its transport, and whether
  the trainer launches the orchestrator for each run or connects to one
  already running.
- **Where `--start` and `--seed` come from** for a run: the trainer, or
  the orchestrator's own command line.
- **The latency generator** — which seeded generator, given that the
  Orin draws its pre-warm gaps from the same seed in its own process.
- **Timeouts** — for the Orin's and the player's connections and for
  the Orin's reply.

## Steps

1. **Start** (O1, O2): the start sequence up to the first capture state.
   *Check:* Done-when 1, start part.
2. **Clock and observation** (O3, O4): latency draw, arrival step,
   stepping on the old command, the freeze. *Check:* Done-when 2–3.
3. **Truth log** (O6). *Check:* Done-when 5.
4. **Trainer link** (O7), once its link is in the diagram.
5. **Command path** (O8). *Check:* a constant command moves the cart at
   STEP rate × 15 µm.
6. **Endings** (O5, O9). *Check:* Done-when 6.
7. **Determinism** (O10). *Check:* Done-when 4.

## Placeholders

- **The Orin's latency** — RND(8, 15) ms, the Orin's measured detection
  time over 250 test images, stands in for D = detection + policy +
  WiFi, to be measured on the Orin.

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
