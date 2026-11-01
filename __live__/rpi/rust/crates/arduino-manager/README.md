# arduino-manager — the service beside the rig's Arduino

The rig's Arduino sits in another room, and its only full record of what
happened goes to a USB port nobody is watching. `arduino-manager` is a
service for a computer plugged into that Arduino by USB, so the board can
be logged and reflashed from the desk.

It is the older `bench-station` without the jump-limit experiment, which
measures the motor's limit and has no part in operating the robot. What it
keeps:

- **the page,** which shows the serial line and the log and links to the
  board's own page. Like the bench station's, it has no buttons. That link
  and the sentence beside it are true only while `calibrate` is on the board;
  the `calibration` firmware has no page, so with it the link goes nowhere.
- **the log:** every line that crosses the serial line goes to a file,
  timestamped, with a new file each time the line opens.
- **flashing:** `POST /flash` takes the `.bin` the PC built and uploads it
  with `arduino-cli`.
- **the command routes:** each of `/calibrate`, `/centre`, `/jog-minus`,
  `/jog-plus`, `/enable`, `/disable`, `/status` and `/stop` sends its
  command down the serial line (*The protocol*).

**What it still needs:** Arduino firmware that speaks *The protocol*.
Neither `calibrate` nor the `calibration` firmware does yet,
so the command routes get no replies. The page, the log and flashing work
without it.

**Keep the USB cable out while the rig runs.** With a computer's USB cable
connected, both limit-switch inputs (D2, D3) have read open, so homing
cannot work (found on 2026-10-27; the shared ground is the suspect,
unproven). Plug it in to flash, then unplug it.

## Settings

[arduino-manager.toml](arduino-manager.toml) holds every value someone
might change between runs, each explained where it is set: the page's
address, the serial port and its baud, `arduino-cli` and the board's FQBN,
and the jog. It is read at start-up, so restart the service after a change.

## The protocol

Each command or reply is one text line ending in `\n`, at 115,200 baud.
Rates are in pulses/s and counts in pulses. `+` is Plus, the direction of
the homing's second seek, and the count is 0 at the first end.

| Manager → Arduino | Does | Arduino's reply when it finishes |
|---|---|---|
| `HOME` | the either-switch homing, then the ramped move to the centre | `OK HOME <first pin> <second pin> <span> <position>` |
| `MOVE <±pulses> <rate>` | a slow move, stopping early at either switch | `OK MOVE <position>` |
| `CENTRE` | the ramped move to half the span | `OK CENTRE <position>` |
| `ENABLE`, `DISABLE` | the driver on or off | `OK ENABLE`, `OK DISABLE` |
| `STOP` | stops whatever is moving | the running command's `HALT stopped <count>`, then `OK STOP` |
| `STATUS` | asks for the board's state | `OK STATUS <anything>` |

| Arduino → manager, unasked | Means |
|---|---|
| `SWITCH <pin> <count>` | a switch tripped during a `MOVE`; the `OK MOVE` follows |
| `HALT <reason> <count>` | the running command stopped short; no `OK` follows |
| `READY` | the firmware has just started: the board was reset, and its homing is gone |
| `LOG <text>`, or anything else | kept in the log only |

The `HALT` reasons are listed in [src/halt.rs](src/halt.rs), each in words.

## Build and run

`__live__/rpi/rust` is the Pi's workspace, with its `Cargo.toml` holding the
versions, and it's separate from the repo's root workspace. That folder is all a
computer needs to build this, with no clone of the repo and no `unor4-hal`. From
that folder:

```bash
cargo build --release
```

Run it with its settings file (Ctrl+C stops it):

```bash
./target/release/arduino-manager crates/arduino-manager/arduino-manager.toml
```

Open `http://<the computer's address>:8080/`. The service opens the serial
port when the board appears and reopens it after a replug, so the order of
plugging in and starting doesn't matter. On Linux, your user must be in the
`dialout` group to open `/dev/ttyACM0`.

Logs go to `logs/` beside the settings file, as `log-<date>_<time>.txt`. In
a log line, `<` is from the board, `>` is to it, and `#` is the manager's
own note.

