# The real loop's timings

**Why this exists:** the simulator trains the policy with timings it draws
at random from bands, and those bands should come from the real robot. This
file lists every gap in time between seeing the pole and the motor acting on
what was seen. For each one it gives the measured spread, or TBD where
nothing has been measured, and how the number was or will be measured.

The simulator has two timing settings these feed:

| Setting | Today | What it stands for |
|---|---|---|
| `__synthetic__/trainer/python/gym.toml` `[env]` `command_rate_min`, `command_rate_max` | 20–30 Hz | how often a command is sent: gap 9 |
| `__synthetic__/trainer/python/settings.toml` `rnd_delay_min`, `rnd_delay_max` | 8–15 ms | how long after a frame's capture its command takes effect: gaps 1–8 added up. Its own comment says it is detection alone, with the policy and the WiFi hop not in it yet |

## The rules for every entry

- **One clock:** both ends of a gap are read on the same clock, and the
  entry names it. A time read on one computer can't be set against a time
  read on another, since their clocks are not synchronised.
- **Enough samples:** 300 to 3,000 or more per gap, over several runs, so
  the spread and the rare slow cases are real and not one run's luck.
- **What is reported:** the number of samples, the mean, the standard
  deviation (the spread around the mean), the median, the 90th and 99th
  percentiles, the maximum, and the shape of the histogram.
- **The shape is reported as found:** a gap need not follow a bell curve.
  The board's time awake has two separate peaks, entered below as gaps 8a
  and 8b, and gap 8 is expected to be flat across its band. The simulator should draw from the shape measured, not
  from a bell curve fitted to it.
- **How it was measured:** the method is written down, so anyone can
  repeat it.

## The gaps at a glance

| # | Gap | Starts | Ends | Status | 2 sigma (mean ± 2 std dev)* | 3 sigma (mean ± 3 std dev)* |
|---|---|---|---|---|---|---|
| 1 | Camera | the sensor exposes the frame | the brain has read the whole frame | **measured on the Orin** | 43.877 to 46.929 ms | 43.114 to 47.692 ms |
| 2 | Hand-off to the worker | the frame is read by the brain | the worker starts detection | **measured on the Orin** | 0.279 to 1.067 ms | 0.082 to 1.264 ms |
| 3 | Detection | detection starts | the boxes are ready in the worker | **measured on the Orin** | 18.915 to 26.551 ms | 17.006 to 28.460 ms |
| 4 | The brain between stages | the boxes are ready | the policy's 36 inputs are built | **measured on the Orin** | 0.007 to 0.020 ms* | 0.007 to 0.025 ms* |
| 5 | Policy inference | the inputs go to the policy | `a` comes back | **measured on the Orin** | 0.121 to 0.629 ms | 0.246 to 0.756 ms* |
| 6 | Send | `a` comes back | the 3-byte datagram leaves the Orin | **measured on the Orin** | 0.056 to 0.152 ms | 0.032 to 0.176 ms |
| 2–6 | The brain's share, all five together | the frame arrives at the brain | the command leaves the Orin | **measured on the Orin** | 20.007 to 27.783 ms | 18.063 to 29.727 ms |
| 7 | WiFi hop | the datagram leaves the sender | the Uno's processor has it whole | **measured on the bench**, PC as sender | 1.6 to 120.3 ms* | 1.6 to 156.3 ms* |
| 8 | The board applies it | the processor has the datagram | the new rate runs | **measured on the bench** | 1.803 to 4.127 ms | 1.222 to 4.708 ms |
| 8a | The board awake after a tick | the tick wakes the processor | it sleeps again | **measured on the bench** | 34.9 to 40.1 µs | 33.6 to 41.4 µs |
| 8b | The board awake after a byte from the WiFi chip | a byte wakes the processor | it sleeps again | **measured on the bench** | 7.4 to 12.6 µs | 6.1 to 13.9 µs |
| 9 | Cadence | one command sent | the next command sent | **measured on the Orin** | 29.629 to 36.989 ms | 27.789 to 38.829 ms |

\* Mean ± 2 or 3 std dev describes a bell curve, and some gaps are not one.
Where mean − 2 or 3 std dev comes out at zero or below, which no time can
be, the range starts instead at the shortest time measured, and its top is
still mean + 2 or 3 std dev. That applies to gap 4, 2 and 3 sigma (0.010 −
2 × 0.005 = 0.000 ms; 0.010 − 3 × 0.005 = −0.005 ms; shortest 0.007 ms);
gap 5, 3 sigma (0.375 − 3 × 0.127 = −0.006 ms; shortest 0.246 ms); and gap
7, 2 and 3 sigma (48.3 − 2 × 36.0 = −23.7 ms; 48.3 − 3 × 36.0 = −59.7 ms;
shortest 1.6 ms).

Gaps 1 to 8 added up are what the simulator calls the latency. Gap 9 is its
command rate.

## What the trainer needs

The trainer draws two timings at random for every command: how long after
a frame's exposure its command takes effect (the latency), and how long
until the next command (the command rate). These are the gaps that make up
each, and what they come to together.

The target is each setting's 2 std dev range, mean ± 2 std dev, the last
column.

