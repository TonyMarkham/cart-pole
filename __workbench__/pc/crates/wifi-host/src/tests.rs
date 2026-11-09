//! Checks of the WiFi test's arithmetic, each against a value worked out by
//! hand.

use crate::{
    board::parse_report, clock_fit::ClockFit, stats::Stats, sync_exchange::SyncExchange,
    widener::Widener,
};

/// 48 counts is 1 µs at 48 MHz, so 4,800 counts held between t1 and t2 is
/// 100 µs; the PC waited 1,000 µs, so the round trip is 900 µs, and the
/// midpoint is halfway between t0 and t3.
#[test]
fn given_a_sync_when_measured_then_the_r4s_hold_is_taken_off_its_round_trip() {
    let exchange = SyncExchange {
        number: 0,
        t0_ns: 1_000_000.0,
        t1: 10_000,
        t2: 14_800,
        t3_ns: 2_000_000.0,
    };
    assert!((exchange.round_trip_ns() - 900_000.0).abs() < 1e-6);
    assert_eq!(exchange.pc_mid_ns(), 1_500_000.0);
}

/// The two stamps straddle a wrap: t1 just below 2³², t2 just past it.
#[test]
fn given_stamps_across_a_wrap_when_held_time_is_taken_then_it_is_the_short_way_round() {
    let exchange = SyncExchange {
        number: 0,
        t0_ns: 0.0,
        t1: u32::MAX - 23,
        t2: 24,
        t3_ns: 10_000.0,
    };
    // 48 counts = 1 µs.
    assert!((exchange.round_trip_ns() - 9_000.0).abs() < 1e-6);
}

/// 100 s after the first stamp the R4 has counted 4.8 × 10⁹, past one wrap
/// (4,294,967,296): the raw stamp reads 505,032,704 and must widen to 4.8 × 10⁹.
#[test]
fn given_a_stamp_past_a_wrap_when_widened_then_the_wrap_is_added() {
    let widener = Widener::new(0, 0.0);
    let raw = (4_800_000_000u64 - 4_294_967_296) as u32;
    assert_eq!(raw, 505_032_704);
    assert_eq!(widener.widen(raw, 100e9), 4_800_000_000.0);
}

/// Points exactly on pc = 1,000 + 20 × count: the fit finds both.
#[test]
fn given_points_on_a_line_when_fitted_then_the_line_is_found() {
    let points = [(0.0, 1_000.0), (10.0, 1_200.0), (20.0, 1_400.0)];
    let fit = ClockFit::fit(&points).unwrap();
    assert!((fit.rate_ns_per_count - 20.0).abs() < 1e-9);
    assert!((fit.offset_ns - 1_000.0).abs() < 1e-9);
    assert!((fit.pc_ns(30.0) - 1_600.0).abs() < 1e-9);
}

#[test]
fn given_one_point_when_fitted_then_it_is_refused() {
    assert!(ClockFit::fit(&[(0.0, 0.0)]).is_err());
}

/// 1 to 10: mean 5.5; sample variance Σ(x − 5.5)² / 9 = 82.5 / 9, so the
/// standard deviation is √9.1667 ≈ 3.0277. Nearest rank: 50% of 10 is the
/// 5th value, 5; 90% the 9th, 9; 99% rounds up to the 10th, 10.
#[test]
fn given_one_to_ten_when_summed_up_then_the_stats_are_the_hand_worked_ones() {
    let values: Vec<f64> = (1..=10).map(f64::from).collect();
    let stats = Stats::of(&values).unwrap();
    assert_eq!(stats.count, 10);
    assert!((stats.mean - 5.5).abs() < 1e-12);
    assert!((stats.std_dev - (82.5f64 / 9.0).sqrt()).abs() < 1e-12);
    assert_eq!(stats.percentiles[0], (50.0, 5.0));
    assert_eq!(stats.percentiles[1], (90.0, 9.0));
    assert_eq!(stats.percentiles[2], (99.0, 10.0));
}

#[test]
fn given_a_report_line_when_parsed_then_its_payload_and_stamp_come_out() {
    assert_eq!(
        parse_report("PING 01AE09 0001E240"),
        Some(([0x01, 0xAE, 0x09], 123_456))
    );
    assert_eq!(parse_report("PING 1AE09 0001E240"), None);
    assert_eq!(parse_report("SYNC 00000001 0 0"), None);
}
