# Trainer observations

The journal of how we learned from the policy learning (user,
2026-10-24): what each balance-policy training run actually did, what we
worked out from it, and what we changed because of it. Each entry gives
the reward it ran on, the exact command, and what came out, so the next
change to the reward or the settings starts from evidence. Written for
someone new to reinforcement learning: each entry explains the ideas it
needs (what the replay buffer is, why a reward can be gamed) where they
come up, with the numbers that show them. Measurements, not
plans: each number says where it was read. The reward's design lives in
`synthetic-loop-dia.md` (*The reward is graded*, *The pole pays only when
it is slow*), and the trainer's steps live in `trainer-plan.md`.

## Run 1: `trainer/runs/20261024_041436`, the graded reward

### The command

From the repo root, a new run:

```
uv run --project trainer python trainer/src/train.py
```

It ran 2026-10-24 from 04:14 to 05:39, 1 h 25 min, with no stops and no
resume. Settings, as copied into the run's folder at its start:

- **Group:** 4 training instances (`settings.toml` `[orchestrator]
  instances`), seed 0, so episode e ran with seed e. Latency band 8–15 ms.
- **Length:** 200,000 steps in all, the four instances counted together.
  Episodes of at most 1,500 steps, about 19 s of simulated time.
- **SAC:** on the 3090. Learning from step 10,000, replay buffer 1,000,000,
  batch 256, learning rate 0.0003, gamma 0.99, tau 0.005, one update per
  step of the group, two layers of 256.

### The reward it ran on

Each step is scored from the pool alone: the markers the Orin detected,
never MuJoCo's truth.

- **Pole part:** (1 − cos θ) / 2. θ is the Pivot-to-End direction in the
  image. It is 0 hanging, 0.5 level and 1 upright.
- **Cart part:** 1 − u². u is how far the Pivot marker sits from the middle
  of the end-crash bounds, as a fraction of half their width. It is 1 in the
  middle and 0 at a bound.
- **Step reward:** 1.0 × pole part + 0.5 × cart part. A pole hanging with
  the cart in the middle scores 0.5 a step; upright in the middle, 1.5.
- **End crash:** the Pivot marker's x outside 0.26325–0.72639 (the slide's
  ends less 13.6 mm). The step scores −10 and the episode ends.

So a 1,500-step episode scores about 750 for doing nothing, and at most
2,250 for holding the pole upright in the middle the whole time.

### What happened

From `episodes.csv`, 183 episodes, grouped by the run's step count when
each episode ended:

| Steps | Episodes | Crashes | Mean reward | Pole max | Pole mean | Reached upright |
|---|---|---|---|---|---|---|
| 0k–20k | 12 | 0 | 706 | 0.040 | 0.006 | 0 |
| 20k–40k | 12 | 1 | 681 | 0.049 | 0.007 | 0 |
| 40k–60k | 15 | 3 | 617 | 0.052 | 0.007 | 0 |
| 60k–80k | 34 | 29 | 272 | 0.095 | 0.020 | 0 |
| 80k–100k | 40 | 36 | 239 | 0.229 | 0.079 | 4 |
| 100k–120k | 17 | 1 | 1,332 | 1.000 | 0.450 | 17 |
| 120k–140k | 13 | 1 | 1,288 | 1.000 | 0.438 | 13 |
| 140k–160k | 12 | 0 | 1,420 | 1.000 | 0.467 | 12 |
| 160k–180k | 15 | 0 | 1,405 | 1.000 | 0.470 | 15 |
| 180k–200k | 13 | 0 | 1,432 | 1.000 | 0.469 | 13 |

"Pole max" and "pole mean" are the pole part, 0 hanging and 1 upright.
"Reached upright" counts the episodes whose pole part touched 1.0.

1. **Up to ~60,000 steps: sitting still.** The pole hung and the cart
   stayed near the middle, earning most of the 750 a still cart gets.
2. **~60,000–100,000: shoving the cart.** It pushed hard enough to swing
   the pole, and nearly every episode ended in the end crash: 65 of 74.
   Short episodes and −10 crashes pulled the reward down to ~250.
3. **Step 94,204 (episode 109):** the pole reached upright for the first
   time.
4. **From ~100,000: swinging over the top every episode.** Almost no
   crashes (the last was episode 130). Reward 1,300–1,440, with the best
   episode, 172, scoring 1,458.6.

### What the policy actually does: it spins the pole

The pole mean settling at 0.47 was the clue. A balanced pole would sit
near 1.0; a pole turning steadily averages 0.5. MuJoCo's true pole angle
(`target/trainer-run/instance_k/states.csv`, read before the test runs
replaced it, see *Files*) confirms it. In each
instance's last full episode:

| Instance | Episode (its own count) | Full turns in 1,500 steps | Steps within 20° of upright |
|---|---|---|---|
| 0 | 43 | 54.2 | 9% |
| 1 | 45 | 46.6 | 10% |
| 2 | 43 | 52.5 | 11% |
| 3 | 48 | 55.3 | 10% |

That is about 2.8 turns a second, with the cart kept in the middle. In
instance 3's episode 48, the pole passed through upright at 5.8 to
26.9 rad/s, median 20.1.

**Why:** the reward paid for height alone. Spinning collects half the pole
part on every step, with no risk of a crash. Balancing would pay more
(about 2,100 an episode against 1,440), but to get there the policy must
first learn to stop the pole at the top, and spinning pays well long
before that. This is a known trap of swing-up tasks, not a fault in the
loop: the loop ran 200,000 steps on four instances with no failure.

### What was going on inside SAC

**The parts.** SAC (Soft Actor-Critic) is three small neural networks
and a memory:

- **The actor** is the policy. It takes the pool (54 numbers: 6 entries
  × an age and 4 markers' x and y) and gives the command, a number from
  −1 to 1 that becomes the motor's pulse rate. This is what would one day
  run on the Orin.
- **Two critics** each learn to guess how much reward is still to come
  from a pool and a command. That is "how good is this situation if I do
  this?". There are two because a single critic tends to over-guess, and
  SAC trusts the lower of the two.
- **The replay buffer** is the memory: every step taken, saved as a
  record (the pool, the command, the reward, the pool that came back,
  whether the episode ended). It holds up to 1,000,000; run 1 filled
  199,996.
- **Learning** is SAC drawing 256 random records from the buffer, nudging
  the critics to guess better on them, then nudging the actor toward
  commands the critics rate higher. That is one *update*. With 4 instances
  and `gradient_steps = 1` there is one update per 4 new steps, so run 1
  made (200,000 − 10,000) / 4 = 47,500 updates.

**How far ahead it looks.** `gamma = 0.99` means a reward n steps in the
future counts 0.99ⁿ as much as one now. That halves after about 69 steps
and is down to a third after 100, and 100 steps is about 1.3 s of
simulated time. So the critics mostly value the next second or two. That
is long enough to see that swinging up pays, and too short to see that
an episode of balancing beats an episode of spinning.

**The phases, explained:**

1. **Steps 0–10,000: random.** Before `learning_starts` SAC learns
   nothing; it picks commands at random to fill the buffer. A new random
   command every 12 ms mostly cancels out, so the cart jitters near the
   middle and the pole barely moves (pole max 0.03–0.04, 20–23° from
   hanging).
2. **To ~60,000: sitting still.** A centred cart with a hanging pole earns
   0.5 a step, about 750 an episode, with no risk. The critics learned
   that first, and the actor kept the cart quiet. That is a safe strategy
   that pays, and nothing in it hints that the pole could do better.
3. **~60,000–100,000: pushing hard, and crashing.** Somewhere the critics
   noticed that a swinging pole scores a little higher. To swing the pole
   the cart has to move hard, and hard moves on a 0.9 m slide mean
   crashes: 65 of the 74 episodes in this stretch ended at −10. The crashes
   were the learning: each one taught the critics where the ends are, and
   the pole max climbed from 0.10 to 0.36 while it happened.
4. **Step 94,204: over the top.** The pole reached upright for the first
   time, and within 10,000 steps every episode did.
5. **From ~100,000: the spin settles in.** Spinning paid about 0.96 a step
   (0.47 for the pole, plus the cart part, 0.5 × ~0.98) with no crashes,
   and the reward crept from ~1,300 to ~1,440 as it got smoother.

**Why spinning is a trap.** Balancing would pay more, up to 1.5 a step.
But the policy can only change a little at a time, and every small step
from spinning toward balancing pays *less* at first. A pole slowed at the
top falls back down one side, and the steady income of the spin is gone
before balancing has been learned. So SAC stays where it is: the best
choice among its near neighbours, not the best overall. That is called a
*local optimum*. The fix is to change the reward so that the neighbours
pay more, and that was Change 1.

**Exploration: how random it stays.** SAC adds randomness to the actor's
commands, so that it keeps trying things it has not tried. It also pays
itself a bonus for being random, the *entropy* bonus. The size of that
bonus, the *entropy coefficient*, is tuned automatically toward a set
amount of randomness (SB3's default target, −1 for one command). Read
from run 1's snapshots (`model_<steps>.zip`):

| Steps | Entropy coefficient |
|---|---|
| 10,000 | 1.0 |
| 20,000 | 0.47 |
| 30,000 | 0.22 |
| 40,000 | 0.11 |
| 50,000 | 0.050 |
| 60,000 | 0.024 |
| 70,000 | 0.011 |
| 80,000 | 0.0055 |
| 90,000 | 0.0031 |
| 100,000 | 0.0044 |
| 120,000 | 0.0086 |
| 140,000 | 0.016 |
| 160,000 | 0.028 |
| 180,000 | 0.027 |
| 200,000 | 0.041 |

How to read it:

- **It starts at 1.0.** At that size the randomness bonus is worth about
  as much as the task's own reward (0.5–1.5 a step), so early on SAC is
  paid to be random.
- **Then it falls by the same factor, about 0.47, every 10,000 steps.**
  That is the speed of a steady push. Each update moves its logarithm by
  about the learning rate, 0.0003, when the push keeps the same direction:
  2,500 updates (10,000 steps ÷ 4) × 0.0003 = 0.75, and e^−0.75 = 0.47.
  So for 90,000 steps the actor was more random than the target, and SAC
  kept turning the bonus down. (This entry first called that the fastest
  it can move. It isn't: Adam, the optimizer, can take steps a little
  bigger than the learning rate, and after the resume it did, 0.00043 an
  update; see *The randomness bonus* in the next entry.)
- **At 90,000 it turns round.** That is just before the first swing over
  the top. Once the actor committed to a strategy it became less random
  than the target, so SAC turned the bonus back up, and it rose tenfold
  by 200,000.
- **What that means for the resume:** the actor isn't frozen. SAC is
  still paying it to try things, which is what it needs in order to find
  its way from spinning to balancing.

### Also found: a bug in what SAC stored at an episode's end

**The bug:** when an episode ended, `vec_env.py` handed SAC the episode's
last pool as a view of the array it then overwrote with the next episode's
first pool. So SAC stored the wrong pool as "the pool after the last step".

**What it affected:** only 183 of the 199,996 stored steps, the steps that
ended an episode.
- The rewards were right: they were scored before the overwrite.
- A crash step is terminal, so SAC never looks at the pool after it, and
  crashes were unaffected.
- At a time-limit end, SAC estimated the value of what follows from the
  next episode's start instead of from the pole's real state. That is 112
  steps in 200,000.

Fixed (see the change below).

### Files

All in `trainer/runs/20261024_041436/`:
- `episodes.csv`: one row per episode;
- `loop.log`: the run's own output;
- `latest.zip` and `latest_buffer.pkl`: the model and replay buffer at
  step 200,000;
- `model_<steps>.zip`: snapshots every 10,000 steps.

Run 1's frames and true states (`states.csv`, MuJoCo's cart and pole per
step) were in `target/trainer-run/instance_0` to `instance_3`, and are
gone: the short test runs for Change 1 wrote to the same folder and
replaced them (2026-10-24, noticed after the measurements above were
taken). Every run wrote there, settings.toml's `--out`, so each run
replaced the last one's. Fixed: see *Each run keeps its own loop output*
below.

## Change 1: the pole pays only when it is slow (2026-10-24)

**The new pole part:** (1 − cos θ) / 2 × exp(−(ω / 4)²). ω is how fast the
pole is turning, in rad/s. The trainer reads it from the newest two pool
entries with both Pivot and End: the change in θ between them, over the
gap between their ages. The 4 is `gym.toml`'s `[reward] pole_speed`. The
cart part, the weights and the end crash are unchanged.

**What it pays:**
- A still pole keeps its full height, so hanging in the middle still
  scores 0.5 a step and upright 1.5.
- At 2 rad/s the height pays 0.78 of itself; at 4 rad/s, 0.37.
- At the spin's 20 rad/s through the top it pays 1 in 10¹¹.
- With fewer than two entries holding both markers, the speed is unknown
  and the pole part is 0.

**Why this shape:**
- **Multiplied, not subtracted.** The speed factor multiplies the
  height, so it only bites where the pole is high. Hanging, the height is
  0 and speed costs nothing, so the policy can swing as hard as it likes
  low down: swinging up still needs speed, and it is still paid. A
  penalty *subtracted* for speed would cost the same at the bottom, and
  the cheapest way to avoid it would be to never swing at all, back to
  sitting still.
- **What it pays most for** is arriving at the top slowly and staying
  there: high and slow is the only way to collect the full pole part.
- **A bell shape, exp(−(ω/4)²).** It is flat near 0, so small wobbles
  while balancing cost little, and it falls to nearly nothing well before
  the spin's speeds.
- **Why 4 rad/s.** Candidates were measured on run 1's last spinning
  episode (instance 3, episode 48), scoring every step of it with each
  candidate:

  | pole_speed | The spin's pole part, per step | Kept at a 1 rad/s wobble | at 2 rad/s | at 3 rad/s |
  |---|---|---|---|---|
  | 2 | 0.011 | 0.78 | 0.37 | 0.11 |
  | 3 | 0.016 | 0.89 | 0.64 | 0.37 |
  | **4** | **0.020** | **0.94** | **0.78** | **0.57** |
  | 5 | 0.024 | 0.96 | 0.85 | 0.70 |

  All of them kill the spin (0.475 before), so the choice is about the
  balance. 4 is a judgement call between the two ends. 2 or 3 would
  punish an honest wobble hard while balancing is still being learned.
  5 is gentler still, and lets a pole passing slowly over the top keep
  more. If the next run balances but wobbles, a higher value is the knob
  to try.
- **The speed from the markers is good enough.** On that episode, the
  speed read from the markers matched MuJoCo's true speed within
  0.03 rad/s (median), 0.08 at the 95th percentile.

**Its effect on run 1's own steps,** rescored with the new reward:
- the mean reward per step falls from 0.720 to 0.480;
- over the last 100,000 steps (the spin) it falls from 0.932 to 0.503.
  That 0.503 is the cart part alone: the spin now earns nothing for the
  pole.

**Reading the console under the new reward:**
- **Reward:** a spinning or hanging episode now scores about 750 (the cart
  part only); one that holds the pole up and slow approaches 2,250.
  Anything well above 750 means the pole spends time up and slow.
- **Pole max and mean:** still the height alone, 0 hanging and 1 upright.
  Spinning keeps the mean at ~0.47; balancing pushes it toward 1.

### Continuing a run on a changed reward

A resume now rescores the run's replay buffer with `gym.toml`'s `[reward]`
as it is at the resume. Without that, run 1's 200,000 stored steps would
go on paying the spin, and SAC would keep learning it from them.

**How:** SAC learns from its replay buffer, not from the episodes as they
run. Every step it took is saved there as one record:

- the pool it saw;
- the command it chose;
- the reward that step scored;
- the pool that came back after the command;
- whether the episode ended there, and how.

The reward was computed from that returned pool and nothing else
(`reward.py`'s `score()`). So the resume goes through every record, runs
the returned pool through `score()` again with the `[reward]` in
`gym.toml` now, and writes the result over the stored reward. Nothing
else in the record changes: the pools, the command and the episode end
are what really happened. Only the score given to them is new. The code
is `rescore()` in `trainer/src/train.py`.

**Why it is exact:** `score()` sees the same pool it saw when the step ran,
so the new number is exactly what the step would have scored had the new
reward been in place then. The proof was to run it with the old reward:
it gave back the stored numbers (see *Checked*).

**When it happens:** at the resume, in memory, before training goes on.
The rescored buffer reaches the run's folder at the next save
(`[train] save_every` steps later), and until then the files on disk
still hold the old rewards. `episodes.csv` is never rescored: its rows
keep the reward each episode was scored with when it ran.

**What a resume does:**
- It checks every stored crash against the current `[end_crash]`, and
  refuses the resume if the crash bounds or the penalty have changed: that
  would move where episodes end.
- It copies the settings it resumed with into the run's folder, as
  `settings_from_<steps>.toml` and `gym_from_<steps>.toml`.

**The exception, run 1 only:** its 183 episode ends lost their last pool
to the bug above, so they can't be rescored, and the resume drops them
from the buffer. That leaves 199,813 steps, every one exact. It also
drops all 71 of its crash steps (the −10s); the steps that led up to each
crash stay.

**A different number of instances:** change `instances` (since
2026-10-31, `instance_rows` and `instance_columns`) in
`trainer/settings.toml` and resume. The buffer is laid out one column per
instance, and the resume regroups it into the new number of columns. SAC
picks stored steps at random, and each step carries everything it needs,
so the regrouping loses nothing. Only up to 5 of the oldest steps are
dropped, to fill whole rows.

**Checked:**
- Run 1's buffer, rescored with the old reward, gave back every kept
  reward within 1.8 × 10⁻⁷. Regrouped for 6 instances it kept 199,812
  steps, and a second rescore (what a later resume does) changed nothing.
- A new short run, 4 instances, 1,200 steps, was resumed three times:
  - to 1,800: every episode end held its own last pool, and rescoring
    reproduced every reward within 6 × 10⁻⁸;
  - to 2,400 with 6 instances, then to 3,000 with 4: each time the buffer
    was regrouped and the rescore changed nothing, and the run closed
    with nothing left on the loop's ports.

`gym.toml`'s `[train] total_steps` is now 400,000. That lets run 1 carry on
for 200,000 more steps, and it is also the length of any new run.

### Each run keeps its own loop output

**The problem:** the orchestrator writes every frame and MuJoCo's true
state for each step (`states.csv`: the cart's position and the pole's
real angle). These files are the only way to check what the pole really
did, as against what the markers said. Until now every launch wrote them
to the same folder, `target/trainer-run`, so each run replaced the one
before it, and run 1's were lost that way.

**The fix:** `train.py` now gives each launch its own folder inside the
training run's folder, `loop_from_<steps>/`, named by the step it started
at. A new run writes `loop_from_0/`; a resume at step 200,000 writes
`loop_from_200000/`, beside it, never over it. Inside, one `instance_k/`
per instance holds its frames, `states.csv` and `observations.jsonl`.
`settings.toml`'s `--out` is still the folder for runs outside training
(the naive trainer, `env_check`).

**Checked:** a new short run with 6 instances (1,200 steps) and a resume
to 1,800. The run wrote `loop_from_0/` and the resume `loop_from_1200/`,
each with 6 instance folders: 202 and 101 frames per instance, and a true
state per step. `target/trainer-run` was untouched, and no process was
left on the loop's ports.

**Disk:** about 21 KB a frame, so 200,000 steps is about 4 GB in the run's
folder. D: had 3.3 TB free.

### The command to continue training on the new reward

From the repo root:

```
uv run --project trainer python trainer/src/train.py --resume
```

**What it does:**
- It continues run 1 (`trainer/runs/20261024_041436`, the most recent run
  with a save) from step 200,000 to 400,000, on as many instances as
  `trainer/settings.toml` says: 6 now. Run 1 took 1 h 25 min for 200,000
  steps on 4; 6 took the same (see *Six instances were no faster than
  four* in the next entry).
- It starts from run 1's policy, which already knows how to swing the pole
  up, and now has to learn to stop it at the top.

**What you should see:** its first three lines are

```
train: 183 stored episode ends lost their last pool (saved before vec_env.py's fix) and are dropped
train: the replay buffer, saved with 4 instance(s), is regrouped for 6, keeping the newest 199,812 of 199,813 rescored steps
train: resuming D:\git\cart-pole\trainer\runs\20261024_041436 at step 200000, episode 183, 6 instance(s); rescored 199,813 stored steps with gym.toml's [reward], mean 0.720 -> 0.480 a step
```

Episode numbers carry on from 183. The rows of `episodes.csv` from
episode 183 on are scored with the new reward; the rows before, with the
old. The frames and true states of this session land in
`trainer/runs/20261024_041436/loop_from_200000/instance_0` to
`instance_5`.

To start fresh on the new reward instead, a new run of 400,000 steps:

```
uv run --project trainer python trainer/src/train.py
```

## Run 1, continued: 200,000 to 400,000 steps on the slow-pole reward

### The command

From the repo root, the resume:

```
uv run --project trainer python trainer/src/train.py --resume
```

It ran 2026-10-24 from 12:27 to 13:52, 1 h 25 min, with no stops. It
carried on from run 1's policy and replay buffer at step 200,000, rescored
with Change 1's reward. Settings, as copied into the run's folder
(`settings_from_200000.toml`, `gym_from_200000.toml`):

- **Group:** 6 training instances (run 1 had 4). Episode numbers carry on
  from 183, each episode e with seed e.
- **Length:** to 400,000 steps (`[train] total_steps`), so 200,000 more,
  the six instances counted together. Episodes of at most 1,500 steps.
- **SAC:** unchanged from run 1.
- **Reward:** Change 1, with `pole_speed = 4.0`.

### What happened

160 episodes, 183 to 342, grouped by the run's step count when each
episode ended. The first five columns are from `episodes.csv`. The rest
are from MuJoCo's true pole angle
(`loop_from_200000/instance_k/states.csv`), per episode:

| Steps | Episodes | Crashes | Mean reward | Full turns per episode | Spinning episodes | Slow episodes | Within 20° of upright | Longest hold | Slow visits to the top |
|---|---|---|---|---|---|---|---|---|---|
| 200k–220k | 12 | 0 | 818 | 32.7 | 8 | 0 | 8.3% | 0.31 s | 1 |
| 220k–240k | 38 | 38 | 233 | 1.7 | 0 | 0 | 1.5% | 0.20 s | 0 |
| 240k–260k | 16 | 5 | 599 | 28.1 | 11 | 0 | 8.9% | 0.43 s | 1 |
| 260k–280k | 12 | 0 | 774 | 38.9 | 12 | 0 | 10.2% | 0.30 s | 0 |
| 280k–300k | 14 | 0 | 795 | 35.6 | 14 | 0 | 9.8% | 0.42 s | 3 |
| 300k–320k | 13 | 0 | 784 | 32.1 | 13 | 0 | 9.0% | 0.79 s | 1 |
| 320k–340k | 12 | 0 | 873 | 14.1 | 4 | 5 | 7.0% | 0.76 s | 7 |
| 340k–360k | 17 | 4 | 782 | 21.2 | 10 | 3 | 6.9% | 0.54 s | 9 |
| 360k–380k | 13 | 1 | 840 | 17.5 | 5 | 3 | 6.4% | 0.54 s | 11 |
| 380k–400k | 13 | 0 | 859 | 13.4 | 3 | 8 | 6.4% | 0.70 s | 7 |

What the columns mean:
- **Full turns:** (highest − lowest pole angle in the episode) ÷ 2π.
- **Spinning episodes:** 15 turns or more.
- **Slow episodes:** fewer than 10 turns, and no crash.
- **Within 20° of upright:** the share of the episode's steps.
- **Longest hold:** the longest unbroken time within 20° of upright, in
  any episode of the band.
- **Slow visits to the top:** visits that stayed within 20° of upright
  for 0.3 s or more. A visit is one stretch of steps there, from arriving
  to leaving.

The phases:

1. **200,000–220,000: still spinning, now for nothing.** The first six
   episodes (183–188, one per instance) are run 1's policy as it was,
   38–54 turns each. Under the new reward the spin earns only the cart
   part, so they scored 754–788, about the 750 of doing nothing.
2. **221,000–244,000: crashing.** 42 episodes in a row ended at the end
   crash, after 124 to 1,290 steps (17 of them within 250 steps, 3 s),
   with the pole hardly turning. Run 1 had not crashed since episode 130.
3. **244,000–320,000: back to spinning, but slower.** 28–39 turns an
   episode on average, against run 1's 47–55, and still nothing for the
   pole.
4. **From ~320,000: slow arrivals at the top.** Episodes with fewer than
   10 turns appear, 19 of the last 55, spread over all six instances. In
   them the pole comes up, slows almost to a stop at the top, and falls
   off again. Slow visits to the top went from 0–3 per band to 7–11.
   The best episode, 327 (instance 1, step 377,156), scored 1,048, the
   highest since the reward changed.

### The pole was starting to balance, and why the score looks small

**What was seen in the players:** the pole held at the top for a moment.
Those are the slow visits. Episode 327 had seven visits of 0.15 s or
more. In them the pole turned at 0.7–3.0 rad/s on average, against the
spin's 14–26 rad/s through the top. In two of them it came almost to a
standstill (0.01 and 0.07 rad/s at the slowest). Each time it then fell,
either back the way it came or over to the other side.

**What it has learned, and what it hasn't:**
- **Learned:** bring the pole up with only just enough speed to reach the
  top. That is the first half of balancing. Before 320,000 almost no
  visit fell back the way it came (4, all short): the pole went over.
  From 320,000, 18 did.
- **Not yet learned:** the second half, the catch. Once the pole leans,
  the cart has to move under it, and that hasn't happened yet. The
  longest hold in the whole run is 0.79 s.

**Why the score barely moves:** a slow visit pays well, but only while it
lasts. Near the top and slow, the pole part is about 0.8–1.0 a step. A
0.5 s visit is about 40 steps, so it adds about 35. Episode 327's visits
added about 300 to the 750 a still cart earns, which made 1,048. Holding
the pole up for the whole 19 s would score about 2,250. So an episode in
the 850–1,050 range means the pole was caught for a moment a few times.
The reward will only climb steeply once the holds get long.

**One policy, six worlds.** The six instances don't learn separately.
They are six copies of the loop, all driven by the same actor and all
feeding the same replay buffer. When one instance holds the pole for a
moment and the others don't, that is chance, not a better learner: a
different seed, a different start, a different moment of the swing.
After 320,000, every instance had at least one slow episode (instance 0:
5 of 9; instance 1: 4 of 9; instance 5: 1 of 9).

### Why it crashed at 221,000 (the likeliest reading)

The resume dropped run 1's 183 episode ends, because the old bug had
lost their last pool (*The exception, run 1 only*, in Change 1). That
took **every** crash record out of the buffer, all 71 of them. The steps
leading up to each crash stayed, but no record said "here the episode
ended, at −10".

The critics only know what the buffer shows them. With no crash record
left, nothing kept reminding them that the slide's ends are costly, and
after about 20,000 steps (3,500 updates) that knowledge had worn away.
The actor drove into the ends, and each crash put a new record back. By
244,000 there were 42 of them, and the crashes nearly stopped (6 more in
the remaining 156,000 steps).

This is a reading, not a proof: no measurement here isolates it. But the
timing fits: the first crash came 21,000 steps into the resume, after
none in run 1's last 52 episodes. It cost about 23,000 steps, and it
came from a choice made in Change 1: to drop the lost episode ends
rather than keep approximate records of them. A run on the fixed code
never loses an episode end, so this can't recur.

### Why it went back to spinning

Under the new reward, spinning and hanging earn the same: the cart part
only. Nothing pushes the actor off the spin it already knows. So once
the crashes were learned again, it fell back into the spin, but more
loosely, with about two thirds of the turns. What changed things is that
a pole arriving at the top *slowly* does get paid, and SAC's randomness
made some arrivals slow. From 320,000 those arrivals were common enough
for the critics to learn that they pay.

### The randomness bonus

The entropy coefficient (see *Exploration: how random it stays* in run
1), read from the snapshots:

| Steps | Coefficient |
|---|---|
| 200,000 | 0.041 |
| 210,002 | 0.020 |
| 220,004 | 0.010 |
| 230,000 | 0.0062 |
| 240,002 | 0.0046 |
| 260,000 | 0.0058 |
| 280,004 | 0.0062 |
| 300,002 | 0.0047 |
| 320,000 | 0.0045 |
| 340,004 | 0.0055 |
| 360,002 | 0.0053 |
| 380,000 | 0.0053 |
| 400,004 | 0.0058 |

The snapshot steps aren't round any more: a save lands on the first group
step past each 10,000, and the group now takes 6 steps at a time.

- **It halved every 10,000 steps for the first 20,000.** It falls when the
  actor is more random than SAC's target, so right after the reward
  changed, the actor became more random. The likeliest reason: the
  rescored critics no longer rated the spin's commands above the others,
  so the actor had no reason to keep them sharp.
- **That fall was faster per update than anything in run 1.**
  0.041 → 0.020 is a change of 0.72 in its logarithm over 1,666 updates:
  0.00043 an update, above the learning rate of 0.0003. Adam, the
  optimizer, scales each step by how strong the push has been lately
  against how strong it has been over a long time. So after a sudden
  change it can step a little faster than the learning rate. That
  corrected run 1's entry, which had called 0.0003 the limit.
- **Then it held at about 0.005 for the last 160,000 steps,** near run
  1's lowest (0.0031). The actor stayed close to the target randomness
  throughout, so the coefficient had no reason to move.

### Six instances were no faster than four

- **Steps an hour: the same.**
  - Run 1 ran 200,000 steps on 4 instances in 1 h 24 min 44 s; this resume
    ran 200,000 on 6 in 1 h 25 min 0 s.
  - Episodes of 1,500 steps took 155–158 s on 4 and 227–231 s on 6.
  - That is 38.5 steps a second against 39.2.
- **Each instance's step cost the same ~26 ms either way.** One step of the
  whole group took about 104 ms on 4 and 153 ms on 6, as if the instances
  were served one after another, not side by side. Where that happens was
  measured after the run (below, *Why: the GPU takes one process at a
  time*).
- **Fewer updates for the same steps.** SAC makes one update per step of
  the whole group (`gradient_steps = 1`), so 6 instances means one update
  per 6 new steps, not per 4.
  - This resume made 33,333 updates for its 200,000 steps (the snapshots'
    update counter went from 47,499 to 80,832).
  - On 4 instances, the same steps would have given 50,000.
  - So each record was learned from about a third less often, in the same
    time.

So in this run, 6 instances gave the same amount of experience as 4,
and less learning from it.

### Why: the GPU takes one process at a time

Measured 2026-10-24 after the run, with throwaway test runs of 300 group
steps (their files in the scratchpad, not in `trainer/runs`).

**Not the orchestrator.** It already sends every instance's pose before
it waits for any reply, so all six Orins and players work at the same
time. Its own CPU use was too small to show.

**Each part of a step got slower as instances were added.** From the
loop's log (`orin` is the time from sending a pose to getting its pool
back; `trainer` is the time the trainer takes to answer) and the Orins'
`detect_ms`:

| Instances | Orin round trip | of which detection | Trainer's turn | One group step |
|---|---|---|---|---|
| 1 | 28.5 ms | 5.1 ms | 22.3 ms | ~51 ms |
| 6 | 92 ms | 35 ms | 57 ms | ~150 ms |

So 1 instance makes about 20 steps a second and 6 make about 39: six
times the instances for twice the steps.

**Detection, timed on its own** (the Orin's own detector code on 300 saved
frames, with nothing else running):

| Detectors running at once | TensorRT time per frame |
|---|---|
| 1 | 2.0 ms |
| 3 | 6.5 ms |
| 6 | 13.3 ms |

It is exactly in proportion. A GPU runs one *process* at a time. Each
process (each Orin, each Unity player, the trainer's PyTorch) gets its
own context on the GPU, and Windows hands the GPU to the contexts in
turns. Work from two processes never runs side by side, even when each is
far too small to fill the GPU. (Decoding and resizing the JPEG, on the
CPU, barely changed: 0.6 → 0.9 ms.)

**Why the GPU still shows under 50%:** each of these jobs is tiny, and
after each one its process stops and waits for the result (TensorRT's
`cudaStreamSynchronize`, Unity's `ReadPixels`) before it sends more. The
GPU then sits idle through the switch to the next process and the
waiting. It is busy less than half the time, and still everyone queues
for it. Utilization measures how full the GPU is, not whether processes
are waiting their turn.

**What else is on the GPU, and on the CPU.** The Unity players redraw
nonstop between requests. The project sets no frame cap
(`QualitySettings` `vSyncCount: 0`, no `targetFrameRate`), and the
diagram notes that "the player renders between triggers". During a
6-instance test each player used about 2.2 CPU cores (13 cores' worth
for six), and the whole CPU sat at 55%, which is the Task Manager graphs'
50–60% on every core. Every redraw is also another turn on the GPU that
the Orins and SAC wait behind. A detection benchmark run beside the
6-instance loop took 8.9 ms against 2.6 ms idle.

### Files

All in `trainer/runs/20261024_041436/`:
- `episodes.csv`: the rows from episode 183 on are this resume, scored
  with the new reward;
- `loop_from_200000/instance_0` to `instance_5`: about 33,360 frames per
  instance, plus each instance's `states.csv` and `observations.jsonl`,
  5.0 GB in all;
- `latest.zip` and `latest_buffer.pkl`: the model and the buffer at step
  400,004;
- `model_<steps>.zip`: the snapshots every 10,000 steps, run 1's and this
  resume's.

### How it was measured

With read-only scratch scripts, run by `uv run --project trainer`:
- **The table's last five columns:** from each `states.csv`.
  - A row's episode is the instance's own count (0, 1, 2…).
  - An instance's j-th row in `episodes.csv` from episode 183 on is its
    j-th episode there.
- **"Upright":** the pole angle within 20° of π, once whole turns are
  taken off.
- **A visit falls back** when the pole leaves upright on the side it
  arrived from.
- **The coefficient and the update count:** from each snapshot,
  `SAC.load(path).log_ent_coef.exp()` and `_n_updates`.

### To continue

`[train] total_steps` is 400,000 and the run has reached it, so a plain
`--resume` now only says it is already done. Raise `total_steps` in
`trainer/gym.toml` first: `train.py` reads it again on every resume.
(Raised to 600,000 on 2026-10-24: see *Continuing a run: raise
total_steps first*, below.)

## Change 2: the players draw only when asked (2026-10-24)

**The problem it solves:** six instances ran no faster than four (*Six
instances were no faster than four*, above). Every process that uses
the GPU queues for it, one at a time, and the Unity players were the
worst offenders. They did nothing useful between frames, yet each one
redrew its window nonstop: about 2.2 CPU cores each, and one more turn
on the GPU per redraw, which the Orins' detections and SAC's updates
had to wait behind.

**What changed** (`SimulationLoopController.cs`, in the
`simulation.cart-pole.sim-loop` package; the diagram's *The players draw
only when asked*):
- **The scene renders only for a trigger.** The player's camera is
  switched off, so Unity never renders it on its own. When a pose
  arrives, the player renders it once, into the 728×544 frame it sends
  to the Orin, exactly as before.
- **The window shows that same frame,** stretched to the 960×720 window.
  It used to be a second, separate render of the scene. Between frames
  it holds the last one, so during a reset, or while the trainer thinks,
  the picture stands still.
- **Between triggers the player sleeps.** It used to check for a
  message on every frame, as fast as it could loop, which is what kept a
  core busy. Now it waits for the next message and wakes the moment one
  arrives, or after 100 ms at most so the window stays responsive. So no
  trigger waits for the player's next frame to come round.

**Also new, for watching rather than speed:** the windows open in a grid
(4 across at 960×752 pixels apart, from `trainer/settings.toml`'s
command), each player moved into place by the new `PlayerController`.

**What it should do, and what it can't:**
- **Should:** free about 13 CPU cores, and every GPU turn the redraws
  took. Detection idle took 2.6 ms, but 8.9 ms beside the running loop,
  so the redraws were costing every other process too.
- **Can't:** remove the queue itself. Six Orins detecting at once still
  take turns on the GPU (13.3 ms for six, measured with no Unity at
  all), and so do the six renders. Making the instances truly side by
  side would need one detector serving all six frames in one call. That
  would change the loop's design, where each instance has its own Orin
  just as the robot has its own, so it isn't done here.
- **A side effect:** the frames may now repeat exactly from run to run.
  The SSAO shading's noise changed on every render, and the players used
  to render between triggers a varying number of times. Now a run's nth
  frame is its player's nth render. Not checked yet, and nothing depends
  on it.

**Not measured yet.** The player must be rebuilt first (the user
validates Unity). Then the same short test as before, with 1 and 6
instances, gives the before-and-after:
- each instance's step parts: the `orin … ms` and `trainer … ms` in the
  run's `loop.log`, and `detect_ms` in `observations.jsonl`;
- each player's CPU use while the run goes.

The test is a training run on copies of the two tomls: 300 group steps
(`total_steps` = 300 × instances), `learning_starts = 100`, `cwd` set to
the repo's absolute path, and `runs_dir` outside `trainer/runs`, so no
real run is touched.

**Before** (from *Why: the GPU takes one process at a time*):

| Instances | Orin round trip | of which detection | Trainer's turn | Steps per second |
|---|---|---|---|---|
| 1 | 28.5 ms | 5.1 ms | 22.3 ms | about 20 |
| 6 | 92 ms | 35 ms | 57 ms | about 39 |

**After** (2026-10-24, the rebuilt player, the same test):

| Instances | Orin round trip | of which detection | Trainer's turn | Steps per second |
|---|---|---|---|---|
| 1 | 22.4 ms | 5.4 ms | 17.8 ms | about 25 |
| 6 | 45.5 ms | 15.6 ms | 17.0 ms | about 96 |

"Steps per second" is the instances ÷ (round trip + trainer's turn), the
same sum that matched the real runs' 39 before.

What it shows:
- **Six instances now run about 2.5 times as fast as before:** about 96
  steps a second against 39. 200,000 steps would take about 35 minutes
  instead of 1 h 25 min (an estimate from the step's parts, not yet a
  real run).
- **The trainer's turn no longer grows with the instances:** 17 ms
  for 1 and for 6, against 22 and 57 before. SAC's update on the GPU was
  waiting behind the players' redraws.
- **Detection still grows,** from 5.4 to 15.6 ms. That is the six Orins
  taking turns on the GPU, the part this change can't remove (13.3 ms
  for six with no Unity at all).
- **The frames are unharmed:** in both tests the detector found all
  four markers in all 301 of instance 0's frames.
- **The CPU:** the six players' load was gone. A sample taken
  35 s in found no process above 0.1 core, but the 6-instance test had
  already finished by then, so it doesn't show a player during a run.
  The Task Manager graphs during the run looked "much improved" (the
  user).

### Prototype: 12 instances, a 4 × 3 grid (2026-10-24)

The instance limit went from 6 to 12 (user, 2026-10-24), and
`trainer/settings.toml` now says `instances = 12`. The same test with 12:

| Instances | Orin round trip | of which detection | Trainer's turn | Steps per second |
|---|---|---|---|---|
| 1 | 22.4 ms | 5.4 ms | 17.8 ms | about 25 |
| 6 | 45.5 ms | 15.6 ms | 17.0 ms | about 96 |
| 12 | 85.3 ms | 28–30 ms | 18.5 ms | about 116 |

- **Twice the instances, 20% more steps.** The trainer's turn stays at
  ~18 ms, but the Orin round trip nearly doubles. The detections queue
  for the GPU about 2.3 ms each (28 ms for 12, 15.6 for 6), and the
  renders queue the same way. So the round trip grows with every
  instance, and each one added buys less: the queue for the GPU is now
  the limit, as the measurement without Unity predicted.
- **The frames are unharmed:** all four markers in all 301 frames of
  instances 0 and 11.
- **Resources, 25 s in:** CPU 40% in all. The 12 players used 7.5 cores
  together, 0.6 each, and 7.6 GB of memory; the Pythons (12 Orins and the
  trainer) 3.7 cores and 6.2 GB. 20.9 GB of memory was still free, and the
  GPU held 9.6 of its 24 GB.
- **Learning per step halves again.** One update per group step
  (`gradient_steps = 1`) is one update per 12 new steps, against one per
  6 on 6 instances and one per 4 in run 1. Each stored step is learned
  from half as often as on 6, in the same time. Whether that matters is
  for a training run to show; `gradient_steps` in `gym.toml` is the knob.
- **The grid:** the windows went to x = 0, 960, 1920, 2880 and y = 0,
  752, 1504. Three rows of 752 px are 2,256 px tall, 96 more than a 4K
  display's 2,160, so the bottom row's lowest 96 px (and whatever the
  taskbar covers) are off screen.

### How many instances to train with (an extrapolation)

**Two things set how fast SAC learns, and instances trade one for the
other:**
- **Steps a second:** new experience coming into the replay buffer.
- **Updates a second:** SAC actually learning from it. One update costs
  about 16 ms: the trainer's turn is 0.6–2.2 ms before learning starts
  and 16.4–17.8 ms once it has, at 1, 6 and 12 instances alike.

With `gradient_steps = 1` there is one update per group step, so every
instance added makes the group step longer and the updates fewer.

**The model.** A group step takes the Orin round trip plus ~1.5 ms of the
trainer's own plus 16 ms per update. Measured round trips: 22.4 ms on 1,
45.5 on 6, 85.3 on 12. Between those, a straight line: about 4.6 ms per
added instance up to 6, and about 6.6 ms per instance beyond 6. The rows
for 4 and 8 instances are estimates; 1, 6 and 12 were measured.

| Instances | `gradient_steps` | Steps/s | Updates/s | Updates per step |
|---|---|---|---|---|
| 1 | 1 | 25 | 25 | 1 |
| 4 | 1 | ~74 | ~19 | 1/4 (run 1's) |
| 6 | 1 | 96 | 16 | 1/6 |
| 8 | 1 | ~105 | ~13 | 1/8 |
| 12 | 1 | 116 | 9.7 | 1/12 |
| 6 | 2 | ~76 | ~25 | 1/3 |
| 12 | 2 | ~101 | ~17 | 1/6 |

**What it shows:**
- **Steps a second level off.** The GPU queue makes each instance cost
  ~6.6 ms of the group step, so the steps a second can never pass about
  150, however many instances run. Beyond 6, each added instance brings
  only ~3 more steps a second.
- **Updates a second only fall** as instances are added, unless
  `gradient_steps` rises, and that costs steps a second instead.
- **12 instances with `gradient_steps = 2` are about the same as 6 with
  1:** 101 against 96 steps a second, 17 against 16 updates a second,
  for twice the windows, memory and GPU memory.

**Recommendation: 6 instances, `gradient_steps = 1`.** That gets 83% of
12's steps a second with 64% more updates a second, and each stored step
is learned from twice as often. The six windows fit in two rows. Run 1
learned at one update per 4 steps, so one per 6 is close to what has
been seen to work. Whether more updates per step would learn faster in
the hour is not known: a training run would have to show it.

**The bigger lever is the update's cost.** 16 ms for one update of two
layers of 256, at a batch of 256, is slow. Its cause hasn't been measured.
If an update cost a few ms, the same hour would hold several times the
updates at any instance count.

## Continuing a run: raise `total_steps` first

**Every resume, the same two steps:**

1. **Raise `[train] total_steps` in `trainer/gym.toml`** past the step the
   run has reached. It is the run's finish line, counted over the whole
   run, every instance's steps together, not the steps of one session. A
   run that has reached it resumes, prints `train: already at step <n> of
   <total_steps>`, and stops. `train.py` reads the value again on every
   resume, so raising it is all a longer run needs.
2. **Then, from the repo root:**

   ```
   uv run --project trainer python trainer/src/train.py --resume
   ```

   It continues the most recent run in `trainer/runs/` that has a save.

**Where things stand (2026-10-24):** run 1
(`trainer/runs/20261024_041436`) is at step 400,004, and `total_steps`
is now 600,000. So the next `--resume` runs about 200,000 more steps:
about 35 minutes on 6 instances, an estimate from the speed test. It
writes that session's frames and true states to `loop_from_400004/`.
The buffer was saved with 6 instances and the reward hasn't changed, so
nothing is regrouped, and the rescore changes nothing. No episode ends
are dropped either: the ones that had lost their last pool went at the
last resume, and every step stored since holds its own.

For the session after that, raise it again: to 800,000, for another
200,000.

(Later the same day: that session ran, in 34 min, and run 1 is now at
step 600,002. See the next entry. `total_steps` was then raised to
800,000 with Change 3, for another 200,000, and then to 1,100,000:
sessions are now 500,000 steps, *The command*, in Change 3.)

## Run 1, continued: 400,000 to 600,000 steps

### The command

The same resume, from the repo root:

```
uv run --project trainer python trainer/src/train.py --resume
```

It ran 2026-10-24 from 14:45:46 to 15:20:08, **34 min 22 s**, with no
stops. It carried on from step 400,004 with the policy, the critics and
the replay buffer as they were. Settings, as copied into the run's folder
(`settings_from_400004.toml`, `gym_from_400004.toml`):

- **Group:** 6 training instances, as in the last session, now in the
  3 × 2 window grid, with the players drawing only when asked (Change 2).
  Episode numbers carry on from 343.
- **Length:** to 600,000 steps, so 200,000 more.
- **SAC and the reward:** unchanged (Change 1's slow-pole reward,
  `pole_speed = 4.0`). So nothing was rescored, regrouped or dropped.

### What happened

132 episodes, 343 to 474, 22 per instance. **Every one ran the full
1,500 steps: no crashes at all.** The columns are the same as in the last
entry (*What happened*, above, says what each one means):

| Steps | Episodes | Crashes | Mean reward | Full turns per episode | Spinning episodes | Slow episodes | Within 20° of upright | Longest hold | Slow visits to the top |
|---|---|---|---|---|---|---|---|---|---|
| 400k–420k | 12 | 0 | 812 | 4.7 | 1 | 10 | 1.5% | 0.32 s | 3 |
| 420k–440k | 12 | 0 | 812 | 3.7 | 1 | 11 | 2.1% | 0.55 s | 1 |
| 440k–460k | 12 | 0 | 820 | 4.0 | 1 | 10 | 1.5% | 0.43 s | 2 |
| 460k–480k | 12 | 0 | 839 | 5.3 | 2 | 9 | 2.9% | 0.53 s | 3 |
| 480k–500k | 18 | 0 | 837 | 3.3 | 0 | 17 | 2.1% | 0.41 s | 3 |
| 500k–520k | 12 | 0 | 875 | 7.0 | 1 | 8 | 4.1% | 0.53 s | 4 |
| 520k–540k | 12 | 0 | 841 | 3.7 | 1 | 10 | 2.0% | 0.52 s | 1 |
| 540k–560k | 12 | 0 | 859 | 5.8 | 0 | 9 | 3.3% | 0.37 s | 1 |
| 560k–580k | 12 | 0 | 853 | 5.7 | 2 | 10 | 3.1% | 0.65 s | 4 |
| 580k–600k | 18 | 0 | 862 | 1.4 | 0 | 18 | 1.6% | 0.49 s | 5 |

(Two bands hold 18 episodes: all six instances finish an episode
together, every 9,000 steps, and three such rounds fell in those bands.)

In short:
- **The spin is gone.** 112 of the 132 episodes are slow (under 10
  turns), against 19 of the last session's final 55. Only 9 spun.
- **The crashes are gone.** None in 200,000 steps.
- **The score crept up**, 812 → 862 a band, and the best episode, 473
  (instance 4, the last round), scored 1,007.
- **But the pole spends less time at the top, not more.** Within 20° of
  upright fell from 6.4% (the last session's final bands) to 1.5–4.1%.
  Slow visits to the top fell from 7–11 a band to 1–5. The longest hold,
  0.65 s (episode 445), is shorter than the last session's 0.79 s. There
  is still no catch.

### Where the score comes from now

The reward has two parts (see *The reward it ran on*, in run 1): the
cart part, up to 0.5 a step for the cart in the middle of the slide, and
the pole part, up to 1 a step for the pole high *and* slow. Splitting each
episode's score into the two, from MuJoCo's true state (an approximation:
the real reward reads the detected markers, but the two parts add up to
within a few points of the logged score):

| Steps | Mean reward | Cart part | Pole part | Highest point (median) | Pole above horizontal |
|---|---|---|---|---|---|
| 200k–240k | 374 | 323 | 58 | 158° | 19% |
| 240k–280k | 674 | 618 | 58 | 180° | 43% |
| 280k–320k | 790 | 722 | 68 | 180° | 43% |
| 320k–360k | 820 | 689 | 132 | 180° | 31% |
| 360k–400k | 849 | 706 | 145 | 180° | 28% |
| 400k–440k | 812 | 731 | 82 | 90° | 9% |
| 440k–480k | 829 | 735 | 95 | 168° | 12% |
| 480k–520k | 852 | 731 | 122 | 179° | 14% |
| 520k–560k | 850 | 731 | 120 | 179° | 15% |
| 560k–600k | 858 | 730 | 128 | 112° | 12% |

"Highest point" is the highest the pole got in the episode, 0° hanging
and 180° upright; half the band's episodes got higher, half didn't.
(The first rows are the last session's, with its crashes: a crash costs
10 and ends the episode, which is why its early cart part is low.)

- **The cart part is close to its ceiling.** 1,500 steps × 0.5 is 750,
  and the policy gets 730. The cart stays within about ±0.2 m of the
  middle (±0.31 m at the most) of a ±0.45 m slide.
- **The pole part dropped at the start of this session and has climbed
  back,** 82 → 128 a band, but not past the 145 of 360k–400k.
- So the total is flat: what the cart part gained, by no longer crashing
  or spinning, made up for what the pole part lost.

### What the policy does now: a slow pump, then a swing near the top

Every episode starts with the pole hanging still (the reset puts it
there). The policy then **pumps the swing up**, moving the cart back and
forth in time with the pole, the way you pump a playground swing. Here
is the best episode, 473, as the pole's highest point in each second:

```
 0s: 24°  1s: 26°  2s: 59°  3s:118°  4s:113°  5s:101°  6s:111°  7s: 95°
 8s:168°  9s:180° 10s:140° 11s:179° 12s:122° 13s:179° 14s:164° 15s:179°
16s: 85° 17s: 97° 18s:179°
```

It takes about 8 s, almost half the episode, to bring the pole to the
top. After that it keeps the pole swinging high, reaching the top every
second or two, and twice lets the swing die down and pumps it up again.
Other episodes show the same two halves:
- **Episode 466** (instance 3): reaches the top at 8 s and passes through
  it every second until 15 s, then once more at 17 s.
- **Episode 448** (instance 3, score 964, the third best): never reaches
  the top. The pole swings to 117°–149° and back, again and again, from
  7 s to the end.
- **Episode 352** (instance 3, early in the session): the pole never gets
  above 59°. A slow start like this was common early on: in 400k–440k
  half the episodes never got the pole above horizontal (the 90° median).

The cart's command is mostly near the top speed, one way or the other.
On 20,000 stored observations, the actor's average command is beyond 95%
of the top speed, either way, for 20–38% of them, depending on the
snapshot. That is normal for a swing-up: the pump is strongest with the
cart driven hard.

### Why the score is stuck: the swing pays nearly as well as a visit

**A pendulum is slow at the ends of its swing.** At the highest point of
each swing it stops, for an instant, before it falls back. So a pole
swinging to 150° is slow and high twice every swing, and the pole part
pays it well: at 150°, (1 − cos 150°) ÷ 2 is 0.93 of the full height. The
reward was made to pay a slow pole at the top (Change 1), and it does,
but it also pays a slow pole *near* the top, at every turnaround of a high
swing.

The numbers show it. Episode 448, which never came within 20° of upright,
earned **234** from the pole. Episode 466, which went over the top every
second for seven seconds, earned **184**: going over the top is fast, so
it pays little. Episode 473, the best, earned 283.

**What the policy has learned is a safe way to earn the pole part:**
pump the swing high and keep it there, with the cart near the middle. It
never crashes and never costs anything. Trying to stop the pole at the
top costs nothing directly either, but it fails almost every time: the
pole falls off one side, the swing is lost, and the next few seconds go
to pumping it up again. So the critics rate the swing above the attempt.
(This is a reading of the numbers, not a measurement of the critics.)

**Balancing would still pay far more.** A pole held upright pays nearly 1
a step: held for the last 10 s of an episode, about 800 steps, it would
earn about 800 from the pole alone. A whole episode of swinging earns
about 280 at best. The
policy is on a small hill, and the big one is across a valley it hasn't
crossed: to get there it has to catch the pole, which means moving the
cart under it the moment it leans, the second half of balancing (*What it
has learned, and what it hasn't*, in the last entry). It hasn't done that
yet, and this session gave it fewer chances, with fewer visits to the top.

This is common in reinforcement learning. The name for it is a **local
optimum**: a behaviour that is better than anything close to it, but not
the best there is. A reward that pays for progress toward the goal (here,
height and slowness) helps the policy find the goal, but it also creates
hills like this one along the way.

### How random the actor is

- **The entropy coefficient** (the randomness bonus; see *The randomness
  bonus* in the last entry) fell from 0.0058 to 0.0033 over this session,
  mostly in the first 120,000 steps, then held at 0.0032–0.0036. That is
  just above run 1's lowest, 0.0031.
- **The actor's own randomness didn't fall.** The actor's command is a
  bell curve: a middle (its best guess) and a spread (how far it strays
  from it). On the same 20,000 stored observations, the median spread was
  0.58 at step 400,004 and 0.62 at 600,002 (0.52–0.62 at every snapshot
  since 200,000). That spread is before the squash that keeps commands
  within the top speed, so where the middle is near the top speed, the
  spread that is left is much smaller.

The two fit together. SAC's auto-tuning holds the actor's randomness near
its target, and the coefficient is the price it pays to do that: when the
actor stays random enough on its own, the price falls. So the actor still
explores about as much as it did. It just explores around the swing.

### Speed: as measured

- **34 min 22 s for 200,000 steps,** about 97 steps a second. The speed
  test estimated 35 min and 96 steps a second (*Change 2*). The last
  session, before Change 2, took 1 h 25 min for the same steps: 2.5×
  faster.
- **An episode of 1,500 steps took 91–97 s,** against 227–231 s last
  session.
- **From the loop's log at the end:** Orin round trip about 43 ms,
  trainer's turn about 14 ms.
- **Updates:** 33,332 (the counter went from 80,832 to 114,164), one per
  group step, as before. Run 1 has now had 114,164 updates in 600,002
  steps.

### Also found: the Orins exited with an error at the very end

After the trainer stopped the run, all six synthetic Orins exited with
exit code 1. Each printed `ConnectionResetError: [WinError 10054] An
existing connection was forcibly closed by the remote host`, then
`websockets.exceptions.ConnectionClosedError: no close frame received or
sent`. In the two earlier sessions they exited with 0.

**Nothing was lost.** It happened after the run had stopped and the
model and buffer were saved (`latest.zip` 15:20:06, the log's end
15:20:08). Every instance's `observations.jsonl` and `states.csv` end on
the same last step, 33,355, with whole lines. The players exited with 0.

**Why (found from the code, not reproduced).** The flaw is in the
orchestrator's shutdown, and the Orin's keepalive pings expose it.

- **A WebSocket goodbye is a handshake.** One side sends a close frame,
  the other answers with its own, and only then is the connection itself
  closed (RFC 6455, section 7.1.1). The orchestrator sends its close
  frame to each Orin and then drops the connection at once
  (`orchestrator/src/main.rs`, the shutdown after `run`), without waiting
  for the Orin's answer.
- **The Orin pings the orchestrator every 20 s.** The `websockets`
  library (17.3) does this by default (`ping_interval=20`) to check the
  other end is alive. In the middle of a run the orchestrator answers
  each ping the next time it reads from the socket, and skips it.
- **A ping can be left unread at the end.** Between the orchestrator's
  last read of the Orin's pool and the shutdown is the trainer's last
  turn: about 1 s here, because the trainer saves the model and buffer
  before it says stop. A ping that arrives then sits unread.
- **Closing a connection with unread data in it resets it.** The
  orchestrator's side then sends a reset (TCP RST) instead of an orderly
  close. On Windows a reset throws away whatever the receiver hasn't read
  yet, and that includes the close frame sent a moment earlier. So the
  Orin sees `ConnectionResetError` and `no close frame received or
  sent`, and exits with 1.
- **It is chance.** A ping lands in a 1 s window about once in 20. The six
  Orins connect within milliseconds of each other, so their pings come
  together, and all six fail or none do. That fits: this session all six,
  the two before it none.
- **Without a ping it still isn't a proper close,** only a quiet one: the
  Orin's answering close frame arrives at a connection already gone, and
  the Orin exits on the end of the connection.

The fix belongs in the orchestrator: after sending its close frame, read
from each Orin until its close frame comes back (with a time limit), and
only then drop the connection. Reading also takes in any waiting ping.
The Orin is right to treat a cut connection as an error. On the robot,
a lost connection is a real failure.

**Fixed (2026-10-24, the user: "plz fix it, even if it isn't really
hurting").**
- **The design:** the diagram's shutdown has a note at 22: the close is
  a handshake. `orchestrator-plan.md` has it as step 10.
- **The code:** `orchestrator/src/main.rs` now sends every Orin its close
  frame first, then waits on each (`await_close`). The wait reads until
  the Orin's close frame comes back, the connection ends, or 5 s pass,
  and passes over anything else, such as a ping. Only then does it drop
  the connection.
- **The proof:** a new test, `the_close_frame_reaches_a_client_whose_ping_is_unread`
  in `orchestrator/src/tests.rs`. A client pings, the server leaves the
  ping unread for 0.2 s, then closes. With the fix, the client gets the
  close frame and an orderly end. Run the old way (close, then drop at
  once), it failed 3 times out of 3 with `os error 10054`, "An existing
  connection was forcibly closed by the remote host": exactly the Orins'
  error. So the cause is confirmed, not only read from the code.
- **Seen in a real run:** the next session's log ends with every
  `synthetic Orin exited: exit code: 0` (*The close handshake works*, in
  the 600,000–1,100,000 entry).

### Files

All in `trainer/runs/20261024_041436/`:
- `episodes.csv`: the rows from episode 343 on are this session;
- `loop_from_400004/instance_0` to `instance_5`: 33,356 frames per
  instance, plus each instance's `states.csv` and `observations.jsonl`,
  5.0 GB in all;
- `latest.zip` and `latest_buffer.pkl`: the model and the buffer at step
  600,002;
- `model_<steps>.zip`: snapshots every 10,000 steps, to `model_600002.zip`.

### How it was measured

With read-only scratch scripts, run by `uv run --project trainer`, the
same way as in the last entry (*How it was measured*):
- **The tables:** from each instance's `states.csv`, matched to
  `episodes.csv` from episode 343 on.
- **The cart and pole parts:** the reward's formula applied to the true
  pole angle, its turning speed (from neighbouring rows), and the Pivot
  marker's x in the frame.
- **The actor's spread:** 20,000 observations drawn at random from
  `latest_buffer.pkl`, given to each snapshot's actor; the spread is
  `exp(log_std)` of its bell curve.
- **The second-by-second traces:** the pole's highest point in each
  second of simulated time.

### To continue (ideas, none tested, none decided)

`total_steps` is 600,000 and the run has reached it, so it must be
raised before the next `--resume` (*Continuing a run: raise
`total_steps` first*).

What might get the policy across the valley:
- **More of the same.** The slow arrivals took about 120,000 steps to
  appear after the reward changed. A catch may also just take longer.
  Cheapest to try: raise `total_steps` to 800,000 and resume.
- **More updates per step.** The run has made one update per 6 steps. At
  6 instances, `gradient_steps = 2` would cost about 16 ms more per group
  step (*How many instances to train with*): about 76 steps a second
  instead of 97, but about 25 updates a second instead of 16.
- **A reward that pays near the top less than at it.** The swing earns
  because the pole part pays 0.93 at 150°. A reward that rises more
  steeply toward upright would pay the turnarounds of a swing less and a
  hold just as much. That would be a change to the reward, so it would
  start in the diagram (*The reward is graded*).

(Chosen 2026-10-24: the third, as Change 3, below.)

## Change 3: the pole's height on a log scale (2026-10-24)

**The problem it solves:** the policy learned that a high swing pays
nearly as well as reaching the top (*Why the score is stuck*, above). The
old height, (1 − cos θ) ÷ 2, is 0.93 at 150° and 1 upright: almost no
difference. So nothing pushed the policy the last 30° to the top, where
the catch has to happen. The user's idea: score the height on a log
scale, so higher is much better than almost as high.

### The new height

Measure how far the pole is from upright, d, in degrees (180 hanging, 0
upright), and score that on a log scale:

  height = 1 − ln(d ÷ ε) ÷ ln(180 ÷ ε), and 1 when d is ε or less

ε is the **upright band**, `[reward] upright_band` in `trainer/gym.toml`,
set to 5°. Within 5° of upright counts as fully upright.

**Why a log scale:** a log scale pays for ratios, not differences. Every
halving of the distance from upright pays the same step up:
ln 2 ÷ ln 36 = 0.19. Going from 40° away to 20° away is worth as much as
from 20° to 10°, and from 10° to 5°. The closer the pole gets, the more
each degree is worth.

| Pole angle | Distance from upright | Old height | New height |
|---|---|---|---|
| 0° (hanging) | 180° | 0 | 0 |
| 60° | 120° | 0.25 | 0.11 |
| 90° (horizontal) | 90° | 0.50 | 0.19 |
| 120° | 60° | 0.75 | 0.31 |
| 150° | 30° | 0.93 | 0.50 |
| 160° | 20° | 0.97 | 0.61 |
| 170° | 10° | 0.99 | 0.81 |
| 175° and up | 5° or less | 1.00 | 1.00 |

The rest of the reward is unchanged:
- **The slow factor** still multiplies the height: a pole has to be slow
  to be paid.
- **The cart part, the crash and the weights** are the same as before.

### What it does to the episodes we have

The pole part of last session's episodes, from MuJoCo's true state, old
height against new:

| Episode | What the pole did | Old | New |
|---|---|---|---|
| 448 | swung to 117–149°, never reached the top | 234 | 102 |
| 466 | over the top every second for 7 s | 184 | 114 |
| 473 | the best: pumped up, then at the top often | 283 | 148 |
| All 132 | the session's average | 111 | 64 |

**The order flips.** Under the old height, 448's swing beat 466's visits
to the top. Under the new one, reaching the top pays more than swinging
below it. A pole held within 5° for the last 10 s of an episode still
earns about 800, as before, so a hold is now worth about 5–6 times the
best swing instead of about 3.

### The cost, named once

The new height also pays less on the way up: 60° pays 0.11 instead of
0.25. So pumping from hanging is paid less until the pole is high. The
policy already knows how to pump, so this matters less for run 1 than it
would for a new run.

### The logs keep the old height

`episodes.csv`'s `pole_max` and `pole_mean` columns stay the plain
height, (1 − cos θ) ÷ 2. They describe where the pole went, not what it
was paid, so they still compare across the whole run. The `reward`
column is the new score.

### Carrying run 1 over

The next `--resume` rescores the whole replay buffer with the new height,
the same way as for Change 1: each stored step's reward is worked out
again from the pool it was read from. The stored steps then teach the new
reward, not the old one. Nothing is dropped and nothing is regrouped.

**Checked in memory** (on run 1's buffer at step 600,002, nothing saved):
- **The new height by hand,** at 0°, 90°, 150°, 170°, 175°, 178°, 180°
  and the same angles on the other side: all exactly as the formula
  gives.
- **The old height patched back in,** the rescore gives every one of the
  599,802 stored steps back as stored, to within 0.0000072. The small
  differences are only in the last session's steps. Those were scored
  live from the full-precision pool, while the buffer keeps a rounded
  copy (32-bit). The steps of earlier sessions had already been rescored
  from the buffer, and they come back exactly.
- **With the new height,** the average reward per stored step falls from
  0.525 to 0.502. The last session's episodes fall from 841 to 794 on
  average, 47 less, matching the true-state estimate above (111 → 64)
  exactly.

### What to expect

- **The score drops at the resume,** by about 47 an episode for the same
  behaviour. That is the new scale, not a worse policy.
- **The critics need time to adjust.** Their values were learned on the
  old reward, so the first several thousand steps are a relearning. After
  Change 1 the randomness bonus moved fast for about 20,000 steps
  (*The randomness bonus*).
- **What to look for:** more time within 20° of upright, more slow visits
  to the top, and longer holds. Episodes like 448, a high swing that
  never reaches the top, should become rarer.

### Files

- `synthetic-loop-dia.md`: the new section *The pole's height is scored
  on a log scale*.
- `trainer-plan.md`: T3a, and step 9.
- `trainer/src/reward.py`: `upright_height`, used by `score`.
- `trainer/src/gym_settings.py`: `upright_band`, checked to lie between 0
  and 180.
- `trainer/gym.toml`: `[reward] upright_band = 5.0`, and `[train]
  total_steps`, now 1,100,000 (below).

### The command

From the repo root:

```
uv run --project trainer python trainer/src/train.py --resume
```

It continues run 1 from step 600,002 to 1,100,000 and writes that
session's frames and true states to `loop_from_600002/`.

**Sessions are now 500,000 steps, up from 200,000** (user, 2026-10-24,
since Change 2 made a session 2.5× faster). `[train] total_steps` is
1,100,000. What that means, at last session's measured 97 steps a
second:
- **Time:** about 1 h 26 min.
- **Episodes:** about 333, 55 or 56 per instance.
- **Updates:** about 83,000, one per group step of 6, taking run 1 to
  about 197,000.
- **Disk:** about 12.5 GB of frames and states in `loop_from_600002/`
  (5.0 GB per 200,000), with 3.3 TB free on D:.
- **Saves:** still every 10,000 steps, now about every 1.7 minutes. A
  stopped session loses at most that.

For the session after that, raise `total_steps` by 500,000 again, to
1,600,000.

## Run 1, continued: 600,000 to 1,100,000 steps, the first on the log height

### The command

The resume above, from the repo root:

```
uv run --project trainer python trainer/src/train.py --resume
```

It ran 2026-10-24 from 15:48:24 to 17:15:56, **1 h 27 min 32 s**, with no
stops. Settings, as copied into the run's folder
(`settings_from_600002.toml`, `gym_from_600002.toml`):

- **Group:** 6 training instances, as before. Episode numbers carry on
  from 475.
- **Length:** to 1,100,000 steps, so 500,000 more, the first of the
  longer sessions.
- **The reward:** Change 3's log-scale height (`upright_band = 5.0`). The
  resume rescored the whole replay buffer with it first, so every stored
  step teaches the new reward. Nothing was dropped or regrouped.
- **SAC:** unchanged.

### What happened

330 episodes, 475 to 804, 55 per instance. **Every one ran the full
1,500 steps: no crashes**, as in the last session. The columns mean what
they did in the last two entries (*What happened*, in *Run 1, continued:
200,000 to 400,000 steps*), but the bands are 50,000 steps wide now, not
20,000, so each holds 30 or 36 episodes:

| Steps | Episodes | Crashes | Mean reward | Full turns per episode | Spinning episodes | Slow episodes | Within 20° of upright | Longest hold | Slow visits to the top |
|---|---|---|---|---|---|---|---|---|---|
| 600k–650k | 30 | 0 | 785 | 1.6 | 0 | 29 | 1.2% | 0.90 s | 7 |
| 650k–700k | 36 | 0 | 800 | 7.0 | 6 | 26 | 3.6% | 1.14 s | 7 |
| 700k–750k | 30 | 0 | 792 | 4.3 | 3 | 26 | 2.4% | 0.77 s | 5 |
| 750k–800k | 36 | 0 | 782 | 2.5 | 2 | 33 | 1.3% | 0.44 s | 3 |
| 800k–850k | 30 | 0 | 789 | 2.1 | 1 | 29 | 1.0% | 0.35 s | 1 |
| 850k–900k | 36 | 0 | 779 | 11.1 | 11 | 19 | 4.6% | 0.66 s | 7 |
| 900k–950k | 30 | 0 | 775 | 5.1 | 4 | 24 | 2.0% | 0.40 s | 1 |
| 950k–1000k | 36 | 0 | 781 | 5.3 | 7 | 28 | 2.0% | 1.20 s | 2 |
| 1000k–1050k | 30 | 0 | 773 | 1.0 | 1 | 29 | 0.3% | 0.23 s | 0 |
| 1050k–1100k | 36 | 0 | 785 | 2.6 | 1 | 33 | 1.4% | 0.62 s | 6 |

**The scores are on the new scale,** so they can't be set beside the last
session's logged scores (812–862). To compare the two sessions on one
scale, here are both scored from MuJoCo's true state with the log height
(the same estimate as in Change 3's *What it does to the episodes we
have*):

| Session | Score on the log height | Within 20° of upright | Episodes reaching 150° | Longest hold |
|---|---|---|---|---|
| 400k–600k (last) | 781 → 803 a band (50k bands) | 1.6–2.9% | 71 of 132 (54%) | 0.65 s |
| 600k–1.1M (this) | 774–800 a band, no trend | 0.3–4.6% | 135 of 330 (41%) | 1.20 s |

In short:
- **The score didn't rise.** It wandered between 773 and 800 a band all
  session, and ended where it began, 785. On one scale the last session
  ended at 803, so this session's average is a little lower.
- **The longest holds doubled.** 1.20 s (episode 735) and 1.14 s
  (episode 531), the first holds over a second in run 1. The last
  session's longest was 0.65 s. These are the first real catches, below.
- **But the policy got the pole high less often.** 41% of episodes
  reached 150°, against 54%. In some stretches it hardly pumped at all:
  in 1,000k–1,050k only 2 of 30 episodes reached 150°.
- **The spin came back in bursts.** 850k–900k had 11 spinning episodes;
  in two rounds (870,002 and 951,002) all six instances spun, in the
  first 18 to 63 turns. A spin scores no better than not pumping: that
  round's six scored 728–777, against about 775 for a round that didn't
  pump. The pole part pays nothing, and the cart moves more.

### The first catches

In the last two sessions the pole sometimes came up slowly and stopped
near the top for a moment, then fell. This session, in a few episodes,
**the cart kept it there.** Here is episode 735's visit (instance 2,
step 996,002), every 0.1 s; "from upright" is the pole's angle from
straight up, one side + and the other −:

| Time | Pole from upright | Pole turning | Cart | Cart speed |
|---|---|---|---|---|
| 14.31 s | −16.0° | +2.61 rad/s | +0.057 m | +0.83 m/s |
| 14.41 s | −2.6° | +2.11 rad/s | +0.142 m | +0.84 m/s |
| 14.51 s | +8.7° | +0.88 rad/s | +0.221 m | +0.37 m/s |
| 14.60 s | +9.6° | −0.65 rad/s | +0.224 m | −0.34 m/s |
| 14.71 s | +1.0° | −1.31 rad/s | +0.150 m | −0.72 m/s |
| 14.81 s | −5.4° | −1.35 rad/s | +0.084 m | −0.73 m/s |
| 14.91 s | −6.4° | +1.71 rad/s | +0.059 m | +0.51 m/s |
| 15.01 s | −2.5° | +1.97 rad/s | +0.079 m | +0.71 m/s |
| 15.10 s | +0.7° | −0.48 rad/s | +0.100 m | −0.18 m/s |
| 15.21 s | +1.9° | −0.33 rad/s | +0.105 m | −0.17 m/s |
| 15.30 s | +5.7° | +1.99 rad/s | +0.122 m | +0.63 m/s |
| 15.41 s | +16.1° | +0.57 rad/s | +0.170 m | −0.07 m/s |
| 15.61 s | +22.3° | +0.96 rad/s | +0.091 m | −0.55 m/s |

The pole stayed within 10° of upright for about a second, then fell away
to the + side.

**How to tell a catch from a slow visit: the pole turns back while it
is still near the top.** Near upright, gravity always pulls the pole
*away* from upright. A pole left alone can turn back only once: it rises,
slows, stops short of the top, and falls back the way it came. To turn
back again, toward upright, something has to push it, and the only thing
that can is the cart. At 14.60 s the pole is 9.6° past upright and was
moving further away; a moment later it is moving back, and the cart has
just reversed. At 14.91 s it happens again on the other side. In this
visit the pole's turning changed direction 12 times in 1.27 s.

Counting the visits within 20° of upright in which the pole turned back
four times or more:
- **This session: 17 visits, in 17 of 330 episodes** (5 in 100). The
  longest lasted 1.27 s (episode 735), 1.21 s (531) and 0.92 s (503).
- **The last session: 4 visits, in 4 of 132 episodes** (3 in 100), the
  longest 0.58 s.

So the policy has started on the second half of balancing, moving the
cart under the pole when it leans (*What it has learned, and what it
hasn't*, in the 200,000–400,000 entry). It does it rarely and loses the
pole after a second, but it has done it, which run 1 had not before.

### Half the time, the policy didn't pump

All six instances finish an episode together every 9,000 steps (1,500
steps × 6). Call the six episodes that finish together a **round**. The
six run the same policy at the same time, so they tend to behave alike,
and a round shows what the policy was doing then. Here are the rounds of
one stretch; "reached 150°" counts the round's episodes whose pole got
that high:

| Round ends at step | Mean reward | Reached 150° | Highest point (median) | Pole part (log height) |
|---|---|---|---|---|
| 708,002 | 824 | 6 of 6 | 180° | 93 |
| 717,002 | 788 | 4 of 6 | 180° | 55 |
| 726,002 | 799 | 4 of 6 | 179° | 69 |
| 735,002 | 771 | 0 of 6 | 73° | 36 |
| 744,002 | 776 | 0 of 6 | 80° | 45 |
| 753,002 | 779 | 1 of 6 | 81° | 47 |
| 762,002 | 766 | 0 of 6 | 67° | 36 |
| 771,002 | 772 | 1 of 6 | 74° | 39 |
| 780,002 | 788 | 1 of 6 | 103° | 59 |
| 789,002 | 815 | 5 of 6 | 179° | 90 |

For about 54,000 steps, six rounds in a row, the policy hardly pumped,
then it pumped again. Over the whole session, **27 of the 55 rounds had
at most one episode reaching 150°.** In the last session it was 5 of 22.

What a round that doesn't pump looks like: episode 751 (round 1,023,002),
the pole's highest point in each second:

```
 0s: 15°  1s: 14°  2s: 33°  3s: 31°  4s: 22°  5s: 10°  6s:  9°  7s: 26°
 8s: 24°  9s: 21° 10s: 30° 11s: 28° 12s: 39° 13s: 24° 14s: 22° 15s: 12°
16s: 37° 17s: 32° 18s: 31°
```

The pole wobbles below 40° for the whole episode while the cart stays
near the middle. When the policy does pump, it pumps as before: episode
503 (round 645,002) reached the top at 13 s, after a slow start:

```
 0s: 25°  1s: 24°  2s: 10°  3s: 14°  4s: 17°  5s: 33°  6s: 26°  7s: 41°
 8s: 68°  9s: 54° 10s: 65° 11s:125° 12s:174° 13s:180° 14s:128° 15s:130°
16s:179° 17s:180° 18s:118° 19s:120°
```

The first four rounds of the session (to 636,002) hardly pumped either:
2 of their 24 episodes reached 150°. That was the critics relearning
on the new reward (*What to expect*, in Change 3). The fifth round,
645,002, all six did.

### Why: the log height pays the pump a third as much

**What pumping is worth.** Splitting the rounds into those that pumped
(4 or more of 6 reached 150°) and those that didn't (at most 1), and
scoring each with both heights from the true state:

| | Rounds | Cart part | Pole part, old height | Pole part, log height | Score on the log height |
|---|---|---|---|---|---|
| Last session, pumped | 12 | 730 | 129 | 72 | 802 |
| Last session, didn't | 5 | 734 | 66 | 44 | 777 |
| This session, pumped | 21 | 727 | 128 | 70 | 796 |
| This session, didn't | 27 | 734 | 63 | 42 | 775 |

- **When it pumps, it pumps as well as before.** The pole part on the
  old height is 128 against 129.
- **But pumping now earns only about 21 more an episode than not
  pumping** (796 against 775). On the old height it earned about 60 more
  (859 against 799 in the last session's logged scores). The log height
  cut what pumping is worth by about two thirds, and the policy pumps
  half as often.

**Why the cut is so large: the pump is paid on the way up, and that is
where the log height pays least.** A pump raises the swing a little each
time, and each turnaround, where the pole is slow and so is paid, comes
a little higher than the last. What tells the policy the pump is working
is that each turnaround pays a little more. Here is what a turnaround
pays at each height, and what the next 30° up adds:

| Turnaround at | Old height | Added by the 30° below it | Log height | Added by the 30° below it |
|---|---|---|---|---|
| 30° | 0.07 | 0.07 | 0.05 | 0.05 |
| 60° | 0.25 | 0.18 | 0.11 | 0.06 |
| 90° (horizontal) | 0.50 | 0.25 | 0.19 | 0.08 |
| 120° | 0.75 | 0.25 | 0.31 | 0.11 |
| 150° | 0.93 | 0.18 | 0.50 | 0.19 |
| 180° (upright) | 1.00 | 0.07 | 1.00 | 0.50 |

Up to horizontal, each step up the swing now pays about a third of what
it did, and from 90° to 120° under half.
This is the cost named in Change 3 (*The cost, named once*), now
measured: the log height was meant to make the top worth more than near
the top, and it does, but it also made the first two thirds of the climb
nearly free of pay.

**Why the policy can't look past that: it plans about 1.3 s ahead.**
SAC's discount, `gamma = 0.99`, counts a reward 100 steps ahead at a
third of its value, and one 300 steps ahead at a twentieth. Here 100
steps is about 1.3 s of simulated time, because the loop's steps are
short (12.57 ms on average). The pump takes 9–15 s to bring the pole to
150° (the median episode, per band). So a pump's payoff at the top is far
beyond what the critics count when the pump starts. They see the next
turnaround or two, and those have to be paid well enough on their own.
Under the old height they were; under the log height, up to horizontal,
they are barely paid. (This is a reading of the numbers and of what the
discount means, not a measurement of the critics.)

**And why the policy swings between pumping and not:** with only about
21 an episode in it, out of about 785, the difference is small against
the noise in the critics' estimates. Each round the policy has had 1,500
more updates, enough to tip it either way. In the last session the
difference was about 60, and the policy pumped in most rounds.

### How random the actor is

- **The entropy coefficient** (the randomness bonus) went from 0.0033 to
  0.0021 at step 870,002, its lowest in run 1, then back up to 0.0035 by
  the end.
- **The actor's spread didn't change:** a median of 0.59–0.68 at every
  50,000-step snapshot, on the same 20,000 observations. (These were
  drawn from the buffer at step 1,100,000, so the figures for older
  snapshots differ slightly from the last entry's, which used the buffer
  at 600,002.)

As in the last session, the actor explores about as much as it did.

**The buffer is now full.** It holds the newest 1,000,000 steps
(`buffer_size`), so since step 1,000,000 each new step pushes out the
oldest. It now starts at about step 100,000: the first 100,000 steps,
the start of the spin, are gone from it.

### Speed: as measured

- **1 h 27 min 32 s for 500,000 steps,** 95 steps a second. The estimate
  was 1 h 26 min at 97.
- **An episode of 1,500 steps took 92–101 s** (median 94 s).
- **From the loop's log:** Orin round trip about 45 ms, trainer's turn
  about 15–22 ms, as last session.
- **Updates:** 83,332 (114,164 → 197,496), one per group step of 6. Run 1
  has now had 197,496 updates in 1,100,000 steps.

### The close handshake works

All six synthetic Orins exited with code 0 at the end of the run, and all
six players with 0 (the log's last lines). This is the fix in *Also
found: the Orins exited with an error at the very end*, seen in a real
run for the first time. One run is not proof against a 1-in-20 chance,
but the test already reproduced the failure and showed the fix.

### Files

All in `trainer/runs/20261024_041436/`:
- `episodes.csv`: the rows from episode 475 on are this session;
- `loop_from_600002/instance_0` to `instance_5`: 83,389 frames per
  instance, plus each instance's `states.csv` and `observations.jsonl`,
  13 GB in all;
- `latest.zip` and `latest_buffer.pkl`: the model and the buffer at step
  1,100,000;
- `model_<steps>.zip`: snapshots every 10,000 steps, to
  `model_1100000.zip`.

### How it was measured

With read-only scratch scripts, run by `uv run --project trainer`, as in
the last entries:
- **The tables:** from each instance's `states.csv`, matched to
  `episodes.csv` from episode 475 on (and from 343 for the last session,
  redone in 50,000-step bands).
- **The scores on one scale:** the reward's formula, with both heights,
  applied to the true pole angle, its turning speed and the Pivot
  marker's x.
- **Catches:** in each visit within 20° of upright, the number of times
  the pole's turning speed changed sign.
- **Rounds:** episodes grouped by the step at which they ended.
- **The entropy coefficient** from each snapshot; **the actor's spread**
  as in the last entry.

### To continue (ideas, none tested, none decided)

`total_steps` is 1,100,000 and the run has reached it, so it must be
raised before the next `--resume`, to 1,600,000 for the next 500,000
(*Continuing a run: raise `total_steps` first*).

What might get the policy pumping every time, and catching more:
- **More of the same.** The catches are new, and the policy may still
  settle. But nothing in this session's bands trends up.
- **A height that pays the climb and the top.** For example, the
  average of the old height and the log one: 0.18 at 60°, 0.35 at 90°,
  0.72 at 150°, 1 upright. The climb would pay about two thirds of what
  it did under the old height. **Checked afterwards, it doesn't work:**
  the pole part is a sum of height × slowness over the steps, so the
  average of the two heights gives the average of the two totals.
  Episode 448's high swing then scores 168 (234 and 102 averaged) against
  149 for episode 466's trips over the top (184 and 114). The swing would
  win again: the trap Change 3 removed would come back.
- **A longer horizon.** `gamma = 0.995` counts 200 steps ahead (about
  2.5 s), `0.998` 500 (about 6.3 s): long enough to see several
  turnarounds of a pump. The cost is that values take longer to learn.
  **A resume can't change it today:** `train.py` loads the saved model
  with the settings it was made with, and reads `gym.toml`'s `[sac]` only
  for a new run. The same holds for `gradient_steps`, the idea in the
  last entry.

(Chosen 2026-10-24, after `reward-research.md`: a new run, run 2, from
step 0, with DeepMind's multiplied swing-up reward, the last 6 commands
added to what the policy sees, and γ = 0.998. The design is in the
diagram: *The reward is a product*, *The policy sees its last commands*
and *The discount*.)

## Change 4: run 2, a new start with a published reward (2026-10-24)

**The problem it solves:** in run 1's last session the policy pumped the
swing up in only half its rounds. Pumping earned only about 21 points an
episode more than not pumping, out of about 785 (*Why: the log height
pays the pump a third as much*, above). Each change to run 1's reward
had made a new trap (the spin, then the high swing), so for run 2 the
reward comes from what others use instead: `reward-research.md` collects
the published cart-pole swing-up rewards, quoted from their code and
papers, and scores run 1's episodes under each. Run 2 makes three
changes at once and starts from step 0 (user: "I'm ok with starting back
at 0").

**The cost, named once:** with three changes at once, if run 2 does
better we won't know which change did it.

### 1. The reward: DeepMind's product

DeepMind's Control Suite is the standard benchmark for this task, and
its swing-up reward multiplies four parts, each from 0 to 1:

  reward = height × speed × centring × command

| Part | What it measures | Worst | Some values |
|---|---|---|---|
| Height | (1 − cos θ) ÷ 2, our old height | 0, hanging | 90°: 0.50, 150°: 0.93, upright: 1 |
| Speed | how slowly the pole turns | 0.5, fast | still: 1, 1 rad/s: 0.96, 2: 0.85, 5: 0.55, 10: 0.50 |
| Centring | how near the middle the cart is | 0.5, far off | middle: 1, halfway to a bound: 0.81, at a bound: 0.57 |
| Command | how gently it drives | 0.8, top speed | standing still: 1, half speed: 0.95, top speed: 0.8 |

The speed and centring parts use a bell curve, 10^(−x²): 1 at the centre,
0.1 at the margin (5 rad/s for the speed, just past an end-crash bound
for the centring). The crash still scores −10 and ends the episode.

**Why a product is different from a sum:** in run 1's reward the cart
part was *added*, so the policy collected about 730 an episode just by
keeping the cart in the middle, with the pole hanging. In a product a
hanging pole makes the whole step 0, whatever the cart does: the only way
to earn is to bring the pole up. The other three parts can only scale the
pay down, and never by more than half (0.8 for the command). Run 1's slow
factor went all the way to 0 for a fast pole, and a pump is fast except
at its turnarounds; here a fast pole still keeps half its pay.

**The command part is new to us.** It costs a little to drive flat out.
On a real rig in one of the papers, a policy that drove the cart flat
out one way and then the other broke a cart wheel in under 2 seconds.
Run 1's actor commanded beyond 95% of the top speed in 13–51% of its
steps, depending on the snapshot.

### What it does to the episodes we have

Run 1's last two sessions scored with the new reward, through the new
`reward.py`, on the pools the Orin actually reported (`observations.jsonl`;
the command part held at 1, since the logs don't keep the commands):

| Behaviour | Run 1's reward | The product |
|---|---|---|
| A round that didn't pump | 775 | 74 |
| A round that pumped | 796 | 251 |
| Episode 448: a high swing, never the top | 834 | 296 |
| Episode 466: over the top every second | 826 | 329 |
| A spinning episode | about 750 | about 315 |
| The pole held upright, still, all episode | 2,250 | 1,500 |

(Run 1's column counts the cart part, which is why episode 448 beats 466
there. The spin's figure is from MuJoCo's true state, as in the diagram.)

- **Pumping now pays more than three times not pumping,** where it paid
  about 3% more.
- **Going over the top beats the high swing,** without a log scale.
- **The risk, named once: a spin pays a little more than a pump** (315
  against 251). Lowering the speed part's floor would make the spin pay
  less, but below about 0.2 the high swing beats going over the top
  again. No floor avoided both traps on run 1's episodes. What lies past
  both is a hold, worth about 5 times either, and the discount (part 3)
  is what lets the policy see it.

### 2. The policy sees its last 6 commands

The policy's input was the pool: the Orin's last 6 frames of markers, 54
numbers. It now also gets the last 6 commands it sent, newest first, 60
numbers in all. At the start of an episode they are 0, the command for
standing still.

Why:
- **The commands say how the cart is moving.** A command is the
  stepper's pulse rate, so the newest command is the cart's speed now and
  the ones before it say how it got there. The markers show that only as
  tiny changes in the Pivot marker's x from frame to frame.
- **The newest frame is already old.** It is 8–15 ms old when it arrives
  (the latency band), and the cart has moved since, on the last command.
  On the rig, with the policy's own time and WiFi added, it will be
  older. (A correction: I said 45 ms earlier. That is the Orin's round
  trip in wall time, while MuJoCo waits; in simulated time the frame is
  8–15 ms old.)
- **A real cart-pole needed them.** NFQ2.0, on a real cart-pole with
  about 100 ms of delay, did not learn from the current state alone. It
  needed at least the last 2 states and actions, and used 6.

This keeps to "not a toy": on the robot, the Orin sends the commands, so
it has them. The number of past positions stays 6.

**Why this needs a new run:** the observation is the input to SAC's
networks, so run 1's networks, built for 54 numbers, can't take 60. A
`--resume` of run 1 is now refused before anything starts.

### 3. The discount: γ = 0.998

SAC judges each action by the rewards that follow it, each counted less
the further off it is (*Why the policy can't look past that*, above).
How much a reward still counts, by how far ahead it is:

| Ahead | Steps | γ = 0.99 (run 1) | γ = 0.998 (run 2) |
|---|---|---|---|
| 1.3 s | 100 | 37% | 82% |
| 3.8 s | 300 | 5% | 55% |
| 6.3 s | 500 | 0.7% | 37% |
| 8 s, a typical pump to the top | 640 | 0.2% | 28% |

So the payoff at the top of a pump, mostly invisible to run 1 when the
pump started, now counts for about a quarter. Both hardware papers in
`reward-research.md` used 0.999. The cost: values that look further
ahead take longer to learn, and are noisier on the way.

### Checked before the run

With a stand-in for the loop, nothing launched (trainer-plan step 10–12):
- **The reward by hand:** hanging 0 (also with the cart off centre and at
  top speed); upright and still 1; at top speed 0.8; at a bound 0.5746;
  at 5 rad/s 0.55; past a bound −10 and the episode ended. All exact.
- **The commands:** all 0 after a reset; after seven steps the latest six,
  newest first, on all six instances; an episode's last observation keeps
  its own commands and the next episode starts at zeros.
- **SAC:** builds with 60 inputs and γ = 0.998, learned 400 steps, and
  keeps γ through a save and load. The resume's rescore, which now reads
  each stored step's command too, gives every stored reward back.

Not yet checked: any of it through the real loop. Run 2's first session
is the first.

### What to look for

- **Pumping in every round,** not half: episodes reaching 150°, and the
  round table from *Half the time, the policy didn't pump*.
- **The spin.** Turns per episode: if spinning episodes (15 turns or
  more) take over, the risk above has happened.
- **Holds and catches:** visits within 20° of upright, the longest hold,
  and the pole turning back near the top (*The first catches*).
- **Gentler commands:** how often the actor commands near the top speed.
- **The scores can't be compared with run 1's directly,** since the scale
  changed: 1,500 is a perfect episode now, against about 2,250. The
  journal will compare behaviour, as above.

### Files

- `reward-research.md`: the research this change comes from.
- `synthetic-loop-dia.md`: *The policy sees its last commands*, *The
  reward is a product*, *The discount*.
- `trainer-plan.md`: T3b, T8b, the discount in T9, and steps 10–13.
- `trainer/src/reward.py`: the product. `trainer/src/gym_settings.py`:
  its values, each checked. `trainer/src/loop_env.py` and
  `trainer/src/vec_env.py`: the commands in the observation.
  `trainer/src/train.py`: the rescore reads the commands, and a resume
  with a different observation is refused.
- `trainer/gym.toml`: `[env] past_commands = 6`; `[reward]`
  `speed_margin = 5.0`, `centre_margin = 1.1`, `speed_floor = 0.5`,
  `centre_floor = 0.5`, `command_floor = 0.8`; `[sac] gamma = 0.998`;
  `[train] total_steps = 500000`.

### The command

From the repo root:

```
uv run --project trainer python trainer/src/train.py
```

A new run, in a new folder `trainer/runs/<date>_<time>/`, episodes from
0. At run 1's last speed:
- **Time:** about 1 h 27 min. The first 10,000 steps, about 4 minutes,
  are random actions to fill the replay buffer before learning starts.
- **Episodes:** about 333, 55 or 56 per instance.
- **Updates:** about 82,000 (one per group step of 6, after the first
  10,000 steps).
- **Disk:** about 13 GB of frames and states in `loop_from_0/`.

## Change 5: one command every 34-50 ms, the orchestrator timing the frames (2026-10-30)

**What this changes:** the policy now decides 20 to 30 times a second
instead of about 80, and nothing at all happens between its decisions.

### Why: run `trainer/runs/20261030_143755`

This run was stopped at 582,012 steps, on the five-factor reward
(`trainer/reward-plan.md`) with `learning_rate = 0.0001`. It had not
balanced: its longest stay within 20 degrees of upright was 0.78 s. Its
measures, read while it ran:

- **The critic** is the network that predicts how much future score a
  command will earn. The policy learns only from what the critic says about
  its commands, so the critic has to tell commands apart.
- **At 460,000 steps a single command barely moved the critic's score.**
  Swapping each kind of input for another capture's, and measuring how far
  the score moved (root mean square, RMS):

  | Swapped for another capture's | RMS change in the score |
  |---|---|
  | all marker positions | 0.798 |
  | the 6 ages | 0.146 |
  | the ages, keeping only the delay-timing noise | 0.145 |
  | the 6 past commands | 0.060 |
  | the command itself | 0.022 |

  The random timing moved the score about 7 times as much as the command
  did. A small effect is expected: at 12.57 ms a step, one command is a
  small part of the 6.3 s the critic looks ahead, and rewards are small.
  Tallec, Blier and Ollivier (2019) describe the effect: as the time step
  shrinks, the difference between actions vanishes compared with the value
  (cited from memory, not checked for this entry).
- **The critic had not learned that the rail ends everything.** The last
  step before a strike truly earns 0.007; the critic scored it 16.38, and
  the pole hanging above the pole near upright (17.06 against 14.12).
- **Nothing was found broken in the wiring.** The reward equalled its five
  factors' product in all 402,419 captures checked. A strike is stored as a
  true end and a time limit is not, stored steps stay paired after the
  buffer wraps, and a save changes nothing (tested on the real buffer class
  at a tiny size; the live run's stored flags were not read).
- **The ages in raw milliseconds were not the cause.** Learning the true
  future score from the record reached R² 0.19 and 0.23 with raw ages, 0.22
  and 0.22 with ages divided by 85, and 0.22 and 0.21 with no ages at all
  (R²: 1 is perfect, 0 no better than the average). So they stayed as they
  are. That test measured predicting the score, not ranking commands.
- **Other runs fell below 10 after 2,000,000 steps** (the user, 2026-10-30;
  those runs were not read for this entry). An episode's score is not used
  in training, which sees only each step's reward; it is only a log. In this
  run, under 10 meant the cart hit the rail in 300 of 305 episodes.

### What changed

The design is in `synthetic-loop-dia.md`. In short:

- **Each capture draws two numbers:** the detector's delay, 8-15 ms as
  before, and a command rate, 20-30 Hz (`gym.toml` `[env] command_rate_min`
  and `command_rate_max`), which sets when the next capture comes.
- **Nothing happens between commands.** No render, no detection, no
  message: only MuJoCo and the synthetic Arduino's ramp run, and the
  trainer sees only the states it acts on.
- **The orchestrator alone decides when a frame happens.** The synthetic
  Orin no longer takes a seed or a delay band; it is handed the pool's
  pre-warm gaps. The firmware needed no change: its ramp is the motor's
  limit, not perception timing.

Values that moved with it, so that time on the rig stays the same:

| Value | Was | Now | Why |
|---|---|---|---|
| Gap between captures | 10-16 ms, mean 12.57 | 34-50 ms, mean 41.5 | the change itself |
| `max_episode_steps` | 1,500 | 460 | still about 19 s |
| `gamma` | 0.998 | 0.9934 | still about 6.3 s ahead: 1 / (1 - 0.9934) = 152 steps |
| `[end.strike_gap] buffer` | 0.0136 m | 0.046 m | 0.669 m/s x (52 + 16) ms |

The buffer ends an episode 32 mm sooner at each end of the rail, because
the cart travels further between two checks. Unchanged: the reward's
calculation, `total_steps`, `learning_starts`, `buffer_size` and
`learning_rate`. A run saved before this cannot resume: its stored commands
were each held for one delay, so the trainer refuses it.

### What to read scores against now

An episode's most is now 460, not 1,500. A hanging pole earns about 3.4
(0.0075 x 460), not 11.25.

### Checked before a run

- `cargo test -p orchestrator`: 21 passed. The trainer's tests: 51 passed.
- A physics-only run of 2,000 captures at seed 7: gaps of 34 to 50 ms only,
  mean 41.57 ms. Two runs gave identical `states.csv` files.
- `--command-rate-max 60` is refused at load: 16.67 ms is less than 15 ms +
  the 2 ms timestep.
- `env_check.py 3` on the real loop (orchestrator, Orin and player, one
  instance, random commands): episodes of 460, 460 and 108 steps, the last
  ended by the left strike gap; 19.1 s and 19.4 s for the full ones; rewards
  3.402, 3.693 and 0.689; every process exited 0.

### What this does not show

No training has run on this. Whether the critic now separates commands
better is the question for the next run: repeat the swap test above and
compare the command's 0.022 and the timing's 0.145. Holding commands longer
is an inference from the measurement and from Tallec et al., not a tested
fix.

### The command

```
uv run --project trainer python trainer/src/train.py
```

## Run `20261101_154007`: the policy learned to hit the wall on purpose

The first long run on DeepMind's reward (`__synthetic__/trainer/python/gym.toml`,
12 instances), stopped at 247,572 steps. By the end every episode drove right
at about 0.65 m/s and hit the strike gap in about 22 steps.

| Steps | Episode length | Score per episode | Near upright (within 20°) |
|---|---|---|---|
| 0-80,000 | about 155 steps, both walls | 7-12 (best 93) | up to 0.8% of steps |
| 80,000-90,000 | 315 | 20.6 (best 141) | 0.66% |
| 130,000 | 81, right wall only | 0.8 | 0 |
| 190,000+ | 22, right wall only | 0.12 | 0 |

### Why ending the episode paid

SAC pays the policy for acting randomly, on top of the reward:

- While the episode continues: `Q_target = r + γ (Q_next − α log π)`
- When it ends: `Q_target = r`

`Q` is the critic's score for the situation, `r` the reward, `γ` the discount,
`α` the weight on randomness, and `log π` how sure the policy was of its action,
high when very sure. When `α log π` is larger than the reward, each step alive
costs more than it earns, and an episode's end stops the cost.

Read from the run's snapshots, on the same observations:

| Steps | α | log π | `−α log π` a step | Critic, hanging still | Critic, at the wall |
|---|---|---|---|---|---|
| 10,008 | 1.08 | +4.62 | −4.97 | −26 | −26 |
| 80,004 | 1.92 | +4.59 | −8.83 | −186 | −183 |
| 90,000 | 1.86 | −0.68 | +1.26 | −163 | −159 |
| 190,008 | 0.90 | +0.45 | −0.40 | −57 | −27 |

The reward was about 0.06 a step, so at step 80,004 the penalty was about 150
times larger. This run says nothing either way about DeepMind's reward.

### Why the actor was so sure

It wasn't sure: its randomness was stuck at the top limit. The actor's command
is a bell curve squashed into [−1, 1], and Stable-Baselines3 (SB3) limits
`log_std`, the setting for the curve's width, to at most 2. A curve that wide
squashes nearly every command to full speed, either way, which looks like great
certainty: 84% of actions were beyond ±0.9.

The cause is the inputs' scale. The pool's ages reached the network in
milliseconds, down to −246, beside markers and actions within ±1. An untrained
actor's raw `log_std`, before the limit, on those inputs, across six seeds:

| Ages given in | Raw `log_std` range |
|---|---|
| milliseconds | −4.35 to +12.06 |
| seconds | −0.17 to +0.11 |

So the seed decides whether a run starts pinned. In this run the raw value was
5.27 at step 10,008 and dropped below 2 only around step 80,000, when full-rate
actions fell from 85% to 10% within about 4,000 steps. By then the critic's
scores were deeply negative, and `α` was slow to fall: at a learning rate of
0.0001, `log α` moves about 0.0001 an update, and one update per 12 samples made
only 15,733 updates by step 190,008.

## Change 6: ages in seconds, a smaller randomness weight, more updates (2026-11-01)

| Value | Was | Now | Why |
|---|---|---|---|
| `[observation.pool_and_commands] age_unit_ms` | none (raw ms) | 1000 | ages arrive in seconds, on the other inputs' scale |
| `[sac] ent_coef` | none (SB3's `"auto"`, starting at 1.0) | `"auto_0.1"` | tuned as before, from a start near the reward's size |
| `[sac] gradient_steps` | 1 | 2 | one update per 6 samples instead of per 12 |

The pool on the wire keeps its ages in ms. A run saved before this cannot
resume: the observation changed.

### Checked before a run

- **Offline, the old settings against the new, 733 updates, four seeds:** an
  environment that only draws observations at the run's scales and pays 0.06 a
  step, no physics. Old: three of four seeds pinned, seed 0 at α 1.0754, log π
  +4.58 (the run itself: 1.0754, +4.62); the fourth was not. New: none pinned,
  raw `log_std` −0.09 to −0.12, log π −0.68.
- **The real loop, 6,000 steps, 12 instances, the same values:** every process
  exited cleanly. Commands at 95%+ of full rate:

  | Steps | Commands at 95%+ | Mean reward | Mean upright |
  |---|---|---|---|
  | 1-1,200 (random) | 5% | 0.010 | 0.011 |
  | 1,201-2,400 | 4% | 0.028 | 0.035 |
  | 2,401-3,600 | 3% | 0.037 | 0.045 |
  | 3,601-4,800 | 4% | 0.048 | 0.061 |
  | 4,801-6,000 | 3% | 0.067 | 0.088 |

  The trial before this change, with the same random start, showed 79-82% from
  step 1,201. At step 6,000 (798 updates): α 0.0923, raw `log_std` −0.11, log π
  −0.68, so the randomness term was +0.063 a step, a small reward for staying
  alive rather than a cost. 8 of the 12 first episodes reached the time limit.

### What this does not show

798 updates is too few to show balancing. And the wall can still pay: SAC tunes
`α` until `log π` is about +1 (SB3's target entropy, −1 for one action), so at
that point `−α log π` is about −α a step. Whether `α` is small by then, against
the reward, is for a long run to show. Watch for episodes shortening towards one
wall, and compare `α` (`measure.py`'s entropy table) with the reward per step.

### The command

From the repo root:

```
uv run --project __synthetic__/trainer/python python __synthetic__/trainer/python/src/train.py
```

## Run `20261101_163458`: the policy swings the pole up and balances it

The first run on Change 6's settings, and the first to balance. By the end,
every episode swung the pole up from hanging in about 2.2 s and then held it
upright for the rest of the episode, about 18.5 s, without letting it drop.
The run took all 4,000,008 steps, with no stops and no strikes after its
first 55 episodes.

### The command

From the repo root:

```
uv run --project __synthetic__/trainer/python python __synthetic__/trainer/python/src/train.py
```

It ran from 2026-11-01 16:35 to 2026-11-02 05:28, about **12 h 53 min**
(read from the run folder's file times), so about 86 steps a second. The
code was commit `9087b8fd`, with no uncommitted changes (`latest.json`).
Settings, as copied into the run's folder (`gym_from_0.toml`, the same as
today's `gym.toml`):

- **Group:** 12 training instances (a 3 x 4 grid), seed 0.
- **The reward:** DeepMind's (`dm_control_reward.md`), each step
  r = upright x small_control x small_velocity x centered, between 0 and 1.
- **The episode:** starts with the pole hanging; ends at a strike gap or at
  the time limit, which was **500 steps** in this run, about 20.8 s at the
  mean gap of 41.5 ms (Change 5 and `gym.toml`'s comment still say 460). So
  an episode's score is at most 500.
- **SAC:** Change 6's values (ages in seconds, `ent_coef = "auto_0.1"`,
  `gradient_steps = 2`), with `learning_rate = 0.0001`, `buffer_size =
  2,000,000`, `learning_starts = 1,200` and `gamma = 0.9934`.

### What happened

8,018 episodes. Measured with `measure.py episodes` (its columns are
defined at the top of `src/measures/episodes.py`): **Ends** are episodes
stopped by a strike gap; **Full turns** how far the pole went round, in
whole turns; **Spinning** episodes of 15 turns or more; **Within** the share
of all captures with the pole within 20° of upright; **Longest hold** the
longest unbroken time within 20°, in any episode of the band. A selection of
the 50,000-step bands:

| Steps | Episodes | Ends | Mean score | Full turns | Spinning | Within 20° | Longest hold |
|---|---|---|---|---|---|---|---|
| 0-50k | 116 | 55 | 69.4 | 18.5 | 52 | 6.2% | 1.03 s |
| 50k-100k | 102 | 0 | 124.7 | 35.5 | 102 | 12.0% | 0.92 s |
| 100k-150k | 101 | 0 | 162.9 | 20.8 | 72 | 16.7% | 1.41 s |
| 150k-200k | 97 | 0 | 190.3 | 5.7 | 2 | 10.6% | 1.10 s |
| 250k-300k | 101 | 0 | 234.6 | 7.5 | 0 | 26.3% | 2.18 s |
| 350k-400k | 102 | 0 | 264.1 | 7.1 | 0 | 35.5% | 3.41 s |
| 400k-450k | 101 | 0 | 329.0 | 5.7 | 0 | 55.8% | 10.46 s |
| 450k-500k | 97 | 0 | 394.9 | 2.5 | 0 | 74.7% | 17.75 s |
| 500k-550k | 102 | 0 | 415.4 | 1.2 | 0 | 80.2% | 17.95 s |
| 950k-1,000k | 102 | 0 | 436.2 | 0.9 | 0 | 86.6% | 18.70 s |
| 1,950k-2,000k | 97 | 0 | 444.9 | 0.8 | 0 | 88.6% | 18.76 s |
| 2,950k-3,000k | 101 | 0 | 450.7 | 0.8 | 0 | 89.0% | 18.78 s |
| 3,950k-4,000k | 102 | 0 | 451.9 | 0.8 | 0 | 89.3% | 18.78 s |

In short:

- **No wall-hitting this time.** The strike gap ended 55 episodes, all in
  the first 50,000 steps; after that, every episode ran to the time limit.
  This is what run `20261101_154007` lacked: with α (SAC's weight on staying
  random) starting at 0.1, staying alive never cost more than it earned.
- **First it spun, then it learned to stop.** Up to 150,000 steps the pole
  was driven round and round (up to 35.5 turns an episode). By 150,000 the
  spinning had stopped, and the time near upright grew steadily.
- **It learned to balance between 400,000 and 500,000 steps.** The longest
  hold jumped from 3.4 s to 10.5 s and then to 17.8 s, nearly the whole
  episode after the swing-up. The first episode to score over 400 was
  episode 924, at 453,852 steps.
- **After 500,000 steps it only polished.** The score rose slowly from 415
  to 452, and the full turns fell from 1.2 to 0.8: the swing-up stopped
  going round and became one swing back and then up.

### What one episode looks like now

Read from MuJoCo's state in the capture record (`captures_from_0.csv`), for
the 252 whole episodes in its last 200 MB, about the last 120,000 steps
(median, with the range in brackets):

| What | Value |
|---|---|
| Swing-up: time until the pole is first within 20° of upright | 2.25 s (2.18-2.32), 54 steps |
| After that, share of the episode within 20° | 100% in every episode |
| Worst tilt from 1 s after arriving to the end | 2.1° (1.4-3.5°) |
| Worst tilt in the last 10 s | 1.9° (1.2-3.5°) |
| Cart's farthest from the rail's middle in the last 10 s | 44 mm (23-89 mm) |
| Commands at 95% of full rate or more | 6.8% (5.8-7.6%) |

So the pole doesn't just stay within 20°: within a second of arriving it
settles to within about 2-3° of straight up and stays there, and the cart
stays within a few centimetres of the middle. The ends of the rail are about
0.44 m away.

### Why the score stops near 452, not 500

The score is now limited by the swing-up, not by the balancing. In those
same 252 episodes:

- **The 54 swing-up steps** earn about 9 points (7.4-10.4) in all, because
  the upright factor is small while the pole is low.
- **The about 446 held steps** earn 0.993 each (0.991-0.995). The factors
  while held: upright 1.000, small_control 0.996, small_velocity 0.998,
  centered 1.000. The small loss is the commands and the pole's small
  movements.

446 x 0.993 + 9 is about 452. A faster swing-up is the only room left
worth more than a few points.

### How random the actor is

SAC's weight on staying random (α), from `measure.py entropy`:

| Steps | α |
|---|---|
| 10,008 | 0.0864 |
| 100,008 | 0.0216 |
| 500,004 | 0.0256 |
| 1,000,008 | 0.0307 |
| 2,000,004 | 0.0202 |
| 3,000,000 | 0.0127 |
| 4,000,008 | 0.0135 |

Change 6 asked whether α would be small, against the reward, once it had
tuned itself. It was: about 0.013 at the end, against about 0.9 of reward a
step, so the randomness term is roughly 1.5% of the reward, not the 150
times of the last run.

### What this does not show

- **Only the synthetic loop.** Every number here is from simulation: the
  MJCF, the rendered frames and the synthetic detector. Nothing has run on
  the real rig.
- **Only seed 0.** One run, one seed. Whether every seed learns to balance,
  and by about 500,000 steps, is not tested.
- **The policy was not tested apart from training.** All the numbers are
  from training episodes, where SAC still picks its commands with some
  randomness. A run of the saved actor alone, choosing its most likely
  command each time, has not been made.
- **The 2-3° hold was measured from MuJoCo's true angle**, which the policy
  never sees: it acts on the detected markers, with the detector's delay.
