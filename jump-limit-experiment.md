# The jump-limit experiment — measuring how fast the cart's stepper can change speed

A balance policy is being trained in simulation to drive a real
cart-pole: a cart on a rail, pushed by a stepper motor through a belt,
with a pole hinged on top. In simulation the motor never fails, so
nothing stops the policy from learning commands the real motor cannot
follow. One number, the **jump limit `m`**, tells the training how fast
the real motor can change speed before it slips. Nobody has measured it,
and training cannot run without it. This document says exactly what `m`
is, what the experiment must produce, and the procedure expected to
produce it, for whoever designs the control circuit and test harness that
measure it. It stands alone: every term is defined here, and every rig
figure names where it comes from.

Status: a brief for the experiment (2026-10-25; revised 2026-10-27 for
the DM542 driver, the 24 V supply and either-switch homing), not a
record of one. Nothing in it has been measured. The harness is built:
the Jump-limit experiment button on the `calibrate` firmware's page
runs this procedure (`calibrate/README.md`, *The jump-limit
experiment*).

## 1. The property to measure

**`m`, the jump limit:** the fastest the commanded pulse rate can change
without the stepper motor slipping, on this rig, at its training supply
voltage and driver current. Unit: pulses per second, per second
(pulses/s²).

The terms in that sentence:

- **Pulse, pulse rate.** The motor is driven by a stepper driver (a
  Leadshine DM542, which replaced a TI DRV8825 on 2026-10-26). Every
  electrical pulse on the driver's PUL input turns the motor one
  microstep, a fixed small angle; the DIR input sets which way. The pulse rate is pulses per second, and it sets the motor's
  speed, so it sets the cart's speed. On this rig one pulse moves the
  belt 15 µm, so 24,000 pulses/s is 0.36 m/s (section 2).
- **The command.** The policy never sends positions. Each of its
  commands is one number, a pulse rate (whole pulses per second, its sign
  the direction), and the motor runs at that rate until the next command
  replaces it. The rate changes at once when a command arrives: there is
  no ramp between commands.
- **Change rate `j`.** How fast the pulse rate is changing:
  `j = (c_new − c_old) / Δt`, where `c_old` and `c_new` are two
  consecutive commands in pulses/s and `Δt` is the seconds between them.
  Example: 0 then 24,000 pulses/s, 16 ms apart, is
  `j = 24,000 / 0.016 = 1,500,000 pulses/s²`.
- **Slipping (losing steps).** A stepper turns exactly one microstep per
  pulse only while its rotor keeps up with the pulses. The rotor, belt,
  cart and pole all have mass, and the motor's torque (its turning
  force) can only speed them up so fast, less so the faster it already
  turns. When the pulse rate rises faster than that, the rotor falls
  behind the driver's rotating magnetic field and slips back: steps are
  lost, the cart ends up short of where the pulse count says, and the
  motor may stall altogether. The stepper reports none of this itself:
  it is open loop, meaning nothing measures where it actually is; the
  firmware only counts the pulses it sent.
- **The unit as an acceleration.** At 15 µm per pulse,
  1,000,000 pulses/s² is a cart acceleration of 15 m/s². The example
  above, 1,500,000 pulses/s², is 22.5 m/s².

**What `m` is used for.** The drive's ramp: every tick, the rate the
firmware sends moves towards the last command by at most `m` × the tick,
so the motor is never asked to change speed faster than it can follow,
whatever the policy sends (`synthetic-loop-dia.md`, *The acceleration
ceiling*; user, 2026-10-28). Until 2026-10-28 it set a smoothness factor
in the training reward instead (the root `reward-plan.md` of the time,
section 4, said why that was removed; deleted 2026-10-28, in git history
at `1e6a1001`). `m` is written in the trainer's settings file
`trainer/settings.toml`, under `[drive]`, as `jump_limit`.

**It depends on.** The supply voltage and the driver's current setting
(both set the motor's torque), the microstep setting (it sets the
microstep angle), the moving mass and the friction. So `m` is valid only
for the rig as it will train, and is measured again if any of these
change.

## 2. The rig

