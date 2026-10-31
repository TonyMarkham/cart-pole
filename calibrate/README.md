# calibrate — centre the cart on the rail

The stepper reports nothing about where the cart is, so the only way to
put the cart at the centre of the rail is to measure the rail in pulses.
`calibrate` is firmware for the rig's Arduino UNO R4 WiFi that does
that:

1. Slowly move the cart in one direction (Minus, DIR low) until
   **either** limit switch trips: the first end, pulse 0.
2. Reverse, and slowly move until a switch trips again, counting every
   pulse: the second end. The count is the span between the switches.
   The switch the cart starts this move on counts only once it has read
   released for `release_pulses`, so its contact bouncing as the cart
   leaves is not a trip.
3. Move fast, ramping up and down, half the span back: the centre.

Every move counts a switch as tripped only once it has read pressed
without a break for `trip_us` (1 ms). A spike on the switch wire, or
the contact chattering as the rig shakes, is far shorter, so it is not
taken for a trip; one halted the first jump-limit run 143 mm from the
switch. The wait is the same at every real trip, about 2 pulses at
`seek_speed`, so it cancels in the span and the noise floor.

Which switch is at which end, and which way DIR moves the cart, do not
matter: either switch is an end. The report names the pin that tripped
at each end, so the wiring can still be checked, and both ends tripping
the same pin halts as a wiring fault.

The centre is halfway between the two switches' trip points. Both are
reached moving outward at the same slow rate, so any lag in sensing a
trip is the same at both ends and cancels.

Before the cart moves, the board joins the WiFi network in `wifi.toml`
and serves a page with the controls, a big **STOP**, and everything the
board knows (*The page*). With `run_at_start` the board homes the cart
by itself at power-up; without it, it waits for the page's Calibrate
button. A failed join is reported on the serial port, and the program
goes on without the page.

The wiring is [control-design.md](../control-design.md): D2 and D3 the
switches, D4 STEP, D5 DIR, D6 ENABLE, D8 FAULT. The code is
[src/main.rs](src/main.rs) (the actions), [src/axis.rs](src/axis.rs)
(the pulses, the ramp and the switch checks),
[src/wifi.rs](src/wifi.rs) (the join and the requests) and
[src/page.rs](src/page.rs) (the page).

## What it reports

- **On the board's USB serial port** (115,200 baud): the WiFi join and
  the board's address, each step, and the span in pulses and mm. The
  first run on the DM542 (2026-10-27) measured 59,828 pulses, 897.4 mm.
- **Over WiFi:** the page (*The page*).
- **The LED (D13):** steady while the cart is homed; the motor stays
  energised and holds it. Fast flashing means a halt switched the
  driver off.
- **It halts, and switches the driver off,** when a seek runs for
  `seek_timeout_ms` (34 s) without any switch tripping, when both ends trip the same
  pin, when a switch trips during a fast move, or when D8 reads low (the
  DRV8825's FAULT input; the DM542 has no fault line, so D8 is
  unconnected and this should never happen). The cart is then free and
  must be homed again.
- **It stops and keeps holding** when STOP is pressed, when a switch is
  already pressed as the homing begins (jog the cart off it, then press
  Calibrate), and when Go to centre is pressed before a homing.

The serial port and the page say which. With `run_at_start`, every
power-up or reset homes the cart again: the WiFi join takes up to about
25 s, then it waits `start_ms` (3 s) before the cart moves.

## Settings

[calibrate.toml](calibrate.toml) holds every value someone might change
between runs: whether power-up homes the cart, `seek_speed` (the slow
seeks and jogs), `home_speed` (the fast move to the centre) and its
acceleration, how long a seek may run before giving up, the jog's length,
the switch release distance, how long a switch must read pressed to
count (`trip_us`), how often a slow move checks for STOP, the
pauses and the baud. The board has
no file system, so `build.rs` reads the file and compiles the values
in: **a change takes effect at the next build and upload.** The mm in
the report come from `[drive]` in
[trainer/settings.toml](../trainer/settings.toml).

Its `[wifi]` table sets the hostname the router lists (`cart-pole`) and
the page's TCP port (80). The network's name and password are in
`wifi.toml`, which git ignores so the password never reaches the
repository; [wifi.example.toml](wifi.example.toml) is its template. The
board's radio is 2.4 GHz only: a 5 GHz network is never found.

## Build and upload

It needs Rust's `thumbv7em-none-eabihf` target, `cargo-binutils` (for
`cargo objcopy`) and `arduino-cli` with the `arduino:renesas_uno` core.
Every command runs in PowerShell, from this folder.

**Go to the crate's folder**, from the repo's root. The build reads
`.cargo/config.toml` here, which sets the board's target, so it must run
from this folder:

