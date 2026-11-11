use crate::{
    constants::{DM542_ENA_LEAD_NS, DM542_STEP_HIGH_MIN_NS, TICK_COUNT_NS},
    direction::Direction,
    error::{AppError, result::AppResult},
    settings::{DIR_SETUP_NS, PULSE_LOW_NS, TRIP_US},
    sleep,
    switch::Switch,
    wake::Wake,
};

use core::{
    fmt::{self, Write},
    time::Duration,
};
use embedded_hal::digital::PinState;
use ra4m1::{GPT164, GPT165, GPT167, GPT321};
use unor4_hal::{
    elc::ElcE,
    gpio::{
        Pull, Switch as SwitchInput, SwitchPinBuilder, SwitchSetup,
        contact::Nc,
        pin_map::{self, D2, D3, D4, D5, D6},
    },
    step::{
        CounterBuilder, CounterSetup, DirPinBuilder, DirSetup, EnaPinBuilder, EnaSetup, NoDivider,
        PulPinBuilder, PulSetup, StepError, StepGenerator, StepGeneratorBuilder, StepSetup,
        TickBuilder, TickSetup,
    },
    wake::{WakeSetup, WakeTimer, WakeTimerBuilder},
};

// The rig's pins, as __documentation__/control-design.md wires them. Each output reaches the
// DM542 driver through a PNP transistor, so a low pin lights its input and a
// high one leaves it unlit (__documentation__/control-design.md, *Why the transistors*). Every
// setup is a named `const`, so `cargo check` runs the HAL's checks on it (the
// HAL's README, *Builders: write them as named `const`*).

/// PUL on D4 (GPT320 output B): rests HIGH, LOW for `pulse_low_ns` per pulse,
/// then HIGH for at least the DM542's off time.
const PUL: PulSetup<D4, NoDivider> = PulPinBuilder::new()
    .with_pin(pin_map::d4)
    .with_timing(PULSE_LOW_NS, DM542_STEP_HIGH_MIN_NS)
    .build();

/// DIR on D5, `dir_setup_ns` before the next pulse after each change.
const DIR: DirSetup<D5> = DirPinBuilder::new()
    .with_pin(pin_map::d5)
    .with_setup(DIR_SETUP_NS)
    .with_forward(Direction::FORWARD.level())
    .build();

/// ENA on D6: a high pin leaves the driver's ENA input unlit, which enables
/// it; a low one lights it and frees the motor (__documentation__/control-design.md, ENA+).
const ENA: EnaSetup<D6> = EnaPinBuilder::new()
    .with_pin(pin_map::d6)
    .with_enabled(PinState::High)
    .with_lead(DM542_ENA_LEAD_NS)
    .build();

/// GPT321 counts every pulse in hardware; GPT164 and GPT165 make the 2 ms
/// tick and number it. The HAL guide's choice; nothing else here uses them.
const COUNTER: CounterSetup<GPT321> = CounterBuilder::new().with_timer::<GPT321>().build();
const TICK: TickSetup<GPT164, GPT165> =
    TickBuilder::new().with_timers::<GPT164, GPT165>().build();

/// The step generator, from the parts above.
pub const STEPPER: StepSetup<D4, NoDivider, D5, GPT321, GPT164, GPT165, D6> =
    StepGeneratorBuilder::new()
        .with_pul(PUL)
        .with_dir(DIR)
        .with_ena(ENA)
        .with_counter(COUNTER)
        .with_tick(TICK)
        .build();

/// The limit switches' NC contacts, on D2 and D3: high when pressed, and so is
/// a broken wire. The board's own pull-ups are the lines' only ones
/// (__documentation__/control-design.md). The edge latch (`with_edge`) catches each press in
/// hardware, even one released before the processor looks, and `with_wake`
/// makes the press end a `WFI`, so a move sleeps until a switch is pressed
/// instead of reading it over and over (the HAL's README, *Switches*).
pub const D2_SWITCH: SwitchSetup<D2, Nc> = SwitchPinBuilder::new()
    .with_pin(pin_map::d2)
    .with_contact(Nc)
    .with_pull(Pull::Internal)
    .with_edge()
    .with_wake()
    .build();
pub const D3_SWITCH: SwitchSetup<D3, Nc> = SwitchPinBuilder::new()
    .with_pin(pin_map::d3)
    .with_contact(Nc)
    .with_pull(Pull::Internal)
    .with_edge()
    .with_wake()
    .build();

