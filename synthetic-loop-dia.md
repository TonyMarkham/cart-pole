# Synthetic loop — sequence of one run (planned)

The policy will learn inside this loop, so this shows how one frame travels
from MuJoCo's truth to what the robot sees, through the trainer's policy,
and back to the motor. It is the **planned** sequence (decided 2026-10-23),
not today's code: today the player returns each frame to the orchestrator,
which forwards it to the synthetic Orin and steps a fixed
`--frame-interval-ms`.

The orchestrator stands in for the physical world: MuJoCo and the synthetic
Arduino are libraries inside it, and it is the only participant that sends
MuJoCo anything. The synthetic Orin is the robot's detector: it triggers the
Unity player, its camera, and replies to the orchestrator with the marker
pool only, never any timing. In production the policy runs on the Orin; in
the synthetic loop everything runs on this workstation, so the policy lives
in the trainer, which is where it is trained. The pose rides through the
Orin's sim edge to Unity; detection sees only pixels, and the policy only
the marker pool. Only the pool, the current state, reaches the trainer: the
trainer generates the reward or penalty and decides continue or stop
itself, and nothing in the control loop touches training. The trainer
starts each run by launching the orchestrator, with the command its
`trainer/settings.toml` names (1). Stop ends this
run, one episode for now; the trainer itself carries on and starts the
next run (1). A failed run (3 failed frames in the pool) is different: it
ends training, so the trainer is told the run failed (26) and starts no
further run. Only one message to another process is in flight at any
moment, and MuJoCo steps in-process while the Orin works, so nothing can
race.

MuJoCo's time is the clock, and it runs at a fixed deltatime: its 2 ms
timestep. When the orchestrator takes a capture state, it draws the Orin's
latency, a seeded RND(8, 15) ms, and caches the arrival time, the capture
time plus the latency. MuJoCo keeps stepping on the old command while the
Orin works, as the real Arduino keeps stepping until the next packet
lands. The observation reaches the trainer at the first step whose time is
later than the arrival time; if MuJoCo gets there before the Orin has
replied, the orchestrator freezes it until the reply is in. MuJoCo runs far
faster than real time, so in practice it waits at the arrival step on
nearly every observation, and the run goes as fast as the Orin's round
trip allows. At that same step the trainer's command takes effect and the
next capture goes out. Captures sit on the 2 ms grid, so the arrival step
rounds the latency up to the next step: the gap between two captures is
10, 12, 14 or 16 ms. The latency is drawn at random because the policy
cannot observe it directly: the range is the band it trains in.

Not yet in the loop: more than one episode per run.

