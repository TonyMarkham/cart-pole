//! The jump-limit experiment (jump-limit-experiment.md), run from the Pi:
//! the largest change of pulse rate the motor follows without losing
//! steps, `m`, in pulses/s². The steps are `calibrate/src/experiment.rs`'s;
//! the Pi decides them and the Arduino plays each move and trial.
//!
//! The limit switch at the first end is the reference. Each trial starts
//! at the reference switch, moves slowly to the centre, plays one jump,
//! stops gently and returns slowly to the switch. Every pulse is counted,
//! so if the motor kept up, the switch trips at the same count as the
//! return before; a count further off than the noise floor is that many
//! lost steps, and the trial failed.
//!
//! Every trial is one line of the run's results file. If the board resets
//! or its port drops mid-run, the Pi re-homes and repeats the step that
//! was cut short, up to `resumes` times.

use std::{fmt::Write as _, fs::File, io::Write as _, path::PathBuf};

use crate::{
    kind::Kind,
    log::stamp,
    profile::{Ramping, excursion},
    protocol::Command,
    settings::Experiment,
    station::{Failure, Station},
};

/// The reference the trials are measured against (the brief's section 5,
/// step 1).
#[derive(Clone, Debug)]
pub struct Reference {
    /// The switch at the first end.
    pub switch: String,
    /// The mean count at which it tripped over the noise-floor returns.
    pub count: i64,
    /// Their spread, largest minus smallest: a difference no larger is
    /// not a lost step.
    pub noise: u64,
}

/// One kind's findings so far.
#[derive(Clone, Copy, Debug, Default)]
pub struct KindResult {
    /// The largest j whose trials all kept every step.
    pub passed: Option<u32>,
    /// The j at which a trial lost steps, and the most it lost.
    pub failed: Option<(u32, u64)>,
    /// No slip up to the largest j the kind can make: its limit is "at
    /// least" `passed`.
    pub at_least: bool,
    /// The j whose plan did not fit the rail from the centre, which ended
    /// the kind there.
    pub no_room: Option<u32>,
    pub done: bool,
}

/// One trial's outcome.
#[derive(Clone, Copy, Debug)]
pub struct Trial {
    pub kind: Kind,
    pub j: u32,
    pub number: u32,
    /// The switch's count minus the return before: + means the cart came
    /// back further than the count says.
    pub lost: i64,
    pub pass: bool,
}

/// `m`, as far as the experiment got.
#[derive(Clone, Copy, Debug)]
pub enum JumpLimit {
    /// The smallest kind's limit, found by a failure above it.
    Measured(u32),
    /// No kind lost a step up to what it can make: m is at least this.
    AtLeast(u32),
    /// A kind lost steps at j_start: m is below it.
    Below(u32),
    /// Not every kind finished.
    Incomplete,
}

/// The experiment's findings, for the page and the log.
#[derive(Clone, Debug, Default)]
pub struct ExperimentReport {
    pub started: bool,
    pub reference: Option<Reference>,
    pub kinds: [KindResult; 4],
    /// The kind, j and trial running now.
    pub now: Option<(Kind, u32, u32)>,
    pub trials: u32,
    pub last: Option<Trial>,
    pub results: Option<PathBuf>,
}

impl ExperimentReport {
    /// m = the smallest of the kinds' limits, so it is safe for all of
    /// them (section 5, step 5).
    pub fn jump_limit(&self, j_start: u32) -> JumpLimit {
        if self.kinds.iter().any(|kind| kind.done && kind.passed.is_none() && kind.failed.is_some()) {
            return JumpLimit::Below(j_start);
        }
        if !self.kinds.iter().all(|kind| kind.done && kind.passed.is_some() && kind.no_room.is_none()) {
            return JumpLimit::Incomplete;
        }
        let lowest = self.kinds.iter().filter_map(|kind| kind.passed).min().unwrap_or(0);
        let measured = self.kinds.iter().any(|kind| kind.passed == Some(lowest) && !kind.at_least);
        if measured { JumpLimit::Measured(lowest) } else { JumpLimit::AtLeast(lowest) }
    }