/// The wake timer: a one-shot wait that ends a `WFI` once, for every wait that
/// is not the motor's (the ENA lead, a switch's `trip_us`). GPT167 is free, and
/// it stops itself through ELC input E, since the step generator holds B, C
/// and D (the HAL's README, *Wake timer*).
pub const WAKE: WakeSetup<GPT167, ElcE> = WakeTimerBuilder::new()
    .with_timer::<GPT167>()
    .with_input::<ElcE>()
    .build();

/// How long a switch must read pressed without a break to count (`trip_us`).
const TRIP: Duration = Duration::from_micros(TRIP_US as u64);

/// The rig's pins, owned by the HAL's drivers: the step generator makes PUL,
/// sets DIR and drives ENA, and counts every pulse; the two switches are read
/// through their contact type; the wake timer ends each wait that is not the
/// motor's.
pub struct Pins {
    stepper: StepGenerator,
    d2: SwitchInput<D2, Nc>,
    d3: SwitchInput<D3, Nc>,
    wake: WakeTimer,
    /// The direction of the latest move: the level the generator was told to
    /// hold on DIR. The forward level until the first move (the HAL's build).
    direction: Direction,
    /// ICLK in Hz, for timing a move.
    sysclk: u32,
}

impl Pins {
    /// Takes the built drivers and enables the motor driver: ENA to the
    /// enabling level, then the DM542's lead (5 µs) slept through on the wake
    /// timer, so the driver holds the motor still until the first move. The
    /// HAL's `is_enabled()` may turn true up to 2 tick-timer counts after the
    /// lead, so after it the board sleeps one count at a time until it does.
    /// Interrupts must be unmasked first (`intc::enable_interrupts`): the wake
    /// timer's interrupt ends each sleep.
    pub fn new(
        mut stepper: StepGenerator,
        d2: SwitchInput<D2, Nc>,
        d3: SwitchInput<D3, Nc>,
        mut wake: WakeTimer,
        sysclk: u32,
    ) -> AppResult<Self> {
        // A count stop wakes the processor. The HAL enables that wake-up only
        // while a count is armed, so it stays on.
        stepper.set_count_wake(true);
        stepper.enable();
        nap(&mut wake, Duration::from_nanos(u64::from(DM542_ENA_LEAD_NS)))?;
        while !stepper.is_enabled() {
            nap(&mut wake, Duration::from_nanos(TICK_COUNT_NS))?;
        }
        Ok(Self {
            stepper,
            d2,
            d3,
            wake,
            direction: Direction::FORWARD,
            sysclk,
        })
    }

    pub fn sysclk(&self) -> u32 {
        self.sysclk
    }

    /// Whether `switch` is pressed. The NC contact opens when the cart
    /// presses it, and so does a broken wire: both read high.
    pub fn pressed(&self, switch: Switch) -> bool {
        match switch {
            Switch::D2 => self.d2.is_active(),
            Switch::D3 => self.d3.is_active(),
        }
    }

    /// Whether `switch` was pressed since the last look, and clears that.
    fn edge(&mut self, switch: Switch) -> bool {
        match switch {
            Switch::D2 => self.d2.activated(),
            Switch::D3 => self.d3.activated(),
        }
    }

    /// Clears both switches' edge latches: a press before this is not the
    /// next move's.
    pub fn clear_edges(&mut self) {
        for switch in Switch::ALL {
            let _ = self.edge(switch);
        }
    }

    /// Whether `switch` reads pressed without a break for `trip_us`, so a
    /// spike on its wire is not taken for a pressed switch. It reads the pin
    /// now and again when the wake timer ends `trip_us` later, asleep in
    /// between. A release and press again between the two readings leaves
    /// the edge latch set, so that break is seen too.
    pub fn pressed_steady(&mut self, switch: Switch) -> AppResult<bool> {
        let _ = self.edge(switch);
        if !self.pressed(switch) {
            return Ok(false);
        }
        nap(&mut self.wake, TRIP)?;
        Ok(self.pressed(switch) && !self.edge(switch))
    }

    /// Starts the wake timer for `cycles` ICLK cycles from now, rounded up to
    /// whole ns, replacing a wait still running: its end wakes a move
    /// (`Wake::timer`). Zero has ended at once.
    pub fn wake_in(&mut self, cycles: u64) -> AppResult<()> {
        let ns = (cycles * 1_000_000_000).div_ceil(u64::from(self.sysclk));
        self.wake
            .start(Duration::from_nanos(ns))
            .map_err(AppError::wake)
    }

