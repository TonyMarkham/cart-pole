# Trainer Plan — the policy's teacher

The balance policy has to be learned from what the robot will see, so
something has to own the learning: run the loop, score each command, and
decide when an episode or a run is over. The **trainer** is that
participant. It holds the policy being trained; in production the trained
policy runs on the Orin. It starts each run, receives the marker pool for
the policy and each capture's true site positions and pole state for the
reward and the episode end alone, and answers each pool with the next command and
continue, reset or stop.

Status: **plan** (rewritten 2026-10-25), from `synthetic-loop-dia.md`,
the source of truth for the loop, with `__synthetic__/trainer/python/reward-plan.md`, the source of
truth for the reward calculation, and `rl-rewrite-plan.md`, the trainer's
structure. Element numbers below are the diagram's messages.

## Done when

1. **Bring-up** (*Bring-up*): `env_check.py`'s episodes of random
   actions run through the same environment, plugins and link a training
   run uses; the run starts (1) and closes (26).
2. **Episodes in one run.** Every episode after the first is a reset in
   place: one launch of the orchestrator, the Orins and the players for
   the whole session, and episode e runs with seed (seed + e).
3. **A failed run ends training** (26), with no further run, and the
   session resumes from its last save.
4. **A reward change continues a run.** A resume rescores every stored
   step through the reward the gym file names, from the stored inputs.
5. **A trained policy balances the pole** — the criterion is not set yet.

## Requirements (from the diagram and the plans)

- **T1 — the trainer owns the run.** It starts the run by launching the
  orchestrator with the command `__synthetic__/trainer/python/settings.toml` names (1), with
  the instances' rows and columns, the screen size, the ports, seed, latency band, the gym file's command rates,
  `[mjcf]` (the model, its joint, actuator and twelve site names) and
  `[drive]` (full steps per turn, microsteps, belt travel per pulley turn)
  inserted after the program (2), and
  is told when the run has closed, or that it failed (26).
- **T2 — the pool for the policy, MuJoCo's truth for the reward (17).**
  Per instance: the pool, 6 entries newest first, each its age in −ms and
  the 4 marker `(x, y)` in class order, `(−1, −1)` where nothing was
  detected; and the capture's MuJoCo time, twelve sites and pole state
  (*Contracts*, the sites and the pole): MuJoCo's truth, for the reward,
  the episode end and the record alone, never part of the observation.
