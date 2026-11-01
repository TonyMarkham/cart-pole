# calibration — the rig's Arduino, set up for real operation

The robot has to know where its cart is, and the stepper reports nothing,
so a calibration has to find that out before every run. The older `calibrate`
firmware does home the cart, but it was built to measure how hard the motor can
be driven before the belt skips teeth. This crate is the firmware for the
calibration the robot actually operates with.

**Today it reports its pins and moves the cart to either end of the rail.** It
joins the WiFi and sleeps. When a message asks, it answers with the level of
every pin the rig uses, for the
[page on the Orin's webserver](../../../../orin/rust/README.md), or moves the
cart slowly to a limit switch or the rail's centre (*The moves*). While this firmware is on the board the older `calibrate`
firmware's page and buttons are gone; upload `calibrate` again to get them back.

## The moves

The camera calibration needs the cart standing at a known place, and the two
limit switches are the places the board can find by itself
([the plan](../../../../../calibrate-w-homography.md), *The calibration
sequence*). So the Orin asks for each move with one datagram:

```text
datagram "CalibrateIdlerTravelLimit"  ->  the cart seeks to the idler's end
datagram "CalibrateMotorTravelLimit"  ->  the cart seeks to the motor's end
```

The cart moves at `seek_speed`, 30 mm/s, until a switch trips, then stops and
the driver holds it. Only then does the board answer, so the wait can be as long
as `seek_timeout_ms`, 34 s. Whoever asks should wait a little longer than that.

Every answer has the same five keys, for example:

```text
{"status":200,"result":"tripped","message":"the cart stopped at that end's switch","pin":"d3","pulses":59828}
```

The outcomes of a seek:

```text
status  result         pin   the cart
200     tripped        d3    stopped at that end's switch
409     tripped        d2    stopped at the other end's switch
409     already_there  d3    did not move: that end's switch already reads pressed
409     refused        null  did not move: both switches read pressed
408     timed_out      null  stopped: no switch tripped in time
409     stopped        null  stopped: a Stop datagram arrived
```

- **`message`** says the same in words, for whoever is at the rig, so it gives
  numbers and never a setting's name. Every failure's message is in `AppError`
  ([src/error/mod.rs](src/error/mod.rs)), except the two that quote a setting, which
  `build.rs` writes from [calibration.toml](calibration.toml). A failure also goes to
  the USB serial port, with the place in the code it came from.

- **`status`** is 200 when the cart did what was asked and 4xx when it did not, with
  HTTP's meanings: 400 a bad request, 408 out of time, 409 not possible as things
  stand. The board speaks UDP, so this is a number in the answer, not an HTTP
  status. The Orin's webserver returns it as one, for its
  [calibration page](../../../../orin/rust/README.md).
- **`pin`** is the switch that stopped the cart, whichever it is, so a wrong
  guess about which end is which shows here.
- **`pulses`** is how far it went. From one switch to the other, that is the
  rail's span.
- **`refused`** is what a USB cable left in from the Pi can cause, as can a
  broken switch wire: both read the same as a pressed switch.
- **The board is busy during a move.** It listens only for `Stop`. Other datagrams
  that arrive meanwhile, `pins` included, are thrown away and never answered. A
  move ends at a switch, at the time limit, at a `Stop`, or when the rig's power
  is cut.

**To stop a move,** send one more datagram:

```text
datagram "Stop"  ->  ends whichever move is under way
```

The board does not answer the `Stop` itself. The move it ended answers whoever
asked for that move, `409`, `stopped`, with how far the cart got. The cart stops in
one jump and the driver holds it. A `Stop` with no move under way is ignored, like
any other datagram the board doesn't know. A datagram can be lost on the WiFi, so
send it more than once, and don't rely on it in place of the e-stop.

A stop loses no steps, so the board keeps its span, but it no longer counts the cart
as resting on a switch. `MoveToCentre` then answers `not_ready` until a seek has
tripped an end.

**The move to the centre** is one more datagram, with an optional speed:

```text
datagram "MoveToCentre"        ->  to the centre at seek_speed
datagram "MoveToCentre 5000"   ->  to the centre at 5,000 pulses/s (75 mm/s)
```

The centre is halfway between the two switches' trip points, so the board has to
measure that distance first. A switch counts only when the board saw it trip
during a move: one that merely reads pressed may be a broken wire, not the cart.
So the calibration is always the same, wherever the cart starts:

1. A seek that trips one end. If the cart starts on that end's switch the answer
   is `already_there` and nothing counts, so seek to the other end instead.
2. A seek that trips the other end. Its `pulses` is the span.

From a cart resting on a switch after a reset, that takes one seek off the switch
and one back: the first seek's start proves nothing. Until the span is measured,
`MoveToCentre` is answered `not_ready` and the cart stays put. The board forgets
the span at every reset, when a seek times out, and when a seek trips the wrong
switch. The cart then moves half the span from the switch it rests on, and the
board answers when it has stopped. At `seek_speed` from an end that takes about 15 s;
a slower speed takes longer, so whoever asks must wait for as long as it asked for.

```text
status  result     pin   the cart
200     centred    null  reached the centre
409     tripped    d3    stopped: a switch tripped on the way, so steps were lost
409     stopped    null  stopped: a Stop datagram arrived
409     not_ready  null  did not move: no span yet, or the cart is not on a switch
400     bad_speed  null  did not move: not a whole number from 1 to centre_speed_max
409     refused    null  did not move: both switches read pressed
```

Like the seeks it has no ramp: it starts and stops in one jump. That is why
`centre_speed_max` limits the speed.

**The ends are a first guess.** `[idler]` and `[motor]` in
[calibration.toml](calibration.toml) say which switch and direction each datagram
uses. Which end the idler is at, and which way DIR moves the cart, have not been
checked on the rig. Start the first run with the cart mid-rail, with no USB cable
in: a cart already resting on a switch, sent the wrong way, could not see that
switch again and would push on against the end of the rail until the time limit.

**To try one,** from the Orin, with the cart mid-rail and clear:

```bash
python3 -c "import socket; s=socket.socket(socket.AF_INET,socket.SOCK_DGRAM); s.settimeout(40); s.sendto(b'CalibrateIdlerTravelLimit',('192.168.0.220',4210)); print(s.recv(256).decode())"
```

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

Any datagram that is neither this nor one of the moves is ignored. D2 and D3 are inputs, measured on the pin;
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

[calibration.toml](calibration.toml) holds the serial speed, the slow seek's
values, each with where it comes from, the two ends of the rail, how long the
board waits on its WiFi chip, the hostname and the UDP port. The board has no file system, so `build.rs` compiles them
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
