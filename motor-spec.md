# Motor spec — the 23HS32-4004S stepper that drives the cart

The rig's speed, force and acceleration limits all come from this motor,
so its figures belong in one place, each traced to the maker. This file
holds the maker's specification and torque curve for the
StepperOnline 23HS32-4004S (NEMA 23, 2.4 N·m, 4.0 A), read 2026-10-25.
It is a reference, not a plan: what the rig derives from these figures
(the corner speed, the top rate, the force) is `rig-drive-research.md`'s.

## Sources

All three are the maker's (StepperOnline, OMC), read 2026-10-25:

- **Product page** —
  https://www.omc-stepperonline.com/nema-23-stepper-motor-2-4nm-339-79oz-in-4a-57x57x82mm-8mm-shaft-4-wires-23hs32-4004s
  (*Specifications* tab). Its *Downloads* list the two PDFs below.
- **Full datasheet** (drawing A1214, rev 0, 2025-08-01, 255.89 KB) —
  https://files.omc-stepperonline.com/23HS32-4004S_Full_Datasheet.pdf
- **Torque curve** (213.89 KB) —
  https://files.omc-stepperonline.com/23HS32-4004S_Torque_Curve.pdf

The PDFs the user supplied on 2026-10-25 have the same names and the same
sizes as the product page lists (262,028 and 219,024 bytes), and their
contents are what this file quotes. The files server refuses scripted
fetches (HTTP 403); the links open in a browser.

## Electrical

| Figure | Value | Source |
|---|---|---|
| Phases | 2, bipolar | product page; datasheet |
| Step angle | 1.8° (200 full steps/rev), accuracy ±5% non-accumulating | datasheet; product page |
| Rated current | 4.0 A per phase | datasheet; product page |
| Phase resistance | 0.65 Ω ±10% at 25 °C | datasheet; product page |
| Phase inductance | 3.2 mH ±20% at 1 kHz | datasheet; product page |
| Holding torque | 2.4 N·m (339.87 oz·in; the datasheet prints 21.24 lb·in) | datasheet; product page |
| Rotor inertia | 530 g·cm² | datasheet |
| Insulation class | B, 130 °C | datasheet; product page |
| Temperature rise | 80 °C at most | datasheet |
| Ambient | −10 to 50 °C | datasheet |
| Insulation resistance | 100 MΩ | datasheet, comment 3 |
| Dielectric strength | 500 V AC for 1 min, coils to case | datasheet, comment 4 |
| Life | 6,000 h or more at 24 V / 300 rpm | datasheet, comment 2 |

### Wiring

The four leads, bipolar (datasheet, *Motor connection*; product page).
Coil A is black and green; coil B is red and blue.

| Pin | Winding | Lead colour | DM542 terminal (control-design.md) |
|---|---|---|---|
| 1 | A+ | black | A+ |
| 2 | A− | green | A− |
| 3 | B+ | red | B+ |
| 4 | B− | blue | B− |

Full-step sequence, two phases on, facing the mounting end (datasheet).
Steps 1 → 4 turn the shaft clockwise; 4 → 1 counter-clockwise.

| Step | A+ | B+ | A− | B− |
|---|---|---|---|---|
| 1 | + | + | − | − |
| 2 | − | + | + | − |
| 3 | − | − | + | + |
| 4 | + | − | − | + |

## Mechanical

| Figure | Value | Source |
|---|---|---|
| Frame | 56.4 ± 1 mm square (NEMA 23, 57 mm) | datasheet; product page |
| Mounting holes | 4 × Ø5, 47.14 ± 0.2 mm apart | datasheet |
| Body length | 82 ± 1 mm | datasheet; product page |
| Pilot | Ø38.1 (−0.05) × 1.6 mm | datasheet |
| Shaft | Ø8 (−0.012) mm, 21 ± 1 mm long, D-cut 15 ± 0.3 mm long | datasheet; product page |
| Leads | 4, 500 ± 10 mm, UL1007 AWG20 | datasheet; product page |
| Weight | 1.2 kg | product page (the datasheet leaves it blank) |

## Torque curve

Pull-out torque, the most the motor delivers at a speed without losing
steps, tested at **24 V, 4.0 A, 3,200 microsteps per revolution** (1/16
microstepping). The pulse rate depends on the microstep setting; the
speed in rpm does not, so rpm is the column to read. The points are the
plotted dots, read from the chart (to about ±2 N·cm); the belt speed is
derived, at this rig's 96 mm per pulley turn (`rig-drive-research.md`,
*The parts*).

