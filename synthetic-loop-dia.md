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
`trainer/settings.toml` names (1). One run holds many episodes (user,
2026-10-24): between episodes the trainer answers reset, which puts the
run back at the start in place, without relaunching the orchestrator, the
Orin or the player (*Reset in place*); stop ends the run. A failed run (3 failed frames in the pool) is different: it
ends training, so the trainer is told the run failed (26) and starts no
further run. Only one message to another process is in flight at any
moment, and MuJoCo steps in-process while the Orin works, so nothing can
race.

MuJoCo's time is the clock, and it runs at a fixed deltatime: its 2 ms
timestep. When the orchestrator takes a capture state, it draws the Orin's
latency, a seeded RND(MIN, MAX) ms, and caches the arrival time, the
capture time plus the latency. MIN and MAX are the trainer's
`settings.toml` `[orchestrator]` `rnd_delay_min` and `rnd_delay_max`
(user, 2026-10-24), 8 and 15 today, passed to the orchestrator and on to
the Orin (4). MuJoCo keeps stepping on the old command while the
Orin works, as the real Arduino keeps stepping until the next packet
lands. The observation reaches the trainer at the first step whose time is
later than the arrival time; if MuJoCo gets there before the Orin has
replied, the orchestrator freezes it until the reply is in. MuJoCo runs far
faster than real time, so in practice it waits at the arrival step on
nearly every observation, and the run goes as fast as the Orin's round
trip allows. At that same step the trainer's command takes effect and the
next capture goes out. Captures sit on the 2 ms grid, so the arrival step
rounds the latency up to the next step: at 8–15 ms, the gap between two
captures is 10, 12, 14 or 16 ms. The latency is drawn at random because the policy
cannot observe it directly: the range is the band it trains in.

