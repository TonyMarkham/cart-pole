//! Every expected value here is worked out by hand from the tests' own
//! `Config`, `TEST`: 6,400 pulses per revolution, 15 µm per pulse, a top
//! pulse rate of 24,000 pulses/s, and a jump limit of 12,000,000
//! pulses/s², 24,000 a 2 ms tick, so any change from rest finishes in one
//! tick; the ramp's own tests use `RAMP`, 2,000 a tick. The `Stepper`
//! tests give it ±30,000 pulses of travel (`TRAVEL`). 2,000 µs is
//! MuJoCo's 2 ms physics step.

use crate::{Config, Ramp, Stepper, decode_command, encode_command};

// Test values, not the rig's (those are the trainer's [drive] and the
// MJCF's ctrlrange): 200 full steps × 32 microsteps, 96,000 µm a turn,
// 0.36 m/s.
const TEST: Config = Config {
    full_steps_per_rev: 200,
    microsteps: 32,
    belt_um_per_rev: 96_000,
    max_speed_um_per_s: 360_000,
    jump_limit: 12_000_000,
};

// The ramp's tests: 1,000,000 pulses/s² × 2 ms = 2,000 pulses/s a tick.
const RAMP: Config = Config {
    jump_limit: 1_000_000,
    ..TEST
};

const STEP_US: u32 = 2_000;

// The Stepper tests' envelope, pulses either side of center.
const TRAVEL: i32 = 30_000;

#[test]
fn config_derived_values() {
    assert_eq!(TEST.pulses_per_rev(), 6_400); // 200 × 32
    assert_eq!(TEST.max_rate(), 24_000); // 360,000 × 6,400 / 96,000
    // Rounded down, so the cart never passes max_speed_um_per_s:
    // 850,000 × 6,400 / 96,000 = 56,666.7.
    assert_eq!(
        Config {
            max_speed_um_per_s: 850_000,
            ..TEST
        }
        .max_rate(),
        56_666
    );
}

#[test]
fn packet_is_i32_little_endian() {
    assert_eq!(decode_command([0x10, 0x27, 0x00, 0x00]), 10_000); // 0x2710
    assert_eq!(decode_command([0xF0, 0xD8, 0xFF, 0xFF]), -10_000);
    assert_eq!(decode_command(encode_command(-24_000)), -24_000);
}

#[test]
fn whole_pulses_per_step() {
    // 1,000 pulses/s × 2 ms = 2 pulses each step.
    let mut s = Stepper::new(TEST, TRAVEL, 0);
    s.command(1_000);
    for _ in 0..10 {
        assert_eq!(s.advance(STEP_US), 2);
    }
    assert_eq!(s.position(), 20);
}

#[test]
fn slow_rate_carries_the_fraction() {
    // 100 pulses/s × 2 ms = 0.2 pulse a step: one pulse every 5th step,
    // and exactly 10 pulses in 50 steps (0.1 s).
    let mut s = Stepper::new(TEST, TRAVEL, 0);
    s.command(100);
    let emitted: [i32; 5] = core::array::from_fn(|_| s.advance(STEP_US));
    assert_eq!(emitted, [0, 0, 0, 0, 1]);
    let total: i32 = (0..45).map(|_| s.advance(STEP_US)).sum();
    assert_eq!(total + 1, 10);
}

#[test]
fn reverse_rate_moves_back() {
    // -2,500 pulses/s × 2 ms = -5 pulses a step.
    let mut s = Stepper::new(TEST, TRAVEL, 100);
    s.command(-2_500);
    assert_eq!(s.advance(STEP_US), -5);
    assert_eq!(s.position(), 95);
}

#[test]
fn rate_is_clamped_to_the_top_rate() {
    let mut s = Stepper::new(TEST, TRAVEL, 0);
    s.command(100_000);
    assert_eq!(s.command_rate(), 24_000);
    s.command(i32::MIN);
    assert_eq!(s.command_rate(), -24_000);
    // 24,000 × 2 s = 48,000 pulses, but the envelope stops it at 30,000.
    s.command(24_000);
    assert_eq!(s.advance(2_000_000), 30_000);
}

#[test]
fn envelope_stops_the_cart() {
    // 10,000 pulses/s × 2 ms = 20 pulses, but only 10 remain before the
    // +30,000 envelope; after it, none until the command points back.
    let mut s = Stepper::new(TEST, TRAVEL, 29_990);
    s.command(10_000);
    assert_eq!(s.advance(STEP_US), 10);
    assert_eq!(s.position(), 30_000);
    assert_eq!(s.advance(STEP_US), 0);
    s.command(-10_000);
    assert_eq!(s.advance(STEP_US), -20);
}

#[test]
fn start_outside_the_envelope_is_clamped() {
    assert_eq!(Stepper::new(TEST, TRAVEL, 40_000).position(), 30_000);
    assert_eq!(Stepper::new(TEST, TRAVEL, -40_000).position(), -30_000);
}

