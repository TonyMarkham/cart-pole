# Real Pi Plan: `arduino-manager`'s settings options

The board's one saved setting, `runtime_end_margin`, lives in its data flash and
has to survive new firmware. A firmware upload must not quietly wipe or change
it. Someone at the desk also needs to see what the board holds and to put a
known value back. `arduino-manager`, the Pi's service plugged into the Arduino
by USB, already uploads the firmware (`POST /flash`). This plan gives it three
options around the settings.

Status: **plan** (2026-11-03), written from `real-loop-dia.md` alone, the
source of truth. Where this plan and the diagram disagree, the diagram is right.
Section names in *italics* are the diagram's.

## Done when

1. **Pull:** the manager shows the settings the Uno holds now, read from the
   board itself, not from a copy the manager kept.
2. **Firmware and settings:** after one upload, the board runs the new firmware
   and holds the settings given with it; a pull shows them.
3. **Firmware only:** after an upload, the board runs the new firmware and a
   pull shows the same settings as before it.

## Requirements (from the diagram)

- **P1: three options** (decided 2026-11-03, *The Uno's settings*):
  - pull the settings the Uno holds now;
  - upload firmware and settings;
  - upload firmware only, leaving the saved settings in flash alone.
- **P2: the settings are `runtime_end_margin` alone for now**, pulses, in the
  Uno's data flash (*The soft limit*; *The Uno's settings*).
- **P3: only with the rig idle** (follows). The manager reaches the board by
  USB, and the cable is kept out while the rig runs, so the three options are
  used with no run going.
- **P4: an upload leaves the data flash alone.** `unor4-hal` reports that an
  `arduino-cli upload` left the data flash intact on the UNO R4 WiFi
  (2026-10-21). Firmware-only upload rests on that; it is still to be tested on
  the rig (*The Uno's settings*).
- **P5: tunable values in a toml** (`AGENTS.md`): any new value goes in
  `arduino-manager.toml`, beside the ones the manager reads today.

## Not decided by the diagram

Each lands in the diagram first (`real-loop-dia.md`, *The Uno's settings*,
*Open*):

- **How the manager pulls and uploads the settings.** It speaks to the board
  over the USB serial line, one text line per command (the manager's README,
  *The protocol*), but the `calibration` firmware speaks no command protocol
  there today (its README, *What it still needs*). This blocks steps 1 and 2.
- **What the board uses when the data flash holds no setting.**

## Steps

1. **Pull** (P1, P2; blocked on *how the manager pulls*). A route that asks the
   board for its settings over the serial line and returns them; the page shows
   them. *Check:* Done-when 1, against the value the Orin's Arduino Settings
   page reads.
2. **Upload firmware and settings** (P1, P4; blocked on *how the manager
   uploads*). A route that takes the `.bin` and the settings, uploads the
   `.bin` as `POST /flash` does, then sends the settings once the board has
   started again. It is refused while another action runs, as a flash is today.
   *Check:* Done-when 2.
3. **Upload firmware only** (P1, P4). Today's `POST /flash`, named as this
   option. *Check:* Done-when 3, on the rig: pull, upload, pull again, the same
   value. That is the rig test *The Uno's settings* still lists.
4. **The README.** `__live__/rpi/rust/crates/arduino-manager/README.md` gains
   the three options, when to use each, and that the USB cable comes out again
   before a run, written for a layman.

Before the work is called done, `cargo clippy --all-targets --no-deps` is
clean, run from `__live__/rpi/rust`, the manager's own workspace.

## References

- `real-loop-dia.md`: the source of truth for this plan.
- `real-arduino-plan.md`: the firmware that holds the setting and answers the
  manager.
- `real-orin-plan.md`: the Arduino Settings page, the other way the setting is
  set.
- `__live__/rpi/rust/crates/arduino-manager/README.md`: the manager today.
