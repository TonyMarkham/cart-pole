# Gymnasium Research — what a trainer built on it would meet

The trainer's open decisions — the link to the orchestrator (17, 18,
26), the learning method and its tooling, the reward and the end
conditions (`trainer-plan.md`, *Not decided by the diagram*) — will
likely be made around Gymnasium, the standard Python interface between a
learning algorithm and an environment. This doc records what Gymnasium
and its main consumer, Stable-Baselines3, actually require, so later
sessions can decide from facts instead of researching again.

Status: **research** (2026-10-24), read from the projects' docs, source
and PyPI pages (*Sources*). It decides nothing: each decision still lands
in `synthetic-loop-dia.md` first (`AGENTS.md`). Nothing here is
installed; adding Gymnasium or SB3 to `trainer/` needs the user's
permission. Sections marked **For the loop** are this doc's reading of a
fact against the diagram, not decisions.

## Versions (checked 2026-10-24)

| Package | Latest | Released | Python | Notes |
|---|---|---|---|---|
| `gymnasium` | 1.4.0 | 2026-10-05 | ≥ 3.10 (3.10–3.14) | Farama Foundation; successor of OpenAI Gym |
| `stable-baselines3` | 2.9.0 | 2026-06-15 | ≥ 3.10 | needs `gymnasium>=0.29.1,<2.0`, `torch>=2.8,<3.0`, `numpy>=1.20,<3.0`, `cloudpickle` |

The trainer's `uv` project requires Python ≥ 3.12, inside both ranges.

**What Gymnasium is:** a Python library, not a program or a service.
There is no binary to run and nothing to connect to: the trainer imports
it, and the environment is a Python class in the trainer's own process.
Its core depends only on `numpy`, `cloudpickle`, `typing-extensions` and
`farama-notifications`, and its `pyproject.toml` builds no compiled
extension. The compiled parts are optional extras for its bundled
reference environments (`mujoco`, `box2d`, `atari`, …), which a custom
environment like ours does not need. Gymnasium supplies the `Env`
interface, the spaces, the wrappers, the vector environments and
`check_env`; the learning algorithms come from another library, such as
SB3, which depends on PyTorch.

Release changes that matter here:

- **1.0:** `VectorEnv` no longer inherits from `Env`; vector autoreset
  became next-step (see *Vector environments*); the plugin system was
  removed (external envs need an explicit import or `module:env_id`).
- **1.1:** all three autoreset modes in `SyncVectorEnv` and
  `AsyncVectorEnv`, chosen by `metadata["autoreset_mode"]`.
- **1.2.0:** dropped Python 3.8–3.9; the mujoco-py v2/v3 environments
  moved to Gymnasium-Robotics.
- **1.4.0:** `check_env` and the passive checker now require `reset` and
  `step` to return new observation and info objects every call (no
  reused buffers); `AsyncVectorEnv` gained `max_concurrency`.

## The `Env` contract

A custom environment subclasses `gymnasium.Env`, sets
`observation_space` and `action_space` in `__init__`, and implements:

- **`reset(*, seed=None, options=None) -> (observation, info)`** — back
  to an initial state. It must begin with `super().reset(seed=seed)`,
  which seeds `self.np_random` (a `numpy.random.Generator`); an integer
  seed resets the generator, `None` leaves it as it is. All of the
  environment's randomness should come from `self.np_random`.
- **`step(action) -> (observation, reward, terminated, truncated,
  info)`** — one timestep. After `terminated` or `truncated` is true,
  `reset()` must come before the next `step()`.
- **`close()`** — releases resources (windows, connections); closing
  twice must not raise.
- **`render()`** — optional, behaviour set by `render_mode` at
  construction (`None`, `"human"`, `"rgb_array"`, `"ansi"`).

`info` is a dict of diagnostics; the docs say the learning algorithm
should not use it. Each returned observation must lie in
`observation_space`.

**Registration:** `gymnasium.register(id="ns/Name-v0",
entry_point=..., max_episode_steps=...)`, then `gymnasium.make(id,
**kwargs)` (kwargs go to the constructor) or `gymnasium.make_vec(id,
num_envs=...)`.