```mermaid
sequenceDiagram
    autonumber
    box transparent Simulation timeframe
        participant M as MuJoCo (physics crate, in-process)
        participant O as Orchestrator (Rust)
    end
    participant T as Trainer (holds the policy being trained)
    box transparent Observation timeframe
        participant A as Synthetic Arduino (arduino crate, in-process)
        participant N as Synthetic Orin (Python + TensorRT, process)
        participant U as Unity player (process, the Orin's camera)
    end

    T->>O: start the run: launch the orchestrator with the command in trainer/settings.toml
    Note over M,O: Start: load the MJCF (path and names from the trainer's settings.toml [mjcf]),<br/>reset to --start (at rest, cart at 50% of the rail, t = 0)
    O->>M: load + reset(start pose)
    O->>A: new(RIG): the cart starts at 50% of the rail, guaranteed (stands in for homing)
    O->>N: launch --orchestrator ws://… --frames-listen ADDR --out DIR --seed S<br/>(S: the run's seed, the trainer's settings.toml [orchestrator] seed)
    Note over N: pre-warm: builds the 6-entry pool, slots (−1, −1),<br/>at times −d₁, −(d₁+d₂), …, −(d₁+…+d₆) before t = 0<br/>then opens the frame listener
    N-->>O: connects (control socket: the listener is up)
    O->>U: launch SimulationLoop.exe -orin ws://ADDR
    U-->>N: connects (frame socket)
    N-->>O: text {"type":"registered"}: Unity has registered (the frame socket is up)<br/>the observation side is ready: the trigger to start
    O->>M: time(), pose()
    M-->>O: t, qpos (cart m, pole rad): the first capture state

    loop every observation n, until the trainer says stop
        Note over O: capture state = MuJoCo's current t, qpos<br/>(the start state, then each arrival step's)<br/>arrival time = t + seeded RND(8, 15) ms, cached
        par the Orin observes
            O->>N: text {"id":n,"time":t,"joints":[cart, pole]}
            N->>U: text {"id":n,"joints":[cart, pole]} (the trigger)
            U-->>N: binary JPEG, 728×544 greyscale q90
            Note over N: frame does not decode or is not 728×544: a failed frame,<br/>4 × (−1, −1), the error logged<br/>3 failed frames among the pool's 6 entries: the run fails<br/>else letterbox, TensorRT FP16, slot rule:<br/>4 slots (x, y) or (−1, −1)<br/>push onto the pool at age 0 ms, every older entry ages<br/>by the time since the previous capture (the oldest drops)<br/>save frame_NNNNNN.jpg, append to observations.jsonl (with any error)
            N-->>O: id n and the pool, no timing:<br/>6 entries, newest first, each keyed by its age: −ms from the current<br/>capture time (newest = 0), 4 marker (x, y) each<br/>+ failed, when the run fails
        and MuJoCo steps on, still on the old command
            loop every fixed deltatime (2 ms), until a step's time is later than the arrival time
                O->>M: step
                M-->>O: t, qpos
            end
            Note over M: arrival step: frozen here until the Orin's reply is in
        end
        break the Orin reports failed
            Note over O: the run fails: go to the shutdown (22)
        end
        Note over O: states.csv row (the truth log, see Truth log):<br/>id n, capture time t, true cart (m) and pole (rad) at capture,<br/>the 4 observed (x, y) of the pool's newest entry
        O->>T: the pool, and nothing else
        Note over T: from the pool alone: reward or penalty for its last command<br/>(this pool is the next state after it), and continue or stop<br/>the Pivot marker out of bounds is the end crash:<br/>a training failure, penalty, and stop this run<br/>nothing to score on the first observation<br/>then ask the policy for the next command
        T-->>O: the command (i32, sign = direction, magnitude = pulses/s) + continue/stop
        alt continue
            O->>A: receive(the command: i32 LE pulses/s), the only thing it is sent
            Note over A: rate clamped to ±45,333 pulses/s<br/>no clock in the loop, so no pulse count:<br/>the ±30,000-pulse travel limit cannot act here<br/>(the end crash is the trainer's training failure)
            A-->>O: DRV8825 signals: DIR (from the sign), STEP rate (from the magnitude)
            Note over O: cart velocity = STEP rate × 15 µm, m/s, signed by DIR
            O->>M: set_ctrl(the new cart velocity) at the arrival step
            Note over O: the arrival step is also the next capture: the loop goes round
        else stop
            Note over O: skip the rest: go to the shutdown (22)
        end
    end

    O->>N: close the control socket (on the trainer's stop, or a failed run)
    N->>U: close the frame socket
    U-->>O: exits
    N-->>O: exits
    O-->>T: closed (the Orin and the player have exited), or the run failed: training ends
```

**The pool's size.** Three entries are the minimum for the current
position, speed and acceleration; the other three are redundancy, so three
valid entries survive when detections drop out to (−1, −1). A frame that
does not decode, or is not 728×544, is a failed frame: its entry is
4 × (−1, −1) and the error is logged. One failed frame costs about 15 ms
of detection and does not fail the run; 3 among the pool's 6 entries fail
it and end training. Pre-warm entries are no detection, not failed frames.

**Truth log.** `states.csv` pairs MuJoCo's real values at each capture
with what the Orin observed in that frame, one row per observation, for
evaluation after training. Columns: `id,time,cart,pole,left_x,left_y,
right_x,right_y,pivot_x,pivot_y,end_x,end_y` — the capture time (s), the
true cart (m) and pole (rad) at capture, and the pool's newest entry,
(−1, −1) where not detected. It joins `observations.jsonl` on `id`. The trainer never sees it, so nothing in the
control loop touches training. An early stop by the orchestrator on a
truth-versus-observation threshold is not in the loop yet.

**Scoring lag.** To the trainer, each pool is simply the next state after
its last command, and that is what it scores, as a Gymnasium step returns
the next state for the action it was given. Physically, that command has
not acted yet when the frame is taken: it is set at 21, at the arrival step
that is also the next capture, and drives the cart only during the
following steps (15–16). That delay is the real latency of the Orin's
object detection, the only perception in the loop, and the policy learns it
as part of the world, as it must on the real rig.

**The initial pool.** At startup, before any frame exists, the Orin
pre-warms the pool: six entries with no detection (−1, −1) at cumulative
times before the first capture (t = 0), each gap a fresh draw dₖ of
RND(8, 15) ms from the Orin's generator, seeded with `--seed`. These gaps
are not rounded to the 2 ms grid, unlike the live gaps; it barely
matters, since pre-warm entries hold no detection. Every frame, the first
included, is then one push: the newest entry goes in and the oldest
drops, so frame 0's pool ages are 0, −d₁, −(d₁+d₂), …, −(d₁+…+d₅).

