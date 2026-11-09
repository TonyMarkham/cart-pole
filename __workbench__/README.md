# The workbench

**Why it exists:** some of the rig's timings can only be measured with a
clock that sees both ends of what is being timed. The rig itself has no such
clock: the Orin, the Pi and the Arduino each keep their own, and none is
shared. The HAL's bench, two Arduino boards wired to each other on this PC,
has one. The R3 times the R4's pins on its own Timer1, so anything the R4
shows on a pin is timed on that one clock. This folder holds the code that
uses the bench to measure the controller's timings away from the rig.

Nothing here runs on the rig, and nothing here changes the HAL's repository
(`D:\git\unor4-hal`): the bench is only borrowed.

## The bench

Two boards, both plugged into this PC by USB, wired to each other as the
HAL's `Validation.md` describes (*All-in-one validation*):

| Board | Its role here | Its USB port on this PC |
|---|---|---|
| Arduino UNO R4 WiFi (the R4) | runs the rig controller's run loop, `uno_4r/crates/bench-firmware` | COM3 |
| Arduino UNO R3 (the R3) | times the R4's pins, `uno_r3/awake_timer` | COM4 |

The wires between them:

| R4 | R3 | Here |
|---|---|---|
| D3 | D3 | the awake test: high while the R4's processor is awake; the apply test: flipped as each command arrives |
| GND | GND | shared ground |
| D4 (STEP) | D8 | the apply test: the R3 latches each STEP pulse |
| D5 (DIR) | D7 | not used yet |
| D1 ↔ D0, D0 ↔ D1 | | not used: the R4 leaves both as inputs |

The ports are `bench-host.toml`'s `[r3] port` and `[r4] port`.
`arduino-cli board list` shows which is which.

## The folders

- `uno_4r/crates/bench-firmware`: Rust firmware for the R4. It copies the
  rig controller's MOVING loop (the 2 ms tick, the ramp, the soft limit's
  arithmetic, the WiFi frames), and nothing moves: STEP and DIR go only to
  the R3.
- `uno_r3/awake_timer`: an Arduino sketch for the R3, the timing instrument.
- `pc/crates/bench-host`: the PC program that runs the awake test. It sends
  the R4 its commands over WiFi, starts and reads the R3's measurement over
  USB, and writes each run's results.
- `uno_4r/crates/wifi-firmware`: Rust firmware for the R4 for the WiFi test.
  It stamps each datagram and each clock sync on a free-running timer and
  reports the stamps over USB. No step generator, no R3.
- `pc/crates/wifi-host`: the PC program that runs the WiFi test.
- `uno_4r/crates/apply-firmware`: Rust firmware for the R4 for the apply
  test: the controller's MOVING loop, with D3 flipped the moment each
  command is whole.
- `uno_r3/apply_timer`: the R3 sketch that times each D3 edge against the
  STEP pulses on D8.
- `pc/crates/apply-host`: the PC program that runs the apply test.
- `orin/rust/crates/camera-timing`: the Orin program for gap 1, the camera,
  in its own workspace, built on the Orin. `orin/led-circuit.svg` is the LED
  it flashes.
- `orin/rust/crates/stage-timing`: the Orin program for gaps 2 to 6, the
  brain's stages, in the same workspace.

## The awake test: how long the R4's processor stays awake

**What it answers:** the controller's processor sleeps between events, and
wakes for each 2 ms tick and each datagram. While it is awake after a tick,
a datagram that arrives must wait. How long it stays awake is that wait's
upper bound, and nobody has measured it.

**How:** the R4's firmware sets D3 low just before it sleeps (`WFI`) and high
just after it wakes (`src/sleep.rs`, `until_marked`). Each high stretch is one
time awake. The R3 times both edges of each stretch on its Timer1, at 0.5 µs a
count, and keeps a histogram. Meanwhile the PC sends the R4 random commands at
the simulator's 20–30 Hz, so the run loop does exactly the work it does on
the rig.

### Run it

Every command runs from the repo's root, in PowerShell.

**1. Give the R4 the network.** Its WiFi password lives in a file git
ignores. The bench joins the same network as the rig, so copy the
controller's file:

```pwsh
Copy-Item __live__/arduino/rust/crates/controller/config/wifi_credentials.toml __workbench__/uno_4r/crates/bench-firmware/config/wifi_credentials.toml
```

**2. Build the R4's firmware and copy it into an upload image.** These run
from the firmware's folder, since its `.cargo/config.toml` names the board's
target:

```pwsh
cd __workbench__/uno_4r/crates/bench-firmware
cargo build --profile firmware
cargo objcopy --profile firmware -- -O binary ../../../../target/bench-firmware.bin
cd ../../../..
```

**3. Flash the R4:**

```pwsh
arduino-cli upload -p COM3 --fqbn arduino:renesas_uno:unor4wifi --input-file target/bench-firmware.bin
```

