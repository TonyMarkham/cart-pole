use crate::{cart_velocity, latency::Latency, meters_per_pulse};
use arduino::{Direction, Drv8825Signals, RIG};

// SplitMix64's published outputs for seed 0 (the reference
// implementation's first values; e.g. Vigna's splitmix64.c), so the
// generator is checked against the algorithm, not against itself.
#[test]
fn splitmix64_matches_the_reference_for_seed_0() {
    let mut latency = Latency::new(0);
    assert_eq!(latency.next_u64(), 0xE220_A839_7B1D_CDAF);
    assert_eq!(latency.next_u64(), 0x6E78_9E6A_A1B9_65F4);
    assert_eq!(latency.next_u64(), 0x06C4_5D18_8009_454F);
}

#[test]
fn latencies_lie_in_the_band() {
    let mut latency = Latency::new(42);
    for _ in 0..10_000 {
        let ms = latency.next_ms();
        assert!((8.0..15.0).contains(&ms), "{ms} ms is outside [8, 15)");
    }
}

#[test]
fn the_same_seed_gives_the_same_draws() {
    let mut a = Latency::new(7);
    let mut b = Latency::new(7);
    for _ in 0..100 {
        assert_eq!(a.next_ms().to_bits(), b.next_ms().to_bits());
    }
}

// synthetic-arduino-plan.md Done-when 4: ctrl = STEP rate × 15 µm, m/s,
// signed by DIR. 26,667 × 15 µm = 0.400005; 45,333 × 15 µm = 0.679995.
#[test]
fn cart_velocity_is_step_rate_times_15_um_signed_by_dir() {
    let m = meters_per_pulse(&RIG);
    assert_eq!(m, 15e-6);
    let velocity = |dir, step_rate| cart_velocity(Drv8825Signals { dir, step_rate }, m);
    assert!((velocity(Direction::Positive, 26_667) - 0.400_005).abs() < 1e-12);
    assert!((velocity(Direction::Negative, 26_667) + 0.400_005).abs() < 1e-12);
    assert!((velocity(Direction::Negative, 45_333) + 0.679_995).abs() < 1e-12);
    assert_eq!(velocity(Direction::Positive, 0), 0.0);
}
