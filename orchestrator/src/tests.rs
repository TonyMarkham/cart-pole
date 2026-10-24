use crate::latency::Latency;

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
