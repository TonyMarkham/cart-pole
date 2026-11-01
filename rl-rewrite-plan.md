# RL rewrite plan: the trainer, the policy's environment and the reward

The reward has had four versions in two runs (run 1: the graded reward
it spun on, the slow-pole, the log height; run 2: the product), and it
is about to change again, to the
site-based calculation in the reward plan (now `__synthetic__/trainer/python/reward-plan.md`). Each change so far meant
editing one hard-wired function, its three callers and its settings
parser together, and the next one cannot be rescored at all. This plan
replaces the trainer's Python (`__synthetic__/trainer/python/src`, 1,799 lines in 14 modules,
read 2026-10-25) with a design in which the reward, the episode end and
the observation are parts chosen and configured in the toml, every
step's inputs are recorded, and any reward change can be rescored onto
the experience already gathered.

A plan: nothing here is written yet (user, 2026-10-25: "a fresh and
complete rewrite ... once we design it to be flexible"). It follows
`synthetic-loop-dia.md` and `__synthetic__/trainer/python/reward-plan.md`; where they disagree with
the code, the code is the work. Requirements are stated as requirements;
recommendations say so and name their source or say they have none.

## 1. Why the current code is rigid

Read from the code, 2026-10-25:

| What | Evidence | Why it hurts |
|---|---|---|
| One hard-wired reward | `reward.score(pool, action, Reward, EndCrash)` is one formula with fixed inputs, called from `vec_env.py:115`, `loop_env.py:142` and `train.py:209` | A new term or input means editing the formula, three callers and the parser together |
| A settings class per reward version | `gym_settings.Reward` holds run 2's exact fields, each parsed and checked by hand | Each reward change also rewrites the parser and its checks |
| Two copies of the environment | `LoopEnv` and `LoopVecEnv` each carry reset, step, scoring and the command history | Every change lands twice, and the copies can drift |
| Rescoring tied to the observation's layout | `train.rescore` rebuilds the pool by slicing `next_observations[...][:FLOATS]` | The site reward cannot be rescored (sites are never in the observation), and any observation change breaks it |
| The reward decides the episode end | `score` returns the reward and `crashed` together | The end rule cannot change without the reward |
| No tests | `__synthetic__/trainer/python/` has no tests | Nothing guards a change; run 2's checks lived in scratchpad scripts outside the repo |
| Analysis outside the repo | `session4.py`, `catches.py`, `schemes.py` and the rest are scratchpad scripts with run folders hard-coded | The measurements behind each decision cannot be reproduced |

## 2. Requirements (invariants the rewrite must keep)

- **R1 — the policy's input is the observation alone:** the pool (6
  entries × (1 age + 4 markers × 2) = 54 floats) and its own last
  `past_commands` commands. Never a site, a joint value or any other
  MuJoCo truth (`AGENTS.md`, *Not a toy*; `synthetic-loop-dia.md`,
  *Contracts*).
- **R2 — the reward and the episode end are computed from MuJoCo's
  truth** that arrives with each pool (17), the sites and the pole state,
  never from the detected markers (user, 2026-10-25), and never from a
  value worked out where MuJoCo has it (user, 2026-10-28;
  `synthetic-loop-dia.md`, *The reward*).
- **R3 — every tunable lives in a toml** (`AGENTS.md`, *Working with the
  user*): which reward plugin, which end conditions, which observation,
  and every value of each. Only fixed facts copied from a contract stay
  in code, each citing its source.
- **R4 — the link's contract is unchanged:** the lines (17, 18) as
  `synthetic-loop-dia.md` defines them, read and checked by the link
  alone. A malformed line fails the run there.
- **R5 — every scored step is recorded with its inputs,** so any reward
  can be recomputed from them later: on a resume, offline, or by a test.
- **R6 — the same code scores live, on resume and offline.** There is
  one reward implementation; nothing re-derives it.
- **R7 — a run is self-describing:** the toml it ran with (reward plugin
  and values, end conditions, observation, SAC) is saved with each model,
  so a journal entry (`trainer-observations.md`) can name exactly what
  scored it.
- **R8 — `uv` only; nothing installed without permission** (`AGENTS.md`,
  *Python policy*).

## 3. The design

### 3.1 The flow of one step

```
link (real | fake | replay)  one stdout line -> per instance: Capture(id, pool, sites)
   |
episode tracker              per instance: previous Capture, command history, step count
   |                          -> StepInputs for the step just finished
   +--> observation plugin   build(pool, commands) -> the policy's array: never sees sites (R1)
   +--> reward plugin        score(StepInputs) -> (r, factors)
   +--> end plugins          check(StepInputs) -> reason or None
   |
vectorized env               obs, r, done, info (info carries StepInputs and factors)
   |
SAC (SB3)  ->  replay buffer subclass stores StepInputs beside each row (R5)
   |
action plugin                the policy's action -> the i32 command (pulses/s) -> link (18)

run folder (one owner)       manifest, models, buffer, step record; measures read the record
```

