# Trainer Plan — the policy's teacher

The balance policy has to be learned from what the robot will see, so
something has to own the learning: run the loop, score each command by
the observation that follows it, and decide when a run is over. The
**trainer** is that participant. It holds the policy being trained; in
production the trained policy runs on the Orin. It starts each run,
receives only the marker pool, and answers each pool with the next
command and continue or stop.

Status: **plan** (2026-10-23), written from `synthetic-loop-dia.md`
alone, the source of truth. Element numbers below are the diagram's
messages.

## Done when

1. **Bring-up stage 1** (*Bring-up*): the trainer returns a zero
   command; the run starts (1) and closes (26).
2. **Bring-up stage 2:** the trainer returns a scripted command, and
   the detector's slots can be compared with the true positions in
   `states.csv`.
3. **Runs back to back.** A stop ends the run and the trainer starts
   the next one (1); a run reported failed (26) ends training, with no
   further run.
4. **A trained policy balances the pole** — the criterion is not set
   yet.

## Requirements (from the diagram)

- **T1 — the trainer owns the run.** It starts the run by launching the
  orchestrator, with the command its `trainer/settings.toml` names (1),
  and is told when the run has closed, or that it failed (26). The same
  file's `[mjcf]` names the model the run loads and its joint and
  actuator names; the trainer passes them on the command line (2).
- **T2 — the pool, and nothing else (17).** The current state: 6
  entries, newest first, each keyed by its age in −ms from the current
  capture time (newest = 0), each holding the 4 marker `(x, y)` in class
  order (Left, Right, Pivot, End), `(−1, −1)` where nothing was
  detected. Never MuJoCo's truth.
- **T3 — scoring.** From the pool alone, the trainer generates the
  reward or penalty for its last command: this pool is the next state
  after it. There is nothing to score on the first observation.
  Physically, that command has not acted yet when the frame is taken;
  that delay is the Orin's real perception latency, and the policy
  learns it as part of the world.
- **T4 — the end crash.** The Pivot marker out of bounds is a training
  failure: penalty, and stop this run. The bounds must sit inside the
  slide's ends by more than the cart can travel between a capture and
  the stop taking effect, at most 0.68 m/s × 16 ms ≈ 11 mm.
- **T5 — the answer (18).** The command — an `i32`, sign = direction,
  magnitude = pulses per second (the synthetic Arduino clamps it to
  ±45,333) — and continue or stop.
- **T6 — what a stop ends.** Stop ends this run, one episode for now;
  the trainer itself carries on and starts the next run (1). A run that
  failed ends training.
- **T7 — nothing in the control loop touches training.**

## Bring-up — what the trainer returns first

Each stage must pass before the next, so a failure has one place to
look.

1. **Zero command** — the cart holds still, the pole hangs or swings.
   Proves the plumbing: every part runs, frames return, the Pivot and
   End slots track the swinging pole, and the run starts and closes.
2. **Scripted command** — a slow back-and-forth. Proves the command
   path (`i32` → synthetic Arduino → cart speed → MuJoCo → render) and
   lets the detector's slots be compared with the true positions in
   `states.csv`.
3. **Trained policy** — only after 1 and 2 pass; a failure here is
   learning, not plumbing.

## Not decided by the diagram

These are needed to build the trainer; per `AGENTS.md`, each lands in
the diagram first:

- **The learning method and its tooling** (the diagram's *Scoring lag*
  uses a Gymnasium step only as an analogy).
- **The reward**, and the end conditions besides the end crash, such as
  a time limit.
- **The end crash's bounds** — not set.
- **The pole's start** for each run (the cart's is guaranteed at 50%).
  The intent so far is near θ = π: at hinge angle 0 the pole hangs, so
  upright is θ = π.
- **The link to the orchestrator** (17, 18, 26): its transport. The
  start (1) is settled: the trainer launches the orchestrator.
- **How the trained policy reaches the Orin** for production.

## Steps

1. **A stand-in trainer** that starts a run, takes each pool, and
   returns a fixed command and continue, then stop. *Check:* Done-when
   1–2. **In part (2026-10-24):** `trainer/` is a `uv` project (code in
   `src/`). `naive_trainer.py` starts the run (1): it launches
   `target/release/orchestrator.exe` with the command `settings.toml`
   builds (the ports, the seed and `[mjcf]` inserted ahead of `--orin`;
   `launch.py`, without the trainer's `VIRTUAL_ENV`), waits for it and
   reports its exit code; a run exits 0. `pool.py` and `output.py` check
   the pool (17) and the answer (18); `NaiveTrainer.answer()` returns the
   fixed command, then stop. No pool reaches it yet: that waits on the
   link (*Not decided*), and the run's end is only the exit code (26).
2. **Runs back to back** (T6). *Check:* Done-when 3.
3. **Scoring and the end crash** (T3, T4), once the reward and the
   bounds are in the diagram.
4. **Learning**, once the method is in the diagram. *Check:* Done-when
   4.

## Not in this plan

More than one episode per run, pooling, the orchestrator
(`orchestrator-plan.md`), and running the trained policy on the Orin.

## References

- `synthetic-loop-dia.md` — the source of truth for this plan
- `orchestrator-plan.md` — the participant the trainer talks to
