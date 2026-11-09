//! Checks of the apply test's parsing and summing, each against a value
//! worked out by hand.

use crate::{apply_report::ApplyReport, apply_result::ApplyResult, result_kind::ResultKind};

#[test]
fn given_the_r3s_lines_when_parsed_then_each_kind_and_its_numbers_come_out() {
    assert_eq!(
        ApplyResult::parse("A 48000 417 400"),
        Some(ApplyResult {
            kind: ResultKind::Applied,
            ticks: 48_000,
            old_period: 417,
            new_period: 400,
        })
    );
    assert_eq!(
        ApplyResult::parse("E 16 400 417").map(|result| result.kind),
        Some(ResultKind::Early)
    );
    assert_eq!(
        ApplyResult::parse("N 0 400 0").map(|result| result.kind),
        Some(ResultKind::None)
    );
    assert_eq!(ApplyResult::parse("OK stop 0"), None);
    assert_eq!(ApplyResult::parse("A 1 2"), None);
}

/// Two applied commands at 32,000 and 64,000 counts of 16 MHz: 2,000 µs and
/// 4,000 µs, so a mean of 3,000 µs. The early one and the missing one are
/// counted, not timed.
#[test]
fn given_results_when_summed_then_only_applied_delays_are_timed() {
    let applied = |ticks| {
        Some(ApplyResult {
            kind: ResultKind::Applied,
            ticks,
            old_period: 417,
            new_period: 400,
        })
    };
    let report = ApplyReport {
        results: vec![
            (40_000, applied(32_000)),
            (38_400, applied(64_000)),
            (
                40_000,
                Some(ApplyResult {
                    kind: ResultKind::Early,
                    ticks: 10,
                    old_period: 417,
                    new_period: 400,
                }),
            ),
            (38_400, None),
        ],
    };
    let summary = report.summary();
    assert!(
        summary.contains(
            "applied 2, changed before the mark 1, never changed 0, no line from the R3 1"
        ),
        "{summary}"
    );
    assert!(summary.contains("2 delays: mean 3000 us"), "{summary}");
    assert!(
        summary.contains("shortest 2000 us, longest 4000 us"),
        "{summary}"
    );
    let histogram = report.histogram_csv(1_000);
    // 2,000 µs falls in the bin from 2,000, 4,000 µs in the bin from 4,000.
    assert!(histogram.contains("2000,3000,1"), "{histogram}");
    assert!(histogram.contains("4000,5000,1"), "{histogram}");
}