It joins the WiFi as `uno-bench`. Its WiFi chip must run the interrupt bridge,
as the rig's does; if it doesn't, the R4 answers `STATUS` with
`stopped wifi: this bridge has no push mode`.

**4. Compile and flash the R3's sketch.** This replaces whichever HAL
validator part the R3 holds; step 6 puts it back.

```pwsh
arduino-cli compile --fqbn arduino:avr:uno --build-path target/workbench-awake-timer __workbench__/uno_r3/awake_timer
arduino-cli upload -p COM4 --fqbn arduino:avr:uno --input-file target/workbench-awake-timer/awake_timer.ino.hex --verify
```

**5. Run the test.** The values are in `pc/crates/bench-host/bench-host.toml`:
3,000 datagrams by default, about 2 minutes.

```pwsh
cargo run --release -p bench-host -- awake __workbench__/pc/crates/bench-host/bench-host.toml
```

It prints the summary and writes the run into
`pc/crates/bench-host/runs/awake_<seconds>/`, which git ignores:
- `settings.toml`, the settings the run used;
- `datagrams.csv`, every datagram sent, with its time and rate;
- `report.txt`, the R3's report as it sent it;
- `summary.txt`.

**6. Put the HAL's validator back on the R3,** as `Validation.md`
(*All-in-one validation*) says, when the bench is wanted for the HAL again.

### Reading the summary

- **awake stretches timed:** how many times the R4 woke during the
  measurement. About 500 a second come from the tick, plus one per datagram.
- **awake … % of the time:** the stretches' total over the run's length.
- **shortest, mean, longest:** each stretch from wake to sleep, µs.
- **N% of stretches at most X µs:** X is the upper edge of the 2 µs bin that
  holds the N-th percentile.
- **beyond the histogram:** stretches longer than the histogram's range
  (`bins × bin_us`, 500 µs by default); the longest is still given.
- **edges missed:** times the R3 saw the same level twice in a row: a
  stretch too short for it to see both edges. These should be 0.

**How far to trust it:** each edge is timed when the R3's interrupt runs, a
few µs after the edge. Both edges of a stretch wait about the same, so the
delay mostly cancels. What is left is up to one Timer1 overflow handler, a
few µs, on the rare edge that arrives while it runs.

**How it differs from the rig:** the bench R4 reads no limit switches (the
rig reads two pins first on every wake), its stop points are too far away to
reach, and its loop never stops at rest. Each difference changes a stretch
by a few clock cycles at most.

## The WiFi test: how long a datagram takes to reach the R4

**What it answers:** gap 7 of `real-timings.md`, the WiFi hop: from a
datagram leaving the sender to the board's processor having it whole. The
PC stands in for the Orin as the sender.

**How:** the hop starts on the PC and ends on the R4, and the two keep
separate clocks. So the R4 stamps each datagram on its own clock, GPT320, a
free-running 48 MHz timer (`src/clock.rs`), and the PC carries those stamps
into its own clock.

1. **Clock syncs over USB.** The PC stamps a `SYNC` line as it writes it
   (t0). The R4 stamps when it has the line whole (t1) and just before it
   answers (t2). The PC stamps when it has the answer whole (t3). If the line
   takes the same time each way, the R4's (t1 + t2) ÷ 2 happened at the PC's
   (t0 + t3) ÷ 2. This is NTP's four-stamp exchange. Request and answer are
   the same length, 32 bytes, so each takes the same time on the line.
2. **A straight line through the best syncs.** The syncs with the shortest
   round trips waited least on the way, so their midpoints are the truest.
   The PC keeps the shortest 20% in each tenth of the run and fits a
   straight line from the R4's count to the PC's time, which also measures
   how fast the R4's clock really runs.
3. **The datagrams.** Each cycle is one sync, then one random 3-byte command
   over WiFi, stamped by the PC just before it is sent. Then the PC waits for
   the R4's report of its stamp, so the R4 is idle whenever anything reaches
   it. The hop is the R4's stamp, carried into the PC's time by the line,
   less the PC's send stamp.

### Run it

From the repo's root, in PowerShell. The R3 is not used.

**1.** If the WiFi firmware has no `config/wifi_credentials.toml` yet, copy
the controller's, as for the awake test. Then build, flash and run:

```pwsh
cd __workbench__/uno_4r/crates/wifi-firmware
cargo build --profile firmware
cargo objcopy --profile firmware -- -O binary ../../../../target/wifi-firmware.bin
cd ../../../..
arduino-cli upload -p COM3 --fqbn arduino:renesas_uno:unor4wifi --input-file target/wifi-firmware.bin
cargo run --release -p wifi-host -- __workbench__/pc/crates/wifi-host/wifi-host.toml
```