| What | Value | Source |
|---|---|---|
| Board | Arduino UNO R4 WiFi (Renesas RA4M1, Cortex-M4 at 48 MHz); runs the firmware that makes the STEP and DIR signals | `rig-drive-research.md`, *The parts* |
| Driver | Leadshine DM542: pulses up to 200 kHz, each on at least 2.5 µs and off at least 2.5 µs, DIR settled 5 µs before a pulse; optically isolated inputs, driven through PNP transistors | `control-design.md`, from the DM542 manual |
| Driver current | 3.76 A peak, 2.69 A RMS, set by its DIP switches (SW1–3 on, off, off); SW4 on keeps the full current at standstill | `control-design.md`, *Setting the switches* |
| Microstepping | 1/32, 6,400 pulses per turn, fixed (DM542 SW5–8 off, on, off, on) | `trainer/settings.toml` `[drive] microsteps`; `control-design.md` |
| Motor | StepperOnline 23HS32-4004S: NEMA 23, 1.8° (200 full steps per turn), 4.0 A rated, 2.4 N·m holding torque, rotor inertia 530 g·cm² | `motor-spec.md` (the maker's datasheet) |
| Torque curve | maker's pull-out torque, at 24 V and 4.0 A only: 1.86 N·m at 150 rpm falling to 0.74 N·m at 500 rpm | `motor-spec.md`, *Torque curve* |
| Pulley and belt | 48-tooth GT2, 2 mm pitch: 96 mm of belt per turn, pitch radius 15.28 mm | `trainer/settings.toml` `[drive] belt_um_per_rev`; `mujoco-plan.md` |
| Belt per pulse | 96 mm ÷ (200 × 32) = 15 µm | from the two rows above |
| Moving mass | 0.909 kg (cart 0.507 kg, pole 0.402 kg); the rotor adds the equivalent of 0.227 kg at the cart | `mujoco-plan.md`, *The actuator's values* |
| Pole | hinged on the cart, its tip 0.45 m from the hinge; it swings freely | the MJCF's sites (`synthetic-loop-dia.md`, *Contracts*) |
| Rail | the cart's faces meet the end stops 0.440 m left and 0.445 m right of centre | the MJCF's sites (`synthetic-loop-dia.md`, *Contracts*) |
| Limit switches | one at each end of the rail, on D2 and D3; the cart presses it at that end. Homing (`calibrate`): slowly in Minus until either switch trips (the first end, pulse 0), slowly in Plus until the other trips (the second end, the span), back to the middle | `synthetic-loop-dia.md`, *Numbers*; `calibrate/README.md` |
| Supply voltage | 24 V. The user's preference is 12 V, then 24 V, then 30 V, but the DM542 needs 20 V or more, so 12 V is out; 30 V fits the DM542 but not the Arduino's VIN. The bench supply in use falls to 21 V at the 0.5 A the motor draws holding still | user, 2026-10-25 (`motor-spec.md`); `control-design.md`; measured 2026-10-27 |
| Top speed and top pulse rate | at 24 V, 0.72 m/s, 48,000 pulses/s (12 V 24,000; 30 V 60,000). Estimates of the motor's corner speed at the DRV8825's 2.5 A, not measured; at the DM542's 2.69 A each is about 7% lower | `motor-spec.md`, *The top pulse rate per supply* |

**The top pulse rate** is the fastest the firmware will drive the motor:
any command beyond it is cut to it. Written `top` below.

## 3. The stimulus: how the policy drives the motor

The experiment has to reproduce what training will ask of the motor, so
this is the shape of the signal:

- A new command arrives every 10, 12, 14 or 16 ms in simulation (a
  detection delay of 8–15 ms, rounded up to the simulator's 2 ms steps;
  `synthetic-loop-dia.md`). On the real rig the gap will be longer: the
  policy's own computing time and the WiFi hop to the board are not
  measured yet.
- At each command the pulse rate jumps at once from the old value to the
  new one. Between commands it is constant.
- The new value can be anything from −`top` to +`top`, including a full
  reversal in one command.

So the hard cases are single large jumps, and runs of jumps in the same
direction (a staircase), which is how the policy accelerates for longer
than one gap.

For one change rate `j`, a longer gap means a bigger single jump
(`j × Δt`), so the longest gap is the hardest. The procedure uses
`Δt = 0.016 s`; if the real loop's gap is measured longer, use that.

## 4. What the test harness must do

These follow from the procedure in section 5. It is built as the
board's own firmware, `calibrate`: its page's Jump-limit experiment
button (`calibrate/README.md`), on the circuit of `control-design.md`.

- **Make the stimulus exactly.** PUL and DIR to the DM542 with the
  pulse rate held constant, then changed at one chosen instant from
  `c_a` to `c_b`, with no ramp, within the driver's timing (pulses at
  most 200 kHz, each on and off at least 2.5 µs, DIR settled 5 µs
  before a pulse). A reversal is a DIR change at that instant.
  `calibrate` times each trial's pulses on the processor's cycle
  counter, so a jump lands within one pulse period of its instant.