    /// Stops the wake timer's wait, if one runs; nothing fires.
    pub fn cancel_wake(&mut self) {
        self.wake.cancel();
    }

    /// Starts the cart from rest, `direction` at `speed` pulses/s, and returns
    /// the pulse count at the start, for `moved`. With `count`, the generator
    /// stops in hardware after exactly that many pulses, at least 1 (the
    /// HAL's `stop_at`), and the stop wakes the processor (`Wake::counted`).
    /// From here until `halt` each 2 ms tick wakes it too. From rest `queue`
    /// sets DIR at least the DIR set-up time (`dir_setup_ns`) before the
    /// first pulse (the HAL's `queue` doc). The rate starts at the next tick,
    /// so this returns within two ticks, asleep between them; `NotReady` and
    /// `TooLate` only mean the next tick.
    pub fn start(
        &mut self,
        direction: Direction,
        speed: u32,
        count: Option<u32>,
    ) -> AppResult<u32> {
        let speed = i32::try_from(speed).map_err(|_| AppError::stepper(StepError::RateTooHigh))?;
        let rate = direction.rate(speed);
        let start = self.stepper.pulses();
        if let Some(count) = count {
            self.stepper
                .stop_at(start.wrapping_add(count))
                .map_err(AppError::stepper)?;
        }
        self.stepper.set_tick_wake(true);
        loop {
            // `queue` needs the latest tick polled.
            sleep::until(|| self.stepper.poll_tick());
            match self.stepper.queue(rate) {
                Ok(_) => break,
                Err(StepError::NotReady | StepError::TooLate) => {}
                Err(error) => {
                    self.halt(start);
                    return Err(AppError::stepper(error));
                }
            }
        }
        self.direction = direction;
        Ok(start)
    }

    /// What woke the move, for `sleep::until`: a tick (polled here, as the
    /// HAL asks at least once per 2 ms tick while the generator runs), a
    /// press on either switch, the count stop, the wake timer's end. Each is
    /// read once and cleared.
    pub fn woke(&mut self) -> Wake {
        Wake {
            tick: self.stepper.poll_tick().is_some(),
            edges: Switch::ALL.map(|switch| self.edge(switch)),
            counted: self.stepper.stopped_at_count(),
            timer: self.wake.expired(),
        }
    }

    /// Pulses since `start` (`start`'s return), counted in hardware.
    pub fn moved(&self, start: u32) -> u32 {
        self.stepper.pulses().wrapping_sub(start)
    }

    /// Stops the cart in one jump, ends the ticks' wake-ups and the wake
    /// timer's wait, and returns the pulses since `start`. The HAL never cuts
    /// a pulse and returns once the counter has stopped, so the count
    /// includes the last pulse. It cancels a count stop still armed; after
    /// one that fired, the generator is already stopped and `stop` does
    /// nothing more (the HAL's `stop` doc).
    pub fn halt(&mut self, start: u32) -> u32 {
        self.stepper.stop();
        self.stepper.set_tick_wake(false);
        self.wake.cancel();
        self.moved(start)
    }

    /// Every pin's level now as JSON, 1 = high, 0 = low: the levels and no
    /// meanings, which are the Orin page's. The switches are read from the
    /// pin. The HAL cannot read back a pin its step generator owns, so the
    /// outputs are the levels the generator was told to hold, which it holds
    /// whenever the board can answer, between moves: STEP HIGH (every stop
    /// ends HIGH, the HAL's `stop` doc), DIR the latest move's direction, ENA
    /// HIGH once the driver is enabled.
    pub fn write_levels(&self, out: &mut impl Write) -> fmt::Result {
        write!(
            out,
            "{{\"d2\":{},\"d3\":{},\"d4\":1,\"d5\":{},\"d6\":{}}}",
            u8::from(self.d2.is_active()),
            u8::from(self.d3.is_active()),
            u8::from(self.direction == Direction::High),
            u8::from(self.stepper.is_enabled()),
        )
    }
}

/// Sleeps for `wait` on the wake timer: at least that long, since the timer
/// rounds up. Other interrupts, such as a byte from the WiFi chip, end a `WFI`
/// early; the sleep goes on until the wait has ended.
fn nap(wake: &mut WakeTimer, wait: Duration) -> AppResult<()> {
    wake.start(wait).map_err(AppError::wake)?;
    sleep::until(|| wake.expired().then_some(()));
    Ok(())
}
