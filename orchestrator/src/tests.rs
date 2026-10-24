use crate::{
    cart_velocity,
    latency::Latency,
    meters_per_pulse,
    trainer_link::{Next, parse_answer},
};
use arduino::{Direction, Drv8825Signals, RIG};

// The answer (synthetic-loop-dia.md, Contracts, 18): continue with the
// command, reset with the next episode's seed, or stop.
#[test]
fn the_answer_is_continue_reset_or_stop() {
    let a = parse_answer(r#"{"command": -26667, "next": "continue"}"#).unwrap();
    assert_eq!((a.command, a.next, a.seed), (-26_667, Next::Continue, None));
    let a = parse_answer(r#"{"command": 0, "next": "reset", "seed": 18446744073709551615}"#).unwrap();
    assert_eq!((a.next, a.seed), (Next::Reset, Some(u64::MAX)));
    let a = parse_answer(r#"{"command": 0, "next": "stop"}"#).unwrap();
    assert_eq!(a.next, Next::Stop);
}

#[test]
fn a_bad_answer_is_refused() {
    for line in [
        r#"{"command": 0, "next": "reset"}"#,                // a reset needs a seed
        r#"{"command": 0, "next": "continue", "seed": 1}"#, // only a reset carries one
        r#"{"command": 0, "next": "pause"}"#,
        r#"{"command": 0, "continue": true}"#, // the old answer
        r#"{"command": 0, "next": "reset", "seed": -1}"#,
        r#"{"command": 2147483648, "next": "continue"}"#,
    ] {
        assert!(parse_answer(line).is_err(), "{line} was accepted");
    }
}

// SplitMix64's published outputs for seed 0 (the reference
// implementation's first values; e.g. Vigna's splitmix64.c), so the
// generator is checked against the algorithm, not against itself.
#[test]
fn splitmix64_matches_the_reference_for_seed_0() {
    let mut latency = Latency::new(0, 8.0, 15.0).unwrap();
    assert_eq!(latency.next_u64(), 0xE220_A839_7B1D_CDAF);
    assert_eq!(latency.next_u64(), 0x6E78_9E6A_A1B9_65F4);
    assert_eq!(latency.next_u64(), 0x06C4_5D18_8009_454F);
}

#[test]
fn latencies_lie_in_the_band() {
    for (min, max) in [(8.0, 15.0), (2.5, 4.0)] {
        let mut latency = Latency::new(42, min, max).unwrap();
        for _ in 0..10_000 {
            let ms = latency.next_ms();
            assert!((min..max).contains(&ms), "{ms} ms is outside [{min}, {max})");
        }
    }
}

// The band only scales u, the first seed-0 output's top 53 bits / 2^53:
// at 8-15 ms that is 8 + 7u = 14.183 ms, the first latency every seed-0
// trainer run prints; a band of zero width gives its bound.
#[test]
fn the_band_scales_the_same_draw() {
    let first = |min, max| Latency::new(0, min, max).unwrap().next_ms();
    let u = (0xE220_A839_7B1D_CDAF_u64 >> 11) as f64 / 9_007_199_254_740_992.0;
    assert_eq!(first(8.0, 15.0), 8.0 + 7.0 * u);
    assert!((first(8.0, 15.0) - 14.183).abs() < 5e-4);
    assert_eq!(first(10.0, 10.0), 10.0);
}

#[test]
fn a_band_outside_0_min_max_is_refused() {
    assert!(Latency::new(0, 15.0, 8.0).is_err());
    assert!(Latency::new(0, -1.0, 8.0).is_err());
    assert!(Latency::new(0, 8.0, f64::INFINITY).is_err());
    assert!(Latency::new(0, f64::NAN, 8.0).is_err());
}

#[test]
fn the_same_seed_gives_the_same_draws() {
    let mut a = Latency::new(7, 8.0, 15.0).unwrap();
    let mut b = Latency::new(7, 8.0, 15.0).unwrap();
    for _ in 0..100 {
        assert_eq!(a.next_ms().to_bits(), b.next_ms().to_bits());
    }
}

// synthetic-arduino-plan.md Done-when 4: ctrl = STEP rate × 15 µm, m/s,
// signed by DIR. 26,667 × 15 µm = 0.400005; 56,666 × 15 µm = 0.84999.
#[test]
fn cart_velocity_is_step_rate_times_15_um_signed_by_dir() {
    let m = meters_per_pulse(&RIG);
    assert_eq!(m, 15e-6);
    let velocity = |dir, step_rate| cart_velocity(Drv8825Signals { dir, step_rate }, m);
    assert!((velocity(Direction::Positive, 26_667) - 0.400_005).abs() < 1e-12);
    assert!((velocity(Direction::Negative, 26_667) + 0.400_005).abs() < 1e-12);
    assert!((velocity(Direction::Negative, 56_666) + 0.849_99).abs() < 1e-12);
    assert_eq!(velocity(Direction::Positive, 0), 0.0);
}
