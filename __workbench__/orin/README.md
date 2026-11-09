# The camera test's LED: switching it on and off

The camera test (gap 1, `../README.md`, *The camera test*) needs an LED that
the Orin can switch on and off. This file says how it is wired, how to switch
it from the Orin's shell, and the one fix the pin needs before it will drive.
Everything here was done on the Orin on 2026-11-08.

## The wiring

The LED and a 330 Ω resistor are wired in a row between two pins of the
Orin's 40-pin header:

- **pin 33:** the switched pin. 3.3 V when on, 0 V when off.
- **pin 34:** ground, 0 V always.

`led-circuit.svg` is the drawing. Pin 29, the first choice, read 0 V while
held high, and was not used.

Pin 33 has four names, one per layer. Jetson.GPIO's pin table on the Orin
(`/usr/lib/python3/dist-packages/Jetson/GPIO/gpio_pin_data.py`) and NVIDIA's
carrier-board table give the same four:

| Layer | Name |
|---|---|
| The header | pin 33 |
| The carrier board | GPIO13 |
| The chip | `PH.00` (port H, line 0) |
| Linux | `gpiochip0` line 43 |

`camera-timing.toml`'s `[led] line_name` is the Linux name of the line as
`gpioinfo` prints it: `PH.00`.

## The fix the pin needs: switch its output driver on

**Why:** the chip has one control register per pin, separate from Linux's
GPIO line. On 2026-11-08 pin 33's register had the pin's output driver
switched off, so pin 33 read 0 V while Linux reported line 43 as an output
held high. Linux's GPIO tools (`gpioset`, `gpioinfo`) don't change that
register, so they can't fix it.

**The register:** address `0x02434040`. That is the pin-settings block at
`0x2430000` (the kernel's `/sys/kernel/debug/pinctrl/2430000.pinmux`, which
lists `SOC_GPIO21_PH0`) plus `0x4040`, the offset the kernel's pin driver
gives for that pin (`drivers/pinctrl/tegra/pinctrl-tegra234.c`, the
`PINGROUP(soc_gpio21_ph0, …, 0x4040, …)` line).

**Read it** (changes nothing):

```bash
sudo busybox devmem 0x02434040 32
```

It read `0x00000054` before the fix. The bits that are set, from the same
kernel file:

| Bit | Value | Meaning |
|---|---|---|
| 2 | 1 | pull-down on |
| 4 | 1 | **output driver off** (the chip calls this "tristate") |
| 6 | 1 | input on |
| 10 | 0 | GPIO mode (1 would hand the pin to its special function) |

**Write it** with bit 4 cleared, `0x54` → `0x44`:

```bash
sudo busybox devmem 0x02434040 32 0x44
```

Reading it again gives `0x00000044`. The write changes only the live
register, so it is lost when the Orin reboots. After a reboot, read it first,
and write `0x44` if it isn't already.

## Switching the LED

Only one program can hold a GPIO line at a time, so whatever holds line 43
must be stopped before anything else can set it. That includes
`camera-timing` itself. `sudo` is needed because the `tony` account is not
in the `gpio` group that owns `/dev/gpiochip*`.

`--mode=signal` makes `gpioset` keep holding the line at the level given
until it is stopped. Without it, `gpioset` sets the level and exits at once.

**On:**

```bash
sudo gpioset --mode=signal gpiochip0 43=1 &
```

**Off:** stop the program holding the line high, then hold it low.
`pgrep -a gpioset` gives its process number.

```bash
pgrep -a gpioset
```

```bash
sudo kill <process number>
```

```bash
sudo gpioset --mode=signal gpiochip0 43=0 &
```

**Check** which level is held, and by whom:

```bash
sudo gpioinfo gpiochip0 | grep -E '^\s+line\s+43:'
```

It prints `line  43:  "PH.00"  "gpioset"  output  active-high [used]`
while `gpioset` holds it. A meter between pin 33 and pin 34 reads 3.3 V for
on and 0 V for off.
