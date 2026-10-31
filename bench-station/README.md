# bench-station — the Pi beside the rig

The rig's Arduino sits in another room. Today it makes the pulses, runs
the jump-limit experiment, serves the control page over WiFi, and prints
the only full record of a run to a USB port nobody is watching.
`bench-station` is the service for a Raspberry Pi plugged into that
Arduino by USB. It takes over everything that doesn't need exact timing,
so the rig can be run, logged and reflashed from the desk
([bench-station-plan.md](../bench-station-plan.md)).

The Arduino keeps the real-time work: making and counting pulses,
watching the limit switches, and carrying out one command at a time.
The Pi does the rest:

- **the page and its buttons**, which send commands down the serial line.
  STOP is sent at once, during any move.
- **the jump-limit experiment**: the noise floor, which `j` to try next,
  pass or fail, and `m`. The steps are calibrate's
  ([calibrate/README.md](../calibrate/README.md), *The jump-limit
  experiment*).
- **the log**: every line that crosses the serial line goes to a file,
  timestamped, with a new file each time the line opens. Each experiment
  run also writes a results file with one line per trial.
- **flashing**: `POST /flash` takes the `.bin` the PC built and uploads it
  with `arduino-cli`.

**What it still needs:** Arduino firmware that speaks *The protocol*
below. `calibrate` doesn't speak it yet: it does all of this itself, over
WiFi. Until that firmware exists, the page shows every line the board
prints and flashing works, but the buttons get no replies.

## Settings

[bench-station.toml](bench-station.toml) holds every value someone might
change between runs, each explained where it is set:

- the page's address;
- the serial port and its baud;
- `arduino-cli` and the board's FQBN;
- the slow move rate and the jog;
- the experiment's values, moved from `calibrate.toml`'s `[experiment]`.

It is read at start-up, so restart the service after a change. Two values
copy the firmware's own and must match it: `seek_speed` and `accel` in
`[rig]`. The Pi uses them to work out how far a trial carries the cart.

## The protocol

This is the line protocol the plan sketches (*A command protocol*), made
exact. Each command or reply is one text line ending in `\n`, at 115,200
baud. Rates are in pulses/s and counts in pulses. `+` is Plus, the
direction of the homing's second seek, and the count is 0 at the first
end.

| Pi → Arduino | Does | Arduino's reply when it finishes |
|---|---|---|
| `HOME` | the either-switch homing, then the ramped move to the centre | `OK HOME <first pin> <second pin> <span> <position>` |
| `MOVE <±pulses> <rate>` | a slow move, stopping early at either switch | `OK MOVE <position>` |
| `CENTRE` | the ramped move to half the span | `OK CENTRE <position>` |
| `TRIAL <segment> …` | plays one trial plan on the cycle counter | `OK TRIAL <position>` |
| `SEEK REF` | the slow return to the first end's switch | `OK SEEK <count it tripped at>` |
| `ENABLE`, `DISABLE` | the driver on or off | `OK ENABLE`, `OK DISABLE` |
| `STOP` | stops whatever is moving | the running command's `HALT stopped <count>`, then `OK STOP` |
| `STATUS` | asks for the board's state | `OK STATUS <anything>`, shown on the page as it came |

A `TRIAL` segment is `H:<rate>:<ms>` (hold `rate` for `ms`) or
`R:<from>:<to>` (ramp at the firmware's `accel`, never below its seek
rate except at an end that is 0). These are `calibrate/src/profile.rs`'s
pieces, at most 48 to a plan. The longest plan, a staircase, makes a line
of about 1 KB, which the firmware's receive buffer must hold.

| Arduino → Pi, unasked | Means |
|---|---|
| `SWITCH <pin> <count>` | a switch tripped during a `MOVE`; the `OK MOVE` follows |
| `HALT <reason> <count>` | the running command stopped short; no `OK` follows |
| `READY` | the firmware has just started: the board was reset, and its homing is gone |
| `LOG <text>`, or anything else | kept in the log only |