    /// The report as text.
    pub fn write(&self, settings: &Experiment) -> String {
        let mut out = String::new();
        if !self.started {
            out.push_str("jump-limit experiment: not run\n");
            return out;
        }
        let _ = writeln!(out, "jump-limit experiment: {}", settings.conditions);
        let _ = writeln!(
            out,
            "  Δt {} ms, top {} pulses/s, {} trials per j",
            settings.trial_dt_ms, settings.trial_top, settings.trials_per_j
        );
        match &self.reference {
            Some(reference) => {
                let _ = writeln!(
                    out,
                    "  noise floor: {} at pulse {}, spread {} pulses over {} returns",
                    reference.switch, reference.count, reference.noise, settings.noise_repeats
                );
            }
            None => out.push_str("  noise floor: measuring\n"),
        }
        for (kind, result) in Kind::ALL.into_iter().zip(self.kinds) {
            let _ = write!(out, "  {:<10} ", kind.name());
            let _ = match (result.passed, result.failed, result.no_room) {
                (_, _, Some(j)) => write!(out, "no room on the rail at j {j}"),
                (Some(j), None, None) if result.at_least => write!(out, "at least {j} (no slip up to what it can make)"),
                (Some(j), Some((failed, lost)), None) => write!(out, "{j} (lost {lost} at {failed})"),
                (None, Some((failed, lost)), None) => write!(out, "below {failed} (lost {lost} there)"),
                (Some(j), None, None) => write!(out, "passed up to {j}"),
                (None, None, None) if result.done => write!(out, "j_start is above what it can make"),
                (None, None, None) => write!(out, "-"),
            };
            if let Some((now, j, trial)) = self.now
                && now == kind
            {
                let _ = write!(out, "  running: j {j}, trial {trial} of {}", settings.trials_per_j);
            }
            out.push('\n');
        }
        let _ = match self.jump_limit(settings.j_start) {
            JumpLimit::Measured(m) => writeln!(out, "  m = {m} pulses/s², measured"),
            JumpLimit::AtLeast(m) => writeln!(out, "  m = at least {m} pulses/s² (no slip found)"),
            JumpLimit::Below(m) => writeln!(out, "  m is below j_start, {m} pulses/s²: lower j_start"),
            JumpLimit::Incomplete => writeln!(out, "  m: not every kind has finished"),
        };
        if let Some(trial) = self.last {
            let _ = writeln!(
                out,
                "  trials run: {}; last: {} at j {}, trial {}, lost {}: {}",
                self.trials,
                trial.kind.name(),
                trial.j,
                trial.number,
                trial.lost,
                if trial.pass { "pass" } else { "FAIL" }
            );
        }
        if let Some(path) = &self.results {
            let _ = writeln!(out, "  results file: {}", path.display());
        }
        out
    }
}

/// The run's results file: a header, then one line per trial (the brief's
/// section 6).
struct Results {
    file: Option<File>,
}

impl Results {
    const HEADER: &str = "kind,j,c_a,c_b,dt_ms,switch_count,lost,pass";

    fn create(station: &Station) -> (Self, Option<PathBuf>) {
        let dir = &station.settings.station.log_dir;
        let path = dir.join(format!("results-{}.csv", stamp()));
        let file = std::fs::create_dir_all(dir)
            .and_then(|()| File::create(&path))
            .and_then(|mut file| writeln!(file, "{}", Self::HEADER).map(|()| file));
        match file {
            Ok(file) => (Self { file: Some(file) }, Some(path)),
            Err(error) => {
                station.note(&format!("cannot create {}: {error}; trials go to the log only", path.display()));
                (Self { file: None }, None)
            }
        }
    }

    fn line(&mut self, station: &Station, line: &str) {
        if let Some(file) = self.file.as_mut()
            && (writeln!(file, "{line}").is_err() || file.flush().is_err())
        {
            station.note("the results file could not be written; trials go to the log only");
            self.file = None;
        }
    }
}

