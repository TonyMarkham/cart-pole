use crate::{
    analysis::Analysis, board::Board, ping::Ping, r4_address::R4Address, random::Random,
    run_folder, serial_line::SerialLine, settings::host_settings::HostSettings,
    sync_exchange::SyncExchange,
};

use anyhow::{Context, Result, bail};
use std::{
    collections::HashSet,
    fs,
    net::{Ipv4Addr, UdpSocket},
    path::Path,
    thread,
    time::Duration,
};

/// The test's name: its run folders are `wifi_<seconds>`.
const TEST_NAME: &str = "wifi";

/// The files written into each run's folder.
const SUMMARY_FILE: &str = "summary.txt";
const SYNCS_FILE: &str = "syncs.csv";
const PINGS_FILE: &str = "pings.csv";
const HISTOGRAM_FILE: &str = "histogram.csv";
const STRAYS_FILE: &str = "strays.txt";

/// A command's sign byte (real-loop-dia.md, *Contracts*): 0 negative, 1
/// positive.
const SIGN_NEGATIVE: u8 = 0;
const SIGN_POSITIVE: u8 = 1;

/// Any local port: the PC sends from whichever port the system gives.
const ANY_PORT: u16 = 0;

/// The WiFi test. The R4 is asked where it listens; then `warmup_syncs`
/// syncs; then `cycles` times a sync, one datagram, and the wait for its
/// report, so the R4 is idle whenever anything reaches it; then
/// `closing_syncs` syncs. Everything is read on the PC's one clock
/// (`Board::now_ns`); the R4's stamps are carried into it by the clock fit.
pub(crate) fn run(settings: &HostSettings, settings_path: &Path) -> Result<()> {
    let wifi = &settings.wifi;
    if wifi.gap_min_ms > wifi.gap_max_ms {
        bail!("[wifi] gap_min_ms is above gap_max_ms");
    }
    let folder = run_folder::create(settings_path, &wifi.output_dir, TEST_NAME)?;
    println!("run folder: {}", folder.display());

    let mut line = SerialLine::open(&settings.r4.port, settings.r4.baud)?;
    let target = R4Address::ask(
        &mut line,
        Duration::from_millis(settings.r4.status_timeout_ms),
    )?;
    println!("R4 running at {}:{}", target.address, target.port);
    let udp = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, ANY_PORT)).context("opening a UDP socket")?;
    udp.connect((target.address.as_str(), target.port))
        .with_context(|| format!("aiming at {}:{}", target.address, target.port))?;

    let mut board = Board::new(line);
    let mut random = Random::new(wifi.seed);
    let sync_timeout = Duration::from_millis(wifi.sync_timeout_ms);
    let ping_timeout = Duration::from_millis(wifi.ping_timeout_ms);
    let mut exchanges: Vec<SyncExchange> = Vec::new();
    let mut lost_syncs = 0u32;
    let mut number = 0u32;
    let mut sync = |board: &mut Board, exchanges: &mut Vec<SyncExchange>| -> Result<()> {
        match board.sync(number, sync_timeout)? {
            Some(exchange) => exchanges.push(exchange),
            None => lost_syncs += 1,
        }
        number += 1;
        Ok(())
    };

    for _ in 0..wifi.warmup_syncs {
        sync(&mut board, &mut exchanges)?;
    }
    println!("sending {} datagrams", wifi.cycles);

    let mut used = HashSet::new();
    let mut pings: Vec<Ping> = Vec::new();
    for _ in 0..wifi.cycles {
        sync(&mut board, &mut exchanges)?;
        let payload = loop {
            let top = i64::from(wifi.top_rate);
            let rate = random.between(-top, top);
            let size = u16::try_from(rate.unsigned_abs()).unwrap_or(wifi.top_rate);
            let sign = if rate < 0 {
                SIGN_NEGATIVE
            } else {
                SIGN_POSITIVE
            };
            let [low, high] = size.to_le_bytes();
            let payload = [sign, low, high];
            if used.insert(payload) {
                break payload;
            }
        };
        let sent_ns = board.now_ns();
        udp.send(&payload).context("sending a datagram")?;
        let stamp = board.report(payload, ping_timeout)?;
        pings.push(Ping {
            payload,
            sent_ns,
            stamp,
        });
        let gap = random.between(
            i64::try_from(wifi.gap_min_ms).unwrap_or(i64::MAX),
            i64::try_from(wifi.gap_max_ms).unwrap_or(i64::MAX),
        );
        thread::sleep(Duration::from_millis(gap.unsigned_abs()));
    }

    for _ in 0..wifi.closing_syncs {
        sync(&mut board, &mut exchanges)?;
    }
    board.drain(ping_timeout)?;
    for ping in &mut pings {
        if ping.stamp.is_none() {
            ping.stamp = board.late_report(ping.payload);
        }
    }

    let analysis = Analysis::of(&exchanges, &pings, wifi)?;
    let mut summary = analysis.summary(board.strays.len());
    summary.push_str(&format!("syncs lost: {lost_syncs}\n"));
    let files = [
        (SUMMARY_FILE, summary.clone()),
        (SYNCS_FILE, analysis.syncs_csv()),
        (PINGS_FILE, analysis.pings_csv()),
        (
            HISTOGRAM_FILE,
            analysis.histogram_csv(wifi.histogram_bin_us),
        ),
        (STRAYS_FILE, board.strays.join("\n")),
    ];
    for (name, text) in files {
        let path = folder.join(name);
        fs::write(&path, text).with_context(|| format!("writing {}", path.display()))?;
    }
    print!("{summary}");
    Ok(())
}