| Trainer setting | Today | Gap | Shortest | Median | Mean | Std dev | 99% at most | **Target: 2 std dev** |
|---|---|---|---|---|---|---|---|---|
| `rnd_delay_min`, `rnd_delay_max` (`__synthetic__/trainer/python/settings.toml`): the latency | 8–15 ms | 1, camera | 43.103 ms | 45.353 ms | 45.403 ms | 0.763 ms | 47.276 ms | 43.877 to 46.929 ms |
| | | 2–6, the brain's share | 20.529 ms | 23.499 ms | 23.895 ms | 1.944 ms | 26.914 ms | 20.007 to 27.783 ms |
| | | 7, WiFi hop (PC as sender) | 1.6 ms | 48.4 ms | 48.3 ms | 36.0 ms | 131.3 ms | 1.6 to 120.3 ms* |
| | | 8, the board applies it | 1.965 ms | 2.962 ms | 2.965 ms | 0.581 ms | 3.942 ms | 1.803 to 4.127 ms |
| | | **1 to 8 added up**† | **67.197 ms** | **120.214 ms** | **120.563 ms** | **36.065 ms** | **209.432 ms** | **67.197 to 192.693 ms*** |
| `command_rate_min`, `command_rate_max` (`__synthetic__/trainer/python/gym.toml` `[env]`): the command rate | 20–30 Hz | 9, cadence | 22.640 ms (44.2 Hz) | 33.235 ms (30.1 Hz) | 33.309 ms (30.0 Hz) | 1.840 ms | 37.754 ms (26.5 Hz) | **29.629 to 36.989 ms: 27.0 to 33.8 Hz** |

† Each gap was measured in a run of its own, so the total's own spread has
never been measured. The means add exactly. The shortest values added up is
the least the total can be, and the 99% values added up is at least the
total's own 99%, not equal to it. The total's std dev,
√(0.763² + 1.944² + 36.0² + 0.581²) = √1,300.7 = 36.065 ms, is right only
if the gaps vary independently of each other, which has not been checked.
Gaps 8a and 8b are not added: they are waits of 50 µs at most.

\* As in the table above, the range starts at the shortest time measured
where mean − 2 std dev falls below it. Gap 7: 48.3 − 2 × 36.0 = −23.7 ms,
so 1.6 ms. The total: 120.563 − 2 × 36.065 = 48.433 ms, below the 67.197 ms
the shortest values add up to, so 67.197 ms.

The command rate's target in Hz is 1,000 ÷ the interval in ms: 1,000 ÷
36.989 = 27.0 Hz and 1,000 ÷ 29.629 = 33.8 Hz.

What the table shows:
- **The latency today is far too short.** The trainer draws 8–15 ms; the
  measured gaps add up to 120.6 ms on average, and never less than 67.2 ms.
  The 2 std dev target is 67.2 to 192.7 ms.
- **Gap 7 is nearly all of the latency's spread.** Every other gap varies
  by a few ms; the WiFi hop varies from 1.6 ms to 131.3 ms at its 99%, in
  three separate peaks (gap 7, *Result*). It was measured with the PC
  sending, not the Orin, so the Orin's own radio may give other numbers.
- **The command rate sits at the top of today's band.** The trainer draws
  20–30 Hz; the rig sends at 30.0 Hz on average, and 99% of its intervals
  are at most 37.8 ms, 26.5 Hz. The 2 std dev target is 27.0 to 33.8 Hz.
- **The shapes matter** (*The rules for every entry*): gaps 3 and 7 are not
  bell curves, so the trainer should draw from the shapes measured, not from
  a mean and a standard deviation.

## Suggestions for the synthetic loop

These are suggestions, not decisions. A change to how the loop works lands
in `synthetic-loop-dia.md` first, then in the plans and the code
(`AGENTS.md`).

Today the synthetic loop draws one delay per command, RND(8, 15) ms, from
the frame's capture to the command taking effect, and draws when the next
capture comes, RND(20, 30) Hz. The suggestion splits the delay into the
three parts the rig has, each drawn at random; adds a draw for a command
that never arrives; and takes the next capture from the camera's frames, as
the rig does, instead of drawing it:

```text
capture ──RND(a, b)──> the frame reaches the Orin ──RND(c, d)──> the policy's answer ──RND(e, f)──> the motor's rate changes
                                                                                   RND(0, 3000) = 0: the command is lost
next capture: the first camera frame that reaches the Orin after the policy's answer
```

| # | Draw | The synthetic loop today | Reality: the gaps it covers, measured | Suggested setting: 2 std dev |
|---|---|---|---|---|
| 1 | **Capture → the frame reaches the Orin**, RND(a, b) ms | part of RND(8, 15) ms | gap 1: mean 45.403 ms, std dev 0.763 ms, shortest 43.103 ms | a = 43.9, b = 46.9 ms |
| 2 | **The frame reaches the Orin → the policy's answer**, RND(c, d) ms | part of RND(8, 15) ms | gaps 2–5: mean 23.791 ms, std dev 1.943 ms, shortest 20.441 ms | c = 20.4*, d = 27.7 ms |
| 3 | **The policy's answer → the motor's rate changes**, RND(e, f) ms | part of RND(8, 15) ms | gaps 6 + 7 + 8: mean 51.369 ms, std dev 36.005 ms, shortest 3.591 ms | e = 3.6*, f = 123.4 ms |
| 4 | **A lost command**, RND(0, 3000): lost when it comes out 0, and the cart keeps the last command | every command arrives | 1 datagram of 3,000 lost on the WiFi (gap 7, *Result*) | 1 in 3,000 |

