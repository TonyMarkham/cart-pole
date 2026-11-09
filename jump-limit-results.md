# Jump-limit results: how fast the real stepper can change speed

Training needs one number from the real rig: `m`, the fastest the
commanded pulse rate can change before the stepper slips. The drive's
ramp holds every command change to it (`synthetic-loop-dia.md`, *The
acceleration ceiling*); until 2026-10-28 the training reward used it to
score how smooth each command change was. This is the record of the run that
measured it. The procedure, the terms and the rig's figures are in
`jump-limit-experiment.md` (the brief). This file gives only what the
run found.

**Result: `m` = 1,210,937 pulses/s², a measured limit.** At 15 µm of
belt per pulse, that is a cart acceleration of 18.2 m/s². It was written
into the reward's table in `__synthetic__/trainer/python/gym.toml` on 2026-10-28,
and moved the same day to `__synthetic__/trainer/python/settings.toml` `[drive] jump_limit`,
where the drive's ramp reads it (`synthetic-loop-dia.md`, *The
acceleration ceiling*).

## The run

- **When:** 2026-10-28. The trials ran from 00:33:57 to 02:27:17,
  1 h 53 min for 193 trials, about 35 s each. The noise floor was
  measured just before them.
- **Harness:** the old WiFi `calibrate` firmware's Jump-limit experiment
  button. After each trial the board sent that trial's record to the
  bench station Pi (`bench-station/`, `POST /trial`), which saved it to
  `~/bench-station/logs/trials.csv`. The summary below is copied from
  the board's page at http://192.168.0.220/ and agrees with the Pi's
  file trial for trial.