- **T3 — the reward** (`__synthetic__/trainer/python/reward-plan.md`, the source of truth).
  Each scored step t, command c_t, is scored with capture t+1: r = r_tip
  × r_target_speed × r_cart × r_reach × r_speed, in (0, 1], from capture
  t+1's End, Dynamic Target, Target, Pivot, Cart and Cart Target sites and
  the pole's turning speed, every value MuJoCo's
  (the diagram's *The reward*); the commands are not scored, the drive's
  ramp holds the acceleration ceiling (the diagram's *The acceleration
  ceiling*, user 2026-10-28). Capture 0 of an episode is not scored.
  Nothing is subtracted at an end.
- **T4 — the episode end** (the diagram's *The episode end*). An episode ends
  when either strike-face gap is under b, checked after r is paid; b is
  at least the cart top speed × the longest gap between captures and the
  longest latency (68 ms at 20–30 Hz and 8–15 ms). No fall-angle end.
- **T5 — the answer (18).** The command — an `i32`, sign = direction,
  magnitude = pulses per second, at most ± the top pulse rate — and
  continue, reset (with the next episode's seed) or stop.
- **T5a — the link (17, 18, 26)** (the diagram's *The trainer link*): the
  orchestrator's stdin and stdout, held as pipes. First, once per run,
  the trainer's `{"request": "run"}` and the orchestrator's answer,
  `{"run": {"cart_top_speed", "top_pulse_rate", "timestep", "pole_mass",
  "pole_com_distance", "pole_inertia", "gravity"}}` (the diagram's
  *Contracts*, the run's values), kept for the run. One stdout line holds
  the advanced instances' entries, `{"pools": [{"instance", "id", "pool",
  "sites", "pole"}, …]}` (`"pole": {"speed", "com"}`, *Contracts*, the
  pole); one stdin line answers any pending instances,
  `{"answers": [{"instance", "command", "next", "seed"?}, …]}`. A
  malformed line fails the run at the link. The run's end is the
  orchestrator's exit: code 0 closed, 1 failed. Its log is on stderr.
- **T6 — episodes, reset and stop** (*Reset in place*). An episode ends
  by answering reset with the next episode's seed; stop ends the run; a
  failed run ends training.
- **T6a — the start:** every episode starts at rest, the cart at 50% of
  the rail, the pole hanging (θ = 0); upright is θ = π, so the task is
  swing-up, then balance.
- **T7 — nothing in the control loop touches training.**
- **T8 — one vectorized environment** (the diagram's *The trainer's
  environment*, *Training instances*). The group of instances is one SB3
  `VecEnv`; one instance is a group of one. The run is launched and
  asked for its values before the environment is built, since its
  plugins and spaces need them; the first `reset()` returns the first
  pools; `step(actions)` answers every instance with continue and its
  command and returns every next observation; an instance whose episode
  ends is answered alone with reset, and its new episode's first
  observation is returned, its last in `terminal_observation`; `close()`
  answers stop and waits for the run to close.
- **T8a — the time limit:** `[env] max_episode_steps` steps (460, about
  19 s), then the episode is truncated (`TimeLimit.truncated`), not ended.
  A step is one command: `[env] command_rate_min` and `command_rate_max`
  (20 and 30 Hz by default, user 2026-10-30) set how far apart they are,
  and the trainer sees nothing between them (the diagram's *Nothing happens
  between commands*). A run saved with other rates cannot resume.
- **T8b — the observation:** the pool's 30 floats, each age divided by
  `age_unit_ms` (1,000 today, so in seconds), then the actions a of
  the last `past_commands` (6) commands, newest first, 0 before an
  episode's first. Never a site (`rl-rewrite-plan.md`, R1).
- **T8c — the action:** a ∈ [−1, 1], clipped; command = round(a × the
  top pulse rate).
- **T9 — the policy learns by SAC** (the diagram's section of that
  name): Stable-Baselines3's `SAC` with its `MlpPolicy`, on the 3090,
  γ = 0.998 (*The discount*). A truncation is not an end: only an
  episode end ends the value. The randomness weight starts at `[sac]
  ent_coef`'s value (the diagram's section).
- **T10 — four plugins on one pattern** (`rl-rewrite-plan.md`, 3.1; the
  diagram's *The trainer's parts*). The reward, the episode end, the
  observation and the action are each one file holding one class, named
  in the gym file and configured by its own table (the reward by one
  table per factor, `[reward.tip]` and so on, user 2026-10-29); changing one's logic
  is a new file and its name there. The observation is given the pool
  and the command history only, so it cannot reach the sites.
- **T11 — every scored step recorded with its inputs** (R5, R6). The
  step's inputs — the two captures' sites and pole states, c_t,
  c_(t−1), the step index — go to the capture record and beside the step in the replay
  buffer, so live, resumed and offline scoring are one implementation.
- **T12 — a run is self-describing** (R7). Beside every saved model, a
  manifest: both toml files, the four plugins' names and tables, the git
  commit, the MJCF's path and modification time. A resume refuses a
  changed observation or changed end conditions, by name, and rescores a
  changed reward. It loads and rescores before the launch, with the run's
  values its manifest saved (a resume with other values is refused), so
  the loop never waits on it: the synthetic Orin closes its socket after
  about 40 s without an answer, and rescoring a million steps takes 35 s.
- **T13 — every tunable in a toml** (R3): `settings.toml` for the run,
  the gym file for the learning, `measures.toml` for the measures.

## Bring-up — what the trainer returns first

Each stage must pass before the next, so a failure has one place to
look.

1. **Random actions** — episodes of random actions, back to back on one
   instance, through the environment, plugins and link a training run
   uses (`env_check.py`). Proves the command path (action → `i32` →
   synthetic Arduino → cart speed → MuJoCo → render → detection → pool
   and sites), the scoring, and each episode ending and the next beginning
   in place. (The scripted back-and-forth, `naive_trainer.py`, was removed
   on 2026-10-25, user: development scaffolding that, needing the reward's
   `jump_limit` like everything else, checked nothing a training run does
   not.)
2. **Trained policy** — only after 1 passes; a failure here is learning,
   not plumbing.

## Not decided by the diagram

- **How the trained policy reaches the Orin** for production.

## Steps

The build order of `rl-rewrite-plan.md`, section 4; old runs are not
resumed, rescored or analysed (user, 2026-10-25).

1. **Prerequisites in the design:** the diagram's reward and end
   sections and this plan written from the reward plan and
   `rl-rewrite-plan.md`. **Done (2026-10-25).**
2. **The contract types** (`contracts.py`: `Pool`, `Sites`, `Capture`,
   `Command`, `StepInputs`). *Check:* each built from good values and
   refusing bad ones, the message naming the field.
3. **The settings loader and the plugin registry** (`settings.py`,
   `plugins.py`, `learning.py`, `checks.py`). *Check:* the gym file loads;
   an unknown plugin, a missing table, a value out of range and an
   unknown key are refused, naming the key; a plugin outside the repo
   loads by name.
4. **The reward plugin** (`rewards/tip_energy_cart.py`, user
   2026-10-29: `__synthetic__/trainer/python/reward-plan.md`'s r = r_tip × r_energy × r_cart,
   every input MuJoCo's; it reused `rewards/tip_energy.py`, r_tip ×
   r_energy alone, user 2026-10-28; the earlier site plugins are
   deleted). Both went with the nine-site runs, replaced by step 4b. *Check:* `__synthetic__/trainer/python/reward-plan.md`'s
   tables: r_tip 1 on the star, 0.011 hanging, 0.042 sideways; r_energy 1
   upright and still, 0.1 hanging still, 0.56 sideways still, 0.15
   passing straight up at 10 rad/s, at w = 0.2 m and spare_max = 2;
   r_cart 1 within 0.050 m of the middle, 0.940 at 0.1 m, 0.850 at
   0.2 m, 0.737 at the left end, always falling towards and above the
   floor 0.667, at c_w = 0.25 m; r whatever the commands.
4b. **The reward plugin `tip_energy_cart_centre`**
   (`rewards/tip_energy_cart_centre.py`, user 2026-10-29:
   `__synthetic__/trainer/python/reward-plan.md`'s r = r_tip × r_energy × r_cart × r_reach;
   r_tip to Dynamic Target, r_cart from Cart and Cart Target, r_reach to
   Target; its values one table per factor, `[reward.tip]`,
   `[reward.energy]`, `[reward.cart]` and `[reward.reach]`, user
   2026-10-29). Needs the
   twelve sites (step 2, the diagram's *Contracts*, the sites). *Check:*
   r_tip 1 with the pole straight up wherever the cart is; r_cart as in
   step 4 but from Cart and Cart Target; r_reach 1 on Target, 0.790 at
   0.2 m from it, always falling towards and above the floor 0.667, at
   `[reward.reach]` width = 0.2 m; straight up pays more than any lean with the
   cart off the middle (`__synthetic__/trainer/python/reward-plan.md`, section 4).
4c. **The reward plugin `tip_speed_cart_centre`**
   (`rewards/tip_speed_cart_centre.py`, user 2026-10-30:
   `__synthetic__/trainer/python/reward-plan.md`'s r = r_tip × r_target_speed × r_cart × r_reach
   × r_speed, replacing r_energy; `tip_energy_cart_centre.py` stays while
   its run can resume). Its new tables are `[reward.target_speed]` and
   `[reward.speed]`. *Check:* the plan's tables: r_target_speed 1 at or
   below 0.5 rad/s and outside the zone, 0.869 at 1 rad/s, 0.694 at 3 rad/s
   and 0.667 at 10 rad/s passing straight up; a hanging pole's d_x of 0 not
   in the zone; r_speed 1 at 12 rad/s, 0.526 at 13 rad/s and 0.250 at
   20 rad/s, anywhere; r 0.0075 hanging still; both always falling towards
   and above their floors.
5. **The end, observation and action plugins** (`endings/strike_gap.py`,
   `observations/pool_and_commands.py`, `actions/linear_rate.py`).
   *Check:* no end at the centred gaps of 0.440 and 0.445 m, an end
   within b, b below its floor refused; the observation 36 floats, its
   commands zero-filled, `build(pool, history)` taking no sites; −1, 0, 1
   to − the top pulse rate, 0 and the top pulse rate.
6. **The episode tracker** (`episode.py`). *Check:* capture 0 yields no
   inputs; the first scored step takes capture 0's sites and pole state
   and c_(−1) = 0.
7. **The fake link and the environment on it** (`link/fake.py`,
   `env.py`). *Check:* every observation in the space, every reward in
   (0, 1]; a strike-gap end with its reason; a time limit truncated;
   SAC learns on it. (SB3's `check_env` takes a single `gym.Env`, not a
   `VecEnv`, so SAC running on the environment is the check.)
8. **The run folder, the manifest and the capture record**
   (`run_folder.py`, `record.py`). *Check:* a save writes the manifest
   with the four plugins, the git commit and the MJCF's time; a resume
   under another observation is refused by name.
9. **The buffer and the rescore** (`buffer.py`). *Check:* the inputs
   stored at every row; a resume with another `width` rescores every row
   to what the plugin gives directly; a resume with another `buffer` is
   refused.
10. **The replay link and the measures** (`link/replay.py`, `measures/`,
    `measure.py`). *Check:* a recorded session replays to the same record
    line for line; each measure prints its table.
11. **The real link and the training run on the loop** (`link/real.py`,
    `train.py`, `env_check.py`). *Check:* a one-instance `env_check.py`
    run passes, then a short six-instance session
    runs, saves, resumes and rescores. Needs `[drive] jump_limit` (m) in
    `settings.toml`, for the drive's ramp: measured 2026-10-28
    (`jump-limit-results.md`).
12. **The old code removed.** *Check:* nothing imports a removed module.

Steps 2–10 and 12: **built (2026-10-25)**, each check a test in
`__synthetic__/trainer/python/tests/` on the fake link (`uv run --project __synthetic__/trainer/python python -m
unittest discover -s __synthetic__/trainer/python/tests -t __synthetic__/trainer/python/tests`), all passing. Step
11's code is built; it has not run on the loop.

## Not in this plan

The orchestrator (`orchestrator-plan.md`), the reward's calculation
(`__synthetic__/trainer/python/reward-plan.md`), and running the trained policy on the Orin.

## References

- `synthetic-loop-dia.md` — the source of truth for this plan
- `__synthetic__/trainer/python/reward-plan.md` — the source of truth for the reward calculation
- `rl-rewrite-plan.md` — the trainer's structure and build order
- `orchestrator-plan.md` — the participant the trainer talks to