"2 std dev" is the mean less and plus twice the standard deviation, the
spread around the mean: on a bell curve it holds about 95% of the values.

**The next capture is not drawn** (user, 2026-11-08). On the rig the brain
sends its command and then takes the next frame to reach the Orin, so the
time between captures follows from the delays above and the camera's own
frame times:
- the camera exposes a frame every 16.67 ms, at 60 frames a second;
- each frame reaches the Orin its own row 1 draw after its exposure;
- the next capture is the first of those exposures whose frame reaches the
  Orin after the policy's answer (rows 1 and 2). Sending the command, gap 6,
  takes 0.104 ms on average, under half a 2 ms step, so on the grid the
  command leaves at the answer's step.

Worked through with the means: a capture at 0 ms reaches the Orin at
45.4 ms and has its answer at 45.4 + 23.8 = 69.2 ms. The frame exposed at
16.7 ms reached the Orin at 16.7 + 45.4 = 62.1 ms, before the answer; the
one exposed at 33.3 ms reaches it at 78.7 ms, after. So the next capture is
at 33.3 ms, which is what the rig measured (gap 9: 33.3 ms median). This
replaces the command rate's draw, `command_rate_min` and
`command_rate_max` (`gym.toml` `[env]`).

Where the numbers come from:
- **Row 1:** gap 1. 2 std dev: 45.403 ± 2 × 0.763 = 43.877 to 46.929 ms.
- **Row 2:** gaps 2 to 5 together, `answered − arrived` for each of the
  3,000 frames in the stage test's `frames.csv` (the run of gap 2): mean 23.791 ms,
  std dev 1.943 ms, shortest 20.441 ms. 2 std dev: 23.791 ± 3.886 = 19.905
  to 27.677 ms.
- **Row 3:** gaps 6, 7 and 8. Mean 0.104 + 48.3 + 2.965 = 51.369 ms; std
  dev √(0.024² + 36.0² + 0.581²) = 36.005 ms, which adds the three gaps'
  spreads as if each varies independently of the others, not checked. 2 std
  dev: 51.369 ± 72.010 = −20.641 to 123.379 ms.
- **The three means** add up to the whole latency's 120.563 ms
  (*What the trainer needs*).
- \* Where the 2 std dev range's lower end falls below the shortest time
  measured, it starts there instead, as in the tables above: no time can be less
  than the shortest measured. Row 2: 19.905 ms, below 20.441 ms. Row 3:
  −20.641 ms, below the 0.026 + 1.6 + 1.965 = 3.591 ms the three gaps'
  shortest add up to.

How the draws run (user, 2026-11-08):

- **Simulated time, not the PC's:** each phase lasts its drawn time in
  MuJoCo's time, the simulated clock MuJoCo steps through. If the PC has
  not finished the phase's work (the render, the detection, the policy) by
  the end of the drawn time, MuJoCo waits for it. If the PC has finished
  early, the next phase does not begin until the drawn time has passed. So
  the PC's own speed never reaches what the policy sees: only the draws do.
  The loop already works this way for its one delay today
  (`synthetic-loop-dia.md`, *Timing*: MuJoCo freezes at the arrival step
  until the reply is in).
- **On MuJoCo's 2 ms grid:** every draw, and every camera exposure time, is
  rounded to the nearest 2 ms step, MuJoCo's timestep. Today the loop takes
  the first step later than the drawn time instead (*Timing*).
- **Commands in flight:** a command takes effect about 120.6 ms after its
  capture, while captures come every 33.3 ms, so a capture comes while
  earlier commands are still on their way. Each waits, in simulated time,
  for its own step to act, while the next capture goes ahead on its own
  schedule. The diagram's rule that every command takes effect before the
  next capture (*Timing*) would have to go.
## Each gap