## Deployment

It runs on the rig's Pi, in `~/arduino-manager`, with its logs in
`crates/arduino-manager/logs`. Run every step, in this order, each time you deploy.
The running service isn't disturbed until step 6, and stopping and starting it
doesn't reset the board.

**What the Pi needs,** all of which `robo-pi` already has: Rust from
[rustup](https://rustup.rs), `arduino-cli` with the board's core, and permission to
open the serial port (log out and back in after the last command):

```bash
arduino-cli core install arduino:renesas_uno
```

```bash
sudo usermod -aG dialout $USER
```

### On the PC, from `__live__/rpi/rust`, in Git Bash

**1. Check that it compiles.**

```bash
cargo clippy --workspace --all-targets
```

**2. Pack it.** Build output and logs stay behind. This writes
`target/arduino-manager.tgz` at the repo's root:

```bash
tar --exclude=target --exclude=logs --exclude=flash -czf ../../../target/arduino-manager.tgz .
```

**3. Move the package** to the Pi's home folder ([how](../../../../README.md)).

### On the Pi

**4. Back up what's there,** both the source and the running program. The date
and time go in the names, so an earlier backup is never overwritten:

```bash
cd ~ && tar --exclude=target -czf arduino-manager-backup-$(date +%Y%m%d-%H%M%S).tgz arduino-manager && cp arduino-manager/target/release/arduino-manager arduino-manager-program-backup-$(date +%Y%m%d-%H%M%S)
```

**5. Unpack and build.** Files of the same name are replaced and nothing is deleted,
so the logs stay. If the build fails, stop here and fix it:

```bash
mkdir -p ~/arduino-manager && tar -xzf ~/arduino-manager.tgz -C ~/arduino-manager && cd ~/arduino-manager && cargo build --release
```

**6. Stop the running service.** It only needs stopping if it's running. Started
as in step 7, `Ctrl+C` signals don't reach it, so use this:

```bash
pkill -x arduino-manager
```

`bench-station` uses the same port, 8080, and the same serial line, so if that's
the one running, stop it instead:

```bash
pkill -x bench-station
```

**7. Start it.** This keeps it running after the terminal closes, though not after
a reboot, and its messages go to `manager.out`:

```bash
cd ~/arduino-manager && nohup setsid ./target/release/arduino-manager crates/arduino-manager/arduino-manager.toml > manager.out 2>&1 < /dev/null &
```

**8. Check it** at <http://192.168.0.247:8080/>. The heading says `Arduino manager`,
and the serial line says `open` if the board is plugged in. This should refuse,
with `the request has no .bin in its body`, which proves the flash route is up
without touching the board:

```bash
curl -s -X POST http://192.168.0.247:8080/flash
```

### Going back

If the new version misbehaves, stop it (step 6), put the saved program back, and
start it (step 7). Replace `<date-time>` with the one in the file's name:

```bash
cp ~/arduino-manager-program-backup-<date-time> ~/arduino-manager/target/release/arduino-manager
```

To go back to `bench-station` instead, stop the manager, then:

```bash
cd ~/bench-station && nohup ./target/release/bench-station bench-station.toml > service.out 2>&1 &
```

## Flash from the PC

The PC builds the firmware as its README says. This one command, in
PowerShell from the firmware's folder, posts the `.bin` to the service, with
the address replaced by the manager's:

```pwsh
curl.exe --fail-with-body --data-binary "@../../../../../target/calibration.bin" -H "Content-Type: application/octet-stream" http://<address>:8080/flash
```

On this network the address is `192.168.0.247`. The answer is `arduino-cli`'s
output; the log keeps it too. While an action is running, a flash is refused. The
computer the service runs on needs `arduino-cli` with the `arduino:renesas_uno`
core installed; the PC doesn't. The board has to be plugged into that computer by
USB for the flash; the firmware's README says what to check afterwards.

**The router must not forward the page's port.** Anything that reaches
`POST /flash` can reflash the rig.
