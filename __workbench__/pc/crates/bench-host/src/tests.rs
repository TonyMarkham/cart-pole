//! Checks of the PC side's own rules, each against a value worked out by
//! hand.

use crate::{awake::awake_report::AwakeReport, random::Random};

fn lines(text: &str) -> Vec<String> {
    text.lines().map(str::to_owned).collect()
}

/// 10 stretches: 5 in bin 0 (0–2 µs), 4 in bin 3 (6–8 µs), 1 in bin 9
/// (18–20 µs). Half of 10 is 5, all in bin 0, so the median is at most 2 µs;
/// 90% is 9, reached in bin 3, at most 8 µs; 99% is 9.9, rounded up to 10,
/// reached in bin 9, at most 20 µs.
#[test]
fn given_a_report_when_summed_then_the_percentiles_are_the_bins_upper_edges() {
    let report = AwakeReport::parse(&lines(
        "tick_ns 500\ncount 10\nmissed 0\nover 0\nmin_ticks 2\nmax_ticks 38\n\
         sum_ticks 100\nspan_ticks 2000000\nbin_us 2\nbin 0 5\nbin 3 4\nbin 9 1\nend",
    ))
    .unwrap();
    let summary = report.summary();
    assert!(
        summary.contains("50% of stretches at most 2 us"),
        "{summary}"
    );
    assert!(
        summary.contains("90% of stretches at most 8 us"),
        "{summary}"
    );
    assert!(
        summary.contains("99% of stretches at most 20 us"),
        "{summary}"
    );
    // 38 ticks × 500 ns = 19 µs; 100 ticks / 10 = 10 ticks = 5 µs mean.
    assert!(summary.contains("longest 19.0 us"), "{summary}");
    assert!(summary.contains("mean 5.0 us"), "{summary}");
    // 2,000,000 ticks × 500 ns = 1 s; 10 stretches in it.
    assert!(summary.contains("10.0 wakes per second"), "{summary}");
}

/// A stretch beyond the histogram is no bin's, so a percentile that needs it
/// says so instead of naming a bin.
#[test]
fn given_stretches_beyond_the_histogram_when_summed_then_their_percentile_says_so() {
    let report = AwakeReport::parse(&lines(
        "tick_ns 500\ncount 2\nmissed 0\nover 1\nmin_ticks 2\nmax_ticks 9000\n\
         sum_ticks 9002\nspan_ticks 100\nbin_us 2\nbin 0 1\nend",
    ))
    .unwrap();
    let summary = report.summary();
    assert!(
        summary.contains("50% of stretches at most 2 us"),
        "{summary}"
    );
    assert!(
        summary.contains("99% of stretches: beyond the histogram"),
        "{summary}"
    );
}

#[test]
fn given_a_report_without_its_units_when_parsed_then_it_is_refused() {
    assert!(AwakeReport::parse(&lines("count 1\nend")).is_err());
}

/// SplitMix64's first output for seed 0 is 0xE220A8397B1DCDAF (Steele, Lea
/// and Flood's reference, and Java's `SplittableRandom(0).nextLong()`).
#[test]
fn given_seed_0_when_drawn_then_splitmix64_gives_its_published_first_value() {
    assert_eq!(Random::new(0).next_u64(), 0xE220_A839_7B1D_CDAF);
}

#[test]
fn given_a_range_when_drawn_many_times_then_every_draw_lies_in_it_and_both_ends_come() {
    let mut random = Random::new(1);
    let draws: Vec<i64> = (0..10_000).map(|_| random.between(-3, 3)).collect();
    assert!(draws.iter().all(|draw| (-3..=3).contains(draw)));
    assert!(draws.contains(&-3) && draws.contains(&3));
}
