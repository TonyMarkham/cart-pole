use crate::{
    kind::Kind,
    profile::{MAX_SEGMENTS, Ramping, Segment, excursion},
    protocol::{Command, Line},
    settings::Experiment,
};

/// bench-station.toml's [experiment] values.
fn experiment() -> Experiment {
    Experiment {
        conditions: String::new(),
        trial_top: 48_000,
        trial_dt_ms: 16,
        j_start: 620_000,
        j_growth_percent: 25,
        trials_per_j: 10,
        noise_repeats: 10,
        noise_away_pulses: 2_000,
        steady_ms: 200,
        hold_ms: 100,
        end_margin_pulses: 3_000,
        resumes: 3,
    }
}

const RAMPING: Ramping = Ramping { accel: 200_000, seek_speed: 2_000 };

#[test]
fn command_lines() {
    assert_eq!(Command::Move { pulses: -333, rate: 2_000 }.line(), "MOVE -333 2000");
    assert_eq!(Command::Move { pulses: 333, rate: 2_000 }.line(), "MOVE +333 2000");
    assert_eq!(Command::SeekRef.line(), "SEEK REF");
    let trial = Command::Trial(vec![Segment::Hold { rate: 0, ms: 200 }, Segment::Ramp { from: 9_920, to: 0 }]);
    assert_eq!(trial.line(), "TRIAL H:0:200 R:9920:0");
}

#[test]
fn reply_lines() {
    assert!(matches!(Line::parse("OK HOME D2 D3 59828 29914"), Line::Ok { command, values } if command == "HOME" && values.len() == 4));
    assert!(matches!(Line::parse("SWITCH D2 -12"), Line::Switch { pin, count: -12 } if pin == "D2"));
    assert!(matches!(Line::parse("HALT stopped 1500"), Line::Halt(halt) if halt.reason == "stopped" && halt.count == 1500 && !halt.frees_cart()));
    assert!(matches!(Line::parse("HALT no-switch 0"), Line::Halt(halt) if halt.frees_cart()));
    assert!(matches!(Line::parse("READY"), Line::Ready));
    assert!(matches!(Line::parse("LOG stopped: STOP; stopped at pulse 12"), Line::Log(text) if text == "stopped: STOP; stopped at pulse 12"));
    assert!(matches!(Line::parse("HALT unexplained-switch 40"), Line::Halt(halt) if halt.frees_cart()));
    assert!(matches!(Line::parse("HALT into-switch 0"), Line::Halt(halt) if !halt.frees_cart()));
    assert!(matches!(Line::parse("calibrate: joining WiFi network"), Line::Other));
    assert!(matches!(Line::parse("SWITCH D2 lots"), Line::Other));
}

/// j × Δt at j_start: 620,000 × 0.016 = 9,920 pulses/s.
#[test]
fn rates_at_j_start() {
    let settings = experiment();
    assert_eq!(Kind::FromRest.rates(620_000, &settings), (0, 9_920));
    assert_eq!(Kind::AtSpeed.rates(620_000, &settings), (24_000, 33_920));
    assert_eq!(Kind::Reversal.rates(620_000, &settings), (-4_960, 4_960));
}

/// The largest j: top ÷ Δt = 48,000 ÷ 0.016 = 3,000,000 for a jump from
/// rest; half that at speed; twice it for a reversal.
#[test]
fn j_max() {
    let settings = experiment();
    assert_eq!(Kind::FromRest.j_max(&settings), 3_000_000);
    assert_eq!(Kind::AtSpeed.j_max(&settings), 1_500_000);
    assert_eq!(Kind::Reversal.j_max(&settings), 6_000_000);
    assert!(Kind::FromRest.plan(3_000_001, &settings).is_none());
}

#[test]
fn every_plan_fits_the_arduino() {
    let settings = experiment();
    for kind in Kind::ALL {
        let plan = kind.plan(settings.j_start, &settings);
        assert!(plan.as_ref().is_some_and(|plan| plan.len() <= MAX_SEGMENTS), "{}", kind.name());
    }
}

/// From rest at j_start: hold 0 for 200 ms (no motion), 9,920 pulses/s
/// for 100 ms (992 pulses), then a ramp down at 200,000 pulses/s² over
/// 49.6 ms (9,920² ÷ (2 × 200,000) = 246 pulses, plus about 10 because
/// the ramp holds the seek rate, 2,000, for its last 10 ms instead of
/// tapering to 0): about 1,248 in all, all in Plus.
#[test]
fn from_rest_excursion() {
    let settings = experiment();
    let plan = Kind::FromRest.plan(620_000, &settings).unwrap_or_default();
    let (low, high) = excursion(&plan, RAMPING);
    assert_eq!(low, 0);
    assert!((1_240..=1_260).contains(&high), "{high}");
}

#[test]
fn trial_records() {
    let dir = std::env::temp_dir().join(format!("bench-station-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut book = crate::trials::TrialBook::new(&dir);
    assert!(book.record("at speed,1210937,10,24000,43375,16,59830,0,pass\n").is_ok());
    assert!(book.record("at speed,1210937,10,24000,43375,16").is_err());
    assert!(book.record("at speed,lots,10,24000,43375,16,59830,0,pass").is_err());
    assert_eq!(book.count, 1);
    assert!(book.last.as_ref().is_some_and(|place| place.kind == "at speed" && place.j == 1_210_937 && place.trial == 10));
    let text = std::fs::read_to_string(book.path()).unwrap_or_default();
    assert!(text.starts_with("time,kind,j,trial,c_a,c_b,dt_ms,switch_count,lost,pass\n"));
    assert!(text.lines().nth(1).is_some_and(|line| line.ends_with(",at speed,1210937,10,24000,43375,16,59830,0,pass")));
    let _ = std::fs::remove_dir_all(&dir);
}
