# Rig Drive Research — can the board, the driver and the motor make the top speed

The loop trains the policy against the rig's top speed, so that speed has
to be one the real drive can make: the UNO R4 WiFi generating the step
pulses, the DRV8825 taking them, and the stepper turning them into belt
travel. This doc records the numbers, every calculation behind them, and
their sources, so the question is not researched again.

Status: **research** (2026-10-24), from the parts' documentation, the
RA4M1 manual and the user's `unor4-hal`; nothing here is measured on the
rig yet. It decides nothing: the values the loop uses are in
`synthetic-loop-dia.md` (*Numbers*), `mujoco-plan.md` (*The actuator's
values*) and `arduino::RIG`.

## The answer

At 1/32 microstepping and the 30 V supply, the top rate is 56,666
pulses/s, and the electronics have plenty of headroom for it: the DRV8825
takes up to 250,000, and the board's timer builds each pulse from 847
ticks of its 48 MHz clock. What remains is the motor: 0.85 m/s is an
estimate of its corner speed, and only a run on the rig confirms it.

## The parts

| Part | What | Source |
|---|---|---|
| Board | Arduino UNO R4 WiFi: Renesas RA4M1, Arm Cortex-M4 at 48 MHz, 32 kB SRAM, 256 kB flash; an ESP32-S3 for WiFi | user, 2026-10-24; Arduino's UNO R4 WiFi page |
| Driver | TI DRV8825 | `mujoco-plan.md` |
| Motor | 23HS32-4004S: 1.8° (200 full steps/rev), 4.0 A rated, 2.4 N·m holding, 0.9 Ω and 3.4 mH per phase | third-party listings (`mujoco-plan.md`) |
| Phase current | 2.5 A, assumed | the DRV8825's maximum (below) |
| Supply | 30 V DC, the bench supply | user, 2026-10-24 |
| Belt | 48T GT2 at 2 mm pitch: 96 mm per pulley turn | the CAD part `pully-gt2-48t-6w-8b` |
| Microstepping | 1/32, fixed | `arduino::RIG` (a placeholder until the driver's setting is chosen) |

## The calculations

- **Travel per pulse:** 96 mm ÷ (200 × 32) = 96,000 µm ÷ 6,400 = **15 µm**.
- **The motor's corner speed**, where the winding inductance stops the
  current reaching its setting within one step, so torque falls off
  above it: full-step rate ≈ V ÷ (2 × L × I).
  - At 30 V: 30 ÷ (2 × 0.0034 × 2.5) = 30 ÷ 0.017 ≈ **1,765 full
    steps/s** = 8.82 rev/s; × 96 mm = **0.847 m/s**, set as 0.85 m/s in
    the StepJointSet and the MJCF's `ctrlrange`.
  - At the 24 V first planned: 24 ÷ 0.017 ≈ 1,410 steps/s = 7.06 rev/s
    = 0.68 m/s.
  - This is an estimate, not a measurement.
- **The top rate:** 850,000 µm/s ÷ 15 µm = 56,666.7, rounded down to
  **56,666 pulses/s** (`arduino::RIG.max_rate()`; 45,333 at 24 V). A
  pulse period is 1 ÷ 56,666 = **17.6 µs**.
- **The force:** holding torque scaled by current, 2.4 × 2.5 ÷ 4.0 =
  1.5 N·m; ÷ the pulley's pitch radius 15.28 mm = **98 N** at the belt
  (the MJCF's `forcerange`). It comes from the current, not the supply
  voltage, so 30 V does not change it; at the 0.909 kg moving mass it
  gives about 108 m/s². What 30 V buys is torque held to a higher speed.
- **The end-crash margin:** the farthest the cart travels between a
  capture and a stop taking effect, top speed × the longest gap: 0.85
  m/s × 16 ms = **13.6 mm** (11 mm at 24 V). The bounds are the slide's
  ends, ±0.45 m, less the margin, ±0.4364 m; through the measured fit,
  cart = 1.88452 × pivot_x − 0.93250 m, they are **Pivot x 0.26325 to
  0.72639** (`trainer-plan.md`, *What the trainer sees*).

## The driver: 56,666 pulses/s is 23% of its limit

From TI's datasheet (SLVSA73F):

- **STEP frequency:** up to **250 kHz** (7.6 *Timing Requirements*).
- **STEP pulse:** high and low each at least **1.9 µs**. A 17.6 µs
  period at 50% duty is 8.8 µs high and 8.8 µs low.
- **Setup and hold** of the command inputs around a STEP rising edge:
  650 ns each.
- **Supply:** 8.2 V to 45 V (7.3 *Recommended Operating Conditions*), so
  30 V is inside it.
- **Output current:** 2.5 A continuous at most (7.1 *Absolute Maximum
  Ratings*), subject to its thermal limits.

## The board: a hardware timer makes the pulses

- **The timer, not the CPU, makes the pulses.** The RA4M1's General PWM
  Timer (GPT) counts PCLKD, 48 MHz, with a prescaler of 1, 4, 16, 64,
  256 or 1024 and a period of up to 65,535 ticks on its 16-bit channels
  (2 × 32-bit GPT320–321 and 6 × 16-bit GPT162–167). It drives a pin by
  itself, with no work per pulse.
- **The top rate:** 48,000,000 ÷ 56,666 = 847.06, so **847 ticks** at
  prescaler 1, delivering 48,000,000 ÷ 847 = **56,671 Hz**.
- **The rate's resolution near the top:** 48 MHz ÷ 847 − 48 MHz ÷ 848 =
  56,670.6 − 56,603.8 ≈ **67 pulses/s**, about 67 × 15 µm = **1 mm/s**.
  Much finer at lower rates.
- **The slow end:** 1 pulse/s is 48,000,000 ÷ 1,024 = 46,875 ticks at
  prescaler 1/1024, inside the 65,535 limit; the slowest is 48 MHz ÷
  (1,024 × 65,535) ≈ 0.72 Hz. A rate of 0 stops the timer.
- **Counting pulses for the travel limit** (the board's edge, not the
  loop's): one GPT overflow interrupt per period, `GPTn_OVF` in the ICU
  event table, leaves 847 CPU cycles between interrupts at the top rate.
  A small counting handler uses a few percent of the CPU: comfortable,
  and the tightest spot in the chain.
- **Changing the rate without a glitch:** `GTPBR` buffers `GTPR`
  (manual §22.2.21), so a new period takes effect at the end of the
  running one.

## The libraries: the usual ones do not reach it on this board

- **AccelStepper:** about 16,700 steps/s measured on an UNO R4, through
  `run()` (Arduino forum).
- **FastAccelStepper:** 50,000 steps/s on AVR, 200,000 on ESP32 and the
  Raspberry Pi Pico, but the RA4M1 is not among its platforms (its wiki).
- **Neither is needed:** the firmware is the `arduino` crate, so what
  matters is the timer.

## The user's `unor4-hal` (sibling checkout `D:\git\unor4-hal`, 0.1.2)

An independent `no_std` Rust HAL for the UNO R4, with the RA4M1 datasheet
and user manual (rev 1.10) as Markdown in its `Research/` folder.

- **It already drives the GPT**, hardware-verified on the WiFi board:
  `pwm` runs on all six header PWM pins, each fixed to a channel (WiFi
  map: D3 and D11 on GPT321, D5 on GPT320, D6 on GPT163, D9 on GPT167,
  D10 on GPT162). Its `calc_period` picks the smallest prescaler whose
  period fits, then rounds; it refuses a frequency it can only deliver
  more than 5% off.
- **What it lacks for STEP:** `Pwm` takes its frequency once, at
  construction, but the step rate changes with every command, which
  needs a runtime rate change through `GTPBR`. It also has no overflow-
  interrupt pulse count yet; its `intc` can link the `GPTn_OVF` event.
  Both are additions to `unor4-hal`.

## Microstepping is fixed, never switched on the fly

Changing the DRV8825's microstep mode while the motor runs can make it
jump or lose steps, and every pulse count after the switch is off. The
mode is set once, on the driver's mode pins, and `arduino::RIG.microsteps`
matches it. The fallback, only if the board cannot keep up (it can, by
the numbers above), is 1/16 fixed: 30 µm a pulse, a top rate of 28,333
pulses/s.

## Still to measure on the rig

- The motor's real top speed at 30 V, against the 0.85 m/s estimate.
- The STEP rate a scope sees at the top command.
- The DRV8825's actual current setting, against the assumed 2.5 A.

## Sources

- TI DRV8825 datasheet (SLVSA73F), specifications — https://www.ti.com/document-viewer/DRV8825/datasheet/specifications
- Arduino UNO R4 WiFi — https://docs.arduino.cc/hardware/uno-r4-wifi/
- Renesas RA4M1 — https://www.renesas.com/en/products/microcontrollers-microprocessors/ra-cortex-m-mcus/ra4m1-32-bit-microcontrollers-48mhz-arm-cortex-m4-and-lcd-controller-and-cap-touch-hmi
- RA4M1 User's Manual rev 1.10, §22 GPT (§22.2.21 `GTPBR`) and the ICU
  event table — `D:\git\unor4-hal\Research\Renesas RA4M1 User Manual - 1.10.md`
- `unor4-hal` — `D:\git\unor4-hal` (`crates/unor4-hal/src/pwm/`, `CHANGELOG.md`)
- FastAccelStepper wiki — https://github.com/gin66/FastAccelStepper/wiki
- Arduino forum, UNO R4 WiFi and stepper motors — https://forum.arduino.cc/t/arduino-r4-wifi-and-stepper-motors/1160746
- The corner-speed, force and margin derivations — `mujoco-plan.md`,
  *The actuator's values*