```pwsh
cd calibrate
```

**Create `wifi.toml`** if this checkout has none (a fresh clone), then
put the network's `ssid` and `password` in it. The build stops with a
message while it is missing:

```pwsh
if (-not (Test-Path wifi.toml)) { Copy-Item wifi.example.toml wifi.toml }
```

**Find the connected Arduino.** The rig's board is the row whose FQBN
is `arduino:renesas_uno:unor4wifi`:

```pwsh
arduino-cli board list
```

**Set `$PORT`** to that board's port (for example `COM8`). This picks
the first UNO R4 WiFi in the list:

```pwsh
$PORT = (arduino-cli board list --format json | ConvertFrom-Json).detected_ports | Where-Object { $_.matching_boards.fqbn -contains 'arduino:renesas_uno:unor4wifi' } | Select-Object -First 1 -ExpandProperty port | Select-Object -ExpandProperty address
```

Or set it by hand, from the list:

```pwsh
$PORT = "COM8"
```

**Check `$PORT`.** It prints the port; nothing printed means no UNO R4
WiFi was found when it was set:

```pwsh
echo $PORT
```

**Make the release build.** `.cargo/config.toml` sets the board's
target, so plain `cargo` builds for it:

```pwsh
cargo build --release
```

**Copy the build into an upload image.** The build is an ELF file;
`arduino-cli upload` takes a raw binary, which `objcopy` writes:

```pwsh
cargo objcopy --release -- -O binary target/calibrate.bin
```

**Upload it to the board on `$PORT`:**

```pwsh
arduino-cli upload -p $PORT --fqbn arduino:renesas_uno:unor4wifi --input-file target/calibrate.bin
```

