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
  That is the fastest it can move. Each update moves its logarithm by
  about the learning rate, 0.0003, and 2,500 updates (10,000 steps ÷ 4)
  × 0.0003 = 0.75, and e^−0.75 = 0.47. So for 90,000 steps the actor was
  more random than the target, and SAC turned the bonus down as fast as
  it could.
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

**A different number of instances:** change `instances` in
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
  steps on 4; 6 is not measured yet.
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