**There is no Gymnasium config file.** An environment is configured in
code: `register()` builds an `EnvSpec` with the fields `id`,
`entry_point`, `reward_threshold`, `nondeterministic`,
`max_episode_steps`, `order_enforce`, `disable_env_checker`, `kwargs`,
`additional_wrappers` and `vector_entry_point`. The spec can be written
out and read back as JSON (`EnvSpec.to_json()`,
`EnvSpec.from_json(text)`), but only when nothing in it is a callable
(the entry point given as a `"module:Class"` string) and `kwargs` is
JSON-serializable. That is a serialization, not a file format people
write by hand. SB3 takes its hyperparameters in code too. The nearest
convention is RL Baselines3 Zoo's, a separate training tool: one YAML
file per algorithm (`hyperparameters/ppo.yml`), keyed by environment id,
with keys such as `policy`, `n_envs`, `n_timesteps`, `learning_rate`,
`normalize`, `env_wrapper` and `env_kwargs`.

## Who owns the clock

Gymnasium is turn-based: the environment is frozen between two `step()`
calls, and the agent calls `step()` when it is ready. Ramstedt & Pal
(NeurIPS 2019) call this the turn-based MDP and contrast it with
real-time settings, where the world moves while the agent thinks.

**For the loop:** the diagram already makes the loop turn-based from the
trainer's side. MuJoCo is frozen at the arrival step until the Orin's
reply (15–16), and the trainer's answer (18) is what the run waits on
next. So the Gymnasium calls line up with the diagram's elements:

| Gymnasium | The diagram |
|---|---|
| `reset(seed)` → first observation | 1–10: launch the orchestrator with that seed, then the first pool (17, n = 0) |
| `step(command)` → next observation | 18–21: the command reaches MuJoCo at the arrival step; 11–16: the next observation; 17: its pool |
| `close()` | 22–26: the shutdown, and the trainer told the run closed |

Two places where the shapes differ:

- **Continue/stop.** The diagram's answer (18) carries continue or stop
  beside the command. Gymnasium has no stop in the action: an episode
  ends when `step()` returns `terminated` or `truncated`, and the agent
  then calls `reset()` (a new run) or `close()`.
- **Where the reward is computed.** In Gymnasium, `step()` returns the
  reward and the end flags, so the environment class computes them. The
  diagram has the trainer generate the reward and decide the stop (T3,
  T4). Both hold if the `Env` class lives in the trainer's process: the
  reward is then trainer code, and the orchestrator still never sees it.

## The timing: a real-time, delayed problem

From the diagram's *Scoring lag*, the command chosen from pool n is set at
the arrival step that is also capture n + 1, so frame n + 1 is taken
before that command has acted. Each pool therefore reflects the commands
before the last one.

The literature on exactly this:

- **Real-Time RL** (Ramstedt & Pal, NeurIPS 2019): when the action chosen
  at step t acts only from t + 1, the previous action becomes part of the
  system's state. Their real-time MDP's state is (s, a_prev): the
  observation augmented with the action still in flight.
- **RL with Random Delays** (Ramstedt, Bouteiller, Beltrame, Pal, Binas;
  arXiv 2010.02966, ICLR 2021): with random observation and action delays,
  the observation alone is not Markov. Their Random-Delay MDP's state is
  the last observation, a buffer of the last K actions sent, and the two
  delays. With timestamps, the agent can tell how old each observation is
  and which actions have already reached the environment. Their DCAC
  algorithm builds on SAC and is evaluated on delay-augmented MuJoCo
  tasks.
- **SB3's tips:** "If there is some delay between action and observation
  (e.g. due to wifi communication), you should give a history of
  observations as input."

**For the loop:** the pool already is a history, 6 frames newest first,
with their ages, which are the real gaps (10–16 ms) between captures. The
actions the trainer sent are not in it, and the literature above says the
in-flight command is part of the state. Whether the trainer appends its
own recent commands to the observation is a decision for the
observation's design.

## `terminated` vs `truncated`

- **`terminated`:** "Whether the agent reaches the terminal state (as
  defined under the MDP of the task)", positive or negative.
- **`truncated`:** "Whether the truncation condition outside the scope of
  the MDP is satisfied", typically a time limit.

Learning algorithms must tell them apart. On termination the value
target is just the reward; on truncation the episode was cut short and
the target bootstraps from the next state's value:
`target = reward + gamma * (1 - terminated) * V(next)`. Treating a time
limit as a termination teaches the agent that the world ends at the
limit.

**For the loop:**

- The end crash (T4) is a termination: the task's terminal state.
- A time limit, if one is chosen, is a truncation.
- A failed run (3 failed frames, 26) is neither: it is the observation
  failing, not the task ending. Gymnasium has no third outcome. Raising
  an exception from `step()` ends training, which matches the diagram's
  failed run.

## Seeding, determinism and `check_env`

`gymnasium.utils.env_checker.check_env(env, skip_render_check=False,
skip_close_check=False)` checks, among other things:

- `reset()` returns `(obs, info)` with `obs` in the space and `info` a
  dict; `reset` accepts `seed` and `options`.
- **Reset determinism:** `reset(seed=123)` twice gives equal observations
  and equal generator states, and `reset(seed=456)` a different state.
- **Step determinism:** two seeded reset-then-step sequences give the same
  observation, reward, flags and info — skipped when the registered spec
  has `nondeterministic=True`.
- **No reused objects** (1.4.0): no observation or info object is shared
  between calls.
- Render and close checks, which create fresh environments from the spec.

`gymnasium.make` wraps the environment as
`TimeLimit<OrderEnforcing<PassiveEnvChecker<Env>>>`: `TimeLimit` only when
the spec has `max_episode_steps`, `PassiveEnvChecker` unless
`disable_env_checker=True`. `OrderEnforcing` raises `ResetNeeded` on a
`step()` before `reset()`.

**For the loop:**

- Each `reset()` would launch a whole run: the orchestrator, the
  synthetic Orin and the Unity player. `check_env` resets several times
  and creates new environments, so one check costs several run starts.
- The physics repeats bit for bit per seed, but the observation jitters
  with Unity's SSAO noise (the diagram's *Determinism*). The step check
  compares observations exactly, so it would fail unless the environment
  is registered `nondeterministic=True`. In the measured pair of runs,
  frame 0 was identical and later frames differed, so the reset check's
  equal first observation is what those runs showed — not a guarantee.

## Spaces

- **`Box(low, high, shape=None, dtype=np.float32, seed=None)`** — a
  bounded or unbounded array. `contains(x)` checks
  `np.can_cast(x.dtype, space.dtype)`, an exact shape, and
  `low ≤ x ≤ high` element-wise, both ends inclusive. Float64 array bounds
  given to a float32 Box are rounded, with a precision warning. An
  integer-dtype Box is effectively discrete.
- **`Discrete(n, start=0)`** — {start, …, start + n − 1}.
  **`MultiDiscrete(nvec)`** — a product of `Discrete`s. SB3 supports
  neither with `start ≠ 0`.

SB3's advice for continuous actions: use a symmetric range, typically
[−1, 1], and rescale inside the environment. Most of its algorithms
sample from a Gaussian centred at 0 with std 1, and wide unnormalized
bounds "can harm learning". It also advises normalizing observations
when their bounds are known (`VecNormalize` for PPO and A2C).

**For the loop:**

- **The observation (the pool, 17):** 6 × 9 floats. The coordinates are
  normalized to the frame, `(−1, −1)` when not detected, and the ages are
  0 for the newest entry, then negative milliseconds (*Contracts*). Its
  bounds are known, which is the case SB3 says to normalize.
- **The action (the command, 18):** an `i32` in pulses/s, clamped by the
  synthetic Arduino to ±45,333 (±0.68 m/s). A continuous policy in
  [−1, 1] scaled by 45,333 and rounded keeps the contract, at 15 µm/s per
  pulse/s of resolution. A `Discrete` set of speeds is the other shape.

## Vector environments

Gymnasium's `SyncVectorEnv` (in-process) and `AsyncVectorEnv` (one
subprocess per environment) batch several environments: rewards and
flags are arrays of length `num_envs`. The autoreset mode sets what
happens at an episode's end:

- **`NEXT_STEP` (the default):** the ending step returns the real final
  observation; the sub-environment resets on the following `step()`,
  whose transition must be masked out of training.
- **`SAME_STEP`:** the ending step already returns the reset observation;
  the final one is in `info["final_obs"]`. Some observation wrappers
  (`NormalizeObservation` among them) do not handle it.
- **`DISABLED`:** nothing resets until `reset(options={"reset_mask":
  ...})`.

Training code is written for one mode; the docs say it cannot be
switched between them.

**SB3 uses its own `VecEnv`** (`DummyVecEnv`, `SubprocVecEnv`), not
Gymnasium's: `step()` returns 4 values (`obs, rewards, dones, infos`,
with `dones = terminated or truncated`), it resets on the same step and
puts the final observation in `infos[i]["terminal_observation"]`, and
reports truncation as `infos[i]["TimeLimit.truncated"]`. `reset()` takes
no seed; `vec_env.seed(seed)` applies at the next reset.

**For the loop:** one environment per run means each sub-environment
launches its own orchestrator, Orin and player, each pair with its own
`orin_port` and `unity_port`. This is the orchestrator plan's *Later —
scaling*, seen from the trainer.

