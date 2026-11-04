# Saving settings on the Arduino: can the HAL write its flash?

Research, not a plan. It records what is known, from public sources, about
saving a setting on the rig's Arduino so that it survives a reset, and what is
not yet known. Written 2026-11-03.

The labels follow `real-loop-dia.md`:

- **Decided**: the user's decision.
- **Open**: not decided, or not known, and nothing proposes an answer yet.

## Why this is needed

During a balance run the Arduino must never let the cart hit a limit switch. It
ramps the cart to a stop short of each switch, at a **stop point** a set number
of pulses before the switch's remembered trip position (`real-loop-dia.md`,
*The soft limit*). That number is `runtime_end_margin`. A pulse is one step
signal to the motor driver, and moves the cart 15 µm, so 100 pulses is 1.5 mm.

Today every number the firmware uses is fixed when it is built: the board has
no file system, so its build reads `calibration.toml` and compiles the values
in (`__live__/arduino/rust/crates/calibration/calibration.toml`, the header).
Changing one means building and uploading the firmware again.

**Decided** (user, 2026-11-03):

- **`runtime_end_margin` is changed from a page, not rebuilt.** The Orin's
  webserver hosts an Arduino Settings page that changes it on the board, and the
  board saves it in its flash memory, so it survives a reset.
- **Only `runtime_end_margin`, for now.** No other setting moves to the page.
- **It binds the balance run alone.** Calibration needs to violate it: its
  seeks must reach the switches.
- **`arduino-manager` gains three options**
  (`__live__/rpi/rust/crates/arduino-manager`, the Pi's service that uploads
  firmware over USB):
  - pull the settings the board holds now;
  - upload the firmware and the settings;
  - upload the firmware only, which leaves the saved settings in flash alone.
  The firmware-and-settings upload is how a fresh board gets its first value.

So the firmware has to write to the board's flash while it runs. The firmware is
Rust, built on `unor4-hal`, the user's own **HAL** (hardware abstraction layer:
the library that turns the chip's raw control registers into functions the
firmware calls). The question this file answers as far as it can: does
`unor4-hal` support writing the flash?

## Terms

- **Code flash:** the chip's memory that holds the firmware, 256 KB on this
  chip (`arduino-manager`'s `src/flash.rs`). An upload writes it.
- **Data flash:** a separate, smaller memory on the same chip, meant for saved
  values, the way older Arduinos used their EEPROM. It keeps its contents with
  the power off.
- **Register:** a fixed address inside the chip that controls a part of it.
  Writing a number there makes the part do something.
- **PAC** (peripheral access crate): a Rust library generated from the chip
  maker's register list. It gives each register a name, so code need not write
  raw addresses. The firmware uses the `ra4m1` PAC (root `Cargo.toml`).

## What is known

- **The chip can do it.** The board's processor, a Renesas RA4M1, has 8 KB of
  data flash rated for 100,000 erase cycles, and Arduino's own EEPROM library
  for the UNO R4 keeps its data there (Arduino forum, citing the RA4M1 hardware
  manual; Arduino's docs list the 8 KB). Saving one setting now and then uses
  almost none of those cycles.
- **The firmware's PAC has no registers for it.** `ra4m1` version 0.2.0, the
  one in the root `Cargo.toml`, lists its parts on docs.rs. The only one about
  flash is `FCACHE`, the flash cache, which speeds up reading the firmware.
  None of the registers that erase or write flash are there (checked for
  `DFLCTL`, `FENTRYR`, `FPMCR`, `FASR`, `FISR`, `FSADDR`, `FCR`, `FEXCR`). So
  code that writes the data flash would use raw register addresses taken from
  Renesas's documents, not names from the PAC.
- **Renesas's documents describe how.** Application note R01AN5367, *Flash
  memory programming*, covers the RA family and puts the RA4M1 among the parts
  with the "MF3" kind of flash; it has a section on controlling the data flash.
  The RA4M1 hardware manual is the other source.
- **One prerequisite, from a Renesas erratum:** the `DFLEN` bit in the `DFLCTL`
  register must be set to 1 before the data flash can be used at all, and a
  short recovery time must pass after that before reading it or starting a
  write.
- **One warning that may apply:** a Renesas technical notice says that while a
  write runs, no interrupt handler may start another flash command, or the
  first byte fails with an illegal-command error. The firmware runs on
  interrupts (the WiFi bridge wakes it with one, `AGENTS.md`, *The real
  robot*). The notice is written about the "FACI" kind of flash controller;
  whether it applies to the RA4M1's MF3 flash is **not checked**.

## Open

- **Whether `unor4-hal` already writes the data flash.** The HAL is not
  published anywhere a web search finds; the only copy is the user's checkout at
  `D:\git\unor4-hal`, outside this repo, which `AGENTS.md` forbids reading
  without the user's word. What is above says only that the PAC beneath it does
  not help: if the HAL has no data-flash driver of its own, one is new work.
- **Whether an upload leaves the data flash alone.** The firmware-only upload
  must keep the saved settings (decided above). No source found says whether
  `arduino-cli`'s upload to the UNO R4 erases the data flash; one Arduino forum
  user meant to test it and reported nothing. It has to be tested on the board:
  save a value, upload the firmware only, read the value back.

## Sources

- [docs.rs: `ra4m1` 0.2.0, every item](https://docs.rs/ra4m1/0.2.0/ra4m1/all.html)
- [Renesas: Flash memory programming (R01AN5367)](https://www.renesas.com/en/document/apn/flash-memory-programming)
- [Renesas erratum: accessing data flash without using the device driver](https://www.renesas.com/in/en/document/tcu/errata-users-manual-hardware-accessing-data-flash-without-using-device-driver)
- [Renesas: notes on self-programming flash memory](https://www.renesas.com/eu/en/document/tcu/notes-self-programming-flash-memory-0)
- [Arduino forum: persistence of EEPROM memory on the R4](https://forum.arduino.cc/t/persistance-resilience-of-eeprom-memory/1178978/5)
- [Arduino forum: does EEPROM data survive upload?](https://forum.arduino.cc/t/does-eeprom-data-survive-upload/1043287)
- [Arduino docs commit: the UNO R4 WiFi's 8 KB data flash](https://github.com/arduino/docs-content/commit/d82259351edbfd33b174c12bed304a553a7e086b)
