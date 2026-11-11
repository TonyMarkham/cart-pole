pub(crate) mod awake_report;

// ---------------------------------------------------------------------------------------------- //

use crate::{
    awake::awake_report::AwakeReport,
    protocol::{R3_END, R3_READY, R3_REPORT, R3_START, R3_STARTED},
    r4_address::R4Address,
    random::Random,
    run_folder,
    serial_line::SerialLine,
    settings::bench_settings::BenchSettings,
    units::MILLIS_PER_SECOND,
};

use anyhow::{Context, Result, bail};
use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    net::{Ipv4Addr, UdpSocket},
    path::Path,
    thread,
    time::{Duration, Instant},
};

/// The test's name: its run folders are `awake_<seconds>`.
const TEST_NAME: &str = "awake";

/// The files written into each run's folder.
const DATAGRAMS_FILE: &str = "datagrams.csv";
const REPORT_FILE: &str = "report.txt";
const SUMMARY_FILE: &str = "summary.txt";

/// `datagrams.csv`'s first line: the datagram's number, 1 if the R3 was
/// timing when it was sent, the time it was sent in ms since the first, and
/// its rate in pulses/s.
const DATAGRAMS_HEADER: &str = "n,timed,sent_ms,rate";

/// How long the R3 may take to answer `start`, and to send its whole report,
/// ms. It answers `start` at once. Its longest report is 260 lines, 250 bins
/// and 10 others, of at most 20 bytes, `bin 249 4294967295` and its `\r\n`,
/// at 10 bits a byte at 115,200 baud: 260 × 20 × 10 / 115,200 ≈ 0.45 s.
const R3_ANSWER_MS: u64 = 2_000;

/// A command's sign byte (__documentation__/real-loop-dia.md, *Contracts*): 0 negative, 1
/// positive.
const SIGN_NEGATIVE: u8 = 0;
const SIGN_POSITIVE: u8 = 1;

/// Any local port: the PC sends from whichever port the system gives.
const ANY_PORT: u16 = 0;

/// The awake test: the R3 is reset and waits; the R4 is asked where it
/// listens; random commands go to it over WiFi, first `warmup_datagrams`
/// with the R3 not yet timing, then `datagrams` with it timing; then the R3's
/// histogram is read, saved and summed up.
pub(crate) fn run(settings: &BenchSettings, settings_path: &Path) -> Result<()> {
    let awake = &settings.awake;
    if awake.gap_min_ms > awake.gap_max_ms {
        bail!("[awake] gap_min_ms is above gap_max_ms");
    }
    let folder = run_folder::create(settings_path, &awake.output_dir, TEST_NAME)?;
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

    let mut random = Random::new(awake.seed);
    let log_path = folder.join(DATAGRAMS_FILE);
    let mut log = BufWriter::new(
        File::create(&log_path).with_context(|| format!("creating {}", log_path.display()))?,
    );
    writeln!(log, "{DATAGRAMS_HEADER}")?;
    let started = Instant::now();

    let mut send = |n: u32, timed: bool, random: &mut Random| -> Result<()> {
        let top = i64::from(awake.top_rate);
        let rate = random.between(-top, top);
        let size = u16::try_from(rate.unsigned_abs()).unwrap_or(awake.top_rate);
        let sign = if rate < 0 {
            SIGN_NEGATIVE
        } else {
            SIGN_POSITIVE
        };
        let [low, high] = size.to_le_bytes();
        udp.send(&[sign, low, high]).context("sending a datagram")?;
        writeln!(
            log,
            "{n},{},{:.3},{rate}",
            u8::from(timed),
            started.elapsed().as_secs_f64() * MILLIS_PER_SECOND
        )?;
        let gap = random.between(
            i64::try_from(awake.gap_min_ms).unwrap_or(i64::MAX),
            i64::try_from(awake.gap_max_ms).unwrap_or(i64::MAX),
        );
        thread::sleep(Duration::from_millis(gap.unsigned_abs()));
        Ok(())
    };

    for n in 0..awake.warmup_datagrams {
        send(n, false, &mut random)?;
    }

    r3.write_line(&format!("{R3_START} {} {}", awake.bin_us, awake.bins))?;
    let wanted = format!("{R3_STARTED} {} {}", awake.bin_us, awake.bins);
    r3.wait_for(&wanted, Duration::from_millis(R3_ANSWER_MS))?;
    println!("R3 timing; sending {} datagrams", awake.datagrams);

    for n in 0..awake.datagrams {
        send(awake.warmup_datagrams + n, true, &mut random)?;
    }
    log.flush()?;

    r3.write_line(R3_REPORT)?;
    let lines = r3.wait_for(R3_END, Duration::from_millis(R3_ANSWER_MS))?;
    let report_path = folder.join(REPORT_FILE);
    let mut report_text = lines.join("\n");
    report_text.push('\n');
    fs::write(&report_path, report_text)
        .with_context(|| format!("writing {}", report_path.display()))?;

    let report = AwakeReport::parse(&lines)?;
    let summary = report.summary();
    let summary_path = folder.join(SUMMARY_FILE);
    fs::write(&summary_path, &summary)
        .with_context(|| format!("writing {}", summary_path.display()))?;
    print!("{summary}");
    Ok(())
}