> ### 1. Camera
>
> - **Starts:** the camera's sensor (the IMX296 on the Orin) exposes the frame.
> - **Ends:** the brain has read the whole frame from GStreamer's pipe.
> - **Clock:** the Orin's.
> - **The camera's timing,** which the numbers below rest on: 60 frames a
>   second, so a new frame every 16.67 ms (`camera-timing.toml`
>   `framerate = 60`; the camera reported 59.999999 frames a second at
>   start-up). Of each 16.67 ms the sensor collects light for only 1 ms, the
>   exposure (`exposure_ns = 1_000_000`, locked as the brain locks it), and
>   for the other 15.67 ms it collects none. So:
>   - a flash shows in a frame only if it overlaps that frame's 1 ms; one
>     that falls in the 15.67 ms between leaves no trace;
>   - a 500 µs flash at a random moment overlaps an exposure if it begins
>     within the 1 ms + 0.5 ms = 1.5 ms before that exposure ends: 1.5 ÷ 16.67
>     ≈ 9% of flashes;
>   - no flash can light two frames, since one exposure ends 15.67 ms before
>     the next begins.
> - **Method:** `__workbench__/README.md`, *The camera test*. The frame's
>   GStreamer stamp can't give it: GStreamer stamps a frame when it passes it
>   on, not at exposure (`real-loop-dia.md`, *Difference 1*). So the Orin
>   flashes an LED in the camera's view from header pin 33
>   (`__workbench__/orin/README.md`), and stamps each flash and each frame's
>   arrival on its monotonic clock
>   (`__workbench__/orin/rust/crates/camera-timing`, which opens the camera
>   with the brain's settings: 60 frames a second, 1 ms exposure, 728×544
>   greyscale). Each flash lasts 500 µs, at a random moment, about 150 ms
>   apart. A frame that shows a flash was exposing during it, so its exposure
>   began between 1 ms before the flash and the flash's end: a band 1.52 ms
>   wide. Gap 1 is the frame's arrival less the band's middle, so each value
>   is good to ±0.76 ms.
> - **Result, 2026-11-08, one run:** run folder
>   `~/workbench-orin/rust/runs/camera_1794118815` on the Orin,
>   `__workbench__/orin/rust/camera-timing.toml`'s settings. 10,000 flashes,
>   531 seen.
>
>   | Count | Mean | Std dev | Shortest | Median | 90% | 99% | 99.9% | Longest |
>   |---|---|---|---|---|---|---|---|---|
>   | 531 | 45.403 ms | 0.763 ms | 43.103 ms | 45.353 ms | 46.381 ms | 47.276 ms | 47.570 ms | 47.570 ms |
>
>   One peak, in 0.5 ms bins:
>
>   | Gap 1 | Flashes |
>   |---|---|
>   | 43.0–43.5 ms | 3 |
>   | 43.5–44.0 ms | 10 |
>   | 44.0–44.5 ms | 46 |
>   | 44.5–45.0 ms | 103 |
>   | 45.0–45.5 ms | 136 |
>   | 45.5–46.0 ms | 118 |
>   | 46.0–46.5 ms | 72 |
>   | 46.5–47.0 ms | 29 |
>   | 47.0–47.5 ms | 11 |
>   | 47.5–48.0 ms | 3 |
>
> - **How far to trust it:**
>   - **The band:** each value is the middle of a 1.52 ms band. A value spread
>     evenly across the band has a standard deviation of 1.52 ÷ √12 ≈
>     0.44 ms, so part of the 0.763 ms is the method's, not the camera's.
>   - **Seen, not counted:** about 9% of flashes overlap an exposure; 5.3%
>     were counted. A frame counts only when the LED's patch is brighter than
>     the threshold halfway between dark and lit (128.1 of 255). A 500 µs
>     flash lights at most half of the 1 ms exposure, so the frames it only
>     partly lit stay under the threshold. 843 frames brightened past 100;
>     531 passed.
>   - **Not an artifact:** in the 200-flash check before this run, all 15
>     frames that brightened past 100, out of 1,800, arrived 44.2–46.0 ms
>     after a flash began, while the flashes fell at random about 150 ms
>     apart. The long run's 312 partly lit frames were not checked this way.
>   - **One run:** the rules ask for several.
> - **What this means:** every picture the brain works on shows the pole as
>   it was about 45 ms earlier (43.1–47.6 ms in this run). That is about 2.7
>   frame periods: by the time a frame arrives, the camera has already
>   exposed the next two. Nothing the Orin does afterwards wins this time
>   back; it is part of every command's delay. Its spread is under 1 ms, so
>   it is almost the same for every frame.

> ### 2. Hand-off to the worker
>
> - **Starts:** the brain has read the frame.
> - **Ends:** the Python worker starts detection on it.
> - **Clock:** the Orin's.
> - **Method:** `__workbench__/README.md`, *The stage test*. The brain's log
>   stamps only `arrived` and `sent`, so
>   `__workbench__/orin/rust/crates/stage-timing` runs the brain's loop as
>   the brain does: the same camera settings, the brain's Python worker
>   unchanged, the same matrix and the same policy. It stamps the end of each
>   stage on the Orin's monotonic clock: `arrived`, `frame_sent`, `boxes`,
>   `inputs`, `answered`, `sent`. Each of gaps 2 to 6 is the difference of two
>   neighbouring stamps, so for every frame the five add up to
>   `sent − arrived`. Only the brain's side is stamped, so this gap ends when
>   the frame's last byte is in the worker's pipe. The pipe holds up to
>   64 KiB, so the worker may still have that much of the 396,032-byte frame
>   to read, and that counts in gap 3. The datagrams went to the workbench
>   PC, not the rig's Arduino, so the cart did not move.
> - **Result, 2026-11-08, one run:** run folder
>   `~/workbench-orin/rust/runs/stage_1794121856` on the Orin,
>   `__workbench__/orin/rust/stage-timing.toml`'s settings. 3,000 frames, all
>   four markers found in every one. The same run gives gaps 3 to 6 and 9.
>
>   | Count | Mean | Std dev | Shortest | Median | 90% | 99% | 99.9% | Longest |
>   |---|---|---|---|---|---|---|---|---|
>   | 3,000 | 0.673 ms | 0.197 ms | 0.370 ms | 0.656 ms | 0.822 ms | 1.372 ms | 2.453 ms | 3.284 ms |
>
>   One peak: 2,013 frames in 0.50–0.75 ms, in 0.25 ms bins, and a thin tail
>   of 85 past 1 ms.
> - **What this means:** handing a frame from the brain to the Python worker
>   takes about two-thirds of a millisecond, and under 1.4 ms for 99% of
>   frames.