The values are in `pc/crates/wifi-host/wifi-host.toml`: 3,000 cycles by
default, about 3 minutes. Each run's folder,
`pc/crates/wifi-host/runs/wifi_<seconds>/`, holds:
- `settings.toml`, the settings the run used;
- `syncs.csv`, every sync's four stamps, its round trip, and whether the
  fit kept it;
- `pings.csv`, every datagram's send stamp, the R4's stamp and its hop;
- `histogram.csv`, the hops in 250 µs bins;
- `summary.txt`;
- `strays.txt`, any line from the R4 that was neither.

### Reading the summary

- **The hop's mean, standard deviation, percentiles and extremes,** µs.
- **The USB round trip:** the sync's time on the line both ways.
- **Off the fitted line:** how far the kept syncs sit from the straight
  line. This is how far the R4's clock wanders from a steady rate, and
  roughly the error in each hop.
- **Half the shortest round trip:** the most a USB line slower one way than
  the other could shift every hop, all by the same amount.
- **The R4's clock, ppm:** how far its 48 MHz really is from 48 MHz.

## The apply test: how long the R4 takes to act on a command

**What it answers:** gap 8 of `real-timings.md`: from the board's processor
having a command whole to the step generator running at the command's new
rate. By design it is 2–4 ms: up to 2 ms waiting for the next tick, then
2 ms more, because a rate queued at one tick runs from the next.

**How:** both ends are timed by the R3 on its Timer1, at 62.5 ns a count.
- **The R4** (`uno_4r/crates/apply-firmware`) runs the controller's MOVING
  loop and flips D3 the moment it has a command whole (`src/run/runner.rs`,
  `look`).
- **The R3** (`uno_r3/apply_timer`) times that edge on INT1. Its Timer1
  latches every STEP falling edge on D8 in hardware (input capture), and a
  falling edge is where one STEP period ends and the next begins.
- **The PC** (`pc/crates/apply-host`) sends commands that alternate
  between 38,400 and 40,000 pulses/s. Each gives a whole number of the R4's
  48 MHz counts per pulse, 1,250 and 1,200, so each period is exact, and
  their difference is within one tick's ramp step, so the rate steps once
  and holds.
- **The delay:** after each mark, the R3 finds the first period that
  differs from the steady one by more than half the two rates' difference.
  The delay is the start of that period less the mark. It sends one line
  per command, which the PC collects.

### Run it

From the repo's root, in PowerShell. The firmware needs
`config/wifi_credentials.toml`, as the other tests do.

```pwsh
cd __workbench__/uno_4r/crates/apply-firmware
cargo build --profile firmware
cargo objcopy --profile firmware -- -O binary ../../../../target/apply-firmware.bin
cd ../../../..
arduino-cli upload -p COM3 --fqbn arduino:renesas_uno:unor4wifi --input-file target/apply-firmware.bin
arduino-cli compile --fqbn arduino:avr:uno --build-path target/workbench-apply-timer __workbench__/uno_r3/apply_timer
arduino-cli upload -p COM4 --fqbn arduino:avr:uno --input-file target/workbench-apply-timer/apply_timer.ino.hex --verify
cargo run --release -p apply-host -- __workbench__/pc/crates/apply-host/apply-host.toml
```

The values are in `pc/crates/apply-host/apply-host.toml`. Each run's
folder, `pc/crates/apply-host/runs/apply_<seconds>/`, holds:
- `settings.toml`;
- `results.csv`, every command's rate, the R3's verdict, the delay, and
  the periods before and after in µs;
- `histogram.csv`, the delays in 50 µs bins;
- `summary.txt`;
- `strays.txt`.

### Reading the summary

- **Applied:** the new rate began after the mark, and the delay is timed.
- **Changed before the mark:** the rate changed before the command
  arrived. That should never happen: if it does, something other than the
  command changed the rate.
- **Never changed:** no new rate within 10 ms, or the next command's mark
  came first. These are counted, not timed.
- **No line from the R3:** the PC waited `result_timeout_ms` and heard
  nothing.

## The camera test: how long the camera takes to deliver a frame

**What it answers:** gap 1 of `real-timings.md`: from the camera exposing a
frame to the frame being read, the moment the brain's log calls `arrived`.
GStreamer's stamp can't give it, because GStreamer stamps a frame when it
passes it on, not when the sensor exposed it.

**How:** the Orin flashes an LED in the camera's view and stamps each flash
and each frame's arrival on one clock, its own monotonic clock.
- **The LED:** wired from a GPIO pin on the Orin's 40-pin header, through a
  resistor, to ground (`orin/led-circuit.svg`). It sits in a corner of the
  picture, clear of the four markers.
- **Calibration:** the program (`orin/rust/crates/camera-timing`) opens the
  camera exactly as the brain does. It takes 30 frames with the LED off and
  30 with it on, and finds the LED as the patch of the picture that changed
  most. It refuses to go on if the patch brightens by less than
  `min_contrast`.
