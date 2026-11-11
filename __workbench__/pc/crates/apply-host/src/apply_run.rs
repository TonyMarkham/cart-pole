use crate::{
    apply_report::ApplyReport,
    apply_result::ApplyResult,
    protocol::{R3_READY, R3_START, R3_STARTED, R3_STOP, R3_STOPPED, R3_TICKS_PER_SECOND},
    r4_address::R4Address,
    random::Random,
    run_folder,
    serial_line::SerialLine,
    settings::host_settings::HostSettings,
    units::HALFWAY,
};

use anyhow::{Context, Result, bail};
use std::{
    fs,
    net::{Ipv4Addr, UdpSocket},
    path::Path,
    thread,
    time::{Duration, Instant},
};

/// The test's name: its run folders are `apply_<seconds>`.
const TEST_NAME: &str = "apply";

/// The files written into each run's folder.
const SUMMARY_FILE: &str = "summary.txt";
const RESULTS_FILE: &str = "results.csv";
const HISTOGRAM_FILE: &str = "histogram.csv";
const STRAYS_FILE: &str = "strays.txt";

/// How long the R3 may take to answer `start` and `stop`, ms: it answers at
/// once, in one short line.
const R3_ANSWER_MS: u64 = 2_000;

/// A positive command's sign byte (__documentation__/real-loop-dia.md, *Contracts*): both rates
/// run the same way, so DIR never changes.
const SIGN_POSITIVE: u8 = 1;

/// Any local port: the PC sends from whichever port the system gives.
const ANY_PORT: u16 = 0;

/// The commands take the two rates in turn: command n is at the low rate
/// when n is a multiple of this, so every command changes the rate.
const RATES_IN_TURN: u32 = 2;

/// The fewest R3 counts the change threshold may be: one count either way is
/// the capture's own rounding, so a threshold of 1 would see changes that
/// aren't there.
const MIN_CHANGE_TICKS: f64 = 2.0;

/// The apply test. The R3 is reset and waits; the R4 is asked where it
/// listens; commands alternating between the two rates go to it over WiFi,
/// first `warmup_datagrams` with the R3 not timing, then `datagrams` with
/// it timing, each followed by a wait for the R3's line about it.
pub(crate) fn run(settings: &HostSettings, settings_path: &Path) -> Result<()> {
    let apply = &settings.apply;
    if apply.gap_min_ms > apply.gap_max_ms {
        bail!("[apply] gap_min_ms is above gap_max_ms");
    }
    if apply.rate_low == 0 || apply.rate_low >= apply.rate_high {
        bail!("[apply] rate_low must be above 0 and below rate_high");
    }
    // The R3's threshold: halfway between the two rates' periods, in its
    // counts. 16 MHz / 38,400 = 416.7 and / 40,000 = 400, so 8.3.
    let low_period = R3_TICKS_PER_SECOND / f64::from(apply.rate_low);
    let high_period = R3_TICKS_PER_SECOND / f64::from(apply.rate_high);
    let change = (low_period - high_period) / HALFWAY;
    if change < MIN_CHANGE_TICKS {
        bail!(
            "[apply] the rates' periods differ by under {} R3 counts: too close to tell apart",
            MIN_CHANGE_TICKS * HALFWAY
        );
    }
    let change_ticks = change.floor() as u64;

    let folder = run_folder::create(settings_path, &apply.output_dir, TEST_NAME)?;
    println!("run folder: {}", folder.display());

    let mut r3 = SerialLine::open(&settings.r3.port, settings.r3.baud)?;
    r3.reset_by_dtr()?;
    r3.wait_for(
        R3_READY,
        Duration::from_millis(settings.r3.ready_timeout_ms),
    )?;
    println!("R3 ready");

    let mut r4 = SerialLine::open(&settings.r4.port, settings.r4.baud)?;
    let target = R4Address::ask(
        &mut r4,
        Duration::from_millis(settings.r4.status_timeout_ms),
    )?;
    println!("R4 running at {}:{}", target.address, target.port);
    let udp = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, ANY_PORT)).context("opening a UDP socket")?;
    udp.connect((target.address.as_str(), target.port))
        .with_context(|| format!("aiming at {}:{}", target.address, target.port))?;

    let mut random = Random::new(apply.seed);
    // Even commands at the low rate, odd at the high: every command changes
    // the rate.
    let send = |n: u32| -> Result<u16> {
        let rate = if n.is_multiple_of(RATES_IN_TURN) {
            apply.rate_low
        } else {
            apply.rate_high
        };
        let [low, high] = rate.to_le_bytes();
        udp.send(&[SIGN_POSITIVE, low, high])
            .context("sending a command")?;
        Ok(rate)
    };
    let gap = |random: &mut Random| {
        let gap = random.between(
            i64::try_from(apply.gap_min_ms).unwrap_or(i64::MAX),
            i64::try_from(apply.gap_max_ms).unwrap_or(i64::MAX),
        );
        thread::sleep(Duration::from_millis(gap.unsigned_abs()));
    };

    for n in 0..apply.warmup_datagrams {
        send(n)?;
        gap(&mut random);
    }

    r3.write_line(&format!(
        "{R3_START} {change_ticks} {}",
        apply.apply_timeout_us
    ))?;
    let started = format!("{R3_STARTED} {change_ticks} {}", apply.apply_timeout_us);
    r3.wait_for(&started, Duration::from_millis(R3_ANSWER_MS))?;
    println!(
        "R3 timing, change threshold {change_ticks} counts; sending {} commands",
        apply.datagrams
    );

    let result_timeout = Duration::from_millis(apply.result_timeout_ms);
    let mut results = Vec::new();
    let mut strays = Vec::new();
    for n in 0..apply.datagrams {
        let rate = send(apply.warmup_datagrams + n)?;
        let deadline = Instant::now() + result_timeout;
        let mut result = None;
        while let Some(left) = deadline.checked_duration_since(Instant::now()) {
            let Some(line) = r3.read_line(left)? else {
                break;
            };
            match ApplyResult::parse(&line) {
                Some(parsed) => {
                    result = Some(parsed);
                    break;
                }
                None => strays.push(line),
            }
        }
        results.push((rate, result));
        gap(&mut random);
    }

    r3.write_line(R3_STOP)?;
    let stop_lines = r3.wait_for_prefix(R3_STOPPED, Duration::from_millis(R3_ANSWER_MS))?;
    strays.extend(stop_lines);

    let report = ApplyReport { results };
    let summary = report.summary();
    let files = [
        (SUMMARY_FILE, summary.clone()),
        (RESULTS_FILE, report.results_csv()),
        (HISTOGRAM_FILE, report.histogram_csv(apply.histogram_bin_us)),
        (STRAYS_FILE, strays.join("\n")),
    ];
    for (name, text) in files {
        let path = folder.join(name);
        fs::write(&path, text).with_context(|| format!("writing {}", path.display()))?;
    }
    print!("{summary}");
    Ok(())
}