The `HALT` reasons are calibrate's (`calibrate/src/halt.rs`), in kebab
case: `driver-fault`, `no-switch`, `pressed-at-start`, `same-switch`,
`switch-on-the-way`, `wrong-reference`, `stopped`, `not-homed`. The first
five switch the driver off, so the cart must be homed again.

`READY` and the segment format are not in the plan's sketch. They are what
the Pi needs to resume after a reset and to send a plan.

## A run survives a reset

If the board resets (`READY`) or its port drops during an experiment, the
Pi waits for the port and homes the cart again. It then returns to the
reference switch and repeats the step that was cut short. A run can do
this `resumes` times (3). Each trial's loss is counted from the return
just before it, so re-homing doesn't change how a trial is judged.

## Build and run on the Pi

The Pi builds the crate itself: the folder is a workspace of its own, so
a copy of this folder is all it needs. On the Pi today the copy is
`/home/tony/bench-station`. In the Pi's shell, from the folder:

```bash
cargo build --release
```

To try it by hand, run it with its settings file (Ctrl+C stops it):

```bash
./target/release/bench-station bench-station.toml
```

Open `http://<Pi's address>:8080/` (on this network,
`http://192.168.0.247:8080/`). The service opens the serial port when the
board appears and reopens it after a replug, so the order of plugging in
and starting doesn't matter. Your user must be in the `dialout` group to
open `/dev/ttyACM0`.

## Run it as a service

Run by hand, the bench station stops when you close the shell or the Pi
reboots. Installed as a systemd service, it starts when the Pi boots and
restarts itself 5 s after a crash, so the page and the log are there
whenever the Pi is on. [bench-station.service](bench-station.service)
is the service's definition; its paths assume the crate is in
`/home/tony/bench-station` and runs as `tony`, so edit them first if
either is different.

Do this once, in the Pi's shell, from the crate's folder, after the
first `cargo build --release`.

**Stop any copy you started by hand.** Only one program can listen on
port 8080 and hold the serial port:

```bash
pkill -x bench-station
```

**Copy the definition to where systemd reads service files:**

```bash
sudo cp bench-station.service /etc/systemd/system/
```

**Make systemd read it:**

```bash
sudo systemctl daemon-reload
```

**Start it now, and at every boot:**

```bash
sudo systemctl enable --now bench-station
```

**Check it is running.** The output should say `active (running)`:

```bash
systemctl status bench-station
```

**After changing the code or `bench-station.toml`,** rebuild, then
restart the service so it runs the new build and reads the settings
again:

```bash
cargo build --release && sudo systemctl restart bench-station
```

**Watch what it prints.** The service's own output goes to systemd's
journal; the log files in `logs/` are written as before. Ctrl+C stops
watching, not the service:

```bash
journalctl -u bench-station -f
```

**Stop it, or stop it starting at boot:**

```bash
sudo systemctl stop bench-station
```

```bash
sudo systemctl disable bench-station
```

A service doesn't search your shell's `PATH`. If `arduino-cli` is
installed in your home folder (for example `~/.local/bin`), set
`arduino_cli` in `bench-station.toml` to its full path, for example
`/home/tony/.local/bin/arduino-cli`.

Logs and results go to `logs/` beside the settings file:
`log-<date>_<time>.txt` and `results-<date>_<time>.csv`. In a log line,
`<` is from the board, `>` is to it, and `#` is the Pi's own note.

## Flash from the PC

The PC builds the firmware as calibrate's README says (`cargo build
--release`, then `cargo objcopy --release -- -O binary
target/calibrate.bin`). This one command, in PowerShell from the
firmware's folder, posts the `.bin` to the Pi:

```pwsh
curl.exe --fail-with-body --data-binary "@target/calibrate.bin" -H "Content-Type: application/octet-stream" http://192.168.0.247:8080/flash
```

The answer is `arduino-cli`'s output; the log keeps it too. While an
action is running, a flash is refused. The Pi needs `arduino-cli` with
the `arduino:renesas_uno` core installed.

**The router must not forward the page's port.** Anything that reaches
`POST /flash` can reflash the rig.