`arduino-cli upload` puts the board into its bootloader itself, by
opening the port at 1,200 baud (unor4-hal's AGENTS.md, *Test bench*).
If the upload cannot reach the board, double-tap its reset button (the
LED fading means the bootloader is waiting) and run the upload again;
in bootloader mode the board can show up on a different port, so run
`arduino-cli board list` and set `$PORT` again first.

With `run_at_start`, the cart starts moving once the WiFi join is done
and `start_ms` has passed, so keep the rail clear before you run it.

**Watch the report:**

```pwsh
arduino-cli monitor -p $PORT --config baudrate=115200
```

## The page

The board hosts one web page: the controls, a big **STOP**, and
everything it knows, so the rig can be run and read without a USB
cable.

**Open it in a browser:** `http://<address>/`. The serial port prints
the address after the join (`wifi: joined; the page is at
http://<address>:80/`); on this network it has been
`http://192.168.0.220/`. If your router resolves the hostname,
`http://cart-pole/` also works.

**The buttons.** Each one sends a POST, so loading or reloading the
page never moves the cart:

| Button | Does |
|---|---|
| **STOP** | Stops a seek or a jog within `stop_check_ms` (0.2 s). The driver keeps holding, so a homed cart stays homed. |
| Calibrate | Homes the cart and centres it (the sequence above). |
| Go to centre | Moves fast to half the span. Needs a finished calibration. |
| Jog minus, Jog plus | Moves `jog_pulses` (5 mm) at `seek_speed`, stopping early at either switch. Use it to move the cart off a switch. |
| Enable motor | Switches the driver on. |
| Disable motor | Switches the driver off: the cart is free and must be homed again. |
| Jump-limit experiment | Measures `m`, the jump limit training needs (*The jump-limit experiment*). Needs a calibration; takes hours. |

**What STOP cannot stop.** The board times the pulses itself, so it can
only look for STOP by pausing between pulses. The slow seeks and jogs
pause every `stop_check_ms` for about 6 ms, which is safe at the seek
rate. The fast ramp to the centre (about 1–2 s) cannot pause without
losing steps, so a STOP pressed during it takes effect when it ends.
The 24 V supply's switch is the stop that always works: the rocker
powers only the Arduino, and the DM542 holds the motor without it.

**While the cart moves, the browser waits.** The board holds its
answer to a button until the action has finished, so the browser shows
its loading spinner through the move and sends nothing else, and the
motion stays smooth. When the action ends, the board answers and the
browser shows the page with the result: after Calibrate, the span and
the pin at each end. The page you pressed the button on stays up while
it waits, and its STOP still works.

Reloading by hand during a move gets a short page with STOP alone.
Reading the reload and sending that page pauses the pulses for a moment,
which shows as a hitch, so leave the page alone until the move ends.
The other buttons are ignored until then.

**What the page shows:**

1. **The run's summary:** the pin that tripped at each end, the span,
   the position, and how the last action ended, with the reason and the
   pulse for a halt.
2. **Every drive pin's level**, read at the moment of the request.
3. **The settings this build was compiled with**, so you can tell which
   build is running.
4. **The serial-port log since power-up**: every line the USB serial
   port printed, from the WiFi join on.

**Or read it from PowerShell.** Set `$BOARD` to the address, then fetch
the page (it comes back as HTML):

```pwsh
$BOARD = "192.168.0.220"
```

```pwsh
curl.exe "http://$BOARD/"
```

**Or press a button from PowerShell,** for example STOP:

```pwsh
curl.exe -X POST "http://$BOARD/stop"
```

The paths are `/stop`, `/calibrate`, `/centre`, `/jog-minus`,
`/jog-plus`, `/enable` and `/disable`.

The pin lines, one per pin (1 = high, 0 = low):

```text
D2 switch       0  released
D3 switch       0  released
D4 STEP         1
D5 DIR          1  plus
D6 ENABLE       1  driver enabled
D8 FAULT        1  unconnected
```

Inputs (D2, D3, D8) are read from the pin; outputs (D4, D5, D6) are the
level the firmware drives. The outputs reach the driver through PNP
transistors, so 0 lights the driver's input: STEP rests at 1 and is 0
during a pulse, DIR 0 is Minus, and ENABLE 0 disables the driver. A
switch reads 1 when pressed or when its wire is open. D8 is unconnected
(the DM542 has no fault line), so it reads 1.

## The jump-limit experiment

Training needs one number it cannot get from the simulator: `m`, the
fastest the motor's pulse rate can change, in pulses/s², without the
motor slipping. The page's **Jump-limit experiment** button measures it,
following [jump-limit-experiment.md](../jump-limit-experiment.md), the
brief that defines `m` and the procedure.

**How a trial sees a slip.** The stepper reports nothing, so the limit
switch at the first end is the reference. Every pulse is counted; if the
motor kept up, a slow return to that switch trips it at the same count
as before. A count further off than the noise floor is that many lost
steps, and the trial failed.

**What it does, in order:**

1. **The noise floor:** `noise_repeats` (10) slow returns to the
   reference switch, each from `noise_away_pulses` (30 mm) away. The
   mean count is the reference; the spread, largest minus smallest, is
   the noise floor.
2. **The four kinds of trial**, each from `j_start` (620,000 pulses/s²)
   up, `j` raised by `j_growth_percent` (25%) after each pass:
   - **from rest:** 0 to `j` × Δt;
   - **at speed:** `trial_top` ÷ 2 to `trial_top` ÷ 2 + `j` × Δt;
   - **reversal:** −c to +c, with 2c = `j` × Δt;
   - **staircase:** from 0, a jump of `j` × Δt every Δt up to
     `trial_top`.

   Each trial: a slow move to the centre; reach `c_a` gently and hold it
   `steady_ms`; the jump; hold `c_b` `hold_ms` (0.1 s); stop gently; a
   slow return to the reference switch. `trials_per_j` (10) trials at
   each `j`. The first trial that loses steps ends the kind: its limit
   is the last `j` that passed. A kind that reaches the largest `j` it
   can make inside ±`trial_top` without a slip is "at least" that.
3. **`m`** is the smallest of the four limits, so it is safe for all of
   them. Write it into `trainer/settings.toml`, `[drive]
   jump_limit`.

**Timing.** A trial's pulses are timed on the processor's cycle counter
against the plan, so a jump lands where the plan puts it, within one
pulse period (21 µs at 48,000 pulses/s), and the loop's own work does
not slow the rate. The slow moves use `seek_speed` and the gentle
changes use `accel`.

**How long.** Each trial is about 30 s, nearly all of it the two slow
moves; a full run is a few hours. The page answers the button at once,
and the board checks for STOP during every slow move, so STOP ends the
run between trials. Reloading shows the progress: the noise floor, each
kind's limit so far, and the trial running. Each reload is a short pause
in a slow move, never in a jump. Every trial's line (kind, `j`, `c_a`,
`c_b`, Δt, the switch count, the lost steps, pass or fail) goes to the
serial port; the page's log holds only the first 2 KB of it.

**Before you run it:** calibrate first; set the supply, the DM542's
switches and the pole as training will have them; and make `conditions`
in `[experiment]` say so, because the board measures none of them and
the report quotes it. A trial whose plan would come within
`end_margin_pulses` (45 mm) of an end ends its kind with "no room".

## The crate

A workspace of its own, built only for the board: the repo's root
workspace builds for the PC. It uses the HAL from the sibling checkout
`D:\git\unor4-hal` (`crates/unor4-hal`). `memory.x` starts the image at
0x4000, after the Arduino bootloader, which keeps uploads over USB
working.