> ### 3. Detection
>
> - **Starts:** detection starts in the worker: pre-processing, TensorRT,
>   post-processing.
> - **Ends:** the boxes are ready.
> - **Clock:** the Orin's.
> - **Method:** as for gap 2, from `frame_sent` to `boxes`. Besides
>   detection itself, this holds the worker reading the last of the frame
>   (up to 64 KiB), its JPEG encode and decode at quality 90 before detection
>   (`brain_worker.py`, `to_jpeg`), and its 80-byte answer back through the
>   pipe.
> - **Result, 2026-11-08, one run:** the run of gap 2.
>
>   | Count | Mean | Std dev | Shortest | Median | 90% | 99% | 99.9% | Longest |
>   |---|---|---|---|---|---|---|---|---|
>   | 3,000 | 22.733 ms | 1.909 ms | 19.669 ms | 22.317 ms | 24.421 ms | 25.670 ms | 26.290 ms | 98.373 ms |
>
>   Two peaks, in 1 ms bins:
>
>   | Gap 3 | Frames |
>   |---|---|
>   | 19–20 ms | 2 |
>   | 20–21 ms | 101 |
>   | 21–22 ms | 1,070 |
>   | 22–23 ms | 815 |
>   | 23–24 ms | 214 |
>   | 24–25 ms | 606 |
>   | 25–26 ms | 185 |
>   | 26–27 ms | 6 |
>   | 98.4 ms | 1, the run's first frame |
>
>   What causes the two peaks is not known. The 98.4 ms is the run's first
>   frame; in the 200-frame check run just before, the first frame took
>   106.0 ms and was likewise the only one over 30 ms. The mean and standard
>   deviation include it. An earlier measurement, **11.9–14.5 ms per frame**
>   from a scratch test outside the brain on 2026-10-23 (`detector-plan.md`,
>   *Results*; removed 2026-11-11, in git at commit `ecced2cc`), is a range only, with no count, mean or standard deviation;
>   what it included has not been compared with this.
> - **What this means:** finding the four markers is nearly all of the
>   brain's share: 22.3 ms of the 23.5 ms median of gaps 2–6. The other four
>   gaps together come to about 1.1 ms. The first frame of a run is the
>   exception, at about 100 ms.

> ### 4. The brain between stages
>
> - **Starts:** the boxes are ready.
> - **Ends:** the policy's 36 inputs are built: pixels to metres, and the
>   pool of the last six frames.
> - **Clock:** the Orin's.
> - **Method:** as for gap 2, from `boxes` to `inputs`. It starts when the
>   boxes are back in the brain, so their trip through the pipe is in gap 3.
> - **Result, 2026-11-08, one run:** the run of gap 2.
>
>   | Count | Mean | Std dev | Shortest | Median | 90% | 99% | 99.9% | Longest |
>   |---|---|---|---|---|---|---|---|---|
>   | 3,000 | 0.010 ms | 0.005 ms | 0.007 ms | 0.010 ms | 0.011 ms | 0.018 ms | 0.075 ms | 0.149 ms |
>
>   One peak: 2,968 frames in 0.005–0.015 ms, in 0.005 ms bins.
> - **What this means:** the brain's own bookkeeping between detection and
>   the policy, turning the markers' pixel positions into metres and adding
>   them to the last six frames, takes about 10 µs.

> ### 5. Policy inference
>
> - **Starts:** the 36 inputs are given to the policy.
> - **Ends:** `a` comes back.
> - **Clock:** the Orin's.
> - **Method:** as for gap 2, from `inputs` to `answered`. Besides the policy
>   itself, this holds the 36 inputs' trip into the worker's pipe and `a`'s
>   trip back.
> - **Result, 2026-11-08, one run:** the run of gap 2.
>
>   | Count | Mean | Std dev | Shortest | Median | 90% | 99% | 99.9% | Longest |
>   |---|---|---|---|---|---|---|---|---|
>   | 3,000 | 0.375 ms | 0.127 ms | 0.246 ms | 0.359 ms | 0.448 ms | 1.032 ms | 1.616 ms | 1.724 ms |
>
>   One peak: 1,880 frames in 0.3–0.4 ms, in 0.1 ms bins, and a tail of 184
>   past 0.5 ms.
> - **What this means:** the trained policy turns the 36 inputs into the
>   command, `a`, in about a third of a millisecond.

> ### 6. Send
>
> - **Starts:** `a` comes back.
> - **Ends:** the 3-byte datagram leaves the Orin.
> - **Clock:** the Orin's.
> - **Method:** as for gap 2, from `answered` to `sent`: the command worked
>   out from `a` and its datagram handed to the Orin's system. The datagram
>   went to the workbench PC over the Orin's network, not to the rig's
>   Arduino; the send returns once the system has the datagram and does not
>   wait for it to arrive.
> - **Result, 2026-11-08, one run:** the run of gap 2.
>
>   | Count | Mean | Std dev | Shortest | Median | 90% | 99% | 99.9% | Longest |
>   |---|---|---|---|---|---|---|---|---|
>   | 3,000 | 0.104 ms | 0.024 ms | 0.026 ms | 0.104 ms | 0.119 ms | 0.172 ms | 0.288 ms | 0.819 ms |
>
>   One peak: 2,701 frames in 0.075–0.125 ms, in 0.025 ms bins.
> - **What this means:** the decided command leaves the Orin, packed into 3
>   bytes and handed to the network, in about 0.1 ms. Its journey to the
>   board is gap 7.

