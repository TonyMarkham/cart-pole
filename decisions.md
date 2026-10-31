# Decisions — removing the hard-coded cart-speed and stepper-pulse values

(2026-10-28: `reward-plan.md` below is the root reward plan of the time,
deleted that day, in git history at `1e6a1001`. The reward's source of
truth is now `trainer/reward-plan.md`.)

**The planner** in this file is the agent that wrote it: Claude, in the
planning session of 2026-10-25. It is not the agent that implements the
file. The implementer implements everything below, entries 1-9 and
*Implementing it*, as written.

This file exists because the planner cannot be trusted to implement a
change whose analysis it bluffed through (user, 2026-10-25: "YOU cannot
be trusted to implement something that you bullshit your way through
analysing"). It is the strategy for removing the hard-coded cart-speed
and stepper-pulse values from the loop's source code: the source of
truth each value is read from instead, and the code and documentation
changes that remove each hard-coded copy. Any implementation follows
this file, never the planner's recollection. Every claim names the files
and lines it rests on, so it can be checked by hand. It is a planning
record: nothing in it is implemented (user, 2026-10-25: "THIS IS A
PLANNING SESSION NOT A DOING SESSION").

The file has two kinds of entry, and says which each is:

- **Decided by the user** (1-4): the user's words, quoted.
- **Follows from the decisions** (5-9): what the user's decisions and
  stated sources of truth leave no choice about, each with what it
  follows from.

Both kinds are the plan, and both are implemented.

Reviewed 2026-10-25 by a second model (GLM-5.3-Flash, read-only); its
findings are applied here.

## Scope

These findings of `hard-coded-in-error.md`:

| # | Value | Where |
|---|---|---|
| 1 | the cart top speed, 850,000 µm/s | `arduino/src/config.rs:36` (`RIG`) |
| 2 | the rail travel, 450,000 µm | `arduino/src/config.rs:35` (`RIG`) |
| 3 | the top pulse rate, `MAX_RATE = 56_666` | `trainer/src/contracts.py:171` |
| 4 | the timestep, `STEP_MS = 2.0` | `trainer/src/endings/strike_gap.py:21` |
| 5 | the timestep, `GRID_MS = 2.0` | `trainer/src/observations/pool_and_commands.py:23` |
| 6 | the timestep, `STEP_MS = 2.0` | `trainer/src/link/fake.py:23` |
| 7 | 200 full steps per rev, 32 microsteps, 96,000 µm per pulley turn | `arduino/src/config.rs:32-34` (`RIG`) |
| 8 | metres per pulse, `METRES_PER_PULSE = 15e-6` | `trainer/src/endings/strike_gap.py:20`, `trainer/src/link/fake.py:24` |

Also `[sweep] peak = 54000` (`trainer/gym.toml:78`): a pulse rate typed
into a setting, which the user called "a redundant setting that should
have ALWAYS been pulled from the source of truth and not invented"
(entry 9).

The same values also sit in comments and docstrings, which
`hard-coded-in-error.md` did not audit (it stripped comments). They are
in scope: a comment that states a removed value goes stale with it. Each
entry's changes list them.

Out of scope:

- the fake rig's geometry (`fake.py:26-30`) and pendulum (`fake.py:33`),
  which are neither cart speed nor stepper pulses;
- the real board's firmware, which has no build. When it gets one it
  needs metres per pulse, the top pulse rate and its travel (entry 4).

## The aim

> "I'd rather codify the motor's spec sheet so that a value coming from
> the MJCF, settings.toml or gym.toml are all that's needed.
> Hard-Coded constants are pretty much universally a bad design"
> (user, 2026-10-25)

A constant set by a datatype's own limit is not in error: "A SEED LIMIT
IS a fair hard coded constant, it is dependant on the limitation of the
datatype" (user, 2026-10-25).

How the entries meet the aim: the motor's spec sheet is codified in
`motor-spec.md`, whose per-voltage tables give the actuator asset's
values. The user authors those values in Unity, and the export writes
them into the MJCF. From there the code reads the MJCF and
`settings.toml` only. Nothing checks the asset against `motor-spec.md`:
that step is the user's, by hand.

## Terms and sources of truth

> The user, 2026-10-25: "The single source of truth for Cart Speed is
> the velocity actuator values in the MJCF", and `[drive]` "gives
> correlation".
>
> **The MJCF** is the file `trainer/settings.toml`'s `[mjcf] model`
> names (line 95):
> `unity/projects/cart-pole/Assets/Simulation/Robot/asy-cart-pole.step~/asy-cart-pole.xml`.
> Not the file of the same name at
> `unity/projects/cart-pole/Assets/asy-cart-pole.step~/asy-cart-pole.xml`,
> a stale export of 2026-10-13 whose actuator (line 1410) is a position
> actuator with `ctrlrange` in metres.
>
> Every entry below uses these terms, and only these:
>
> | Term | Unit | Source of truth |
> |---|---|---|
> | cart speed | m/s | the MJCF's velocity actuator, `03-Cart.Slider.Actuator` |
> | cart top speed | m/s | that actuator's `ctrlrange`: ±0.36 since the user's re-export for the 12 V supply (the MJCF, line 1442) |
> | metres per pulse | m | `[drive]` in `trainer/settings.toml` (entry 3): belt per turn ÷ (full steps per rev × microsteps) = 96,000 µm ÷ (200 × 32) = 15 µm |
> | top pulse rate | pulses/s | none: derived, cart top speed ÷ metres per pulse = 0.36 ÷ 0.000015 = 24,000 |
> | pulse rate | pulses/s | the command on the link, chosen by the trainer (the policy, or the naive trainer's sweep) within ± the top pulse rate |
> | timestep | s | the MJCF's `<option timestep="0.002" ...>` (the MJCF, line 5) |
>
> The two conversions run in opposite directions, through the same
> metres per pulse:
>
> - **The limit, from the MJCF to pulses:** top pulse rate = cart top
>   speed ÷ metres per pulse.
> - **Each command, from pulses to the MJCF:** cart speed = pulse rate ×
>   metres per pulse (`orchestrator/src/main.rs:563-564`), which MuJoCo's
>   actuator follows within its `ctrlrange`.
>
> The motor's electrical figures (inductance, current, the torque curve)
> stay in `motor-spec.md` and are never read by code. The motor's step
> angle enters the code only through `[drive]`'s `full_steps_per_rev`.

## Decided by the user

> ### 1. The physics crate reads the cart top speed from MuJoCo
>
> The physics crate is the one way into MuJoCo, so it reads the cart top
> speed from the loaded model. `physics/src/ffi.rs` copies MuJoCo
> 3.15.0's model structure only up to `nsite` today (`ffi.rs:59-77`;
> `MJ_VERSION = 3015000`, line 17); it is extended as far as the
> actuator's `ctrlrange`. The orchestrator parses no XML.
>
> The user: "the only logical answer to me is the physics crate, THAT
> is the ABI to access MuJoCo infor. DO NOT parse the XML in orch".
> Confirmed: "correct, next".
>
> > #### Changes
> >
> > - `physics/src/ffi.rs`: extend the `mjModel` mirror field by field
> >   from MuJoCo 3.15.0's header, the one `orchestrator.toml`'s
> >   `mujoco_dir` names (line 8):
> >   `greyhound-snake/dist/mujoco/x86_64-windows/3.15.0/include/mujoco/mjmodel.h`.
> >   Today the mirror ends at `nsite` (header line 262); it extends to
> >   `actuator_ctrllimited` and `actuator_ctrlrange` (header lines
> >   807-808). That is every field between: the rest of the size
> >   section and most of the pointer section, a large mirror.
> > - `physics/src/lib.rs`: in `load`, prove the new fields as the
> >   existing ones are proven. Add `Physics::max_speed() -> f64`: the
> >   cart top speed, m/s.
> > - `load` refuses a model whose actuator is not `ctrllimited`, or
> >   whose `ctrlrange` is not symmetric (lo = −hi): *Terms and sources
> >   of truth* bounds every pulse rate by ± the top pulse rate, which
> >   assumes both.
> > - `physics/src/tests.rs`: `max_speed()` on the run's model (named in
> >   `settings.toml` `[mjcf]`) is 0.36.
> > - The timestep needs no change: `load` already measures it
> >   (`lib.rs:158`), and `Physics::timestep()` returns it
> >   (`lib.rs:178`).

> ### 2. The trainer asks the orchestrator
>
> The trainer is a separate Python program. It gets the cart top speed,
> the top pulse rate and the timestep by asking the orchestrator, which
> answers from the model MuJoCo loaded.
>
> The user: "100% the source of truth is the MJCF and python asks Orch
> for it". (The user also said "I KNOW A sounds like a Cache and B
> sounds like an API", which names the two forms the planner offered
> and picks neither; the planner then wrote "Recording #2 as B", and the
> user's reply was "NEXT!".) The messages follow in entry 6.

> ### 3. `[drive]` in settings.toml: metres per pulse
>
> The conversion between cart speed and pulse rate needs three figures
> the MJCF does not hold. They go in a `[drive]` table in
> `trainer/settings.toml`, passed to the orchestrator as flags
> (`--full-steps-per-rev`, `--microsteps`, `--belt-um-per-rev`) the way
> `[mjcf]` is passed today:
>
> ```toml
> [drive]
> full_steps_per_rev = 200   # the motor's 1.8° step (motor-spec.md)
> microsteps = 32            # the DRV8825's setting
> belt_um_per_rev = 96000    # 48T GT2 at 2 mm pitch (the CAD pulley)
> ```
>
> They replace `RIG`'s `full_steps_per_rev`, `microsteps` and
> `belt_um_per_rev` (`arduino/src/config.rs:32-34`).
>
> The user: "approved, next" (earlier: "sold, next").
>
> > #### Changes
> >
> > - `trainer/settings.toml`: the `[drive]` table above, and the comment
> >   listing the flags the trainer inserts (lines 63-67) gains its three.
> > - `trainer/src/settings.py`: read `[drive]`, each a whole number
> >   ≥ 1, and insert the three flags into the orchestrator's command
> >   beside the `[mjcf]` flags (`Settings.command`).
> > - `orchestrator/src/args.rs`: three required arguments,
> >   `--full-steps-per-rev`, `--microsteps`, `--belt-um-per-rev`.
> > - `orchestrator/src/main.rs:259-262`: build the synthetic Arduino's
> >   `Config` from those three and the cart top speed, in place of
> >   `RIG`. The cart top speed goes from `Physics::max_speed()`'s m/s to
> >   the `Config`'s whole µm/s rounded to the nearest µm/s: a plain
> >   floor of the float turns 2.01 m/s into 2,009,999 µm/s.
> >   `Config::max_rate()` (whole pulses/s, rounded down) is then the top
> >   pulse rate, and `meters_per_pulse` (line 557) reads the same
> >   `Config`.
> > - `arduino/src/config.rs`: delete `RIG`'s comment (lines 21-30) and
> >   `RIG` itself (lines 31-37), and its export in
> >   `arduino/src/lib.rs:28`.
> > - `arduino/src/config.rs`, the field comments: lines 7, 9 and 11-12
> >   state 200, 1/32 and 96,000; they say instead where each value
> >   comes from (`[drive]`). Line 17 calls `max_speed_um_per_s` "The
> >   motor's top speed": it is the cart top speed, in µm/s, from the
> >   MJCF.
> > - `orchestrator/src/main.rs:28`: `RIG` leaves the `use arduino::{...}`
> >   import.
> > - `orchestrator/src/main.rs`, the comments naming `RIG` or its
> >   values: line 260 ("Created with RIG alone"), line 298 ("holds only
> >   RIG"), lines 555-556 ("RIG: 96,000 µm / 6,400 = 15 µm") and lines
> >   561-562 ("RIG's top rate: 56,666 × 15 µm = 0.84999 m/s").
> > - Tests: `arduino/src/tests.rs` and `orchestrator/src/tests.rs`
> >   (lines 10, 127, 132) build a `Config` from test values in place of
> >   `RIG`. `orchestrator/src/tests.rs`'s comment (lines 123-124) and
> >   test name (line 126, `cart_velocity_is_step_rate_times_15_um_...`)
> >   state 15 µm and 56,666 → 0.84999: they describe the test's own
> >   `Config` values, and say so. `trainer/tests/support.py`'s settings
> >   fixture gains `[drive]`.

> ### 4. Travel leaves the synthetic Arduino's path
>
> `Config.travel_um` (`arduino/src/config.rs:16`, 450,000 at `:35`) is
> the real board's step-counting envelope: only `Stepper` reads it,
> through `travel_pulses()` (`arduino/src/stepper.rs:34, 54`). The
> synthetic Arduino never does; it only clamps the pulse rate
> (`arduino/src/synthetic.rs:28-29`). In the loop, where the cart meets
> the bases comes from the strike-face sites.
>
> The user: "THAT should have zero usage given MuJoCo state now carries
> site positions specifically to use REAL data from the physics
> source"; "Synthetic Arduino does not need this feature at all does
> it" ... "remove this useless artifact".
>
> The field leaves the `Config` the synthetic Arduino is built from, and
> the loop supplies no travel. Travel stays only with the board's
> `Stepper`, which no loop code uses (its only users are in
> `arduino/src/tests.rs`).
>
> > #### Changes
> >
> > - `arduino/src/config.rs`: delete the `travel_um` field and its
> >   comment (lines 14-16) and `travel_pulses()` with its comment (lines
> >   45-49) from `Config`.
> > - `arduino/src/stepper.rs`: `Stepper` takes its travel itself,
> >   `Stepper::new(config, travel_pulses, position)`, in place of
> >   reading `config.travel_pulses()` (lines 34 and 54); its comment at
> >   line 15 ("±`Config::travel_pulses`") names the new argument.
> > - `arduino/src/tests.rs`: the `Stepper` tests pass the travel they
> >   test (30,000 pulses today); the `RIG.travel_pulses()` assert
> >   (line 12) goes.
> > - The orchestrator: no change; it never reads travel.

## Follows from the decisions

> ### 5. The trainer's uses of the three values
>
> Follows from: the sources of truth (cart top speed and timestep from
> the MJCF, metres per pulse from `[drive]`, pulse rates derived) and
> entry 2 (the trainer asks). The user: "Did I NOT say that EVERY param
> has a known source of truth"; "So 5 is solved then right?"; and the
> correction it rests on, "That assumes top rate is Motor Pulses alone.
> Pulley dims effect ACTUAL cart speed".
>
> | | Where | Today | Becomes |
> |---|---|---|---|
> | a | `Command` check, `trainer/src/contracts.py:183` | ±`MAX_RATE`, 56,666 (`:171`) | ± the run's top pulse rate, checked in `trainer/src/env.py` before each answer |
> | b | `linear_rate`, `trainer/src/actions/linear_rate.py:27` | a × 56,666 | a × the run's top pulse rate |
> | c | strike-gap floor, `trainer/src/endings/strike_gap.py:29` | `MAX_RATE × METRES_PER_PULSE` (`:20`) × the longest gap from `STEP_MS` (`:21`) | the run's cart top speed × the longest gap from the run's timestep and `rnd_delay_max` |
> | d | naive trainer, `trainer/src/naive_trainer.py:38, 48` | `peak` checked against and scaled by 56,666 | entry 9 |
> | e | observation space, `trainer/src/observations/pool_and_commands.py:23, 44` | ages bounded with `GRID_MS = 2.0` | bounded with the run's timestep |
>
> > #### Changes
> >
> > - `trainer/src/plugins.py:34-38`: `RunFacts` gains the run's cart
> >   top speed, top pulse rate and timestep beside `rnd_delay_max_ms`,
> >   so every plugin receives them in `from_table` (entry 7 says when).
> > - `trainer/src/settings.py`: `Gym` keeps the `RunFacts` it was
> >   loaded with, as `Gym.facts`, so the environment can read them.
> > - `trainer/src/contracts.py`: delete `MAX_RATE` and its comment
> >   (lines 169-171). Today `Command` checks that the rate is a whole
> >   number and within ±`MAX_RATE`; it has no i32 check. It becomes: a
> >   whole number within the i32 range, −2³¹ to 2³¹−1, a datatype limit
> >   (the link's command is an `i32`). The top-pulse-rate check (a)
> >   moves to `trainer/src/env.py`,
> >   which reads the top pulse rate from `gym.facts` before each answer
> >   is sent.
> > - `trainer/src/actions/linear_rate.py:27`: command = a × the run's
> >   top pulse rate (b); its docstring (line 1, "a x 56,666") says the
> >   same.
> > - `trainer/src/endings/strike_gap.py`: the floor = the run's cart top
> >   speed × the longest gap, the gap from the run's timestep (c);
> >   delete `METRES_PER_PULSE` and `STEP_MS` (lines 20-21); the
> >   docstrings at lines 8 and 27 ("0.85 m/s x 16 ms", "13.6 mm") state
> >   the formula, not today's numbers.
> > - `trainer/src/observations/pool_and_commands.py`: the age bound
> >   from the run's timestep (e); delete `GRID_MS` (line 23).
> > - Tests. The tests' values for the cart top speed, the top pulse
> >   rate and the timestep live in one place, the fixture in
> >   `trainer/tests/support.py`; the fake link answers with them
> >   (entry 6), and tests that build `RunFacts` directly take them from
> >   there too: `trainer/tests/test_plugins.py:19` and
> >   `trainer/tests/test_settings.py:61, 80`. The literals of the old
> >   top pulse rate become the fixture's: `test_plugins.py:109, 114`
> >   (56666, 28333) and `test_episode_env.py:30` (1000 / 56666). The
> >   fixture's values are not 56,666 or 0.85 (step 6's check bans those
> >   literals from the tests), so the tests that depend on them change
> >   with them: the fixture's `buffer` default
> >   (`trainer/tests/support.py`, `GYM_DEFAULTS`), the expected
> >   strike-gap floor, and the strike-gap test's run length
> >   (`test_episode_env.py`).
> >   `trainer/tests/test_contracts.py:7, 40-43` tests `Command` against
> >   `MAX_RATE`: it tests the i32 range instead, and the top-pulse-rate
> >   check is tested where it moves, in the environment's tests
> >   (`trainer/tests/test_episode_env.py`).

> ### 6. The request and the answer on the link
>
> Follows from: entry 2 (the trainer asks; the orchestrator answers from
> the loaded model) and the link's existing form, one JSON object per
> line, the trainer writing to the orchestrator's stdin and reading its
> stdout (`synthetic-loop-dia.md`, *The trainer link*). The model is
> loaded once per run and cannot change during it, so one request at the
> start serves the run, and the trainer keeps the answer for it.
>
> - **The request**, the trainer's first stdin line:
>   `{"request": "run"}`.
> - **The answer**, the orchestrator's first stdout line, before the
>   first pools line: `{"run": {"cart_top_speed": 0.36,
>   "top_pulse_rate": 24000, "timestep": 0.002}}`, in m/s, pulses/s and
>   s, the names those of *Terms and sources of truth*.
>
> When it can be answered: the orchestrator reads no stdin before its
> first pools line today. Its only stdin read is `trainer_link.rs:113`,
> and `TrainerLink` is created in `run()` (`orchestrator/src/main.rs:443`,
> called at line 139), after every instance's model is loaded
> (`Simulation::new`, line 83). The request is read there, before the
> first pools line.
>
> > #### Changes
> >
> > - `synthetic-loop-dia.md`, *Contracts* and *The trainer link*: the
> >   two messages, first.
> > - `orchestrator/src/trainer_link.rs`: the request and answer message
> >   types, beside today's pools and answers lines; the stdin reader
> >   (line 113) reads the request first.
> > - `orchestrator/src/main.rs`: in `run()`, before the first pools
> >   line, read the request and answer it with `Physics::max_speed()`,
> >   `Config::max_rate()` (entry 3) and `Physics::timestep()`. Every
> >   instance loads the same `--model`, so instance 0's values are the
> >   run's.
> > - `trainer/src/link/__init__.py`: the `Link` interface gains `ask()`,
> >   called once, right after the link is opened and before anything
> >   else; it returns the run's cart top speed, top pulse rate and
> >   timestep. The caller builds the run's `RunFacts` from it (entry 7).
> > - `trainer/src/link/real.py`: `ask()` writes the request, then reads
> >   and checks the answer, and returns it.
> > - `trainer/src/link/fake.py`: the fake link is given the cart top
> >   speed, metres per pulse and timestep by the tests, from their
> >   fixture (`trainer/tests/support.py`), and `ask()` returns them. Its own `STEP_MS` (line 23) and `METRES_PER_PULSE` (line 24)
> >   are deleted, and their uses (lines 44 and 103-106) read the given
> >   values.
> > - Tests: `orchestrator/src/tests.rs` for the request and answer;
> >   `trainer/tests` for each link's answer.

> ### 7. The trainer asks before it builds anything that needs the values
>
> Follows from: entry 2 (the values come only from the orchestrator) and
> what needs them before the environment runs. The gym file's plugins
> are built and checked when it loads (`Gym.load`: the strike-gap
> `buffer` against its floor), and SB3 needs the observation space when
> the environment is constructed (`trainer/src/env.py:56`). Today the
> orchestrator is launched only at the environment's first `reset()`
> (`env.py:84`), after both. So the order changes: launch the
> orchestrator, ask, then load the gym file's plugins and build the
> environment with the answer.
>
> One part of the gym file is needed before the launch, and needs none
> of the answer: `[train]`. A training run's folder comes from `[train]
> runs_dir` (`RunFolder.create`, `train.py:151`; `RunFolder.latest` for
> `--resume`, `train.py:144`); the launch needs that folder for the
> orchestrator's log and `--out` (`train.py:128-131`), and the first
> episode's seed comes from it (`next_episode(folder)`, `train.py:156`).
> `Train.from_table` takes no run values. So `[train]` is read alone
> first, the rest of the gym file after the ask; the gym file is read
> twice. `measure.py:40` needs only `runs_dir`, with no orchestrator, so
> it reads `[train]` alone.
>
> > #### Changes
> >
> > - The order, everywhere a training run starts: read `[train]` alone
> >   (the run folder and the first episode's seed), open the link with
> >   them, `ask()` it (entry 6), build the run's facts from the answer
> >   (`Settings.facts`), `Gym.load(settings.gym, facts)`, then build the
> >   environment with the open link. `naive_trainer.py` and
> >   `env_check.py` need no run folder, so they start at opening the
> >   link.
> > - `trainer/src/settings.py`: a way to read `[train]` alone from the
> >   gym file (through `Train.from_table`, `trainer/src/learning.py`),
> >   without the run values; `Gym.load` still reads and checks the whole
> >   file, `[train]` included.
> > - `trainer/src/measure.py:40`: reads `[train]` alone for `runs_dir`.
> > - `trainer/src/settings.py:64-65`: `Settings.facts` takes the
> >   answer.
> > - `trainer/src/env.py`: `LoopVecEnv` takes the open, asked link in
> >   place of `open_link` (lines 45, 52, 84); its first `reset()` reads
> >   the first pools line instead of launching.
> > - `trainer/src/train.py`: `train(settings, resume, link)` loads the
> >   gym file itself, after the ask; it no longer takes a `Gym`.
> >   `main()` no longer loads the gym file before the launch (line
> >   222). The link is opened (`real_link`, lines 128-131) with the
> >   first episode's seed, and the environment built (line 158) after
> >   `Gym.load`.
> > - `trainer/src/naive_trainer.py:34, 42` and
> >   `trainer/src/env_check.py:22-23`: the same order.
> > - Tests, the same order:
> >   - `trainer/tests/test_episode_env.py` builds `LoopVecEnv` with a
> >     fake link opened and asked first;
> >   - `trainer/tests/test_train.py:39, 80, 84` call `train()` without a
> >     `Gym`, and `trainer/tests/support.py`'s `fake_link` returns the
> >     opener `train()` asks;
> >   - `trainer/tests/test_train.py:94` builds the replay's environment:
> >     it opens the `ReplayLink`, asks it (entry 8), loads the gym file
> >     with that answer, then builds `LoopVecEnv` with it.
> > - The consequence, named once: a gym file with an error is now found
> >   after the orchestrator, the Orins and the players have launched,
> >   not before.

> ### 8. Replay finds the values in the run's manifest
>
> Follows from: `rl-rewrite-plan.md` R7, "a run is self-describing: the
> toml it ran with ... is saved with each model", and the manifest that
> implements it (`trainer/src/run_folder.py:119`, written beside every
> saved model). The run's values are part of what the run ran with. The
> replay link reads a session's capture record today
> (`trainer/src/link/replay.py:22`, through `trainer/src/record.py`),
> which holds none of them.
>
> > #### Changes
> >
> > - `trainer/src/run_folder.py:119`: `build_manifest` records the
> >   orchestrator's answer.
> > - `trainer/src/link/replay.py`: `ask()` returns the values from the
> >   manifest in the run folder that holds the capture record.
> > - `trainer/src/run_folder.py:133`, `refusal`: a resume whose answer
> >   differs from the saved one is refused with the reason. A different
> >   answer changes the observation's bounds (timestep) and the
> >   strike-gap floor (cart top speed), the two things `refusal`
> >   already refuses to see change, through the observation and end
> >   tables; and it changes the action's scale (top pulse rate), which
> >   would make the stored actions mean other pulse rates.
> > - When `refusal` runs: `train.py:146` calls it with the manifest built
> >   from the loaded `Gym` (`run_folder.py:124`), and after entry 7 the
> >   gym file loads only after the ask. So all of `refusal`'s checks,
> >   the existing ones included, run after the launch: a refused resume
> >   is found after the orchestrator, the Orins and the players are up,
> >   and stops them.

> ### 9. `[sweep] peak` is a fraction of the top pulse rate
>
> Follows from: the user, "the 54000 is a redundant setting that should
> have ALWAYS been pulled from the source of truth and not invented",
> and the sources of truth (the top pulse rate is derived, never typed).
> What stays a setting is the choice the sweep makes: how hard it
> drives, as a share of what the rig can do. `peak = 54000` was 0.953
> of the old 56,666.
>
> > #### Changes
> >
> > - `trainer/gym.toml:78`: `peak` becomes a fraction, 0 < peak ≤ 1;
> >   0.953 keeps today's meaning (54,000 ÷ 56,666). Its comments (lines 74, 77) state the
> >   formula, not pulse rates.
> > - `trainer/src/naive_trainer.py:38, 48`: read `peak` as that
> >   fraction; the command at step n is peak × the run's top pulse rate
> >   × cos(2π n / period). Its docstring (line 10, "peak / 56,666")
> >   follows.

## Documentation, before the code

`AGENTS.md`: truth flows from the diagram to the participant plans to
the code. These state the values being removed or the order being
changed, and change first.

> ### `synthetic-loop-dia.md`
>
> - Lines 147 and 217 (R3): the synthetic Arduino is created with, and
>   holds, `RIG`.
> - Line 181, the Arduino's note: "rate clamped to ±56,666 pulses/s",
>   and "the ±30,000-pulse travel limit".
> - Line 183: "cart velocity = STEP rate × 15 µm".
> - Lines 278-292, *The trainer's environment*: "The first `reset()`
>   starts the run (1)" (entry 7), and line 290, "the command is a ×
>   56,666 (the top rate)".
> - Line 351 and lines 440-442: the 13.6 mm the cart travels past a
>   check.
> - Lines 431-439, *Numbers*: `arduino::RIG`'s figures, the 56,666
>   pulses/s top rate and 0.85 m/s.
> - Lines 536-544, *Contracts*, the command and the cart speed: the top
>   rate, 15 µm and the MJCF's `ctrlrange` (±0.85 m/s).
> - *Contracts* and *The trainer link*: entry 6's request and answer.
> - The sequence chart: entry 6's request and answer as a note before the
>   loop begins (line 156), not as two new messages. The chart uses
>   `autonumber` (line 132), which numbers messages and not notes; two
>   new messages would shift today's 11-26 by two, and those numbers are
>   cited throughout the diagram, the plans, code comments and
>   `reward-plan.md` (lines 38, 578), which is the user's. As a note,
>   every existing number stays valid.

> ### The participant plans
>
> - `synthetic-arduino-plan.md`: lines 17, 21-24, 28, 35, 39, 47-51,
>   70-71, 79, 83, 87, 93, 97 (`RIG`, the ±56,666 clamp, 15 µm, the
>   ±30,000-pulse travel, 45,333 at 24 V, 0.85 m/s).
> - `orchestrator-plan.md`: lines 58 and 102 (`RIG`, 15 µm), line 223.
> - `mujoco-plan.md`: lines 302, 306 and 315 (`ctrlrange` ±0.85 m/s,
>   "set as 0.85"), line 648.
> - `trainer-plan.md`: line 36 (T1, the flags inserted after the program:
>   `[drive]`'s join `[mjcf]`'s), lines 53 (13.6 mm), 55 and 85 (±56,666), 57 (T5a,
>   the link: entry 6's request and answer), 73 ("The first `reset()`
>   launches", entry 7), 149 (−56,666, 0, 56,666).

> ### Other plans and the crates' own documents
>
> - `rl-rewrite-plan.md`: lines 100 ("`Command` (the i32 and its
>   ±56,666 bound)"), 131 and 218 ("`a × 56,666`"), 257 ("a command past
>   ±56,666") and 271 ("−56,666, 0 and 56,666").
> - `physics/README.md:40`: "MuJoCo clamps it to the MJCF's `ctrlrange`
>   (±0.85 m/s)". Already stale (the MJCF says ±0.36); it names the
>   source, the `ctrlrange`, without a number, and entry 1's
>   `Physics::max_speed()` joins the README's table of functions.

> ### Reference and research documents naming the removed code
>
> - `motor-spec.md`: lines 136-138, the sentence "The same 0.85 m/s is
>   the synthetic Arduino's top rate, 0.85 m/s ÷ 15 µm = 56,666 pulses/s
>   (`arduino::RIG`) ...", and lines 255-256 (the
>   top rate in "`arduino::RIG` ... and the trainer's `MAX_RATE`", the
>   floor "computed from `MAX_RATE`").
> - `rig-drive-research.md`: lines 13, 33, 48 and 130 name
>   `arduino::RIG` and `RIG.max_rate()` as where the figures live. A
>   research document records its findings as found; these lines say
>   instead where the figures live after this work (`[drive]` and the
>   MJCF).

> ### The trainer's toml comments
>
> - `trainer/gym.toml`: line 30 (`linear_rate: a x 56,666`), line 66
>   (`buffer`'s floor, 0.85 m/s and 13.6 mm), lines 74, 77 and 80-81
>   (entry 9; lines 80-81's "a swing of about +-0.26 m at the peak
>   above" depends on `peak`).

> ### Owned by the user
>
> `reward-plan.md` is the source of truth for the reward, changed only
> by the user. Three of its lines state the removed values:
>
> - line 365, in section 4: the command "at most ±56,666";
> - line 423, in section 4's sources: the diagram's "rate clamped to
>   ±56,666";
> - section 6: `b`'s floor, "0.85 m/s × 16 ms ≈ 13.6 mm".

> ### Left as they are, on purpose
>
> - `hard-coded-in-error.md`: the audit this plan is drawn from; it
>   records the values as found.
> - `trainer-observations.md` (e.g. line 49, 13.6 mm): the user's journal
>   of runs, a record of what each run used.
> - `gymnasium-research.md:238-239` (±56,666, ±0.85 m/s, 15 µm): research
>   describing the loop as it was when written; it names no code
>   location.
> - `HANDOFF.md` (line 65, 13.6 mm): ephemeral, never committed.

## Implementing it

The order to implement in, how each step is checked, and what is not
part of this work.

> ### Order, each with its check
>
> 1. **The documentation above**, except *Owned by the user* and *Left
>    as they are, on purpose*. Check: none of the lines listed still
>    states `RIG`, its figures, the 56,666 clamp, 0.85 m/s, 13.6 mm or
>    the ±30,000-pulse travel, or that the first `reset()` launches the
>    run; the diagram states entry 6's two messages.
> 2. **Entry 1, the physics crate.** Check: `cargo test -p physics`
>    passes, including the new test that `max_speed()` on the run's
>    model is 0.36.
> 3. **Entry 4, travel out of `Config`.** Before entry 3, because entry
>    3 builds a `Config` without `RIG` and must not need a travel value.
>    Check: `cargo test -p arduino --features synthetic` passes (the
>    `synthetic` feature compiles the synthetic Arduino's tests,
>    `arduino/src/tests.rs:103, 111`).
> 4. **Entry 3, `[drive]`, and `RIG` deleted.** Check: `cargo test -p
>    arduino --features synthetic` and `cargo test -p orchestrator`
>    pass; `uv run --project trainer python -m unittest discover -s
>    trainer/tests -t trainer/tests` passes with the fixture's
>    `[drive]`; `RIG` appears nowhere in `arduino/` or `orchestrator/`.
> 5. **Entry 6, the orchestrator's side.** Check: `cargo test -p
>    orchestrator` passes with unit tests beside today's link tests
>    (`orchestrator/src/tests.rs:23-53`): the request line is parsed, a
>    malformed request is refused, and the answer line holds the three
>    values it is given. No test runs the whole orchestrator: every run
>    with the trainer link launches the Orins and the players, and the
>    only run without them (`--no-render`, `orchestrator/src/main.rs:61-67`)
>    has no trainer link. The values themselves are checked in step 2
>    (`max_speed()`) and by `Config::max_rate()`'s test in step 4.
>    `cargo build --release -p orchestrator` succeeds, run only when no
>    training run is going.
>
>    Between steps 5 and 6 the loop cannot run, knowingly: the
>    orchestrator waits for a request the unchanged trainer never sends,
>    so `naive_trainer.py` and `env_check.py` hang. Step 6 ends that.
> 6. **Entries 5-9, the trainer's side,** together: none of them works
>    alone, since each needs the answer entry 7 obtains. Check: `uv run
>    --project trainer python -m unittest discover -s trainer/tests -t
>    trainer/tests` passes; `MAX_RATE`, `METRES_PER_PULSE`, `STEP_MS`
>    and `GRID_MS` appear nowhere in `trainer/src` or `trainer/tests`,
>    and neither do the literals `56666`, `56_666` or `28333`;
>    `trainer/gym.toml`'s `peak` is a fraction.

> ### Not part of this work
>
> - `reward-plan.md`'s three lines (*Owned by the user*): the user's
>   file, left for the user.
> - A live run through the loop: it needs `trainer/gym.toml`'s
>   `jump_limit`, which `reward-plan.md` leaves unmeasured and the user
>   sets. Every check above runs without it.