- **Conditions** (the board's `calibrate.toml` `[experiment]
  conditions`; nothing on the board measures them):
  - a 24 V supply (its sag under load during the run was not measured);
  - the DM542 at 3.76 A peak, 2.69 A RMS (SW1–3 on, off, off);
  - 1/32 microstepping;
  - the pole mounted.
- **Run settings** (the board's own report):
  - Δt, the gap between commands: 16 ms;
  - top, the highest pulse rate: 48,000 pulses/s;
  - 10 trials at each j;
  - j started at 620,000 and rose 25% a step.
- **Homing before the run:** span 59,830 pulses (897.45 mm). The
  reference switch is D3, the first end.

Terms used below: **j** is the change rate, (new rate − old rate) ÷ Δt,
in pulses/s². **Lost** is the switch count after a trial minus the
reference count. A trial fails when |lost| is larger than the noise
floor's spread.

## Each kind's limit

A kind's limit is the largest j at which all 10 trials kept every step.
A kind stops at its first failed trial.

| Kind | Limit, pulses/s² | As cart acceleration | First j that failed | Lost there |
|---|---|---|---|---|
| From rest (0 → +c) | 1,513,671 | 22.7 m/s² | 1,892,088 | 138 |
| At speed (24,000 → up to 48,000) | at least 1,500,000 | at least 22.5 m/s² | none: 1,500,000 is the most this kind can make (top ÷ 2 ÷ Δt) | — |
| Reversal (−c → +c) | 1,513,671 | 22.7 m/s² | 1,892,088 | 139 |
| Staircase (0 → top, one jump every Δt) | **1,210,937** | **18.2 m/s²** | 1,513,671 | 129 |

`m` is the smallest of the four limits, so that it is safe for every
kind: the staircase's 1,210,937. It counts as measured, not "at least",
because the staircase did fail one step higher.

**How exact it is:** j rose 25% at each step, so the staircase's true
limit lies somewhere from 1,210,937 up to just under 1,513,671. `m` sits
at the bottom of that range, so it can be up to 25% below the true
limit. It cannot be above it. The brief's upper bound was 6,200,000
pulses/s² (section 7). The measured limit is about a fifth of that.

## Every j tried

The lost steps at each j: the smallest and largest of its trials.
For the staircase, the rate column gives the size of each jump, j × Δt.

| Kind | j, pulses/s² | Rates, pulses/s | Passed | Lost |
|---|---|---|---|---|
| From rest | 620,000 | 0 → 9,920 | 10 of 10 | 1 to 11 |
| From rest | 775,000 | 0 → 12,400 | 10 of 10 | 1 to 3 |
| From rest | 968,750 | 0 → 15,500 | 10 of 10 | 1 to 3 |
| From rest | 1,210,937 | 0 → 19,374 | 10 of 10 | 1 to 3 |
| From rest | 1,513,671 | 0 → 24,218 | 10 of 10 | 1 to 3 |
| From rest | 1,892,088 | 0 → 30,273 | 0 of 1 | 138 |
| At speed | 620,000 | 24,000 → 33,920 | 10 of 10 | 1 to 3 |
| At speed | 775,000 | 24,000 → 36,400 | 10 of 10 | 1 to 3 |
| At speed | 968,750 | 24,000 → 39,500 | 10 of 10 | 1 to 4 |
| At speed | 1,210,937 | 24,000 → 43,374 | 10 of 10 | 1 to 3 |
| At speed | 1,500,000 | 24,000 → 48,000 | 10 of 10 | 0 to 3 |
| Reversal | 620,000 | −4,960 → 4,960 | 10 of 10 | 3 to 5 |
| Reversal | 775,000 | −6,200 → 6,200 | 10 of 10 | 3 to 5 |
| Reversal | 968,750 | −7,750 → 7,750 | 10 of 10 | 3 to 5 |
| Reversal | 1,210,937 | −9,687 → 9,687 | 10 of 10 | 3 to 5 |
| Reversal | 1,513,671 | −12,109 → 12,109 | 10 of 10 | 3 to 5 |
| Reversal | 1,892,088 | −15,136 → 15,136 | 0 of 1 | 139 |
| Staircase | 620,000 | jumps of 9,920 | 10 of 10 | 1 to 3 |
| Staircase | 775,000 | jumps of 12,400 | 10 of 10 | 0 to 3 |
| Staircase | 968,750 | jumps of 15,500 | 10 of 10 | −1 to 4 |
| Staircase | 1,210,937 | jumps of 19,374 | 10 of 10 | 2 to 6 |
| Staircase | 1,513,671 | jumps of 24,218 | 0 of 1 | 129 |

Every trial's full record (time, kind, j, trial number, c_a, c_b, Δt,
switch count, lost, pass or fail) is in the Pi's
`~/bench-station/logs/trials.csv`.

## What the numbers show

- **Failure is a cliff, not a slope.** At every j that passed, the
  trials lost 0 to 6 steps, apart from the run's very first trial,
  which lost 11. Each failure lost 129 to 139. No trial landed in
  between. So the pass threshold (18, below) decided nothing: any
  threshold from 12 to 128 gives the same limits.
- **The three failures are nearly the same size.** Take away the
  baseline of 1 to 5 that passing trials show, and each lost about 128.
  That is 4 full steps at 1/32 microstepping (4 × 32 = 128). A reading:
  the rotor fell behind by one whole cycle of the motor's magnetic field
  and then caught up again. This reading is mine; I have not checked it
  against a source.
- **The staircase fails where its single jumps pass.** At 1,513,671 the
  staircase makes two jumps 16 ms apart: 0 → 24,218, then on to 48,000
  (the top). From rest made the first jump 10 times out of 10 at that j,
  and at speed made 24,000 → 48,000 10 times out of 10 at 1,500,000. So
  what failed was the two jumps back to back. The second comes while the
  rotor is still catching up with the first, which matches the brief's
  reason for testing a staircase (section 3). Which of the two jumps
  slipped cannot be told: the switch shows only the total at the end.
- **Reversal loses a little more when it passes.** Its passing trials
  lost 3 to 5 at every j, against 0 to 4 for the other kinds. The
  difference stays the same as j rises, so it does not look like the
  onset of slipping. The cause is not established.

## The noise floor, and what it actually measured

The firmware drove slowly to the D3 switch 10 times, moving 2,000 pulses
away between visits, and noted the count each time:

```
2, 5, 6, 8, 10, 13, 14, 16, 18, 20
```

It reported the mean, 11, as the reference count, and the spread, 18
pulses (20 − 2), as the noise floor.

The counts climb about 2 per visit instead of scattering. The firmware
never re-zeroes its count between these visits (`calibrate/src/
experiment.rs`, `run`, step 1). So the 18 is mostly drift building up
over the 10 round trips, not how much the switch varies. The trials, by
contrast, re-zero the count at the switch after each trial
(`axis.set_position`). That is why their lost steps stay at a steady 1
to 5 across all 193 trials instead of growing. Each trial's small
positive "lost" is about one round trip's worth of that same drift.

None of this changes a result, because of the cliff above. If the
experiment is run again with finer j steps, though, the threshold
should come from the scatter between trials (about ±3 here), not from
the drifting 18.

## What `m` holds for

The brief says `m` is valid only for the rig as it was measured (section
1). Measure again if any of these change:

- the supply voltage;
- the DM542's current setting;
- the microstepping;
- the moving mass (the pole, the cart);
- the gap between commands. A longer gap is a bigger jump at the same j
  (brief, section 3). The real loop's gap is not measured yet; 16 ms was
  used.

The ground-split rewiring the user has planned (the supply powering only
the DM542, the Arduino on USB power) changes no item on that list.
