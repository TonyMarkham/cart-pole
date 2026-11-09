use crate::{
    clock_fit::ClockFit,
    hop::Hop,
    ping::Ping,
    protocol::R4_NOMINAL_HZ,
    settings::wifi_settings::WifiSettings,
    stats::Stats,
    sync_exchange::SyncExchange,
    sync_point::SyncPoint,
    units::{NANOS_PER_MICRO, NANOS_PER_SECOND, PAIR, PARTS_PER_MILLION, WAYS_PER_ROUND_TRIP},
    widener::Widener,
};

use anyhow::{Context, Result, bail};
use std::fmt::Write;

/// The whole run, worked out: the clock fit, how well the syncs agree with
/// it, and every hop.
pub(crate) struct Analysis {
    pub(crate) syncs: Vec<SyncPoint>,
    pub(crate) fit: ClockFit,
    pub(crate) hops: Vec<Hop>,
    /// The kept syncs' distance from the fitted line, ns.
    pub(crate) residuals: Stats,
    /// Every answered sync's round trip, ns.
    pub(crate) round_trips: Stats,
    pub(crate) hop_stats: Option<Stats>,
    pub(crate) lost: usize,
}

impl Analysis {
    pub(crate) fn of(
        exchanges: &[SyncExchange],
        pings: &[Ping],
        settings: &WifiSettings,
    ) -> Result<Self> {
        let first = exchanges.first().context("no sync was answered")?;
        if settings.fit_windows == 0
            || !(settings.keep_fraction > 0.0 && settings.keep_fraction <= 1.0)
        {
            bail!(
                "[wifi] fit_windows must be at least 1, and keep_fraction more than 0 and at most 1"
            );
        }
        let widener = Widener::new(first.t1, first.pc_mid_ns());

        let mut syncs: Vec<SyncPoint> = exchanges
            .iter()
            .map(|&exchange| {
                let mid = exchange.pc_mid_ns();
                SyncPoint {
                    exchange,
                    r4_mid: (widener.widen(exchange.t1, mid) + widener.widen(exchange.t2, mid))
                        / PAIR,
                    round_trip_ns: exchange.round_trip_ns(),
                    kept: false,
                }
            })
            .collect();

        let window = syncs.len().div_ceil(settings.fit_windows);
        for chunk in syncs.chunks_mut(window) {
            let keep = ((chunk.len() as f64 * settings.keep_fraction).ceil() as usize).max(1);
            let mut order: Vec<usize> = (0..chunk.len()).collect();
            order.sort_by(|&a, &b| chunk[a].round_trip_ns.total_cmp(&chunk[b].round_trip_ns));
            for &index in order.iter().take(keep) {
                chunk[index].kept = true;
            }
        }

        let points: Vec<(f64, f64)> = syncs
            .iter()
            .filter(|sync| sync.kept)
            .map(|sync| (sync.r4_mid, sync.exchange.pc_mid_ns()))
            .collect();
        let fit = ClockFit::fit(&points)?;
        let residual_values: Vec<f64> = points.iter().map(|&(x, y)| y - fit.pc_ns(x)).collect();
        let residuals = Stats::of(&residual_values).context("no kept syncs")?;
        let round_trip_values: Vec<f64> = syncs.iter().map(|sync| sync.round_trip_ns).collect();
        let round_trips = Stats::of(&round_trip_values).context("no syncs")?;

        let hops: Vec<Hop> = pings
            .iter()
            .map(|&ping| Hop {
                ping,
                hop_us: ping.stamp.map(|stamp| {
                    let count = widener.widen(stamp, ping.sent_ns);
                    (fit.pc_ns(count) - ping.sent_ns) / NANOS_PER_MICRO
                }),
            })
            .collect();
        let hop_values: Vec<f64> = hops.iter().filter_map(|hop| hop.hop_us).collect();
        let lost = hops.len() - hop_values.len();

        Ok(Self {
            syncs,
            fit,
            hops,
            residuals,
            round_trips,
            hop_stats: Stats::of(&hop_values),
            lost,
        })
    }

    /// How far the R4's clock runs from 48 MHz, parts per million, as the
    /// PC's clock sees it: positive is fast.
    pub(crate) fn r4_clock_ppm(&self) -> f64 {
        let hz = NANOS_PER_SECOND / self.fit.rate_ns_per_count;
        (hz / R4_NOMINAL_HZ - 1.0) * PARTS_PER_MILLION
    }