- **The flashes:** each flash is placed at a random moment after a frame's
  arrival and lasts `pulse_us`. Then the next 8 frames are checked for the
  patch lit.
- **Gap 1:** a frame that shows the flash was exposing during it, so its
  exposure started between 1 ms before the flash and the flash's end. Gap 1
  is that frame's arrival less its exposure's start, as a band 1.5 ms wide
  at a 500 µs flash, with the middle reported (`src/gap_one.rs`).

### Run it

On the Orin, with the brain and `calibrate_service.py` not using the camera.

**1. Get the folder onto the Orin and build it.** `__workbench__/orin/rust`
is a workspace of its own, pushed to the Orin and built there, as
`__live__/orin/rust` is (`__live__/orin/rust/DEPLOY.md`). From that folder
on the Orin:

```bash
cargo build --release
```

**2. Find the LED's GPIO line.** `gpioinfo` lists every line with its name.
Find the one for the header pin the LED is wired to, and put its name in
`camera-timing.toml`, `[led] line_name`.

```bash
gpioinfo
```

**3. Run it** from the same folder. Writing to a GPIO line may need `sudo`,
depending on who owns `/dev/gpiochip*` on the Orin.

```bash
./target/release/camera-timing camera-timing.toml
```

10,000 flashes take about 25 minutes at 60 frames/s: each uses 9 frames,
the one it follows and the 8 checked after it. Each
run's folder, `runs/camera_<seconds>/`, holds:
- `settings.toml`;
- `flashes.csv`, every flash's stamps, the frame that showed it, and gap 1's
  band and middle;
- `frames.csv`, every frame's arrival, GStreamer stamp and patch level;
- `summary.txt`.

## The stage test: how long each stage of the brain takes

**What it answers:** gaps 2 to 6 of `real-timings.md`, one by one: the
brain's share of the loop, from a frame arriving to its command leaving. The
brain's own log stamps only those two ends (`arrived` and `sent`), so the
five together are measured, about 23 ms, but not how they split.

**How:** the program (`orin/rust/crates/stage-timing`) runs the brain's loop
as the brain does it: the same camera settings, the same Python worker,
unchanged (`__live__/orin/python/src/brain_worker.py`, started from the
brain's own folder on the Orin), the same matrix and the same policy. It
stamps the end of every stage on the Orin's monotonic clock:

| Stamp | When | Gap it ends |
|---|---|---|
| `arrived` | the reader has the whole frame, as the brain's log | |
| `frame_sent` | the frame's last byte is in the worker's pipe | 2 |
| `boxes` | the boxes are back from the worker | 3 |
| `inputs` | metres, the pool and the 36 inputs are built | 4 |
| `answered` | a is back from the worker | 5 |
| `sent` | the datagram is handed to the system, as the brain's log | 6 |

For every frame the five gaps add up to `sent` less `arrived`, so one run
gives all five for the same frames.

Where the edges fall, since only the brain's side is stamped:
- **Gap 2 to gap 3:** the pipe holds up to 64 KiB, 65,536 bytes, Linux's
  default, and the frame is 728 × 544 = 396,032 bytes. So when the brain's
  write returns, the worker has read all but at most the last 64 KiB of it. Reading that last part is counted
  in gap 3.
- **Gap 3** also holds the worker's JPEG encode and decode before detection
  (`brain_worker.py`, `to_jpeg`) and its 80-byte answer back.
- **Gap 5** holds the 36 inputs' and a's trips through the pipes as well as
  the policy itself.

**The one change from the brain:** the datagrams go to `[send]`'s address,
the workbench PC's discard port, not the rig's Arduino, so the cart never
moves. The brain's log line and its look for the board's replies come after
`sent`, so they are outside the five gaps; they are left out.

### Run it

On the Orin, with no run going in the brain and `calibrate_service.py` not
using the camera. The brain may stay idle: its worker holds the TensorRT
engine in memory, and this program starts a second worker of its own.

**1. Get the folder onto the Orin and build it**, as for the camera test.
From `__workbench__/orin/rust` on the Orin:

```bash
cargo build --release
```

**2. Check it end to end** with 200 frames, about 10 s:

```bash
./target/release/stage-timing stage-timing-short.toml
```

**3. The full run**, 3,000 frames, about 100 s:

```bash
./target/release/stage-timing stage-timing.toml
```

Each run's folder, `runs/stage_<seconds>/`, holds:
- `settings.toml`;
- `frames.csv`, every frame's six stamps, the markers found, a and c, and
  its five gaps and their total;
- `summary.txt`, each gap's count, mean, standard deviation, extremes and
  percentiles, the five together, and the cadence, from one command sent to
  the next (gap 9).
