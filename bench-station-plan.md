# Bench station — a Raspberry Pi beside the rig

The rig's Arduino sits in another room, and today it does everything
itself: it makes the pulses, runs the jump-limit experiment, serves the
control page over WiFi, and prints the only full record of a run to a
USB serial port nobody is watching. This plan moves everything that does
not need exact timing onto a Raspberry Pi wired to the Arduino by USB,
so the Arduino does only real-time work, and the rig can be run, logged,
rebuilt and reflashed from the desk.

Status: an idea (user, 2026-10-27), not designed in detail and not
built. Written down after the first jump-limit run so it is not lost.

## Terms

- **The Arduino:** the rig's UNO R4 WiFi, running `calibrate`
  (`calibrate/README.md`). Its processor is the RA4M1; its WiFi and USB
  go through an ESP32-S3 running the bridge firmware.
- **The bridge:** that ESP32-S3's firmware. The user's fork,
  `D:\git\unor4-wifi-interrupt-bridge` (branch `feature/interrupt`,
  release `v0.6.0-interrupt.2`), adds a UDP push mode to Arduino's
  0.6.0. The rig's board has the fork (user, 2026-10-31); it reported
  0.6.102, this release, the same day.
- **The Pi:** a Raspberry Pi beside the rig, on the home network. Which
  one, and its OS, is open (*Open questions*); the Pi 5 the Orin
  replaced (`AGENTS.md`) may serve.
- **The serial line:** the Arduino's USB port. The bridge passes it
  through to the RA4M1's SCI9, the line `calibrate` prints its log on.

## What it fixes

Each is a limit hit on 2026-10-27:

1. **STOP cannot act during a fast move.** The Arduino checks for STOP
   by pausing between pulses to ask the bridge over AT commands, which
   takes milliseconds, so the fast ramp to the centre and a trial's
   jumps are never checked (`calibrate/README.md`, *What STOP cannot
   stop*).
2. **The full record has nowhere to go.** A jump-limit run is about
   350 trials, about 42 KB of trial lines; the Arduino has 32 KB of RAM
   and keeps only a summary and the first 2 KB of the log. The rest
   reaches the serial port only, and is lost unless something records
   it for the whole few hours.
3. **Results die with a reset.** Everything the board found is in RAM.
4. **Flashing needs the board on this PC.** The board is in another
   room.
5. **The page costs the motion.** Answering a request pauses the pulses
   (a hitch, `calibrate/README.md`, *While the cart moves*).

## The split

| The Arduino: real-time only | The Pi: everything else |
|---|---|
| making and counting every pulse; ramps; a trial's jumps on the cycle counter | the web page and its buttons |
| watching the limit switches | running the experiment: the noise floor, which j next, pass or fail, `m` |
| executing commands: home, move, run a trial plan, enable, disable, STOP | logging every line to a file, one per power-up, kept across resets |
| reporting: switch counts, lost steps, halts | building nothing; flashing the `.bin` the PC built |

The Arduino firmware gets smaller: the WiFi join, the page, the AT
client and the experiment's bookkeeping leave it.

## The link: USB, not D0/D1

- **USB is one cable** for commands, the log and flashing, with no extra
  wiring.
- **Opening it does not reset the board** at 115,200 baud. The bridge
  resets the RA4M1 only when the host sets 1200 baud (into the
  bootloader, for an upload) or 2400 (`UNOR4USBBridge.ino`,
  `usbEventCallback`); any other rate only changes the speed.
- **D0/D1 would need level shifting both ways.** The UNO R4's pins are
  5 V and the Pi's are 3.3 V and not 5 V tolerant: the Arduino's TX
  needs a divider or shifter into the Pi, and the Pi's 3.3 V high may
  not read as high on a 5 V input.
- **Power with both connected is safe.** VIN and USB each reach the
  board through a Schottky diode, so the 24 V supply's regulator carries
  the board and nothing flows back into the Pi (Arduino UNO R4 WiFi
  datasheet ABX00087, *Power Options*). USB joins the Pi's ground to the
  rig's, which is already the supply's negative. With USB in, the rocker
  no longer switches the board off.

## STOP everywhere

Commands arrive on the RA4M1's serial port in the background, through
`unor4-hal`'s interrupt-driven UART (`crates/unor4-hal/src/uart/uart_irq.rs`),
which buffers each byte as it lands. The pulse loop only checks a flag
the receive handler sets, which costs nothing, so STOP works during
every move, the fast ramp and a trial included. Whether a STOP during a
trial should stop at once (itself a jump, which can make the motor slip
and spoils the trial anyway) or ramp down is a choice to make.

## A command protocol (a sketch, not decided)

Text lines, one command or reply per line, so the log stays readable:

| Pi → Arduino | Does |
|---|---|
| `HOME` | the either-switch homing; replies with both ends' pins and the span |
| `MOVE <pulses> <rate>` | a slow move, `+` or `−` |
| `CENTRE` | the ramped move to half the span |
| `TRIAL <segments>` | plays one trial plan (holds and ramps, as `calibrate/src/profile.rs` defines them) |
| `SEEK REF` | the slow return to the reference switch; replies with the count |
| `ENABLE`, `DISABLE`, `STOP`, `STATUS` | as named |

| Arduino → Pi | Means |
|---|---|
| `OK <command> …` | finished, with its numbers |
| `SWITCH <pin> <count>` | a switch tripped |
| `HALT <reason> <count>` | stopped short, and why (`calibrate/src/halt.rs`'s reasons) |
| `LOG …` | a line for the file |

With the experiment's loop on the Pi, a run survives an Arduino reset:
the Pi re-homes and carries on from the last finished trial.

## The Pi's service

One Rust service (axum, as discussed), on the home network only:

- **the page and buttons**, sending commands down the serial line;
- **the experiment**, from `calibrate.toml`'s `[experiment]` values
  (moved to the Pi's own settings);
- **the log**: every line from the serial line to a file, one per
  power-up, and each trial's record as one line of a results file
  (kind, j, c_a, c_b, Δt, the switch count, the lost pulses, pass or
  fail: the brief's section 6);
- **`POST /flash`**: takes the `.bin` the PC built and runs
  `arduino-cli upload` on the board's port. Flashing resets the board,
  so the service saves the log first. Anything on the network could
  reflash the rig, so it must not be reachable from outside.

The PC side: `cargo build`, `cargo objcopy`, then one command that posts
the `.bin` to the Pi.

## What stays as it is

- **The training rig's link.** On the real robot the Orin sends its
  commands to the Arduino over WiFi (`AGENTS.md`, *The real robot*).
  This plan is for the bench tools (homing, the experiment, flashing),
  not the training firmware, unless that is decided otherwise.
- **The circuit.** `control-design.md` is unchanged: the Pi only adds a
  USB cable.

## The alternative kept in view

The bridge fork's UDP push mode could fix STOP and the record without a
Pi: the PC (or a server anywhere) sends STOP as a UDP datagram the
bridge pushes to the RA4M1 unasked, and the Arduino sends each trial's
record out as a datagram. It keeps WiFi in the Arduino's firmware, and a
browser cannot send UDP, so a server still sits between the page and
the board. The Pi plan above removes WiFi from the Arduino instead.

## Open questions

- **Which Pi, and its OS.**
- **`arduino-cli` on the Pi:** it runs on 64-bit Pi OS; whether the UNO
  R4's upload tool in the `arduino:renesas_uno` core has a 64-bit ARM
  build is not checked.
- **STOP during a trial:** stop at once, or ramp down.
- **Where the experiment's settings live** once the loop moves to the
  Pi.
