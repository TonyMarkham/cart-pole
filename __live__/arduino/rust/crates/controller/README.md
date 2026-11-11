# controller — the rig's Arduino, run by the Orin's commands

The trained policy can only balance the real pole if the rig's Arduino turns
each of the Orin's commands into motor pulses the way training did, keeps the
cart out of the ends of the rail, and tells the Orin when something is wrong.
`controller` is that firmware, for the board on the rig, an Arduino UNO R4
WiFi. It is built one slice at a time.

**What it does today:** at every start it claims the motor driver's pins and
both limit switches, opens the USB serial line, sends `READY` down it, and
sleeps. The one thing it answers is `STATUS` over USB, with its state:
`OK STATUS uncalibrated`. The cart doesn't move, and nothing arrives over the
WiFi. If claiming a part fails, it stops there, and each `STATUS` is answered
`OK STATUS stopped <part>: <error>` instead.

**Uploading it replaces whatever firmware was on the board.** The board holds
one firmware at a time.

## Settings

[config/](config/) holds the values someone might change between builds, one
file per part, each value explained where it is set. Among them:

- **`dir_positive`** (`stepper.toml`): the level on the DIR pin, D5, that
  moves the cart the positive way, towards the motor's end. Not yet checked
  on the rig.
- **`pulse_low_ns`** (`stepper.toml`): how long each pulse to the motor
  driver lasts.
- **`baud`** (`usb.toml`): the USB serial line's speed. It must match `baud` in the
  manager's
  [arduino-manager.toml](../../../../rpi/rust/crates/arduino-manager/arduino-manager.toml).

The board has no file system, so `build.rs` compiles these values into the
firmware: a change takes effect at the next build and upload. A value the
board can't use (a level that isn't `high` or `low`, a pulse shorter than the
driver allows, a speed that resets the board) stops the build with a message
naming it.

## How it builds

It is a member of the repo's root workspace (`Cargo.toml` at the root), but
it runs only on the board, so:

- **The root's `default-members` leaves it out.** A plain `cargo build` at the
  root builds for the PC and would fail on this crate.
- **`.cargo/config.toml` here sets the board's target** (the kind of processor
  to compile for), so the commands below must run from this folder.
- **It uses the root's `firmware` profile,** the compiler settings for the
  board: full optimisation.

It needs Rust's `thumbv7em-none-eabihf` target, `cargo-binutils` (for
`cargo objcopy`), and the HAL (the library that drives the board's hardware)
in the sibling checkout `D:\git\unor4-hal`. Every command runs from this
folder:

```pwsh
cd __live__/arduino/rust/crates/controller
```

**Build it:**

```pwsh
cargo build --profile firmware
```

**Copy the build into an upload image.** `cargo build` writes an ELF file (a
program with its layout described); the upload takes the raw bytes of the
board's flash instead. This writes `target/controller.bin` at the repo's root:

```pwsh
cargo objcopy --profile firmware -- -O binary ../../../../../target/controller.bin
```

## How it reaches the board, through `arduino-manager`

The board sits beside the rig, plugged into the Pi by USB.
[arduino-manager](../../../../rpi/rust/crates/arduino-manager/README.md) runs on
the Pi, and its `POST /flash` takes the `.bin` from the PC and uploads it with
`arduino-cli`, so the PC needs no `arduino-cli` of its own. `memory.x` starts
the image at 0x4000, after the Arduino bootloader, which keeps uploads over USB
working.

**1. Check that the manager is running:** its page,
<http://192.168.0.247:8080/>, says `Arduino manager`, and its serial line says
`open` while the board is plugged in. If it isn't running, its README's
*Deployment* starts it.

**2. Post the image** from this folder:

```pwsh
curl.exe --fail-with-body --data-binary "@../../../../../target/controller.bin" -H "Content-Type: application/octet-stream" http://192.168.0.247:8080/flash
```

The answer is `arduino-cli`'s output, which the manager's log keeps too. While
another action is running, the manager refuses the flash.

**3. Check that it runs,** as the next section says.

**4. Unplug the USB cable from the Pi** before the cart moves. With a cable in,
both limit switches have been seen to read open (the manager's README), though
not every time.

**The WiFi chip's bridge is separate,** and an upload doesn't change it.

## Checking that it runs

After an upload you want to know the board is running the new firmware. You
find out by asking it: the manager sends the board `STATUS`, and a running
board answers with its state. Everything below is typed on the Pi.

**1. Log in to the Pi** from the PC, in Git Bash or PowerShell. It asks for
the password unless the PC has a key set up:

```bash
ssh tony@192.168.0.247
```

You're in when the prompt reads `tony@robo-pi:~$`. (`exit` logs out.)

**2. Ask the board.** This does what the **Reset** button on the manager's
page does: it sends `STATUS`, without restarting the board:

```bash
curl -s -o /dev/null -w "reset %{http_code}\n" -X POST http://localhost:8080/reset
```

It prints `reset 303`, which means the manager took the request.

**3. Read the answer** at the end of the manager's log:

```bash
tail -n 5 "$(ls -t ~/arduino-manager/crates/arduino-manager/logs/log-*.txt | head -1)"
```

A working board answers within milliseconds. This is the run of 2026-11-05;
`>` is to the board, `<` is from it, `#` is the manager's own note:

```text
2026-11-05 21:36:07.974 > STATUS
2026-11-05 21:36:07.977 < OK STATUS uncalibrated
2026-11-05 21:36:07.977 # Ask the board: the board answered: uncalibrated
```

**What the answer means:**

| The log shows | Meaning |
|---|---|
| `< OK STATUS uncalibrated` | It runs. |
| `< OK STATUS stopped <part>: <error>` | It stopped at the start: it couldn't claim `<part>`, for the reason `<error>`. |
| no `<` line, then after 60 s `# Ask the board: stopped: no reply to STATUS in time` | It isn't running the firmware. |

**Why not just watch for `READY`?** The board sends `READY` as it starts, but
it almost never arrives. An upload changes the USB line's speed, and after
every speed change the board's USB bridge throws away what the board has sent
(the manager's `station.rs`, `hard_reset`).

**Don't use the manager's HARD RESET ARDUINO for this check yet.** On
2026-11-05 an earlier build never answered `STATUS` after one, and why is not
known.