| Pulses (kpps, 1/16) | Speed (rpm) | Pull-out torque (N·cm) | Belt speed (m/s, derived) |
|---|---|---|---|
| 2.7 | 50 | 201 | 0.08 |
| 8.0 | 150 | 186 | 0.24 |
| 13.3 | 250 | 137 | 0.40 |
| 18.7 | 350 | 95 | 0.56 |
| 26.7 | 500 | 74 | 0.80 |
| 32.0 | 600 | 63 | 0.96 |
| 37.3 | 700 | 53 | 1.12 |
| 42.7 | 800 | 45 | 1.28 |
| 48.0 | 900 | 39 | 1.44 |
| 53.3 | 1,000 | 34 | 1.60 |

Speed (rpm) = pulses/s ÷ 3,200 × 60. Belt speed = rpm ÷ 60 × 0.096 m.

The rig does not run at the curve's conditions: its supply is 24 V, as
the curve's, but the DM542 drives the motor at 2.69 A RMS, not 4.0 A
(`control-design.md`, *Setting the switches*), and no curve for that
current is published. Less current lowers the torque; the effect is not
measured.

## Where the repo's older figures differ

`mujoco-plan.md` (*Parts*) and `rig-drive-research.md` (*The parts*) took
the motor's figures from third-party listings on 2026-10-23, before the
maker's page could be read. Two differ from the maker's:

| Figure | Older docs | Maker |
|---|---|---|
| Phase resistance | 0.9 Ω | 0.65 Ω ±10% |
| Phase inductance | 3.4 mH | 3.2 mH ±20% |

The inductance enters the corner-speed estimate, full steps/s ≈ V ÷
(2 × L × I) (`rig-drive-research.md`, *The calculations*): at 30 V and
2.5 A, 3.2 mH gives about 1,875 full steps/s (0.90 m/s) where 3.4 mH gave
1,765 (0.85 m/s). Checked against the curve: at the curve's 24 V and 4.0 A
the formula gives 24 ÷ (2 × 0.0032 × 4.0) ≈ 937 full steps/s, 281 rpm,
and the curve's knee falls between 150 and 350 rpm.

## How the spec drives the cart actuator

