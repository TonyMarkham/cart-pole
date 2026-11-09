# Synthetic Arduino Plan — the motor's firmware in the loop

The command the policy learns to send has to reach the cart the way it
will on the rig — through the Arduino's firmware to the DM542 stepper
driver (which replaced a DRV8825 on 2026-10-26, `control-design.md`)
— so the loop runs that firmware in-process as the **synthetic
Arduino**. It takes the trainer's command, an `i32` pulse rate, and
turns it into the two signals the real board sends the DM542: DIR
(the direction) and STEP (the pulse rate, on the DM542's PUL input). The orchestrator turns those
into MuJoCo's cart speed.

Status: **plan** (2026-10-23), written from `synthetic-loop-dia.md`
alone, the source of truth. Element numbers below are the diagram's
messages.

## Done when

1. **Created with its `Config` alone** (3): the drive figures and the
   cart top speed; no start position is passed.
2. **The signals are right** (19–20): a command of `+r` gives DIR for
   the positive direction and STEP rate `r`; `−r` gives the other DIR
   and the same rate; 0 gives STEP rate 0.
3. **The clamp holds:** any command beyond ± the top pulse rate gives
   STEP rate at the top pulse rate, with DIR from its sign.
4. **The cart speed matches:** the orchestrator's ctrl is STEP rate ×
   metres per pulse, in m/s, signed by DIR.
5. **The ramp holds the ceiling:** each tick of `dt` µs moves the sent
   rate towards the command by at most m × dt, rounded down; a change
   within that finishes in one tick; a reset (R3) returns both rates to 0.
6. **A limit switch is a FAULT** (the diagram's *The rail's ends*, user
   2026-11-08): a tick given a closed switch answers FAULT with STEP rate
   0; every later tick answers the same and no command is applied, until
   a reset (R3) brings it back to rest, out of FAULT.

## Requirements (from the diagram)

- **A1 — created with its `Config` (3)**: the trainer's `[drive]`
  figures, its jump limit m, and the cart top speed. The cart starts at
  50% of the rail, a guarantee that stands in for the real board's
  homing, so the synthetic Arduino is never told a start position. It
  starts at rest: sent rate 0, command 0.
- **A2 — the command (19).** `receive(the command: i32 LE pulses/s)`, on
  the timestep a command lands (the diagram's *The gobble rule*): it sets
  the command and nothing else; the sent rate moves at that timestep's
  tick (A7). Sign = direction, magnitude = pulses per second. In FAULT
  (A8) a command is not applied.
- **A3 — the clamp.** The command is clamped to ± the top pulse rate:
  the cart top speed (the MJCF actuator's `ctrlrange`) ÷ metres per pulse
  (`[drive]`).
- **A4 — the DM542 signals (20).** DIR from the sign, STEP rate from
  the magnitude of the sent rate after the tick, returned to the
  orchestrator.
- **A5 — no pulse count in the loop.** The loop's only clock is MuJoCo's
  step, which ticks the ramp (A7) but counts no pulses, so the synthetic
  Arduino holds no travel: the rail's ends are its limit switches (A8).
  On the real board, the timer counts the pulses it sends, so a travel
  limit acts there too.
- **A7 — the acceleration ceiling** (the diagram's *The acceleration
  ceiling*). Every tick of `dt`, the sent rate moves towards the
  command by at most m × dt, rounded down to a whole pulse/s. In the
  loop a tick is one 2 ms MuJoCo timestep, once every timestep after
  MuJoCo moves on (19–20), with the command that landed on it, if one did.
  The firmware holds no perception timing: when a command arrives is the
  orchestrator's to decide. A reset returns the
  sent rate and the command to 0 (R3). One fixed rule, the same on the
  board's step generator.
- **A8 — the limit switches and FAULT** (the diagram's *The rail's ends*;
  user, 2026-11-08). Each tick is given the two limit switches, closed or
  open; the orchestrator works them out from the strike faces, since the
  switches are the physical world's. One fixed rule each tick, as the
  controller firmware's (commit `c1b4bd1b`): a closed switch puts it in
  FAULT. In FAULT the cart stops in one jump (sent rate 0, command 0), no
  command is applied, and every tick answers FAULT with STEP rate 0 (20);
  only a reset (R3) leaves it. The 50% start stands in for the
  calibration, so it is calibrated from its creation, and it does not tell
  CALIBRATED from MOVING: a trip in either is a FAULT.
- **A6 — one source with the board.** The firmware is one source built
  two ways: the synthetic edge for the loop, the board's edge for the
  rig.

**The numbers** (as the diagram gives them): the drive figures are the
trainer's `settings.toml` `[drive]`, passed by the orchestrator (200
full steps × 32 microsteps per turn and 96 mm of belt per pulley turn
today, so 15 µm per pulse); the cart top speed is the MJCF actuator's
`ctrlrange`, read from the loaded model by the physics crate
(`mujoco-plan.md`, `motor-spec.md`); the top pulse rate is cart top
speed ÷ metres per pulse, rounded down (`Config::max_rate`); m is
`[drive] jump_limit`, 1,210,937 pulses/s² today (`jump-limit-results.md`),
so a 2 ms tick moves the sent rate at most 2,421 pulses/s.

## Not decided by the diagram

These are needed to build it; per `AGENTS.md`, each lands in the
diagram first:

- **DIR's level** — which sign is the high level on the DIR pin.

## Placeholders

- **The microstep setting** — 32: the DM542's switches set 1/32,
  6,400 pulses per revolution (`control-design.md`, *Setting the
  switches*). The motor's 200 full steps per revolution
  are from third-party 23HS32-4004S listings.

## Steps

1. **Creation** (A1). *Check:* Done-when 1. **Done (2026-10-24):** the
   crate is `__synthetic__/orchestrator/rust/crates/arduino/` (renamed from `firmware/`);
   `SyntheticArduino::new(config)` takes no start position and holds
   only its `Config` (test `synthetic_is_created_with_its_config_alone`,
   `__synthetic__/orchestrator/rust/crates/arduino/src/tests.rs`).
2. **Receive, clamp, signals** (A2–A5). *Check:* Done-when 2–3.
   **Done (2026-10-24):** `receive` decodes the 4-byte `i32`, clamps it
   with `Config::clamp_rate` (the clamp the board's step generator uses
   too) and returns `Dm542Signals { dir, step_rate }` (named
   `Drv8825Signals` until the driver swap). DIR is named by
   the direction it moves the cart (`Direction::Positive` /
   `Negative`), not by a pin level, which is still undecided. The
   synthetic Arduino holds nothing but its `Config`: no tick, no pulse
   count, no travel (A5). Tests `synthetic_receive_returns_dir_and_step_rate`
   and `synthetic_receive_clamps_to_the_top_rate`, on the tests' own
   `Config` (top pulse rate 24,000): ±20,000 give opposite DIRs and rate
   20,000; 0 gives rate 0; 24,001, `i32::MAX` and `i32::MIN` give
   24,000.
3. **The orchestrator's conversion** (`orchestrator-plan.md`, O8).
   *Check:* Done-when 4. **Done (2026-10-24):** test
   `cart_velocity_is_step_rate_times_15_um_signed_by_dir`
   (`__synthetic__/orchestrator/rust/crates/orchestrator/src/tests.rs`, at its test `Config`'s 15 µm a pulse:
   26,667 → ±0.400005 m/s), and the runs in `orchestrator-plan.md`
   step 5.
4. **The ramp** (A1, A7). *Check:* Done-when 5: the shared ramp the
   board's `Stepper` and the synthetic Arduino both hold; `receive` sets
   the command and takes the arrival step's tick, `tick(dt)` the others,
   `reset` returns to rest; the orchestrator ticks it after every step
   but the arrival step (`orchestrator-plan.md`, O8). Since step 5,
   `receive` only sets the command and every timestep has one `tick`.
5. **The limit switches and FAULT** (A2, A7, A8; user, 2026-11-08).
   `receive` sets the command alone; `tick(switches)` ramps, or enters
   FAULT on a closed switch, and answers the signals and whether it is in
   FAULT; `reset` leaves FAULT. *Check:* Done-when 6, and Done-when 5
   under the new split, by unit tests worked by hand on the tests'
   `Config`. **Done (2026-11-08):** `LimitSwitches` and `TickAnswer`, one
   a file; tests `synthetic_ticks_the_ramp`, `synthetic_receive_does_not_tick`,
   `a_closed_limit_switch_is_a_fault_until_reset` and
   `either_limit_switch_trips` pass, with the crate's other 19.

## The board and the top rate

The board is an Arduino UNO R4 WiFi (user, 2026-10-24). Whether it, the
driver and the motor can make the top pulse rate, with every
calculation and source: `rig-drive-research.md` (worked at the 30 V
supply for the DRV8825, which the DM542 replaced on 2026-10-26). In
short, the DM542 takes up to 200,000 pulses/s (`motor-spec.md`, *The top
pulse rate per supply*) and the
board's GPT timer makes each pulse from whole ticks of 48 MHz in
hardware; the user's `unor4-hal` needs a runtime rate change and a pulse
count to drive STEP; the motor's real top speed is still to be measured
on the rig. The microstep setting is fixed,
never switched while the motor runs.

## Not in this plan

The board's own edge: the radio, the step pin, its timer and pulse
count, the travel limit, and homing.

## References

- `synthetic-loop-dia.md` — the source of truth for this plan
- `orchestrator-plan.md` — the participant that calls it