- **Count every pulse sent**, signed by direction, from a fixed start.
- **Know where the cart is against a fixed reference.** The rig's
  reference is the limit switch at the first end homing finds, the
  end the Minus direction reaches: the harness must note the signed
  pulse count at the instant the switch trips.
- **Move slowly when not testing:** homing and returning to the switch at
  a rate low enough that the motor cannot slip.
- **Stop gently:** after a test jump, bring the rate to 0 in small
  changes, well before either end. A sudden stop is itself a jump and can
  make the motor slip, spoiling the trial.
- **Log every trial** (section 6).

The procedure assumes the limit switch as the only way to see where the
cart really is. Anything that measures the motor's or cart's position
independently of the pulse count (an encoder on the motor shaft, say)
would show the slip directly, during the jump, instead of afterwards; it
is not on the rig today.

## 5. The procedure

Before starting: the pole mounted, the supply at the voltage training
will use, the DM542's switches set as they will be (current and
microstepping, 1/32), and the cart homed, so the first end and the
span are known.

```
1. How exact the reference is (the noise floor).
   Drive slowly to the reference switch and note the pulse count where
   it trips. Move away slowly and repeat, 10 times in all. The counts
   differ a little, because a switch never trips at exactly the same
   point. Their spread (largest minus smallest) is the noise floor: a
   later difference no larger than it is not a lost step.

2. One trial: one jump, from rate c_a to rate c_b, at change rate j.
     Δt  = 0.016 s
     c_b = c_a + j × Δt          (both within -top..+top)
   a. Drive slowly to the rail's centre.
   b. Bring the rate to c_a and hold it until the cart runs steadily.
      If c_a is not 0, reach it in jumps no larger than ones already
      passed at this setting.
   c. The jump: at one instant, change the rate to c_b. Hold it 0.1 s.
   d. Stop gently, in small changes, well before either end.
   e. Drive slowly to the reference switch and note the count where it
      trips.
      Every pulse was counted, so if the motor kept up, the switch trips
      at the count step 1 found. A difference larger than the noise floor
      is that many lost steps; the trial failed.

3. The kinds of trial, the jumps a policy makes:
     from rest:   c_a = 0,          c_b = +c
     at speed:    c_a = top / 2,    c_b = top / 2 + j × Δt  (up to top)
     reversal:    c_a = -c,         c_b = +c
     staircase:   from 0, jumps of j × Δt every Δt until top, then hold
                  (the same j, sustained; step 2c becomes the whole
                  staircase)
   c is any rate up to top; use the largest the kind allows for that j.
   (The at-speed row read c_b = top until 2026-10-27, which allows one
   j only; c_b now follows j like the other kinds.)

4. For each kind, raise j step by step (section 7 says where to start),
   10 trials at each j. The kind's limit is the largest j at which none
   of the 10 trials lost a step.

5. m = the smallest of the kinds' limits, so it is safe for all of them.
   The loop can never ask for more than a full reversal, -top to +top,
   at its shortest gap, 0.010 s: 2 × top ÷ 0.010 pulses/s² (9,600,000
   at 24 V). At Δt = 0.016 s no kind can make that much: each can make
   at most top ÷ Δt from rest and as a staircase step (3,000,000 at
   24 V), top ÷ 2 ÷ Δt at speed (1,500,000), and 2 × top ÷ Δt as a
   reversal (6,000,000). If a kind loses no step up to its own most,
   its limit is "at least that".

6. Report as section 6 says, and write m into trainer/settings.toml,
   [drive] jump_limit.
```