**Training instances** (planned, user, 2026-10-24; built after reset in
place, *Reset in place*). Data comes faster from several copies of the
loop at once, so the orchestrator will run a fixed group of **training
instances**, each a bundle that is always together: one MuJoCo, one
synthetic Arduino, one synthetic Orin and one Unity player, the Orin's
pool being that MuJoCo's history and the player that Orin's camera. One
orchestrator owns the whole group, so one launch starts it, one place
lays out the windows, and one stop or failure shuts it all down; one
orchestrator per instance would need something above them doing the
same. The number of instances is a setting per run, `instances`, 1 to
6 (user, 2026-10-24), in the trainer's `settings.toml` `[orchestrator]`.
Instance k (from 0) uses the ports `orin_port + 2k` and `unity_port + 2k`.
The instances step together, each step waiting for the slowest, and the
trainer link carries one pool per instance out and one answer per
instance in, each instance continuing or resetting on its own. This
sequence shows one instance; the group's details (the link's lines, the
logs, the window layout) land here before they are built.

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

    T->>O: start the run: launch the orchestrator with the command in trainer/settings.toml<br/>its stdin and stdout, the trainer's pipes, are the trainer link (17, 18)
    Note over M,O: Start: load the MJCF (path and names from the trainer's settings.toml [mjcf]),<br/>reset to the start: at rest, cart at 50% of the rail, the pole hanging (θ = 0), t = 0
    O->>M: load + reset(start pose)
    O->>A: new(RIG): the cart starts at 50% of the rail, guaranteed (stands in for homing)
    O->>N: launch --orchestrator ws://… --frames-listen ADDR --out DIR --seed S<br/>--rnd-delay-min MIN --rnd-delay-max MAX<br/>(the trainer's settings.toml [orchestrator] seed, rnd_delay_min, rnd_delay_max)
    Note over N: pre-warm: builds the 6-entry pool, slots (−1, −1),<br/>at times −d₁, −(d₁+d₂), …, −(d₁+…+d₆) before t = 0<br/>each dₖ a seeded RND(MIN, MAX) ms<br/>then opens the frame listener
    N-->>O: connects (control socket: the listener is up)
    O->>U: launch SimulationLoop.exe -orin ws://ADDR
    U-->>N: connects (frame socket)
    N-->>O: text {"type":"registered"}: Unity has registered (the frame socket is up)<br/>the observation side is ready: the trigger to start
    O->>M: time(), pose()
    M-->>O: t, qpos (cart m, pole rad): the first capture state

    loop every observation n (counted across the run's episodes), until the trainer says stop
        Note over O: capture state = MuJoCo's current t, qpos<br/>(the start state, then each arrival step's)<br/>arrival time = t + seeded RND(MIN, MAX) ms, cached
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
        O->>T: stdout line {"id":n,"pool":[…6 rows]}: the pool, and nothing else
        Note over T: from the pool alone: reward or penalty for its last command<br/>(this pool is the next state after it), and continue, reset or stop<br/>the Pivot marker out of bounds is the end crash:<br/>a training failure, penalty, and end this episode<br/>the time limit also ends it: reset for the next, or stop<br/>nothing to score on an episode's first observation<br/>then ask the policy for the next command
        T-->>O: stdin line {"command":c,"next":"continue"|"reset"|"stop"}<br/>+ "seed":S on a reset, the next episode's seed<br/>c: the command (i32, sign = direction, magnitude = pulses/s)<br/>MuJoCo stays frozen at the arrival step until it is in
        alt continue
            O->>A: receive(the command: i32 LE pulses/s), the only thing it is sent
            Note over A: rate clamped to ±56,666 pulses/s<br/>no clock in the loop, so no pulse count:<br/>the ±30,000-pulse travel limit cannot act here<br/>(the end crash is the trainer's training failure)
            A-->>O: DRV8825 signals: DIR (from the sign), STEP rate (from the magnitude)
            Note over O: cart velocity = STEP rate × 15 µm, m/s, signed by DIR
            O->>M: set_ctrl(the new cart velocity) at the arrival step
            Note over O: the arrival step is also the next capture: the loop goes round
        else reset
            Note over O: the episode ends: Reset in place (R1–R7),<br/>then the loop goes round from the new episode's first capture
        else stop
            Note over O: skip the rest: go to the shutdown (22)
        end
    end

    O->>N: close the control socket (on the trainer's stop, or a failed run)
    N->>U: close the frame socket
    U-->>O: exits
    N-->>O: exits
    O-->>T: exits, closing stdout: code 0, closed (the Orin and the player have exited),<br/>or code 1, the run failed: training ends
```

**Reset in place** (user, 2026-10-24). Between episodes the run goes back
to the start without relaunching anything: the player's window stays
open, so it can be placed and watched, and an episode pays no launch
(about 3 s each before). The trainer answers a pending pool with
`{"next": "reset", "seed": S}`, S the next episode's seed, and the run
resets:

```mermaid
sequenceDiagram
    participant M as MuJoCo (in-process)
    participant O as Orchestrator
    participant A as Synthetic Arduino (in-process)
    participant N as Synthetic Orin
    participant U as Unity player
    Note over O: R1: the trainer's reset, with seed S (its command is not used)<br/>the episode count goes up by one
    O->>M: R2: reset(start pose): at rest, cart at 50% of the rail,<br/>the pole hanging (θ = 0), t = 0, ctrl 0
    Note over A: R3: nothing to reset: it holds only RIG,<br/>and the 50% start stands in for homing, as at creation (3)
    Note over O: R4: reseed the latency draws with S
    O->>N: R5: text {"type":"reset","seed":S}
    Note over N: R6: pre-warm the pool afresh, its gaps from S (The initial pool):<br/>no entry of the last episode survives, and failed frames count afresh
    N-->>O: R7: text {"type":"reset"}: the pool is ready
    Note over U: unchanged: still connected, it renders the next pose it is sent
```

Then the loop goes round from the new episode's first capture (8): the
capture state is the start pose at t = 0, and its pool is the episode's
first, which the trainer does not score. Observation ids keep counting
across the run's episodes, so the Orin's frame files and every log row
stay unique; MuJoCo's time restarts at 0. Why the pool is pre-warmed
again: an entry from the last episode would show the policy a pole the
physics never had in this one, a jump from the old pose to the start.
Why S reseeds both the latency draws and the pre-warm: an episode after a
reset is then the same episode as a fresh run with seed S (*Determinism*),
whatever came before it. Only one message is in flight at a time, so the
reset cannot race the loop: the Orin has replied to the last observation
before the trainer answers it.

**The pool's size.** Three entries are the minimum for the current
position, speed and acceleration; the other three are redundancy, so three
valid entries survive when detections drop out to (−1, −1). A frame that
does not decode, or is not 728×544, is a failed frame: its entry is
4 × (−1, −1) and the error is logged. One failed frame costs about 15 ms
of detection and does not fail the run; 3 among the pool's 6 entries fail
it and end training. Pre-warm entries are no detection, not failed frames.

**Truth log.** `states.csv` pairs MuJoCo's real values at each capture
with what the Orin observed in that frame, one row per observation, for
evaluation after training. Columns: `episode,id,time,cart,pole,left_x,left_y,
right_x,right_y,pivot_x,pivot_y,end_x,end_y` — the episode (0 for the
run's first, one more at each reset), the capture time (s, from 0 in each
episode), the
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

**The trainer link (17, 18, 26)** (user, 2026-10-24): the orchestrator's
stdin and stdout. The trainer launched the orchestrator (1), so it already
holds both as pipes: no port, no socket, and each run's link is its own.
Per observation, one JSON line each way: the pool out on stdout (17), the
answer in on stdin (18): continue with the command, reset with the next
episode's seed (*Reset in place*), or stop. stdout carries the link and nothing else, so
everything else the orchestrator prints, and everything its children (the
Orin, the player) print, goes to stderr. While the trainer works, MuJoCo
stays frozen at the arrival step, so the trainer's speed, like the Orin's,
stays out of simulated time. The run's end (26) is the orchestrator's
exit: stdout closes, and the exit code says closed (0) or failed (1).

**The trainer's environment** (user, 2026-10-24): the trainer reaches the
loop through a Gymnasium `Env`, one run holding many episodes
(`gymnasium-research.md`; user, 2026-10-24). The first `reset()` starts
the run (1), with the episode's seed as `--seed`, and returns its first
pool (17); `step(a)` answers the pending pool with continue and the
command (18) and returns the next pool (17); every later `reset()`
answers the pending pool with reset and the episode's seed (*Reset in
place*) and returns the new episode's first pool; `close()` answers it
with stop and waits for the run to close (26). A run that fails (26,
code 1) raises, which ends training. The observation is
the pool, 6 × 9 floats; the action is one value a ∈ [−1, 1], and the
command is a × 56,666 (the top rate), rounded to the `i32`. The reward is
graded (below), and the end crash terminates the episode. **The time
limit** (user, 2026-10-24, on the agent's recommendation): 1,500 steps,
about 19 s of simulated time at the mean gap of 12.57 ms, then the
episode is truncated: time to swing up from hanging and then hold the
pole upright for most of it. Reference tasks run 10 s (DeepMind's cart-pole
swing-up, Gymnasium's `CartPole-v1`) to 40 s (`InvertedPendulum-v5`).

**The reward is graded** (user, 2026-10-24), from the pool alone, in two
parts: the pole, low hanging down and rising to high upright, so every
gain in height pays on the way up from the hanging start; and the cart,
low with the Pivot marker close to a support (an end of the slide) and
high in the middle. Both are read from the markers: the pole's angle is
the Pivot-to-End direction in the image, and the cart's place is the
Pivot marker's x (`trainer-plan.md`, *What the trainer sees*). The shapes
and their sum (user, 2026-10-24): the pole part is (1 − cos θ) / 2, θ the
image angle, 0 hanging and 1 upright; the cart part is 1 − u², u the Pivot
marker's offset from the middle of the end-crash bounds as a fraction of
their half-span, 1 in the middle and 0 at a bound; the reward is their
weighted sum, so the cart part teaches even while the pole hangs. The
values (user, 2026-10-24, all in the trainer's `gym.toml`): weights 1.0
for the pole and 0.5 for the cart, so a step scores 0.5 hanging in the
middle and 1.5 upright in the middle; the end-crash bounds are Pivot x
0.26325 to 0.72639, the slide's ends less the 13.6 mm margin, measured; the
end crash scores −10 and terminates the episode. Each step is scored from
the newest pool entry with both Pivot and End detected, and scores 0 when
no entry has them.

**The policy learns by SAC** (Soft Actor-Critic, Stable-Baselines3's
`SAC` with its `MlpPolicy`; user, 2026-10-24, on the agent's
recommendation), on one environment, on the 3090. Why SAC: it is made for
one continuous action, and it is off-policy, reusing every stored
observation many times, so it needs the fewest observations of the usual
choices, and each observation costs real time (about 45 a second). Its
updates run between steps, while MuJoCo stays frozen at the arrival step,
so they cost wall time, never simulated time. Why the 3090 (user,
2026-10-24): on the CPU, measured over 300 steps through the loop
(2026-10-24), a step took 22 ms (median) before learning started and 46
ms once SAC updated every step, one update of the network (the 54 pool
values, two layers of 256, one action) taking 13.5 ms on 16 threads, so
learning doubled the step's wall time. The 3090 is free for it: the loop
runs one thing at a time, so while SAC updates, Unity and the Orin wait
and the GPU's rendering and detection are idle; and SAC needs about 0.5
GB of its 24 GB (the networks under 10 MB, the rest CUDA's own; the
replay buffer stays in main memory). On the 3090, measured the same way
(2026-10-24): 22.1 ms a step before learning and 46.5 ms after, one
update 16.1 ms, 28 MiB held by PyTorch. The 3090 is no faster than the
CPU here, so the doubled step is not the network's arithmetic; where
the time goes is not yet measured. A time-limit truncation is
not a crash: SAC keeps valuing the state it stopped in, and only the end
crash ends the value.

**The initial pool.** At startup, before any frame exists, the Orin
pre-warms the pool: six entries with no detection (−1, −1) at cumulative
times before the first capture (t = 0), each gap a fresh draw dₖ of
RND(MIN, MAX) ms from the Orin's generator, seeded with `--seed`. These gaps
are not rounded to the 2 ms grid, unlike the live gaps; it barely
matters, since pre-warm entries hold no detection. Every frame, the first
included, is then one push: the newest entry goes in and the oldest
drops, so frame 0's pool ages are 0, −d₁, −(d₁+d₂), …, −(d₁+…+d₅).
A reset pre-warms the pool the same way, its gaps from the reset's seed,
before the new episode's first frame (*Reset in place*).

**Determinism.** The orchestrator's latency draws and the Orin's pre-warm
gaps both come from `--seed`, and the freeze at the arrival step keeps the
workstation's speed out of the sim, so the same seed and the same trainer
commands give the same physics: the same ids, capture times, latencies and
true cart and pole, bit for bit. A reset with seed S reseeds both, so an
episode after a reset repeats a fresh run with seed S, its ids offset by
the observations before it. The observation is not bit-identical, by
decision (user, 2026-10-24): Unity's SSAO (blue noise) changes its noise
pattern on every render, and the player renders between triggers, so a
frame's shading jitters slightly from run to run. The detector found every
marker in every frame regardless; run to run, the detections differ by at
most 0.04 px (2026-10-24), and so do `states.csv`'s observed columns and
`observations.jsonl` (which also logs `detect_ms`, a wall-clock time).

Numbers: `arduino::RIG` (200 steps × 32 microsteps, 96 mm per pulley turn:
15 µm per pulse, ±30,000 pulses of travel, 56,666 pulses/s top rate,
0.85 m/s, the motor's estimated corner speed on the rig's 30 V supply,
user 2026-10-24, `mujoco-plan.md`). The
travel limit acts only on the board, whose timer counts the pulses it
sends; the synthetic Arduino receives only the i32, so in the loop the
MJCF slide's ±0.45 m range is what stops the cart. The end crash is
therefore a training failure, judged by the trainer from the pool: the
Pivot marker out of bounds means penalty, and end this episode. The bounds
(*The reward is graded*) sit inside the slide's ends by more than the
cart can travel between a capture and the stop taking effect, at most
0.85 m/s (56,666 pulses/s × 15 µm) × 16 ms (the longest gap to the
arrival step at 8–15 ms) ≈ 13.6 mm, or the simulated cart reaches the
slide's end first. The real board finds its centre by homing: left to the limit
switch, right to the limit switch, then back half the pulses between the
two hits. The synthetic run instead always starts with the cart at 50% of
the rail, a guarantee, so the Arduino is never told a start position. The
pole starts hanging at rest, θ = 0 (user, 2026-10-24), as a pole at rest on
the rig does; upright is θ = π, so the policy must swing the pole up
before it can balance it. The
2 ms step is the MJCF's timestep, and the loop's fixed deltatime. The MJCF
states it, as `<option timestep="0.002" integrator="implicitfast"/>`,
written by the exporter from the rig's StepJointSet, never left to MuJoCo's
defaults: the timestep is both the physics step and the grid the latency
rounds to, and `implicitfast` is MuJoCo's recommended integrator for the
cart's `<velocity>` servo. The Orin latency's range, 8–15 ms today
(`rnd_delay_min`, `rnd_delay_max`), is the detection time measured on
the Orin Nano Super over 250 frames (2026-10-23, `detector-plan.md`
*Results*: TensorRT FP16 8.4–10.4 ms, the whole Python path with
decoding 11.9–14.5 ms). It stands for D = detection + policy + WiFi:
the policy's inference and the WiFi hop to the Arduino are not measured
yet (`orchestrator-plan.md`, *Placeholders*).

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
  `failed` is true when 3 of the pool's 6 entries are failed frames. To
  the trainer (17), one stdout line: `{"id": n, "pool": [...]}`, the same
  6 rows; no `failed`, since a failed pool goes to the shutdown and never
  reaches the trainer.
- **The answer (18):** one stdin line, `{"command": c, "next":
  "continue"}`, `{"command": c, "next": "reset", "seed": S}` or
  `{"command": c, "next": "stop"}`: c is the command below, a JSON
  integer that must fit an `i32`, used only on continue; S, on a reset
  only, is the next episode's seed, a whole number 0 to 2⁶⁴ − 1 like
  `--seed`.
- **The reset (R5, R7):** on the control socket, the orchestrator sends
  `{"type": "reset", "seed": S}` and the Orin replies `{"type": "reset"}`
  once the pool is pre-warmed again.
- **The command (18, 19):** one `i32`, in pulses per second; sign =
  direction, never a distance or position offset. To the Arduino (19) it
  travels as 4 bytes, little-endian; fixed length, no id. Why an `i32`: at 1/32 microstepping the
  top rate is 0.85 m/s ÷ 15 µm = 56,666 pulses/s, past an `i16`'s
  32,767; an `i32` never limits the microstep setting or the speed. One
  pulse per second is 0.015 mm/s, finer than the slowest motion a
  balance needs.
- **The cart speed (20, 21):** meters per pulse = belt travel per
  pulley turn ÷ (full steps per turn × microsteps) = 96 mm ÷ (200 × 32)
  = 15 µm. The 96 mm is the CAD pulley `pully-gt2-48t-6w-8b`: 48 teeth
  at GT2's 2 mm pitch. The MJCF's `ctrlrange` (±0.85 m/s) matches the
  top rate, 56,666 pulses/s × 15 µm.