/// After a step failed: if the board reset or its port dropped and
/// resumes are left, waits for the port, re-homes and returns to the
/// reference switch, so the caller can run the step again. Any other
/// failure, or none left, ends the run with it.
async fn recover(station: &Station, resumes: &mut u32, failure: Failure) -> Result<(), Failure> {
    if !failure.resumable() || *resumes == 0 {
        return Err(failure);
    }
    *resumes -= 1;
    station.note(&format!(
        "experiment: {}; re-homing and repeating the step ({} resumes left)",
        failure.describe(),
        resumes
    ));
    tokio::select! {
        () = station.link.wait_connected() => {}
        () = wait_for_stop(station) => return Err(Failure::Stopped),
    }
    station.pause(station.settings.rig.settle_ms).await?;
    station.home().await?;
    station.pause(station.settings.rig.settle_ms).await?;
    station.seek_ref().await?;
    Ok(())
}

/// The noise floor's returns: the count each tripped the reference at.
async fn noise_floor(station: &Station, switch: &str) -> Result<Vec<i64>, Failure> {
    let settings = &station.settings.experiment;
    let settle = station.settings.rig.settle_ms;
    let mut counts = Vec::new();
    for repeat in 0..settings.noise_repeats {
        if repeat > 0 {
            clear_move(station, i64::from(settings.noise_away_pulses)).await?;
            station.pause(settle).await?;
        }
        let count = station.seek_ref().await?;
        station.note(&format!("noise floor {}: {switch} tripped at pulse {count}", repeat + 1));
        counts.push(count);
        station.pause(settle).await?;
    }
    Ok(counts)
}

/// One trial from the reference switch: to the centre, the plan, back to
/// the switch. Returns the count the switch tripped at.
async fn trial(station: &Station, to_centre: i64, plan: &Command) -> Result<i64, Failure> {
    let settle = station.settings.rig.settle_ms;
    clear_move(station, to_centre).await?;
    station.pause(settle).await?;
    station.command(plan).await?;
    station.pause(settle).await?;
    station.seek_ref().await
}