The simulated motor is one Unity asset,
`unity/projects/cart-pole/Assets/Simulation/MuJoCo/03-Cart.Slider.Actuator.asset`
(greyhound's `StepActuator` ScriptableObject). Its `mj` block becomes the
MJCF's `<velocity name="03-Cart.Slider.Actuator">` on the cart's slide
joint, only through a re-export from Unity. It stands in for the stepper,
the DM542 and the belt together: the firmware turns the command into a
pulse rate the motor follows, so the model is a speed servo,
`force = kv × (ctrl − cart speed)`, clamped. How each value was derived is
in `mujoco-plan.md`, *The actuator's values*; this section traces each
one to the figures above. Values read from the asset and from the
exported `asy-cart-pole.xml`, 2026-10-25. Since then both were set to
the 24 V row as it stood at the DRV8825's 2.5 A, `ctrlLo`/`ctrlHi` ±0.72
and `forceLo`/`forceHi` ±98 (read 2026-10-27). At the DM542's 2.69 A the
24 V row is ±0.669 and ±106 (*The actuator asset per supply voltage*,
2026-10-29), which the asset and the MJCF take only from a re-export in
Unity. The rows below are the 2026-10-25 reading.

| Field | Value | From the motor spec | How |
|---|---|---|---|
| `type` | 1, exported as `<velocity>` | — | the command is a speed, as the stepper's pulse rate is |
| `ctrlLo`, `ctrlHi` | ±0.85 m/s | inductance, with the supply (30 V) and current (2.5 A) | the corner speed: full steps/s ≈ V ÷ (2 × L × I), × 96 mm per 200 full steps. Computed with the older 3.4 mH: 30 ÷ (2 × 0.0034 × 2.5) ≈ 1,765 full steps/s = 8.82 rev/s = 0.847 m/s |
| `forceLo`, `forceHi` | ±98 N | holding torque 2.4 N·m and rated current 4.0 A, with the DRV8825's 2.5 A | torque at the set current, 2.4 × 2.5 ÷ 4.0 = 1.5 N·m, at the belt ÷ the pulley's pitch radius 0.01528 m = 98 N |
| `armature` | 0.227 kg | rotor inertia 530 g·cm² | the rotor's inertia felt at the cart, J ÷ r² = 5.30 × 10⁻⁵ ÷ 0.01528² = 0.227 kg, added to the 0.909 kg moving mass |
| `kv` | 100 N·s/m | — | a modelling choice, not a motor constant (`mujoco-plan.md`): the speed follows the command with a time constant of 1.136 kg ÷ 100 ≈ 11 ms |
| `gear` | 1 | — | the actuator drives the slide directly: `ctrl` in m/s, force in N at the cart; the pulley is folded into the force and the armature |
| `ctrllimited`, `forcelimited` | 1 | — | MuJoCo clamps the command and the force to the ranges above |
| `damping`, `delay` | 0 | — | the slide's friction is the joint's `frictionloss`; the delay is the orchestrator's |
| `kp`, `dampratio`, `timeconst`, `cranklength`, `lengthrange`, `inheritrange` | stored | — | unused by a `<velocity>` actuator |

The synthetic Arduino's top pulse rate is the same `ctrlHi` ÷ metres
per pulse (the trainer's `[drive]`), read from the loaded model at each
run, so the command's range and the actuator's are one figure.

**What the maker's figures change.** The asset was set from third-party
listings on 2026-10-23, and it has not been changed since:

- `armature`: unchanged. The datasheet's 530 g·cm² is the figure
  `mujoco-plan.md` back-computed without a recorded source; it now has
  one.
- `ctrlHi`: with the maker's 3.2 mH and the DM542's 2.69 A the same
  formula gives 30 ÷ (2 × 0.0032 × 2.69) ≈ 1,743 full steps/s = 8.7 rev/s
  ≈ 0.836 m/s, against the 0.85 the asset held then. The curve also shows the motor still
  pulling 34 N·cm at 1,000 rpm (1.60 m/s) at 24 V and 4.0 A: the corner
  speed is where torque starts to fall, not where the motor stops.
- `forceHi`: the curve's lowest-speed pull-out torque is 2.01 N·m at
  50 rpm, below the 2.4 N·m holding torque the force limit is scaled
  from. Scaled the same way, 2.01 × 2.69 ÷ 4.0 ÷ 0.01528 ≈ 88 N.
- The force falls with speed, and a constant `forcerange` cannot follow
  it. At 0.85 m/s (531 rpm) the curve gives about 0.71 N·m at 4.0 A,
  about 31 N at the belt scaled to 2.69 A, against the 106 N the 2.69 A
  rows allow at every speed: near top speed the simulated motor is about
  three times stronger than the real one. The scaling by current
  (2.69 ÷ 4.0) is an assumption: no curve at 2.69 A or 30 V is
  published.

## The actuator asset per supply voltage

Pre-calculated `mj` values for `03-Cart.Slider.Actuator.asset` at each
supply the rig may run on (user, 2026-10-25: 12 V is ideal, 24 V
acceptable, 30 V a last resort). Only the speed range depends on the
voltage. Everything else is the same in all three, and is repeated so
each table is a whole asset.

**The rules behind the tables.** Same as *How the spec drives the cart
actuator*, with the maker's figures:

- `ctrlHi` = the corner speed = V ÷ (2 × L × I) full steps/s × 0.48 mm
  per full step (96 mm ÷ 200), rounded down to the millimetre per second
  so the model never claims more speed than the estimate. V is the
  supply; L = 3.2 mH, the datasheet's inductance; I = 2.69 A, the DM542's
  RMS setting (`control-design.md`, *Setting the switches*). `ctrlLo` =
  −`ctrlHi`.
- `forceHi` = 2.4 N·m × 2.69 A ÷ 4.0 A ÷ 0.01528 m = 105.6, rounded to
  106 N: holding torque scaled by current, at the belt. Current sets it,
  not voltage. (`mujoco-plan.md`'s rule. The curve's lowest-speed torque
  gives 88 N instead; see *What the maker's figures change*.)
- Until 2026-10-29 the tables used I = 2.5 A, the DRV8825's maximum:
  ±0.36, ±0.72 and ±0.90 m/s, and ±98 N.
- `armature` = 530 g·cm² ÷ 0.01528² m² = 0.227 kg.
- `kv` 100, `gear` 1 and the zeros are modelling choices
  (`mujoco-plan.md`), not motor figures.

### The top pulse rate per supply

Not part of the asset: the rate the rig's firmware may send the driver
at each supply, the same corner speed as `ctrlHi` counted in pulses.
Top rate = `ctrlHi` ÷ 15 µm, rounded down, as the firmware works it
out (`Config::max_rate`): each pulse is 15 µm of belt (96 mm ÷ 6,400, at
1/32, `[drive] microsteps`). The corner speed uses the same L = 3.2 mH
and I = 2.69 A as the rules above.

| Supply | Corner speed | Cart speed (`ctrlHi`) | Top rate |
|---|---|---|---|
| 12 V | 697 full steps/s | 0.334 m/s | 22,266 pulses/s |
| 24 V | 1,394 full steps/s | 0.669 m/s | 44,600 pulses/s |
| 30 V | 1,743 full steps/s | 0.836 m/s | 55,733 pulses/s |

- **The driver is not the limit.** The DM542 takes up to 200,000
  pulses/s (Leadshine DM542 manual V1.0, section 2, via
  `control-design.md`), above every rate here.
- **The corner speed is an estimate**, where the torque starts to fall,
  not where the motor stops (*What the maker's figures change*).
- **The current.** I = 2.69 A RMS, the DM542's setting, as in the
  rules above. At the DRV8825's 2.5 A, used until 2026-10-29, the rates
  were 24,000, 48,000 and 60,000 pulses/s, about 7% higher.

### 12 V (ideal)

12 ÷ (2 × 0.0032 × 2.69) = 697.0 full steps/s = 3.485 rev/s = 0.3346 m/s,
rounded down to 0.334 m/s.

| `mj` field | Value |
|---|---|
| `type` | 1 (velocity) |
| `group` | 0 |
| `delay` | 0 |
| `ctrllimited` | 1 |
| `ctrlLo` | −0.334 |
| `ctrlHi` | 0.334 |
| `forcelimited` | 1 |
| `forceLo` | −106 |
| `forceHi` | 106 |
| `gear` | 1, 0, 0, 0, 0, 0 |
| `kv` | 100 |
| `damping` | 0 |
| `armature` | 0.227 |
| `kp`, `dampratio`, `timeconst`, `cranklength`, `lengthrangeLo/Hi`, `inheritrange` | unchanged (unused by `<velocity>`) |

### 24 V (acceptable)

24 ÷ (2 × 0.0032 × 2.69) = 1,394.1 full steps/s = 6.970 rev/s = 0.6691 m/s,
rounded down to 0.669 m/s. The asset holds the 2.5 A row, ±0.72 and ±98
(read 2026-10-27), until it is re-exported.

| `mj` field | Value |
|---|---|
| `type` | 1 (velocity) |
| `group` | 0 |
| `delay` | 0 |
| `ctrllimited` | 1 |
| `ctrlLo` | −0.669 |
| `ctrlHi` | 0.669 |
| `forcelimited` | 1 |
| `forceLo` | −106 |
| `forceHi` | 106 |
| `gear` | 1, 0, 0, 0, 0, 0 |
| `kv` | 100 |
| `damping` | 0 |
| `armature` | 0.227 |
| `kp`, `dampratio`, `timeconst`, `cranklength`, `lengthrangeLo/Hi`, `inheritrange` | unchanged (unused by `<velocity>`) |

### 30 V (last resort)

30 ÷ (2 × 0.0032 × 2.69) = 1,742.6 full steps/s = 8.713 rev/s = 0.8364 m/s,
rounded down to 0.836 m/s. The asset held 0.85, from the older 3.4 mH and
2.5 A, until it moved to the 24 V row (±0.72, read 2026-10-27).

| `mj` field | Value |
|---|---|
| `type` | 1 (velocity) |
| `group` | 0 |
| `delay` | 0 |
| `ctrllimited` | 1 |
| `ctrlLo` | −0.836 |
| `ctrlHi` | 0.836 |
| `forcelimited` | 1 |
| `forceLo` | −106 |
| `forceHi` | 106 |
| `gear` | 1, 0, 0, 0, 0, 0 |
| `kv` | 100 |
| `damping` | 0 |
| `armature` | 0.227 |
| `kp`, `dampratio`, `timeconst`, `cranklength`, `lengthrangeLo/Hi`, `inheritrange` | unchanged (unused by `<velocity>`) |

### What changes with it outside the asset

The speed range is one figure across the loop. Changing the asset's
`ctrlHi` changes these too:

| Value | 12 V | 24 V | 30 V | Where |
|---|---|---|---|---|
| Top rate, `ctrlHi` ÷ 15 µm | 22,266 pulses/s | 44,600 pulses/s | 55,733 pulses/s | derived at each run from the MJCF's `ctrlrange` and `[drive]`: the synthetic Arduino's clamp (`Config::max_rate`), and the trainer's command bound and linear-rate action scale (the run's values from the orchestrator) |
| `b` floor, `ctrlHi` × 16 ms | 5.3 mm | 10.7 mm | 13.4 mm | the gym file's `[end.strike_gap] buffer` at least this (the strike-gap plugin computes it from the run's cart top speed) |
| Force at top speed, from the curve | no curve | about 38 N (85 N·cm at 418 rpm, read between the curve's 350 and 500 rpm points, scaled to 2.69 A) | no curve | none: a constant `forcerange` cannot follow it |

The 24 V figure is the only top-speed force with maker data, because the
torque curve was measured at 24 V (at 4.0 A). Scaling it to 2.69 A by
current is an assumption. No curve is published at 12 V or 30 V.