> ### 2–6. The brain's share, all five together
>
> - **Starts:** the frame arrives at the brain (`arrived`).
> - **Ends:** its command leaves (`sent`).
> - **Clock:** the Orin's monotonic clock, in ms since the Run press.
> - **Method:** two ways. In the brain's own runs, `sent − arrived` for each
>   frame in its `frames.jsonl`, summarised by `gather_runs.py` on the Orin.
>   In the stage test (gap 2, *Method*), the same `sent − arrived`, which is
>   also the sum of its five gaps.
> - **Result, 2026-11-07, the brain's runs:** 4 runs of 30 s, 854–856 frames
>   each, about 3,420 frames in all: run folders `20261108_020532`,
>   `_020615`, `_020658` and `_020745` in `~/orin-runs` on the Orin (the
>   folder names are in UTC). In every run the median was **23.1 ms**, the
>   90th percentile 24.2–24.6 ms and the maximum 27.2–27.7 ms. Their mean and
>   standard deviation were not worked out.
> - **Result, 2026-11-08, the stage test:** the run of gap 2.
>
>   | Count | Mean | Std dev | Shortest | Median | 90% | 99% | 99.9% | Longest |
>   |---|---|---|---|---|---|---|---|---|
>   | 3,000 | 23.895 ms | 1.944 ms | 20.529 ms | 23.499 ms | 25.679 ms | 26.914 ms | 28.214 ms | 99.792 ms |
>
>   Two peaks, gap 3's, in 1 ms bins:
>
>   | Gaps 2–6 | Frames |
>   |---|---|
>   | 20–21 ms | 1 |
>   | 21–22 ms | 69 |
>   | 22–23 ms | 948 |
>   | 23–24 ms | 878 |
>   | 24–25 ms | 287 |
>   | 25–26 ms | 583 |
>   | 26–27 ms | 210 |
>   | 27–28 ms | 19 |
>   | 28–29 ms | 4 |
>   | 99.8 ms | 1, the run's first frame |
>
>   The two ways agree to 0.4 ms at the median, 23.5 against 23.1 ms. The
>   brain's runs' longest was 27.7 ms, so none of their frames, the first
>   included, took 100 ms. One difference between the two: the brain starts
>   its worker once, when the brain starts, and it serves every run; the
>   stage test starts a fresh worker for each run. Whether that is why only
>   the stage test's first frame is slow has not been checked.
> - **What this means:** the Orin's own work on each frame, from having the
>   picture to sending its command, takes about 23.5 ms, and under 27 ms for
>   99% of frames; nearly all of it is detection (gap 3). Added to gap 1, a
>   command leaves the Orin about 45.4 + 23.5 ≈ 69 ms after its picture was
>   exposed (gap 1's mean plus this median, from different runs).

> ### 7. WiFi hop
>
> - **Starts:** the datagram leaves the sender.
> - **Ends:** the Uno's processor has the datagram whole, after the WiFi
>   chip's serial hand-off.
> - **Clock:** the sender's. The Uno's clock can only be used once it has been
>   synchronised to the sender's.
> - **Method:** the bench (`__workbench__/README.md`, *The WiFi test*). The
>   PC sends the bench R4 3,000 random valid commands over WiFi, each one
>   different, 33–50 ms apart. The R4 (`uno_4r/crates/wifi-firmware`) stamps
>   each one on a free-running 48 MHz timer the moment it has it whole. It
>   reports the stamp over USB, and does nothing else, so nothing delays it.
>   The R4's clock is carried into the PC's by NTP's four-stamp exchange over
>   USB: a sync before every datagram and 50 at each end. A straight line is
>   fitted through the 20% of syncs with the shortest round trips in each
>   tenth of the run. The PC stands in for the Orin: the hop measured is
>   PC → router → R4, not the Orin's radio.
> - **Result, 2026-11-07, one run:** run folder
>   `__workbench__/pc/crates/wifi-host/runs/wifi_1794108601`. 3,000 sent,
>   2,999 reported, 1 lost.
>
>   | Count | Mean | Std dev | Shortest | Median | 90% | 99% | 99.9% | Longest |
>   |---|---|---|---|---|---|---|---|---|
>   | 2,999 | 48.3 ms | 36.0 ms | 1.6 ms | 48.4 ms | 104.1 ms | 131.3 ms | 150.1 ms | 606.4 ms |
>
>   Not one bell curve: three peaks, in 10 ms bins.
>
>   | Hop | Datagrams |
>   |---|---|
>   | 0–10 ms | 772 |
>   | 10–40 ms | 132 |
>   | 40–60 ms | 1,430 |
>   | 60–100 ms | 243 |
>   | 100–120 ms | 379 |
>   | 120–160 ms | 42 |
>   | 606 ms | 1 |
>
>   What causes the peaks is not known.
> - **How far to trust it:**
>   - **USB asymmetry:** the sync round trip over USB was 6.1–6.7 ms. If the
>     line is slower one way than the other, every hop is off by up to half
>     the shortest round trip, 3.1 ms, by the same amount for all of them.
>   - **Scatter:** the kept syncs sit off the fitted line by 1.3 ms standard
>     deviation, 2.6 ms at most. So the R4's clock is not a perfectly
>     straight line against the PC's over the run, and each hop carries up
>     to about that much error.
>   - **The R4's clock:** 1,117 ppm (0.11%) slower than 48 MHz, as the PC's
>     clock sees it.
>   - **Small errors next to the hops:** these are a few ms against hops of
>     tens of ms. The 1.6 ms shortest hop is within them.
> - **What this means:** a command takes 48 ms on average to cross the WiFi
>   to the Uno, but how long varies widely from one command to the next:
>   three peaks, at 0–10, 40–60 and 100–120 ms, and once 606 ms. It is the
>   longest and least predictable gap measured so far. The PC was the sender,
>   so the Orin's radio may give other numbers.

> ### 8. The board applies it
>
> - **Starts:** the Uno's processor has the datagram.
> - **Ends:** the step generator runs at a new rate heading for the command.
> - **Clock:** the bench R3's Timer1, 62.5 ns a count.
> - **Expected by design:** **2–4 ms**. The ramp is re-aimed at once, but it
>   steps only at the next 2 ms tick (0–2 ms away), and the rate queued there
>   runs from the tick after (2 ms more; the controller's
>   `cart/stop_points.rs`, `cap`). Datagrams arrive with no relation to the
>   tick, so the delay should be spread evenly across the band: mean 3 ms,
>   standard deviation 2 ms ÷ √12 ≈ 0.58 ms.
> - **Method:** the bench (`__workbench__/README.md`, *The apply test*). The
>   bench R4 (`uno_4r/crates/apply-firmware`) runs a copy of the controller's
>   MOVING loop and flips D3 the moment it has a command whole. The PC sends
>   commands alternating between 38,400 and 40,000 pulses/s, 33–50 ms apart.
>   The 1,600 pulses/s between them is within one tick's ramp step, so each
>   command finishes in one tick and the rate holds until the next. The R3
>   (`uno_r3/apply_timer`) times the D3 edge and latches every STEP pulse's
>   falling edge in hardware. The delay runs from the edge to the start of
>   the first period at the new rate.
> - **Result, 2026-11-07, one run:** run folder
>   `__workbench__/pc/crates/apply-host/runs/apply_1794109500`. Of 3,000
>   commands, 2,515 were applied and timed:
>
>   | Count | Mean | Std dev | Shortest | Median | 90% | 99% | 99.9% | Longest |
>   |---|---|---|---|---|---|---|---|---|
>   | 2,515 | 2.965 ms | 0.581 ms | 1.965 ms | 2.962 ms | 3.786 ms | 3.942 ms | 3.957 ms | 3.958 ms |
>
>   Flat from 2 to 4 ms, about 300 in every 250 µs bin, as the design
>   expects, and its mean and standard deviation match the design's 3 ms and
>   0.58 ms. The 30 below 2.0 ms are 1.965 ms at the shortest, 35 µs short.
>   That fits two known offsets, neither checked one by one:
>   - the R3 times the D3 edge a few µs late;
>   - the step generator writes a one-period transition period at the tick
>     (unor4-hal `step/step_generator.rs`: "a tick-written transition period
>     lasts one period only"), so the first period that differs can begin up
>     to one period, 25–26 µs, before the tick.
> - **Not timed:**
>   - **No change seen:** for 484 commands (16%) the R3 saw no new rate. The
>     sketch reports a command that way when the rate doesn't change within
>     10 ms, and when the next command's mark comes before its rate changed.
>     Which of the two each was is not recorded. Most were single, 351 of
>     them; 57 ran two in a row, 5 three, 1 four.
>   - **No line:** for 1 command the R3 sent no line at all.
> - **Limits:** the bench R4 reads no limit switches; and the D3 edge is
>   timed when the R3's interrupt runs, a few µs late.
> - **What this means:** once a command has reached the Uno, the motor runs
>   at the new rate 2–4 ms later, spread evenly across that band, because
>   the board changes speed only on its 2 ms tick. Small and predictable, as
>   designed. For 16% of commands the test saw no change of rate; which of
>   the two reasons above applied to each is not recorded.

> ### 8a. The board awake after a tick
>
> - **Starts:** the 2 ms tick wakes the Uno's processor from its sleep
>   (`WFI`).
> - **Ends:** it goes back to sleep, the tick's work done: the count, the
>   ramp, the soft limit, and queuing the next rate.
> - **Why it matters:** a datagram that arrives while the processor is awake
>   waits until it looks again, so this bounds that wait.
> - **Clock:** the bench R3's Timer1, 0.5 µs a count.
> - **Method:** the bench (`__workbench__/README.md`, *The awake test*). The
>   bench R4 runs a copy of the controller's MOVING loop and holds D3 high
>   while awake. The PC sends it random commands at 33–50 ms gaps, and the R3
>   times every high stretch into 2 µs bins. The run's stretches fall into
>   two separate peaks; this gap is the upper one, 26–50 µs. Gap 8b is the
>   lower.
> - **Result, 2026-11-07:** run folder
>   `__workbench__/pc/crates/bench-host/runs/awake_1794107491`, 125.2 s.
>
>   | Count | Mean | Std dev | Range |
>   |---|---|---|---|
>   | 61,609 | 37.5 µs | 1.3 µs | 26–50 µs |
>
>   The longest stretch of the run, 49.0 µs, is in this peak. The mean and
>   standard deviation are worked out from the bins' centres, so each is
>   good to about a bin, 2 µs.
> - **Limits:**
>   - **Missed edges:** 2,565 edges (2.3% of the run's stretches, both
>     peaks together) came too close together for the R3's interrupt, so
>     their stretches are not counted.
>   - **Timing error:** each edge is timed when the R3's interrupt runs, a
>     few µs after it.
>   - **Bench, not rig:** the bench R4 reads no limit switches, which the rig
>     does first on every wake.
> - **What this means:** every 2 ms the board spends about 37.5 µs on its
>   tick, and a command that arrives meanwhile waits until the tick's work is
>   done: 50 µs at the very most. That is tens of µs against gaps of ms.

> ### 8b. The board awake after a byte from the WiFi chip
>
> - **Starts:** a byte from the WiFi chip wakes the Uno's processor from its
>   sleep (`WFI`). Each datagram reaches it as a 17-byte frame,
>   `+UDPRX: 0,3|`, the 3 bytes, then `\r\n`, and every byte wakes it.
> - **Ends:** it goes back to sleep, the byte read.
> - **Why it matters:** as for gap 8a, a datagram that arrives while the
>   processor is awake waits until it looks again.
> - **Clock:** the bench R3's Timer1, 0.5 µs a count.
> - **Method:** the same run as gap 8a, its lower peak, 8–26 µs.
> - **Result, 2026-11-07:** run folder
>   `__workbench__/pc/crates/bench-host/runs/awake_1794107491`. 49,130
>   wakes for 3,000 datagrams, about 16 each.
>
>   | Count | Mean | Std dev | Range |
>   |---|---|---|---|
>   | 49,130 | 10.0 µs | 1.3 µs | 8–26 µs |
>
>   The mean and standard deviation are worked out from the bins' centres,
>   so each is good to about a bin, 2 µs.
> - **Limits:** as for gap 8a, the same run's.
> - **What this means:** each byte of a datagram wakes the board for about
>   10 µs, and a command that arrives meanwhile waits that long, 26 µs at the
>   very most. Like gap 8a, tens of µs against gaps of ms.

> ### 9. Cadence
>
> - **Starts:** one command is sent.
> - **Ends:** the next command is sent.
> - **Clock:** the Orin's.
> - **Method:** two ways. In the brain's own runs, the gap between the
>   `stamp`s of consecutive frames the brain used, from `frames.jsonl`,
>   summarised by `gather_runs.py`. In the stage test (gap 2, *Method*), the
>   gap between consecutive `sent` stamps, on the Orin's monotonic clock. The
>   stage test leaves out what the brain does after `sent`, its log line and
>   its look for the board's replies, so its cadence may be a little shorter
>   than the brain's.
> - **Result, 2026-11-07, the brain's runs:** the same 4 runs as gaps 2–6.
>   The median was **33.3 ms** in every run, two camera frames at 60 frames
>   a second, and the maximum 37.8–43.7 ms. Their mean and standard deviation
>   were not worked out. This uses the frames' GStreamer stamps, which mark
>   when GStreamer passed each frame on, not when it was exposed (gap 1,
>   *Method*). So each interval also carries how much that hand-on varies
>   from frame to frame, which has not been measured.
> - **Result, 2026-11-08, the stage test:** the run of gap 2.
>
>   | Count | Mean | Std dev | Shortest | Median | 90% | 99% | 99.9% | Longest |
>   |---|---|---|---|---|---|---|---|---|
>   | 2,999 | 33.309 ms | 1.840 ms | 22.640 ms | 33.235 ms | 35.648 ms | 37.754 ms | 40.129 ms | 40.791 ms |
>
>   One peak, in 1 ms bins:
>
>   | Cadence | Commands |
>   |---|---|
>   | 22–24 ms | 5 |
>   | 26–28 ms | 11 |
>   | 28–30 ms | 77 |
>   | 30–32 ms | 541 |
>   | 32–34 ms | 1,392 |
>   | 34–36 ms | 748 |
>   | 36–38 ms | 203 |
>   | 38–40 ms | 18 |
>   | 40–41 ms | 4 |
>
>   (Pairs of 1 ms bins are added together here; none fell in 24–26 ms.)
> - **What this means:** the Orin sends a command every 33.3 ms, every
>   second camera frame: 30 commands a second. Nearly 9 in 10 intervals are
>   30–36 ms, and 99% are under 38 ms. The simulator draws its
>   command rate from 20–30 Hz (`gym.toml` `[env]`), so the rig runs at the
>   top of that band.
> - **Why the camera's frame interval is not a gap of its own:** a new frame
>   enters the loop only after the last frame's command has been sent. The
>   brain takes the camera's next frame after `sent` and drops, unread, any
>   that arrived during the pass (the brain's `run/run_loop.rs`,
>   `next_after(camera.seq())`). So the interval between the camera's frames
>   shows in the loop only as the wait from `sent` to the next frame, 0 to
>   16.7 ms at 60 frames a second, and that wait is already inside this gap.