async fn wait_for_stop(station: &Station) {
    while !station.stop_pressed() {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}

/// A slow move that must not reach a switch: the experiment's moves stay
/// well clear of both ends, so a switch on the way means the cart is not
/// where the count says. The driver is switched off, as calibrate does.
async fn clear_move(station: &Station, pulses: i64) -> Result<(), Failure> {
    if let Some((pin, count)) = station.slow_move(pulses).await? {
        station.note(&format!("{pin} tripped at pulse {count} during a slow move: switching the driver off"));
        let _ = station.command(&Command::Disable).await;
        station.state().rig.homed = false;
        return Err(Failure::Halted(crate::halt::Halt { reason: "switch-on-the-way".to_owned(), count, detail: None }));
    }
    Ok(())
}

/// The whole experiment: the noise floor, then each kind from j_start,
/// raising j by `j_growth_percent` until a trial loses steps or j reaches
/// the most the kind can make. Needs a finished homing.
pub async fn run(station: &Station) -> Result<String, Failure> {
    let settings = &station.settings.experiment;
    let settle = station.settings.rig.settle_ms;
    let ramping = Ramping { accel: station.settings.rig.accel, seek_speed: station.settings.rig.seek_speed };
    let (span, switch) = {
        let state = station.state();
        match (state.rig.span, &state.rig.first, state.rig.homed) {
            (Some(span), Some(first), true) => (span, first.clone()),
            _ => return Err(Failure::NotHomed),
        }
    };
    let (mut results, path) = Results::create(station);
    station.state().experiment = ExperimentReport { started: true, results: path, ..ExperimentReport::default() };
    station.note(&format!(
        "jump-limit experiment: {}; Δt {} ms, top {} pulses/s",
        settings.conditions, settings.trial_dt_ms, settings.trial_top
    ));
    let mut resumes = settings.resumes;

    // 1. The noise floor: slow returns to the reference switch.
    let counts = loop {
        match noise_floor(station, &switch).await {
            Ok(counts) => break counts,
            Err(failure) => recover(station, &mut resumes, failure).await?,
        }
    };
    let (low, high) = (counts.iter().copied().min().unwrap_or(0), counts.iter().copied().max().unwrap_or(0));
    let reference = Reference {
        switch: switch.clone(),
        count: counts.iter().sum::<i64>() / i64::from(settings.noise_repeats.max(1)),
        noise: low.abs_diff(high),
    };
    station.note(&format!("noise floor: reference pulse {}, spread {} pulses", reference.count, reference.noise));
    station.state().experiment.reference = Some(reference.clone());

    // 2–4. Each kind, j raised step by step. The cart starts each trial
    // at the reference switch, where the last return left it.
    let to_centre = i64::from(span / 2) - reference.count;
    let room = i64::from(span / 2) - i64::from(settings.end_margin_pulses);
    let mut at_switch = counts.last().copied().unwrap_or(reference.count);
    for (index, kind) in Kind::ALL.into_iter().enumerate() {
        let most = kind.j_max(settings);
        let mut j = settings.j_start;
        while j <= most {
            let Some(plan) = kind.plan(j, settings) else {
                station.state().experiment.kinds[index].no_room = Some(j);
                break;
            };
            let (lowest, highest) = excursion(&plan, ramping);
            if lowest < -room || highest > room {
                station.note(&format!(
                    "{} at j {j}: the plan needs {lowest}..{highest} pulses from the centre, more than the rail has",
                    kind.name()
                ));
                station.state().experiment.kinds[index].no_room = Some(j);
                break;
            }
            let (c_a, c_b) = kind.rates(j, settings);
            let plan = Command::Trial(plan);
            let mut worst = 0;
            let mut failed = false;
            for number in 1..=settings.trials_per_j {
                station.state().experiment.now = Some((kind, j, number));
                let (count, start) = loop {
                    // A resume ends at the reference switch: count from
                    // where that return tripped.
                    let start = station.state().rig.position.unwrap_or(at_switch);
                    match trial(station, to_centre, &plan).await {
                        Ok(count) => break (count, start),
                        Err(failure) => recover(station, &mut resumes, failure).await?,
                    }
                };
                let lost = count - start;
                let pass = lost.unsigned_abs() <= reference.noise;
                at_switch = count;
                worst = worst.max(lost.unsigned_abs());
                {
                    let mut state = station.state();
                    state.experiment.trials += 1;
                    state.experiment.last = Some(Trial { kind, j, number, lost, pass });
                }
                results.line(
                    station,
                    &format!(
                        "{},{j},{c_a},{c_b},{},{count},{lost},{}",
                        kind.name(),
                        settings.trial_dt_ms,
                        if pass { "pass" } else { "fail" }
                    ),
                );
                station.note(&format!(
                    "trial {} j {j} (c_a {c_a}, c_b {c_b}, Δt {} ms) {number}/{}: switch at {count}, lost {lost}: {}",
                    kind.name(),
                    settings.trial_dt_ms,
                    settings.trials_per_j,
                    if pass { "pass" } else { "FAIL" }
                ));
                station.pause(settle).await?;
                if !pass {
                    failed = true;
                    break;
                }
            }
            let mut state = station.state();
            let result = &mut state.experiment.kinds[index];
            if failed {
                result.failed = Some((j, worst));
                break;
            }
            result.passed = Some(j);
            if j == most {
                result.at_least = true;
                break;
            }
            let raised = (u64::from(j) * u64::from(100 + settings.j_growth_percent) / 100).min(u64::from(most)) as u32;
            j = raised.max(j + 1);
        }
        let mut state = station.state();
        state.experiment.kinds[index].done = true;
        state.experiment.now = None;
    }

    let limit = station.state().experiment.jump_limit(settings.j_start);
    Ok(match limit {
        JumpLimit::Measured(m) => {
            format!("m = {m} pulses/s², measured: write it into __synthetic__/trainer/python/settings.toml [drive] jump_limit")
        }
        JumpLimit::AtLeast(m) => format!("m = at least {m} pulses/s²: no slip found"),
        JumpLimit::Below(m) => format!("m is below j_start, {m}: lower j_start and run again"),
        JumpLimit::Incomplete => "m: not every kind finished".to_owned(),
    })
}
