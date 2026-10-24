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
- **T3a — a graded reward** (the diagram's *The reward is graded*; user,
  2026-10-24), in two parts read from the markers: the pole, low hanging
  and high upright, from the Pivot-to-End image angle; and the cart, low
  with the Pivot marker near a support and high in the middle, from
  pivot_x (*What the trainer sees*). Pole part (1 − cos θ) / 2; cart
  part 1 − u², u = the Pivot marker's offset from the middle of the
  end-crash bounds over their half-span; reward = their weighted sum.
  Values, in `gym.toml`: weights pole 1.0, cart 0.5; scored from the
  newest entry with Pivot and End, 0 if none has both.
- **T4 — the end crash.** The Pivot marker out of bounds is a training
  failure: penalty, and stop this run. The bounds must sit inside the
  slide's ends by more than the cart can travel between a capture and
  the stop taking effect, at most 0.68 m/s × 16 ms ≈ 11 mm. Set (user,
  2026-10-24, in `gym.toml`): Pivot x 0.26181 to 0.72784, the measured
  slide ends less 11 mm (*What the trainer sees*); a crash scores −10
  and terminates the episode, whose `reset()` stops the run.
- **T5 — the answer (18).** The command — an `i32`, sign = direction,
  magnitude = pulses per second (the synthetic Arduino clamps it to
  ±45,333) — and continue or stop.