    /// The summary, for `summary.txt` and the screen.
    pub(crate) fn summary(&self, strays: usize) -> String {
        let mut text = String::new();
        let kept = self.syncs.iter().filter(|sync| sync.kept).count();
        let min_round_trip_us = self.round_trips.min / NANOS_PER_MICRO;
        // Writing to a String never fails.
        let _ = writeln!(text, "WiFi hop, PC send -> R4 has the datagram whole:");
        let _ = writeln!(
            text,
            "  datagrams: {} sent, {} reported, {} lost",
            self.hops.len(),
            self.hops.len() - self.lost,
            self.lost
        );
        if let Some(stats) = &self.hop_stats {
            let _ = writeln!(
                text,
                "  {} hops: mean {:.0} us, standard deviation {:.0} us, shortest {:.0} us, longest {:.0} us",
                stats.count, stats.mean, stats.std_dev, stats.min, stats.max
            );
            for &(percent, value) in &stats.percentiles {
                let _ = writeln!(text, "  {percent}% at most {value:.0} us");
            }
        }
        let _ = writeln!(text, "clock sync over USB:");
        let _ = writeln!(
            text,
            "  {} syncs answered, {kept} kept for the fit",
            self.syncs.len()
        );
        let _ = writeln!(
            text,
            "  round trip: shortest {:.0} us, median {:.0} us, longest {:.0} us",
            min_round_trip_us,
            self.round_trips
                .percentiles
                .first()
                .map_or(0.0, |&(_, value)| value / NANOS_PER_MICRO),
            self.round_trips.max / NANOS_PER_MICRO
        );
        let _ = writeln!(
            text,
            "  kept syncs off the fitted line: standard deviation {:.0} us, largest {:.0} us",
            self.residuals.std_dev / NANOS_PER_MICRO,
            self.residuals.min.abs().max(self.residuals.max.abs()) / NANOS_PER_MICRO
        );
        let _ = writeln!(
            text,
            "  every hop may be off by up to half the shortest round trip, {:.0} us, if the USB line is slower one way than the other",
            min_round_trip_us / WAYS_PER_ROUND_TRIP
        );
        let _ = writeln!(
            text,
            "  R4 clock: {:+.1} ppm from 48 MHz, as the PC's clock sees it",
            self.r4_clock_ppm()
        );
        let _ = writeln!(text, "stray lines from the R4: {strays}");
        text
    }

    /// `syncs.csv`: every answered sync.
    pub(crate) fn syncs_csv(&self) -> String {
        let mut text = String::from("number,t0_ns,t1_raw,t2_raw,t3_ns,round_trip_us,kept\n");
        for sync in &self.syncs {
            let exchange = sync.exchange;
            let _ = writeln!(
                text,
                "{},{:.0},{},{},{:.0},{:.1},{}",
                exchange.number,
                exchange.t0_ns,
                exchange.t1,
                exchange.t2,
                exchange.t3_ns,
                sync.round_trip_ns / NANOS_PER_MICRO,
                u8::from(sync.kept)
            );
        }
        text
    }

    /// `pings.csv`: every datagram, its stamp and its hop, blank if lost.
    pub(crate) fn pings_csv(&self) -> String {
        let mut text = String::from("n,payload,sent_ns,stamp_raw,hop_us\n");
        for (n, hop) in self.hops.iter().enumerate() {
            let [a, b, c] = hop.ping.payload;
            let stamp = hop
                .ping
                .stamp
                .map_or(String::new(), |stamp| stamp.to_string());
            let value = hop.hop_us.map_or(String::new(), |us| format!("{us:.1}"));
            let _ = writeln!(
                text,
                "{n},{a:02X}{b:02X}{c:02X},{:.0},{stamp},{value}",
                hop.ping.sent_ns
            );
        }
        text
    }

    /// `histogram.csv`: the hops in bins of `bin_us`, from the shortest's bin
    /// to the longest's.
    pub(crate) fn histogram_csv(&self, bin_us: u32) -> String {
        let mut text = String::from("from_us,to_us,count\n");
        let width = f64::from(bin_us.max(1));
        let values: Vec<f64> = self.hops.iter().filter_map(|hop| hop.hop_us).collect();
        let Some(stats) = &self.hop_stats else {
            return text;
        };
        let first = (stats.min / width).floor() as i64;
        let last = (stats.max / width).floor() as i64;
        let mut counts = vec![0u64; (last - first + 1).unsigned_abs() as usize];
        for value in values {
            let bin = ((value / width).floor() as i64 - first).unsigned_abs() as usize;
            if let Some(count) = counts.get_mut(bin) {
                *count += 1;
            }
        }
        for (index, count) in counts.iter().enumerate() {
            let from = (first + index as i64) as f64 * width;
            let _ = writeln!(text, "{from:.0},{:.0},{count}", from + width);
        }
        text
    }
}