Four parts are plugins on one pattern: the reward, the end conditions,
the observation and the action. Each is one file holding one class,
`from_table(table)` to read and check its own toml table, and the one
method its role needs. The toml names which file is used; changing the
logic is a new file and a new name, and the old file stays so old runs
stay readable. The manifest records which were used (3.2, the run
folder).

### 3.2 The parts and their interfaces

Each part is one module (or one folder of small modules), with a narrow
interface; the others reach it only through that interface.

- **`contracts.py`: the contract types,** immutable and checked once
  where they enter: `Pool`, `Sites`, `Capture` (`id`, `pool`, `sites`),
  `Command` (a whole number in the i32's range; its ± top-pulse-rate
  bound is checked by the environment, from the run's values) and `StepInputs`. Everything
  inside the trainer can trust them instead of re-checking at each use.
- **`link/`: the trainer link, behind one interface,**
  `next_captures() -> {instance: Capture}`, `answer(answers)`,
  `finish()`, with three implementations:
  - `real.py`: launches the orchestrator and speaks the lines (17, 18),
    from today's `launch.py` and `output.py`; it replaces
    `next_pools()` plus `Run.sites`.
  - `fake.py`: a scripted rig in Python for the tests, so nothing needs
    the orchestrator, six Unity players and six Orins.
  - `replay.py`: plays a recorded run's step record back, so a strange
    episode can be re-driven through the trainer and debugged without
    the loop.
- **`episode.py`: the episode tracker,** one per instance: the previous
  capture, the command history (zero-filled at an episode's start), the
  step count, and the first-step rule (`synthetic-loop-dia.md`, *The
  reward*: capture 0 is not scored). Its one output is `StepInputs`: the
  previous and current `Sites`, `c_t`, `c_(t−1)`, the step index, and
  the previous and current `PoleState` (the pole's speed, angle, centre
  of mass and hinge point, `synthetic-loop-dia.md`, *Contracts*, the
  pole). This is the only place those are assembled.
- **`observations/`: the observation, a plugin,** one file per
  observation, each with `space()` and `build(pool, commands)`. It is
  given the pool and the commands only, never the capture, so it cannot
  reach the sites: R1 is enforced by the interface, not by care. The
  first plugin, `observations/pool_and_commands.py`, is today's (54 +
  `past_commands` floats). A change of observation is a new file; a
  resume compares the manifest's observation name and refuses a
  different one by name, where run 1's resume under run 2's observation
  was refused only by mismatched array sizes.
- **`actions/`: the action mapping, a plugin,** one file per mapping
  from the policy's action in [−1, 1] to the `Command`, with
  `command(action, history) -> Command`. The first,
  `actions/linear_rate.py`, is `a × the top pulse rate`, rounded. This is the
  next likely thing to vary: the 2026 hardware paper put a low-pass
  filter on its command after an unfiltered policy broke a wheel
  (`reward-research.md` 3.4). A filtered or rate-limited mapping is
  a new file.
- **`rewards/`: the reward, a plugin** (user, 2026-10-25: the reward
  calculation is its own object and file, so changing the reward logic
  is swapping a plugin). One file per reward scheme, each holding one
  class with the same two methods:
  `from_table(table) -> Reward`, which checks its own values, and
  `score(inputs: StepInputs) -> (r, factors)`, where `factors` names
  each part of `r` for the log. `__synthetic__/trainer/python/reward-plan.md`'s calculation
  is the plugin, `rewards/tip_speed_cart_centre.py` (user, 2026-10-30:
  r_target_speed and r_speed replaced r_energy;
  `tip_energy_cart_centre.py` stays while its run can resume;
  `tip_energy.py` and `tip_energy_cart.py` went with the nine-site
  runs). Its values sit one table per factor, `[reward.tip]`,
  `[reward.target_speed]`, `[reward.cart]`, `[reward.reach]` and
  `[reward.speed]` (user,
  2026-10-29), which the scheme names in `TABLES` and any scheme using
  that factor shares. The toml's `[reward] scheme`
  names the plugin; nothing outside its file knows how it computes.
  Changing the reward logic is a new file and a new name in the toml.
  The earlier site plugins are deleted (user, 2026-10-28): the runs they
  scored lack the pole state, so they can be neither resumed nor
  rescored. A plugin may read only `StepInputs`: a scheme that
  needs an input not yet in it adds that input to `StepInputs` (and so
  to the record and the buffer) first.
- **`endings/`: the episode end, plugins,** one file per condition,
  each with `check(inputs: StepInputs) -> reason or None`; the toml
  lists which are on, and the first is `endings/strike_gap.py`, the
  strike-face rule. Kept apart from the reward (section 1, the fifth
  row).
- **`env.py`: one vectorized environment** (SB3's `VecEnv`) built from
  the parts above. One instance is a group of one, so `LoopEnv` goes.
  `info` carries the `StepInputs`, the factors, the end reason, and
  `TimeLimit.truncated` and `terminal_observation` as today
  (`vec_env.py`).
- **`buffer.py`: the replay buffer,** a subclass of SB3's `ReplayBuffer`
  that stores each step's `StepInputs` in arrays beside the buffer's own,
  at the same row, read from the `infos` SB3 passes to `add`. Verified
  in the installed SB3 2.9.0: `ReplayBuffer.add` receives `infos`
  (`buffers.py:247`), SAC takes a `replay_buffer_class`, and a step runs
  `env.step` → callback → store (`off_policy_algorithm.py:568–583`). It
  is saved and loaded with the buffer.
- **`train.py`: the training run:** SB3's SAC built from `[sac]`, the
  progress log, saves, and the resume. On a resume it rescores every
  stored row through the named reward plugin from the stored
  `StepInputs` (R6). It
  refuses, with the reason, when the end conditions have changed, since
  a rescore cannot move where stored episodes ended.
- **`settings.py`: one loader** for `settings.toml` and the gym file.
  Each part checks its own table (the reward its factor tables,
  `[reward.tip]` and so on); the loader only routes tables to parts.
- **`run_folder.py`: the run folder, one owner.** The only module that
  knows the folder's layout (models, buffer, step record, logs). It
  writes a manifest beside every saved model: the toml files used, the
  four plugins' names, the git commit, and the MJCF's path and
  modification time. A resume or a rescore reads the manifest and
  checks it is using what the run used (R7), instead of each caller
  guessing paths.
- **`record.py`: the step record:** every step's `StepInputs`, factors
  and end reason, per instance, written to the run folder as it goes
  (R5), for the measures, the replay link and the journal.
- **`measures/`: the analysis, plugins over the step record,** one file
  per measure: the band tables, the holds and catches, the spin
  bursts, the entropy and the actor's spread, each run on a session to
  print the numbers a `trainer-observations.md` entry needs. They read
  the run folder through `run_folder.py`, so no run folder is
  hard-coded, and the figures behind every journal entry can be
  reproduced. One tool beside them rescores an old run's `states.csv`
  through any reward plugin (the true poses, turned into sites by
  `physics`).
- **`tests/`:** every plugin against hand-computed values (the tables
  in `__synthetic__/trainer/python/reward-plan.md`, the strike gaps, the observation's shape and
  contents, the action's rounding and bound), the contract types'
  refusals, the episode tracker's first step, the buffer's store and
  rescore, the manifest's checks and the settings loader's refusals.
  They run on the fake link, so they need no orchestrator.

### 3.3 The gym file's shape

A sketch: the names and values are `__synthetic__/trainer/python/reward-plan.md`'s.

```toml
[env]
command_rate_min = 20.0                # Hz, added 2026-10-30 (synthetic-loop-dia.md)
command_rate_max = 30.0
max_episode_steps = 460

[observation]
plugin = "pool_and_commands"           # observations/pool_and_commands.py

[observation.pool_and_commands]
past_commands = 6

[action]
plugin = "linear_rate"                 # actions/linear_rate.py: a × the top pulse rate, rounded

[reward]
scheme = "tip_speed_cart_centre"       # the plugin: rewards/tip_speed_cart_centre.py (__synthetic__/trainer/python/reward-plan.md's calculation)

[reward.tip]                           # one table per factor, checked by the plugin; values: __synthetic__/trainer/python/reward-plan.md
width = 0.2

[reward.target_speed]
zone_dx = 0.154
limit = 0.5
width = 1.0
floor = 0.667

[reward.cart]
zone = 0.050
width = 0.25
floor = 0.667

[reward.reach]
width = 0.2
floor = 0.667

[reward.speed]
limit = 12.0
width = 1.0
floor = 0.250

[end]
plugins = ["strike_gap"]               # endings/strike_gap.py; each listed one is checked

[end.strike_gap]
buffer = 0.0136                        # b, metres

[train]
# ...as today

[sac]
# ...as today, gamma = 0.998
```

For any of the four plugins: changing its logic is a new file and its
name in the toml; tuning it is its table. Nothing else changes.

## 4. Build order

Each step ends with its check. Steps 2–10 need no orchestrator.

1. **Prerequisites in the design.** `synthetic-loop-dia.md`'s reward
   sections rewritten to the reward plan (`__synthetic__/trainer/python/reward-plan.md`),
   then `trainer-plan.md` rewritten from the diagram and this plan.
   *Check:* the diagram, `__synthetic__/trainer/python/reward-plan.md`, `trainer-plan.md` and
   this file agree.
2. **The contract types.** *Check:* tests build each from good values
   and refuse bad ones (a pool of the wrong shape, a site with two
   coordinates, a command outside the i32), each with a message naming the
   field.
3. **The settings loader and the plugin registries.** *Check:* tests
   load `gym.toml`, and refuse an unknown plugin name, a missing table
   and a value out of range, each with a message naming the key.
4. **The reward plugin, `tip_speed_cart_centre`.** *Check:* tests reproduce
   `__synthetic__/trainer/python/reward-plan.md`'s tables (`r_tip` from hanging, 0.011, to
   straight up, 1.00, wherever the cart is; `r_target_speed` 1 at or below
   0.5 rad/s and outside the zone, 0.667 passing straight up at 10 rad/s;
   `r_cart` 1 within 0.050 m of the middle, 0.737 at the left end, never
   down to its floor; `r_reach` 0.790 at 0.2 m from Target, never down
   to its floor; `r_speed` 1 at 12 rad/s, 0.526 at 13 rad/s, never down to
   its floor), and a second, trivial test plugin loads by name, so
   swapping a scheme is proven before anything depends on it.
5. **The end, observation and action plugins.** *Check:* the strike-gap
   test passes at the gaps of 0.440 and 0.445 m and ends within `b` of
   contact; the observation's space is 60 floats, its commands
   zero-filled at an episode's start, and its `build` takes no sites;
   `linear_rate` maps −1, 0 and 1 to − the top pulse rate, 0 and the
   top pulse rate.
6. **The episode tracker.** *Check:* capture 0 yields no `StepInputs`;
   the first scored step takes capture 0's sites and pole state and
   `c_(−1)` = 0.
7. **The fake link and the environment on it.** *Check:* Gymnasium's
   and SB3's environment checks pass; a crash ends with its reason; a
   time limit is `TimeLimit.truncated`.
8. **The run folder, the manifest and the step record.** *Check:* a
   save writes the manifest with the four plugin names, the git commit
   and the MJCF's time; a resume under a different observation plugin is
   refused by name.
9. **The buffer and the rescore.** *Check:* a short SAC run on the fake
   link stores `StepInputs` at every row; a resume with a changed
   `width` rescores every row to the value the plugin gives directly; a
   resume with a changed `buffer` is refused with its reason.
10. **The replay link and the measures.** *Check:* a recorded fake-link
    session replays to the same observations and rewards; each measure
    prints its table from the step record.
11. **The real link and the training run on the loop.** *Check:* the
    one-instance `env_check.py` run passes, then a
    short six-instance session runs, saves, resumes and rescores.
12. **Old runs through the new reward:** dropped (2026-10-28). The
    reward needs MuJoCo's pole state, which no old run recorded.
13. **The old code removed.** *Check:* nothing imports a removed module.

## 5. Dropped with the design they served

Not to be re-introduced as options:

- `loop_env.py`'s single-instance environment: one instance is a group
  of one (3.2, `env.py`).
- `observation_fit.py`: it fitted the markers to the truth so that the
  end-crash bounds and the reward could be stated in marker terms. Both
  now read the sites.
- `reward.py`'s pool-based scoring, its image angle and its marker
  centring, and `gym_settings`'s `[end_crash]` pixel bounds and penalty:
  replaced by the reward plugin and the strike-gap condition.
- `train.rescore`'s observation slicing: replaced by the stored
  `StepInputs`.
- The scratchpad analysis scripts (`session4.py`, `catches.py`,
  `schemes.py` and the rest): replaced by `measures/`.
- `loop_env.action_value` and `command`, the action mapping inside the
  environment: replaced by the action plugin.

## 6. Open decisions

Each with a recommendation; the choice is the user's.

- **Scope.** Recommended: all of `__synthetic__/trainer/python/src`, the link included, so
  no old piece constrains the new shape. The link's contract (R4) does
  not change, only its code's shape.
- **Test framework.** Python's own `unittest` needs no install;
  `pytest` is easier to read and run but is a new dependency, which
  needs permission (R8). Recommended: `unittest`, unless permission is
  given.
- **Old runs.** The rewrite will not resume runs 1 and 2: their
  observations, rewards and buffers differ. Recommended: fresh runs only;
  the old runs stay as records, rescorable offline from their
  `states.csv` (step 12).
- **The jump limit `m`.** Settled (user, 2026-10-28): measured on the
  rig (`jump-limit-results.md`) and held by the drive's ramp, not the
  reward; it is `settings.toml` `[drive] jump_limit`
  (`synthetic-loop-dia.md`, *The acceleration ceiling*).
- **The algorithm.** Recommended: keep SB3's SAC. The flexibility this
  plan needs is in the environment and the reward, not in the learner,
  and SAC has run both past runs.