**Determinism.** The orchestrator's latency draws and the Orin's pre-warm
gaps both come from `--seed`, and the freeze at the arrival step keeps the
workstation's speed out of the sim, so the same seed and the same trainer
commands give the same physics: the same ids, capture times, latencies and
true cart and pole, bit for bit. The observation is not bit-identical, by
decision (user, 2026-10-24): Unity's SSAO (blue noise) changes its noise
pattern on every render, and the player renders between triggers, so a
frame's shading jitters slightly from run to run. The detector found every
marker in every frame regardless; run to run, the detections differ by at
most 0.04 px (2026-10-24), and so do `states.csv`'s observed columns and
`observations.jsonl` (which also logs `detect_ms`, a wall-clock time).

Numbers: `arduino::RIG` (200 steps × 32 microsteps, 96 mm per pulley turn:
15 µm per pulse, ±30,000 pulses of travel, 45,333 pulses/s top rate). The
travel limit acts only on the board, whose timer counts the pulses it
sends; the synthetic Arduino receives only the i32, so in the loop the
MJCF slide's ±0.45 m range is what stops the cart. The end crash is
therefore a training failure, judged by the trainer from the pool: the
Pivot marker out of bounds means penalty, and stop this run. The bounds
are not set yet; they must sit inside the slide's ends by more than the
cart can travel between a capture and the stop taking effect, at most
0.68 m/s (45,333 pulses/s × 15 µm) × 16 ms (the longest gap to the
arrival step) ≈ 11 mm, or the simulated cart reaches the slide's end
first. The real board finds its centre by homing: left to the limit
switch, right to the limit switch, then back half the pulses between the
two hits. The synthetic run instead always starts with the cart at 50% of
the rail, a guarantee, so the Arduino is never told a start position. The
2 ms step is the MJCF's timestep, and the loop's fixed deltatime. The MJCF
states it, as `<option timestep="0.002" integrator="implicitfast"/>`,
written by the exporter from the rig's StepJointSet, never left to MuJoCo's
defaults: the timestep is both the physics step and the grid the latency
rounds to, and `implicitfast` is MuJoCo's recommended integrator for the
cart's `<velocity>` servo. The Orin latency's range,
8–15 ms, is actual object-detection inference time over 250 test images,
a placeholder for D = detection + policy + WiFi, to be measured on the
Orin (`orchestrator-plan.md`, *Placeholders*).

**Contracts** — what the messages carry, in full:

- **The pose (11, 12):** `joints` is `[{"name": the cart joint, "qpos":
  cart m}, {"name": the pole joint, "qpos": pole rad}]`, the MJCF joint
  names from the trainer's `[mjcf]` as the keys. The orchestrator sends
  `{"id", "time", "joints"}`; the Orin forwards `{"id", "joints"}`
  unchanged, and Unity drives each joint by its name.
- **The frame (13):** 728×544 greyscale JPEG, quality 90. Greyscale is
  the content, not the channel count: Unity's `GreyscaleEncoder` (the
  dataset generator's own) writes BT.601 luma into all three RGB
  channels, and the Orin decodes it to one channel. The size is exactly
  half the IMX296's native 1456×1088 (`DATASETS.md`, *Image spec*).
- **The marker frame (each pool entry):** 4 slots in class order — 0
  Left (tag 376), 1 Right (596), 2 Pivot (920), 3 End (937), the repo's
  class-id contract. Each slot is the box center `(x, y)`, normalized to
  [0, 1], or `(−1, −1)` when that class was not detected; −1 lies outside
  the valid range, so "not detected" can never be a real position. When
  a class has several boxes, the slot takes the highest-confidence one.
  `(−1, −1)` means *not detected*, whatever the cause: occluded, missed,
  out of frame, or a failed frame.
- **The pool (14, 17):** 6 entries × (1 age + 4 × `(x, y)`) = 54 floats.
  Ages are relative to the current capture so the input never depends on
  absolute clock values, and the policy sees the real spacing between
  frames. On the wire (14), one text message: `{"id": n, "pool":
  [[age_ms, x0, y0, x1, y1, x2, y2, x3, y3], …6 rows, newest first],
  "failed": false}` — row 0 is this frame, age 0; slot k is class k;
  `failed` is true when 3 of the pool's 6 entries are failed frames.
- **The command (18, 19):** 4 bytes, one `i32`, little-endian, in
  pulses per second; fixed length, no id; sign = direction, never a
  distance or position offset. Why 4 bytes: at 1/32 microstepping the
  top rate is 0.68 m/s ÷ 15 µm = 45,333 pulses/s, past an `i16`'s
  32,767; an `i32` never limits the microstep setting or the speed. One
  pulse per second is 0.015 mm/s, finer than the slowest motion a
  balance needs.
- **The cart speed (20, 21):** meters per pulse = belt travel per
  pulley turn ÷ (full steps per turn × microsteps) = 96 mm ÷ (200 × 32)
  = 15 µm. The 96 mm is the CAD pulley `pully-gt2-48t-6w-8b`: 48 teeth
  at GT2's 2 mm pitch. The MJCF's `ctrlrange` (±0.68 m/s) matches the
  top rate, 45,333 pulses/s × 15 µm.
