# calibration — the rig's Arduino, set up for real operation

The robot has to know where its cart is, and the stepper reports nothing,
so a calibration has to find that out before every run. The older `calibrate`
firmware does home the cart, but it was built to measure how hard the motor can
be driven before the belt skips teeth. This crate is the firmware for the
calibration the robot actually operates with.

**Today it only reports its pins.** It joins the WiFi, sleeps, and when a
message asks, answers with the level of every pin the rig uses, for the
[page on the Orin's webserver](../../../../orin/rust/README.md).
It moves nothing: it sends no pulses, keeps the driver's three outputs
high, and acts on no message. The homing moves are not in it yet, so while
this firmware is on the board the older `calibrate` firmware's page and
buttons are gone; upload `calibrate` again to get them back.

## What it answers

The stepper reports nothing, and the switches and driver are wired by
hand, so the page needs the pins' levels to show whether the wiring is
right. The board reports levels only (1 = high, 0 = low), and the page
holds what they mean, which keeps the board simple. A **datagram** is one
UDP message, sent over the network with no connection set up and no
guarantee it arrives:

```text
datagram "pins"  ->  datagram {"d2":0,"d3":0,"d4":1,"d5":1,"d6":1}
```

Any other datagram is ignored. D2 and D3 are inputs, measured on the pin;
D4, D5 and D6 are outputs, reported as the level the firmware sets.
[control-design.md](../../../../../control-design.md) has the wiring.

**It never polls for a message.** Polling means asking over and over
whether anything has arrived, as the older `calibrate` firmware does. This
board's WiFi chip runs the user's
[interrupt bridge](https://github.com/TonyMarkham/unor4-wifi-interrupt-bridge)
(the WiFi chip's own firmware), whose **push mode** sends each datagram to
the processor unasked. Each byte raises an **interrupt**, a signal that
ends the processor's sleep, so it sleeps until there is something to do.
Push mode covers UDP only, which is why the message is a datagram and not
a web request. `src/wifi.rs` has the details and sources.

**It needs that bridge on the board.** With Arduino's stock bridge the
join succeeds but push mode is refused, and the board prints that on its
USB serial port (115,200 baud) and answers nothing; it does not fall back
to polling. The serial port also shows the bridge's version (0.6.102 for
the latest fork, 0.6.0 for stock) and the board's address, or why the join
failed. A failed join is tried again only by resetting the board.

## Settings

[calibration.toml](calibration.toml) holds the serial speed, the hostname
and the UDP port. The board has no file system, so `build.rs` compiles them
in: a change takes effect at the next build and upload. The network's name
and password are in `wifi.toml`, which git ignores;
[wifi.example.toml](wifi.example.toml) is its template.

## How it builds

It is a member of the repo's root workspace (`Cargo.toml` at the root),
but it runs only on the board, so:

- **The root's `default-members` leaves it out.** A plain `cargo build` at
  the root builds for the PC and would fail on this crate.
- **`.cargo/config.toml` here sets the board's target** (the kind of
  processor to compile for), so the commands below must run from this
  folder.
- **It uses the root's `firmware` profile,** the compiler settings for the
  board: full optimisation, because a slow pulse loop slows the cart.
- **Its dependencies' versions are in the root's `[workspace.dependencies]`.**
  Only the feature no other crate needs, `uart-irq`, is set here.

It needs Rust's `thumbv7em-none-eabihf` target, `cargo-binutils` (for
`cargo objcopy`), `arduino-cli` with the `arduino:renesas_uno` core, and
the HAL (the library that drives the board's hardware) in the sibling
checkout `D:\git\unor4-hal`. Every command runs from this folder:

```pwsh
cd __live__/arduino/rust/crates/calibration
```

**Create `wifi.toml`** if this folder has none, then put the network's `ssid`
and `password` in it. The build stops with a message while it is missing:

```pwsh
if (-not (Test-Path wifi.toml)) { Copy-Item wifi.example.toml wifi.toml }
```

**A replaced `wifi.toml` can be missed by the build.** `Copy-Item` keeps the
file's old date, so if this folder has been built before, Cargo sees nothing
new and keeps the old network in the image. Give the file today's date, then
build:

```pwsh
(Get-Item wifi.toml).LastWriteTime = Get-Date
```

**Build it:**

```pwsh
cargo build --profile firmware
```

**Copy the build into an upload image.** `cargo build` writes an ELF file;
`arduino-cli upload` takes a raw binary:

```pwsh
cargo objcopy --profile firmware -- -O binary ../../../../../target/calibration.bin
```

## How it reaches the board

`memory.x` starts the image at 0x4000, after the Arduino bootloader, which
keeps uploads over USB working. There are two ways to upload it.

**From the desk, through the Pi,** when the board is plugged into the Pi and
[arduino-manager](../../../../rpi/rust/crates/arduino-manager/README.md) is running
there. This is how it reached the rig on 2026-10-31, and `arduino-cli` isn't
needed on the PC. From this folder:

```pwsh
curl.exe --fail-with-body --data-binary "@../../../../../target/calibration.bin" -H "Content-Type: application/octet-stream" http://192.168.0.247:8080/flash
```

**Directly,** when the board is plugged into this PC, with `$PORT` set to the
board's port (`arduino-cli board list` shows it):

```pwsh
arduino-cli upload -p $PORT --fqbn arduino:renesas_uno:unor4wifi --input-file ../../../../../target/calibration.bin
```

**The WiFi chip's bridge is separate,** and an upload doesn't change it. The
fork's `docs/flash-interrupt-procedure.md` covers flashing it.

## How it runs

Nothing starts it. The board runs the firmware as soon as the upload ends, and
at every power-up after that, from the barrel jack. Unplug any USB cable from
the Pi once the upload is done: with one in, both switches have been seen to
read open (the manager's README), though not every time.

**To check it,** read the manager's page, <http://192.168.0.247:8080/>, while
the cable is in. A good start ends with these two lines, though the first few
seconds' lines can be missed while the Pi reopens the port after a flash:

```text
wifi: bridge 0.6.102; push mode on
wifi: joined; listening at 192.168.0.220 UDP port 4210
```

Then open the [pin page](../../../../orin/rust/README.md), which shows
whether the board answers. A browser pointed at the board's own address gets
nothing, because this firmware has no web page.
