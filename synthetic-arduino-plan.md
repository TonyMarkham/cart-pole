# Synthetic Arduino Plan — the motor's firmware in the loop

The command the policy learns to send has to reach the cart the way it
will on the rig — through the Arduino's firmware to the DRV8825 stepper
driver — so the loop runs that firmware in-process as the **synthetic
Arduino**. It takes the trainer's command, an `i32` pulse rate, and
turns it into the two signals the real board sends the DRV8825: DIR
(the direction) and STEP (the pulse rate). The orchestrator turns those
into MuJoCo's cart speed.

Status: **plan** (2026-10-23), written from `synthetic-loop-dia.md`
alone, the source of truth. Element numbers below are the diagram's
messages.

## Done when

1. **Created with `RIG` alone** (3): no start position is passed.
2. **The signals are right** (19–20): a command of `+r` gives DIR for
   the positive direction and STEP rate `r`; `−r` gives the other DIR
   and the same rate; 0 gives STEP rate 0.
3. **The clamp holds:** any command beyond ±45,333 pulses/s gives STEP
   rate 45,333, with DIR from its sign.
4. **The cart speed matches:** the orchestrator's ctrl is STEP rate ×
   15 µm, in m/s, signed by DIR.

## Requirements (from the diagram)

- **A1 — created with `RIG` (3).** The cart starts at 50% of the rail,
  a guarantee that stands in for the real board's homing, so the
  synthetic Arduino is never told a start position.
- **A2 — the command, and nothing else (19).** `receive(the command:
  i32 LE pulses/s)`, once per observation at the arrival step; sign =
  direction, magnitude = pulses per second.
- **A3 — the clamp.** The rate is clamped to the rig's top rate,
  ±45,333 pulses/s.
- **A4 — the DRV8825 signals (20).** DIR from the sign, STEP rate from
  the magnitude, returned to the orchestrator.
- **A5 — no pulse count in the loop.** There is no clock in the loop, so
  the synthetic Arduino keeps no pulse count, and the ±30,000-pulse
  travel limit cannot act there; the end crash is the trainer's
  training failure (`trainer-plan.md`). On the real board, the timer
  counts the pulses it sends, so the travel limit acts there.
- **A6 — one source with the board.** The firmware is one source built
  two ways: the synthetic edge for the loop, the board's edge for the
  rig.

**The numbers** (`arduino::RIG`, as the diagram gives them): 200 full
steps × 32 microsteps per turn and 96 mm of belt per pulley turn, so
15 µm per pulse; ±30,000 pulses of travel (±0.45 m); a top rate of
45,333 pulses/s (0.68 m/s).

## Not decided by the diagram

These are needed to build it; per `AGENTS.md`, each lands in the
diagram first:

- **DIR's level** — which sign is the high level on the DIR pin.

## Placeholders

- **The microstep setting** — 32 (1/32, the DRV8825's finest) until the
  driver's setting is chosen. The motor's 200 full steps per revolution
  are from third-party 23HS32-4004S listings.

## Steps

1. **Creation** (A1). *Check:* Done-when 1.
2. **Receive, clamp, signals** (A2–A4). *Check:* Done-when 2–3.
3. **The orchestrator's conversion** (`orchestrator-plan.md`, O8).
   *Check:* Done-when 4.

## Not in this plan

The board's own edge: the radio, the step pin, its timer and pulse
count, the travel limit, and homing.

## References

- `synthetic-loop-dia.md` — the source of truth for this plan
- `orchestrator-plan.md` — the participant that calls it
