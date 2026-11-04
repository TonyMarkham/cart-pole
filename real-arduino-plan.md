# Real Arduino Plan: the board's firmware for a balance run

The policy's commands only balance the pole if the board turns each one into
pulses the way the synthetic Arduino did in training, and keeps the cart off the
limit switches while doing it. So the rig's Arduino (an UNO R4 WiFi driving the
DM542 stepper driver) needs a **Run**: it takes each 4-byte command from the
Orin, ramps towards it, counts every pulse, and slows the cart to rest before
either end of the rail. It also keeps one setting, `runtime_end_margin`, in its
data flash.

Status: **plan** (2026-11-03), written from `real-loop-dia.md` alone, the
source of truth. Where this plan and the diagram disagree, the diagram is right.
Section names in *italics* are the diagram's.

## Done when

1. **A command moves the cart as in training.** A 4-byte datagram of `+r`
   drives DIR for the positive direction and, after the ramp, STEP at `r`;
   `−r` the other DIR at the same rate; 0 brings the cart to rest. Anything
   beyond ±44,600 runs at 44,600.
2. **The ramp holds the ceiling.** Each tick moves the sent rate towards the
   command by at most m × the tick, rounded down (m = 1,210,937 pulses/s²).
3. **Every pulse is counted**, so the cart's position in pulses is known
   between the two cached trip positions.
4. **The soft limit holds.** Sent towards an end at top speed, the cart comes
   to rest at that end's stop point, `runtime_end_margin` pulses short of the
   cached trip position, and the switch does not trip. Moving away from an end
   is never capped.
5. **A trip is a fault stop.** A switch tripping during a run stops the cart in
   one jump.
6. **After a calibration, `MoveToCentre` works from anywhere**, using the
   cached centre and the pulse count, with no seek.
7. **`runtime_end_margin` survives a reset.** Set over WiFi, it is read back
   after a reset with the same value.
8. **The board never polls.** It sleeps until a datagram arrives.

## Requirements (from the diagram)

- **A1: one firmware for the seeks and Run** (*The command and the Arduino's
  Run*, follows). The seeks and Run are in the same firmware, today's
  `calibration` crate (`__live__/arduino/rust/crates/calibration`).
- **A2: the command.** One UDP datagram of 4 bytes, an `i32` little-endian, in
  pulses per second, sign = direction, to port 4210 (*Contracts*).
- **A3: the clamp, ±44,600** (*The command and the Arduino's Run*).
- **A4: the ramp.** Every tick the sent rate moves towards the command by at
  most m × the tick, rounded down. It is the synthetic Arduino's `Ramp`
  (`__synthetic__/orchestrator/rust/crates/arduino`), **copied, not shared**
  (`AGENTS.md`: live and synthetic share no code).
- **A5: DIR and STEP.** DIR from the sign, STEP's rate from the size of the
  sent rate, STEP made by the board's GPT timer in hardware (*the board's
  edge*).
- **A6: the pulse count.** Every pulse STEP sends is counted, signed by DIR.
- **A7: the cached rail** (decided 2026-11-02). After a calibration the board
  keeps both switches' trip positions and the centre, in pulses, so the cart is
  returned to the centre from wherever it stopped, with no seek.
- **A8: the soft limit** (decided 2026-11-02 and 2026-11-03). Each tick, while
  the cart moves towards an end, the sent rate is at most v_cap = √(2 × m × d),
  d the pulses left to that end's stop point (`runtime_end_margin` short of its
  cached trip position), 0 once there. Away from an end, no cap. It binds the
  commands only: the seeks must pass the stop points to reach the switches.
- **A9: a trip during a run stops the cart in one jump**, as in a seek (*The
  soft limit*, follows).
- **A10: no run state** (decided 2026-11-02, "uno is the dumb muscle"). No
  message enters or leaves a run; the board applies each command as it
  arrives. Every rule is one fixed rule per tick or per command (decided
  2026-10-28).