## The learning algorithms SB3 offers

SB3's guidance on choosing:

- **Continuous actions, one process:** SAC, TD3, CrossQ, TQC.
  **Several processes:** PPO, TRPO, A2C.
- **Discrete actions, one process:** DQN with extensions. **Several
  processes:** PPO, A2C.
- Model-free methods are sample-inefficient and "may need millions of
  interactions"; SB3 points to DroQ (in SBX) for extreme sample
  efficiency on continuous tasks.
- Start with a shaped, informative reward on a simplified problem, and
  test the environment with `check_env` and random actions first.

**For the loop**, as arithmetic: the Orin's round trip with the real
player was 18–40 ms per observation, mean 22 ms (a trainer run on this
PC, 2026-10-24: the `orin … ms` figure on each `observation` line the
orchestrator prints). So one environment gives at most about 45
observations per second of wall time, before the trainer's own time, and
a million observations take at least 1,000,000 × 0.022 s ≈ 6.1 hours.

## Reference tasks (for comparison, not design)

Gymnasium's own pendulum-on-a-cart tasks:

| | `CartPole-v1` (classic control) | `InvertedPendulum-v5` (MuJoCo) |
|---|---|---|
| Action | `Discrete(2)`: push left or right | `Box(−3, 3, (1,))`: force on the cart |
| Observation | cart position, cart velocity, pole angle, pole angular velocity | cart position, pole angle, cart velocity, pole angular velocity |
| Reward | +1 every step | +1 every step the pole is upright (\|angle\| < 0.2 rad) |
| Terminated | pole beyond ±12° or cart beyond ±2.4 | \|angle\| > 0.2 rad, or a non-finite state |
| Truncated | 500 steps | 1000 steps |
| Start | every value uniform in (−0.05, 0.05) | uniform within ±`reset_noise_scale` (0.01) |

Both observe the true state, velocities included, with no delay. Ours
observes marker positions in camera frames, with their ages and a
10–16 ms delay, and no velocities except what the pool's history implies.

## Sources

- Gymnasium on PyPI — https://pypi.org/project/gymnasium/
- Gymnasium release notes — https://github.com/Farama-Foundation/Gymnasium/releases
- Gymnasium `pyproject.toml` (dependencies, extras) — https://github.com/Farama-Foundation/Gymnasium/blob/main/pyproject.toml
- `Env` API — https://gymnasium.farama.org/api/env/
- Creating a custom environment — https://gymnasium.farama.org/introduction/create_custom_env/
- Handling time limits — https://gymnasium.farama.org/tutorials/gymnasium_basics/handling_time_limits/
- Fundamental spaces — https://gymnasium.farama.org/api/spaces/fundamental/
- `Box` source — https://github.com/Farama-Foundation/Gymnasium/blob/main/gymnasium/spaces/box.py
- `check_env` source — https://github.com/Farama-Foundation/Gymnasium/blob/main/gymnasium/utils/env_checker.py
- `EnvSpec` and `register` source — https://github.com/Farama-Foundation/Gymnasium/blob/main/gymnasium/envs/registration.py
- RL Baselines3 Zoo configuration — https://rl-baselines3-zoo.readthedocs.io/en/master/guide/config.html
- Misc wrappers — https://gymnasium.farama.org/api/wrappers/misc_wrappers/
- Vector API — https://gymnasium.farama.org/api/vector/
- Vector autoreset modes — https://farama.org/Vector-Autoreset-Mode
- `CartPole-v1` — https://gymnasium.farama.org/environments/classic_control/cart_pole/
- `InvertedPendulum-v5` — https://gymnasium.farama.org/environments/mujoco/inverted_pendulum/
- Stable-Baselines3 on PyPI — https://pypi.org/project/stable-baselines3/
- SB3 `setup.py` (dependencies) — https://github.com/DLR-RM/stable-baselines3/blob/master/setup.py
- SB3 RL tips — https://stable-baselines3.readthedocs.io/en/master/guide/rl_tips.html
- SB3 custom environments — https://stable-baselines3.readthedocs.io/en/master/guide/custom_env.html
- SB3 vectorized environments — https://stable-baselines3.readthedocs.io/en/master/guide/vec_envs.html
- Ramstedt & Pal, *Real-Time Reinforcement Learning*, NeurIPS 2019 — https://arxiv.org/abs/1911.04448
- Ramstedt, Bouteiller, Beltrame, Pal, Binas, *Reinforcement Learning with Random Delays*, ICLR 2021 — https://arxiv.org/abs/2010.02966
