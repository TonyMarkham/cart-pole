//! Every expected value here is worked out by hand from `RIG`: 6,400
//! pulses per revolution, 15 µm per pulse, ±30,000 pulses of travel,
//! 45,333 pulses/s top rate. 2,000 µs is MuJoCo's 2 ms physics step.

use crate::{RIG, Stepper, decode_command, encode_command};

const STEP_US: u32 = 2_000;

#[test]
fn rig_derived_values() {
    assert_eq!(RIG.pulses_per_rev(), 6_400); // 200 × 32
    assert_eq!(RIG.travel_pulses(), 30_000); // 450,000 µm × 6,400 / 96,000
    assert_eq!(RIG.max_rate(), 45_333); // 680,000 × 6,400 / 96,000 = 45,333.3
}

#[test]
fn packet_is_i32_little_endian() {
    assert_eq!(decode_command([0x10, 0x27, 0x00, 0x00]), 10_000); // 0x2710
    assert_eq!(decode_command([0xF0, 0xD8, 0xFF, 0xFF]), -10_000);
    assert_eq!(decode_command(encode_command(-45_333)), -45_333);
}

#[test]
fn whole_pulses_per_step() {
    // 1,000 pulses/s × 2 ms = 2 pulses each step.
    let mut s = Stepper::new(RIG, 0);
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
    let mut s = Stepper::new(RIG, 0);
    s.command(100);
    let emitted: [i32; 5] = core::array::from_fn(|_| s.advance(STEP_US));
    assert_eq!(emitted, [0, 0, 0, 0, 1]);
    let total: i32 = (0..45).map(|_| s.advance(STEP_US)).sum();
    assert_eq!(total + 1, 10);
}

#[test]
fn reverse_rate_moves_back() {
    // -2,500 pulses/s × 2 ms = -5 pulses a step.
    let mut s = Stepper::new(RIG, 100);
    s.command(-2_500);
    assert_eq!(s.advance(STEP_US), -5);
    assert_eq!(s.position(), 95);
}

#[test]
fn rate_is_clamped_to_the_top_rate() {
    let mut s = Stepper::new(RIG, 0);
    s.command(100_000);
    assert_eq!(s.rate(), 45_333);
    s.command(i32::MIN);
    assert_eq!(s.rate(), -45_333);
    // 45,333 × 1 s = 45,333 pulses, but the envelope stops it at 30,000.
    s.command(45_333);
    assert_eq!(s.advance(1_000_000), 30_000);
}

#[test]
fn envelope_stops_the_cart() {
    // 10,000 pulses/s × 2 ms = 20 pulses, but only 10 remain before the
    // +30,000 envelope; after it, none until the command points back.
    let mut s = Stepper::new(RIG, 29_990);
    s.command(10_000);
    assert_eq!(s.advance(STEP_US), 10);
    assert_eq!(s.position(), 30_000);
    assert_eq!(s.advance(STEP_US), 0);
    s.command(-10_000);
    assert_eq!(s.advance(STEP_US), -20);
}

#[test]
fn start_outside_the_envelope_is_clamped() {
    assert_eq!(Stepper::new(RIG, 40_000).position(), 30_000);
    assert_eq!(Stepper::new(RIG, -40_000).position(), -30_000);
}

#[test]
fn pulses_follow_the_rate_integral_across_a_reversal() {
    // +333 pulses/s for 30 steps (0.06 s → 19.98 pulses), then -333 for
    // 30 steps: back where it started, within one pulse.
    let mut s = Stepper::new(RIG, 0);
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

#[cfg(feature = "synthetic")]
#[test]
fn synthetic_tick_returns_the_step_signal() {
    use crate::{StepSignal, SyntheticArduino};
    // 26,667 pulses/s (0.4 m/s) × 2 ms = 53.33 pulses: 53, 53, 54 over
    // three steps, 160 pulses = 26,667 × 0.006 s rounded down.
    let mut a = SyntheticArduino::new(RIG, 0);
    a.receive(encode_command(26_667));
    let pulses: [i32; 3] = core::array::from_fn(|_| a.tick(STEP_US).pulses);
    assert_eq!(pulses, [53, 53, 54]);
    assert_eq!(a.tick(STEP_US), StepSignal { pulses: 53, micros: STEP_US });
}