#[test]
fn pulses_follow_the_rate_integral_across_a_reversal() {
    // +333 pulses/s for 30 steps (0.06 s → 19.98 pulses), then -333 for
    // 30 steps: back where it started, within one pulse.
    let mut s = Stepper::new(TEST, TRAVEL, 0);
    s.command(333);
    for _ in 0..30 {
        s.advance(STEP_US);
    }
    assert_eq!(s.position(), 19);
    s.command(-333);
    for _ in 0..30 {
        s.advance(STEP_US);
    }
    assert_eq!(s.position(), 0);
}

// A fresh synthetic Arduino on TEST, sent `rate`, then one tick with both
// switches open: from rest TEST's ramp reaches any rate in that one tick.
#[cfg(feature = "synthetic")]
fn received_then_ticked(rate: i32) -> crate::TickAnswer {
    use crate::{LimitSwitches, SyntheticArduino};
    let mut a = SyntheticArduino::new(TEST, STEP_US);
    a.receive(encode_command(rate));
    a.tick(LimitSwitches::OPEN)
}

#[cfg(feature = "synthetic")]
#[test]
fn synthetic_is_created_with_its_config_alone() {
    use crate::SyntheticArduino;
    // synthetic-arduino-plan.md Done-when 1: no start position is passed.
    assert_eq!(*SyntheticArduino::new(TEST, STEP_US).config(), TEST);
}

#[cfg(feature = "synthetic")]
#[test]
fn synthetic_receive_returns_dir_and_step_rate() {
    use crate::{Direction, Dm542Signals};
    // Done-when 2: +r and -r give opposite DIRs and the same STEP rate;
    // 0 gives STEP rate 0. 20,000 pulses/s is 0.3 m/s at 15 µm a pulse.
    // A fresh one each time: from rest, TEST's ramp reaches any rate in
    // the one tick after receive.
    let signals = |rate| received_then_ticked(rate).signals;
    assert_eq!(
        signals(20_000),
        Dm542Signals {
            dir: Direction::Positive,
            step_rate: 20_000
        }
    );
    assert_eq!(
        signals(-20_000),
        Dm542Signals {
            dir: Direction::Negative,
            step_rate: 20_000
        }
    );
    assert_eq!(signals(0).step_rate, 0);
}

#[cfg(feature = "synthetic")]
#[test]
fn synthetic_receive_clamps_to_the_top_rate() {
    use crate::{Direction, Dm542Signals};
    // Done-when 3: past ± the top pulse rate (24,000) the STEP rate is
    // 24,000, DIR from the sign; i32::MIN has no positive i32 twin, the
    // clamp's edge.
    let signals = |rate| received_then_ticked(rate).signals;
    assert_eq!(
        signals(24_001),
        Dm542Signals {
            dir: Direction::Positive,
            step_rate: 24_000
        }
    );
    assert_eq!(
        signals(i32::MAX),
        Dm542Signals {
            dir: Direction::Positive,
            step_rate: 24_000
        }
    );
    assert_eq!(
        signals(i32::MIN),
        Dm542Signals {
            dir: Direction::Negative,
            step_rate: 24_000
        }
    );
    assert_eq!(
        signals(-24_000),
        Dm542Signals {
            dir: Direction::Negative,
            step_rate: 24_000
        }
    );
}

#[test]
fn ramp_limit_rounds_down() {
    // m × the tick, rounded down: the rig's 1,210,937 × 2 ms = 2,421.874.
    assert_eq!(
        Config {
            jump_limit: 1_210_937,
            ..TEST
        }
        .ramp_limit(STEP_US),
        2_421
    );
    assert_eq!(RAMP.ramp_limit(STEP_US), 2_000);
}

#[test]
fn ramp_moves_at_most_m_dt_a_tick() {
    // 0 -> 10,000 at 2,000 a tick: five ticks, then it holds.
    let mut r = Ramp::new();
    r.command(&RAMP, 10_000);
    let rates: [i32; 6] = core::array::from_fn(|_| r.tick(&RAMP, STEP_US));
    assert_eq!(rates, [2_000, 4_000, 6_000, 8_000, 10_000, 10_000]);
}

#[test]
fn ramp_small_change_finishes_in_one_tick() {
    let mut r = Ramp::new();
    r.command(&RAMP, 1_500);
    assert_eq!(r.tick(&RAMP, STEP_US), 1_500);
    r.command(&RAMP, -500);
    assert_eq!(r.tick(&RAMP, STEP_US), -500);
}

#[test]
fn ramp_reversal_takes_its_time() {
    // +24,000 -> -24,000 is 48,000 at 2,000 a tick: 24 ticks.
    let mut r = Ramp::new();
    r.command(&RAMP, 24_000);
    for _ in 0..12 {
        r.tick(&RAMP, STEP_US);
    }
    assert_eq!(r.rate(), 24_000);
    r.command(&RAMP, -24_000);
    for _ in 0..23 {
        r.tick(&RAMP, STEP_US);
    }
    assert_eq!(r.rate(), -22_000);
    assert_eq!(r.tick(&RAMP, STEP_US), -24_000);
}