## 6. What the experiment must hand back

- **`m`**, in pulses/s², and whether it is a measured limit or an "at
  least" (no slip found within what the loop can ask).
- **Each kind's limit**, the same way.
- **The conditions:** date, supply voltage (and how far it falls under
  load), the DM542's current setting, microstepping, the pole mounted
  or not, `Δt` used. The board measures none of these but `Δt`:
  `calibrate.toml` `[experiment] conditions` carries them into its
  report.
- **The noise floor**, in pulses.
- **Every trial:** kind, `c_a`, `c_b`, `j`, `Δt`, the switch count
  after, the lost steps, pass or fail.

## 7. Where to start `j`

A ceiling the true value cannot exceed (an upper bound), not an estimate
of `m`: starting from rest, the motor pushes the cart with at most 106 N
(its holding torque 2.4 N·m scaled to the DM542's 2.69 A RMS of its
rated 4.0 A, 1.61 N·m, at the pulley's 15.28 mm radius), and it must
speed up 1.136 kg (the 0.909 kg cart and pole plus 0.227 kg for the
rotor, which resists speeding up as if it were mass on the cart).
106 N ÷ 1.136 kg ≈ 93 m/s², which at 15 µm per pulse is about
6,200,000 pulses/s². Friction, the pole's swing, and the torque falling
as the motor speeds up all make the real limit lower; how much lower is
what the experiment finds. Starting near a tenth of the ceiling (about
620,000 pulses/s², `calibrate.toml` `[experiment] j_start`) and raising
`j` by about a quarter each time reaches the ceiling in about ten
steps. (Until 2026-10-27 this section used the DRV8825's 2.5 A: 98 N,
5,800,000 and 580,000.)

## 8. Open questions for the designer

- **DIR's level:** which pin level is the positive direction is still
  not decided for the loop (`synthetic-arduino-plan.md`, *Not decided
  by the diagram*). The experiment does not need it: homing stops at
  either switch, and every trial runs from the centre and back to the
  same reference switch.
- **The board's firmware** for the experiment is built: `calibrate`
  makes and counts every pulse in software (section 4). The training
  firmware still needs the board's timer to make the pulses, with its
  rate changed while running and a pulse count (`rig-drive-research.md`,
  *The user's `unor4-hal`*).
- **The real gap between commands** (section 3) is not measured; until
  it is, use 16 ms.
- **The supply.** The bench supply falls from 24 V to 21 V at 0.5 A,
  and more when the motor accelerates. The torque at speed, and so
  `m`, hold only for the supply it is measured on; measure on the one
  training will use.
- **The training model's top speed matches the rig.** The actuator
  asset (`Assets/Simulation/MuJoCo/03-Cart.Slider.Actuator.asset`) and
  the MJCF training loads (`trainer/settings.toml` `[mjcf] model`) both
  hold `ctrlrange` ±0.72 m/s, the 24 V row, 48,000 pulses/s (read
  2026-10-27). If either moves to another supply's row, `m` is measured
  again at that supply.

## Sources

- The definition of `m`, the rule it enters and "measured on the rig":
  `synthetic-loop-dia.md`, *The acceleration ceiling*.
- The command's shape and the gaps: `synthetic-loop-dia.md` (the loop's
  design): *Contracts* (the command), the 10–16 ms gaps, *Numbers*
  (homing).
- The motor's figures: `motor-spec.md`, from the maker's datasheet and
  torque curve. The driver's: `control-design.md`, from Leadshine's
  DM542 manual V1.0. The drive figures: `trainer/settings.toml`
  `[drive]`. The masses: `mujoco-plan.md`, from the MJCF.
- The harness: `calibrate/README.md`, *The jump-limit experiment*;
  its settings: `calibrate/calibrate.toml` `[experiment]`.
- The procedure itself has no published source. Its choices (the four
  kinds, 10 trials per `j`, raising `j` by a quarter, the limit switch as
  the reference, testing at the longest gap) are this brief's, each with
  its reason given where it appears.