- **T5a — the link (17, 18, 26)** (the diagram's *Trainer link*): the
  orchestrator's stdin and stdout, which the trainer holds as pipes
  because it launched the orchestrator (1). Per observation it reads one
  stdout line, `{"id": n, "pool": [6 rows]}`, and writes one stdin line,
  `{"command": c, "continue": bool}`. The run's end (26) is the
  orchestrator's exit: stdout closes, then exit code 0 (closed) or 1
  (failed: training ends). The orchestrator's log arrives on stderr, the
  trainer's console.
- **T6 — what a stop ends.** Stop ends this run, one episode for now;
  the trainer itself carries on and starts the next run (1). A run that
  failed ends training.
- **T6a — the start** (user, 2026-10-24): every run starts at rest with
  the cart at 50% of the rail and the pole hanging, θ = 0, as on the
  rig. Upright is θ = π, so the task is swing-up, then balance, and the
  reward must lead there from hanging.
- **T7 — nothing in the control loop touches training.**
- **T8 — a Gymnasium `Env`, one run per episode** (the diagram's
  *Trainer's environment*; user, 2026-10-24). `reset()` starts a run and
  returns pool 0; `step(a)` answers with continue and returns the next
  pool; `reset()` or `close()` on an open run answers stop and waits for
  it to close; a failed run raises. Observation: the pool, 6 × 9.
  Action: a ∈ [−1, 1], command = round(a × 45,333). Until the reward,
  the end crash and the time limit are decided: reward 0, no
  termination, truncation at a placeholder length (`TimeLimit`).

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

- **The learning method** (`gymnasium-research.md` holds the research
  for this decision). Its tooling is settled: a Gymnasium `Env` (T8).
- **The time limit** (the reward and the end crash are settled, T3a,
  T4): `gym.toml`'s `max_episode_steps` is a placeholder until then.
- **How the trained policy reaches the Orin** for production.

## Steps

1. **A stand-in trainer** that starts a run, takes each pool, and
   returns a fixed command and continue, then stop. *Check:* Done-when
   1–2. **In part (2026-10-24):** `trainer/` is a `uv` project (code in
   `src/`). `naive_trainer.py` starts the run (1): it launches
   `target/release/orchestrator.exe` with the command `settings.toml`
   builds (the ports, the seed, the latency band `rnd_delay_min` and
   `rnd_delay_max`, and `[mjcf]` inserted ahead of `--orin`;
   `launch.py`, without the trainer's `VIRTUAL_ENV`), waits for it and
   reports its exit code. `pool.py` and `output.py` check the pool (17)
   and the answer (18) and carry their link encodings
   (`Pool.from_rows`, `Answer.line`); `launch.py` reads each stdout line
   (anything but a pool message for the next id fails the run), asks the
   trainer, writes the answer to stdin, and after the stop expects only
   EOF and the exit code (T5a).
   **Done-when 1 done (2026-10-24):** command 0, 250 observations: every
   pool answered, the stop on the 250th, the run closed with exit 0,
   reported as "the run closed". The pole started hanging at rest, so it
   stayed still rather than swinging. Since stage 1 passed, the naive
   trainer no longer sends a constant (user, 2026-10-24): it sends stage
   2's back-and-forth.
   **Done-when 2 done (2026-10-24):** the naive trainer is a scripted
   agent on the environment (T8): at step n its action is (peak /
   45,333) × cos(2π n / period), a command of peak × cos(2π n / period)
   pulses/s, for one episode of `max_episode_steps`. All of it is in
   `gym.toml` (`[sweep]` peak and period, `[env]` max_episode_steps):
   26,667 pulses/s (0.4 m/s), period 320 (about 4 s), 640 steps. The run
   closed with exit 0; the cart swung −0.255 to +0.258 m and the pole
   −9.9° to +11.7°, Pivot and End detected in all 641 frames.
   `observation_fit.py` compared the slots with `states.csv` (*What the
   trainer sees*). `launch.py`'s own trainer loop is gone: every trainer
   drives the environment.

## What the trainer sees, fitted to the truth (2026-10-24)

Measured with `observation_fit.py` on two stage-2 runs: 640 observations
over ±0.26 m, and 1,000 observations (period 500) over −0.41 to +0.40 m.
Both runs give the same fit; the figures are the wider run's.

- **Cart from the Pivot marker's x:** cart = 1.88452 × pivot_x −
  0.93250 m, so 1 px of Pivot x is 2.589 mm. Residual RMS 0.14 mm, max
  0.49 mm; a quadratic fits no better, so the map is a straight line.
  Pivot y stays within 0.3 px.
- **The slide's ends in marker terms:** ±0.45 m is pivot_x 0.25604 to
  0.73361; the end-crash margin inside them, ±0.4391 m (11 mm in), is
  pivot_x 0.26181 to 0.72784. These are 0.05 m past the measured range,
  extrapolated along a line that holds to 0.5 mm over ±0.40 m.
- **Pole from the Pivot-to-End direction:** with the image angle
  atan2((end_x − pivot_x) × 728, (end_y − pivot_y) × 544), 0 for End
  straight below Pivot, pole = 0.99866 × image angle − 0.00011 rad,
  residual RMS 0.026°, max 0.075°: the image angle is the pole angle,
  same sign. Measured over −10° to +12° about hanging only; near upright
  (θ = π) it is not measured yet. The Pivot-to-End length stays 173.5 to
  174.0 px.
2. **Runs back to back** (T6), through the environment (T8): each
   `reset()` closes the open run and starts the next. *Check:* Done-when
   3, and Gymnasium's `check_env` passes on the environment. **In part
   (2026-10-24):** `gymnasium` 1.4.0 is the trainer's one dependency
   (`uv add`, user's permission). `loop_env.py` is the environment
   (`LoopEnv`, registered nondeterministic with `TimeLimit` at
   `gym.toml`'s `[env] max_episode_steps`, 250 for now; `gym.toml`
   holds the environment's settings, apart from `settings.toml`'s run
   settings, user 2026-10-24); `launch.py`'s `Run` is one
   run, from launch to exit, which the naive trainer drives too.
   `env_check.py` drove two episodes of random actions, each truncated
   after 250 steps; each `reset()` stopped the open run, which closed
   with exit 0, and started the next. Then `check_env` passed, with no
   warnings, over 11 more run starts (seeds 0, 123 and 456 among them),
   every one closed cleanly. The naive trainer still runs: 250 pools,
   closed, the same latencies as before. Since check_env passed,
   `env_check.py` no longer runs it (user, 2026-10-24): its short runs
   test the API, not motion; it is run by hand when the environment's
   interface changes. Not yet run: a failed run
   through the environment (RunFailed), and Done-when 3's "training
   ends" path.
3. **Scoring and the end crash** (T3, T3a, T4). **Done (2026-10-24):**
   `reward.py` scores each pool; `LoopEnv.step()` returns its reward and
   terminates on the end crash; the values are `gym.toml`'s `[reward]`
   and `[end_crash]`. Checked by hand without a run: hanging in the
   middle 0.5, upright in the middle 1.5, level 1.0, at a bound 0, a
   step past it −10 and crashed, an older entry used when the newest
   lacks a marker, 0 when none has both. With the real player:
   - the sweep at peak 42,000, period 320: 640 steps, truncated by the
     time limit, the cart −0.40 to +0.41 m (pivot_x 0.281 to 0.711,
     inside the bounds), reward 190.3, 0.297 a step: the cart part
     falls as the cart swings wide, and the pole hangs;
   - the same peak, period 500 (about ±0.63 m): the end crash at step
     64, −10, terminated, the run stopped and closed with exit 0;
   - `env_check`'s random actions: two episodes of 640 steps, rewards
     317.9 and 297.6 (about 0.48 a step: the cart stays near the middle,
     the pole hangs); `check_env` passed.
4. **Learning**, once the method is in the diagram. *Check:* Done-when
   4.

## Not in this plan

More than one episode per run, pooling, the orchestrator
(`orchestrator-plan.md`), and running the trained policy on the Orin.

## References

- `synthetic-loop-dia.md` — the source of truth for this plan
- `orchestrator-plan.md` — the participant the trainer talks to