#[test]
fn ramp_retargets_mid_way() {
    // Heading for 10,000; at 4,000 the command becomes 0: back down.
    let mut r = Ramp::new();
    r.command(&RAMP, 10_000);
    r.tick(&RAMP, STEP_US);
    assert_eq!(r.tick(&RAMP, STEP_US), 4_000);
    r.command(&RAMP, 0);
    assert_eq!(r.tick(&RAMP, STEP_US), 2_000);
    assert_eq!(r.tick(&RAMP, STEP_US), 0);
}

#[test]
fn ramp_clamps_the_command() {
    let mut r = Ramp::new();
    r.command(&RAMP, i32::MAX);
    assert_eq!(r.command_rate(), 24_000);
}

#[test]
fn stepper_pulses_follow_the_ramped_rate() {
    // 2,000 pulses/s × 2 ms = 4 pulses, then 4,000 × 2 ms = 8.
    let mut s = Stepper::new(RAMP, TRAVEL, 0);
    s.command(10_000);
    assert_eq!(s.rate(), 0);
    assert_eq!(s.advance(STEP_US), 4);
    assert_eq!(s.advance(STEP_US), 8);
    assert_eq!(s.rate(), 4_000);
}

#[cfg(feature = "synthetic")]
#[test]
fn synthetic_ticks_the_ramp() {
    use crate::{LimitSwitches, SyntheticArduino};
    // Done-when 5: receive sets the command alone, and every tick moves the
    // sent rate by at most RAMP's 2,000: 0 -> 5,000 is 2,000, 4,000, then
    // 5,000; reset returns to rest.
    let mut a = SyntheticArduino::new(RAMP, STEP_US);
    let rate = |a: &mut SyntheticArduino| a.tick(LimitSwitches::OPEN).signals.step_rate;
    a.receive(encode_command(5_000));
    assert_eq!(rate(&mut a), 2_000);
    assert_eq!(rate(&mut a), 4_000);
    assert_eq!(rate(&mut a), 5_000);
    assert_eq!(rate(&mut a), 5_000);
    a.reset();
    assert_eq!(rate(&mut a), 0);
}

#[cfg(feature = "synthetic")]
#[test]
fn synthetic_receive_does_not_tick() {
    use crate::{LimitSwitches, SyntheticArduino};
    // A2: the sent rate moves only at the tick. Two commands before one
    // tick: the last one is what the tick heads for (1,500 is within one
    // tick of RAMP's 2,000).
    let mut a = SyntheticArduino::new(RAMP, STEP_US);
    a.receive(encode_command(5_000));
    a.receive(encode_command(1_500));
    assert_eq!(a.tick(LimitSwitches::OPEN).signals.step_rate, 1_500);
}

#[cfg(feature = "synthetic")]
#[test]
fn a_closed_limit_switch_is_a_fault_until_reset() {
    use crate::{Direction, Dm542Signals, LimitSwitches, SyntheticArduino};
    // Done-when 6 (A8), on RAMP, 2,000 a tick.
    let left = LimitSwitches {
        left: true,
        right: false,
    };
    let stopped = Dm542Signals {
        dir: Direction::Positive,
        step_rate: 0,
    };
    let mut a = SyntheticArduino::new(RAMP, STEP_US);
    a.receive(encode_command(5_000));
    let first = a.tick(LimitSwitches::OPEN);
    assert_eq!((first.signals.step_rate, first.fault), (2_000, false));
    // The trip: the cart stops in one jump, 2,000 -> 0, past the ramp.
    let trip = a.tick(left);
    assert_eq!((trip.signals, trip.fault), (stopped, true));
    // Still FAULT with the switch open again, and a command is not applied.
    a.receive(encode_command(3_000));
    let after = a.tick(LimitSwitches::OPEN);
    assert_eq!((after.signals, after.fault), (stopped, true));
    assert!(a.fault());
    // R3: out of FAULT, at rest; the next command ramps from 0 again.
    a.reset();
    assert!(!a.fault());
    let rest = a.tick(LimitSwitches::OPEN);
    assert_eq!((rest.signals.step_rate, rest.fault), (0, false));
    a.receive(encode_command(3_000));
    assert_eq!(a.tick(LimitSwitches::OPEN).signals.step_rate, 2_000);
}

#[cfg(feature = "synthetic")]
#[test]
fn either_limit_switch_trips() {
    use crate::{LimitSwitches, SyntheticArduino};
    // A8: the right switch alone, or both, trip as the left does.
    for switches in [
        LimitSwitches {
            left: false,
            right: true,
        },
        LimitSwitches {
            left: true,
            right: true,
        },
    ] {
        let mut a = SyntheticArduino::new(RAMP, STEP_US);
        a.receive(encode_command(-5_000));
        let answer = a.tick(switches);
        assert_eq!((answer.signals.step_rate, answer.fault), (0, true));
    }
}