- **A11: `runtime_end_margin` in the data flash** (decided 2026-11-03), through
  `unor4_hal::dflash::DataFlash` (*The Uno's settings*): `take(&Clocks)` once
  per boot; a byte is written once per erase of its 1 KB block; after a reset
  during a write or erase, that area is erased again before it is trusted;
  erased bytes are tested with `blank_check()`, not `read()`.
- **A12: set and read over WiFi** from the Orin's Arduino Settings page, and
  pulled and uploaded by `arduino-manager` over USB (*The Uno's settings*).
- **A13: the interrupt bridge, never polled** (decided 2026-10-31,
  `src/wifi.rs`).

## Not decided by the diagram

Each lands in the diagram first (`real-loop-dia.md`, the *Open* lists):

- **How a command is told from a 4-byte text message.** `pins` and `Stop` are
  both 4 bytes; read as commands they are full speed to the right. This blocks
  step 5.
- **The tick's length.** It sets m × the tick (2,421 pulses/s at 2 ms).
- **DIR's level for a positive command.**
- **What the board does when commands stop coming.**
- **Whether a datagram is read during a fast move** without pausing the
  pulses.
- **What the board does with a command after a trip, or before a
  calibration**, when there are no cached trip positions.
- **The settings messages** over WiFi, and their answers.
- **The settings channel to `arduino-manager`** over the USB serial line.
- **The value used when the data flash holds no setting.**
- **`runtime_end_margin`'s value**, found on the rig through the settings page.

## Steps

Every file keeps one type, named after it, and its `use crate::…` lines come
first, then a blank line, then the external crates (`AGENTS.md`, *Repository
map*, `__live__/`). `cargo clippy --all-targets --no-deps` from the repo root
is clean before any step is called done.

1. **The cached rail** (A7). `rail.rs`'s `Rail` keeps, after a seek pair, the
   trip position of each switch and the centre, all in pulses on one count that
   starts at the first trip. `MoveToCentre` uses them from any position instead
   of answering `not_ready` unless the cart rests on a switch. *Check:*
   Done-when 6, on the rig, after a seek pair and a move off the centre.
2. **STEP from the GPT timer, with a pulse count** (A5, A6). A step generator
   that changes its rate while running and counts the pulses it sends. It needs
   both from `unor4-hal` (*the board's edge*); what the HAL offers is asked of
   the user, not read. *Check:* Done-when 3: a known pulse count sent away from
   and back to a switch trips it at the same count.
3. **The ramp** (A3, A4). A copy of the synthetic `Ramp` in its own file, with
   its clamp and its rule, ticked by the timer at the tick's length (blocked on
   *the tick's length*). *Check:* Done-when 2, in host tests copied from the
   synthetic crate's.
4. **The soft limit** (A8, A9). Its own type: given the sent rate, its
   direction, the pulse count, the cached trip positions and
   `runtime_end_margin`, it caps the rate at √(2 × m × d). It is applied after
   the ramp, every tick. A trip stops the cart in one jump. *Check:* Done-when
   4–5: host tests for the cap at and around the stop point; on the rig, top
   speed towards each end stops short of the switch.
5. **The command datagram** (A2, A10). The main loop hands each 4-byte command
   to the ramp (blocked on *how a command is told from a 4-byte text
   message*). *Check:* Done-when 1.
6. **The setting in the data flash** (A11). One type holds
   `runtime_end_margin`: at boot it takes `DataFlash`, reads the setting's
   block, and uses it if the block is not blank (the blank case is blocked on
   *the value used when the data flash holds no setting*). A new value erases
   the block, then writes it. The setting's place in the flash is one named
   constant. *Check:* Done-when 7.
7. **The settings messages** (A12): set and read over WiFi (blocked on *the
   settings messages*), and the serial channel for `arduino-manager` (blocked
   on *the settings channel*). A write is refused while the cart moves, since
   it blocks the board for its whole run (*The Uno's settings*, follows).
8. **The README.** `__live__/arduino/rust/crates/calibration/README.md` gains
   the command, the soft limit, the setting and its messages, written for a
   layman.

## References

- `real-loop-dia.md`: the source of truth for this plan.
- `real-orin-plan.md`: the brain, which sends the commands, and the Arduino
  Settings page.
- `real-pi-plan.md`: `arduino-manager`, which uploads the firmware and the
  settings.
- `synthetic-arduino-plan.md`: the synthetic twin, whose ramp is copied.
- `jump-limit-results.md`: m.
- `__live__/arduino/rust/crates/calibration/README.md`: the firmware today.
