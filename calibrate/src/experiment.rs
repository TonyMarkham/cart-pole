//! The jump-limit experiment (jump-limit-experiment.md): the largest
//! change of pulse rate the motor follows without losing steps, `m`, in
//! pulses/s². The limit switch at the first end is the reference: each
//! trial starts at the centre, plays one jump, stops gently, and returns
//! slowly to that switch. Every pulse is counted, so if the motor kept
//! up, the switch trips at the same count as before; a count further off
//! than the noise floor is that many lost steps, and the trial failed.

use core::fmt::{self, Write};

use unor4_hal::delay::Delay;

use crate::{
    action::Action,
    axis::Axis,
    direction::Direction,
    halt::Halt,
    kind::Kind,
    monitor::Monitor,
    outcome::Outcome,
    report::Report,
    settings::{
        CONDITIONS, END_MARGIN_PULSES, J_GROWTH_PERCENT, J_START, NOISE_AWAY_PULSES, NOISE_REPEATS, REPORT_HOST,
        REPORT_PORT, SETTLE_MS,
        TRIALS_PER_J, TRIAL_DT_MS, TRIAL_TOP,
    },
    switch::Switch,
    wifi::Wifi,
};

/// The reference the trials are measured against (the brief's section 5,
/// step 1).
#[derive(Clone, Copy, Debug)]
pub struct Reference {
    /// The switch at the first end.
    pub switch: Switch,
    /// The mean count at which it tripped over the noise-floor returns.
    pub count: i32,
    /// Their spread, largest minus smallest: a difference no larger is
    /// not a lost step.
    pub noise: u32,
}

/// One kind's findings so far.
#[derive(Clone, Copy, Debug, Default)]
pub struct KindResult {
    /// The largest j whose trials all kept every step.
    pub passed: Option<u32>,
    /// The j at which a trial lost steps, and the most it lost.
    pub failed: Option<(u32, u32)>,
    /// No slip up to the largest j the kind can make: its limit is "at
    /// least" `passed`.
    pub at_least: bool,
    /// The j whose plan did not fit the rail from the centre, which ended
    /// the kind there.
    pub no_room: Option<u32>,
    pub done: bool,
}

/// One trial's outcome, the last one run.
#[derive(Clone, Copy, Debug)]
pub struct Trial {
    pub kind: Kind,
    pub j: u32,
    pub number: u32,
    /// The switch's count minus the reference: + means the cart came back
    /// further than the count says.
    pub lost: i32,
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
#[derive(Clone, Copy, Debug, Default)]
pub struct ExperimentReport {
    pub started: bool,
    pub reference: Option<Reference>,
    pub kinds: [KindResult; 4],
    /// The kind, j and trial running now.
    pub now: Option<(Kind, u32, u32)>,
    pub trials: u32,
    pub last: Option<Trial>,
}

impl ExperimentReport {
    /// m = the smallest of the kinds' limits, so it is safe for all of
    /// them (section 5, step 5).
    pub fn jump_limit(&self) -> JumpLimit {
        if self.kinds.iter().any(|kind| kind.done && kind.passed.is_none() && kind.failed.is_some()) {
            return JumpLimit::Below(J_START);
        }
        if !self.kinds.iter().all(|kind| kind.done && kind.passed.is_some() && kind.no_room.is_none()) {
            return JumpLimit::Incomplete;
        }
        let lowest = self.kinds.iter().filter_map(|kind| kind.passed).min().unwrap_or(0);
        let measured = self.kinds.iter().any(|kind| kind.passed == Some(lowest) && !kind.at_least);
        if measured { JumpLimit::Measured(lowest) } else { JumpLimit::AtLeast(lowest) }
    }

    pub fn write(&self, out: &mut impl Write) -> fmt::Result {
        if !self.started {
            return writeln!(out, "jump-limit experiment: not run");
        }
        writeln!(out, "jump-limit experiment: {CONDITIONS}")?;
        writeln!(out, "  Δt {TRIAL_DT_MS} ms, top {TRIAL_TOP} pulses/s, {TRIALS_PER_J} trials per j")?;
        match self.reference {
            Some(reference) => writeln!(
                out,
                "  noise floor: {} at pulse {}, spread {} pulses over {NOISE_REPEATS} returns",
                reference.switch.name(),
                reference.count,
                reference.noise
            )?,
            None => writeln!(out, "  noise floor: measuring")?,
        }
        for (kind, result) in Kind::ALL.into_iter().zip(self.kinds) {
            write!(out, "  {:<10} ", kind.name())?;
            match (result.passed, result.failed, result.no_room) {
                (_, _, Some(j)) => write!(out, "no room on the rail at j {j}; ")?,
                (Some(j), None, None) if result.at_least => write!(out, "at least {j} (no slip up to what it can make)")?,
                (Some(j), Some((failed, lost)), None) => write!(out, "{j} (lost {lost} at {failed})")?,
                (None, Some((failed, lost)), None) => write!(out, "below {failed} (lost {lost} there)")?,
                (Some(j), None, None) => write!(out, "passed up to {j}")?,
                (None, None, None) => write!(out, "-")?,
            }
            if let Some((now, j, trial)) = self.now
                && now == kind
            {
                write!(out, "  running: j {j}, trial {trial} of {TRIALS_PER_J}")?;
            }
            writeln!(out)?;
        }
        match self.jump_limit() {
            JumpLimit::Measured(m) => writeln!(out, "  m = {m} pulses/s², measured")?,
            JumpLimit::AtLeast(m) => writeln!(out, "  m = at least {m} pulses/s² (no slip found)")?,
            JumpLimit::Below(m) => writeln!(out, "  m is below j_start, {m} pulses/s²: lower j_start")?,
            JumpLimit::Incomplete => writeln!(out, "  m: not every kind has finished")?,
        }
        if let Some(trial) = self.last {
            writeln!(
                out,
                "  trials run: {}; last: {} at j {}, trial {}, lost {}: {}",
                self.trials,
                trial.kind.name(),
                trial.j,
                trial.number,
                trial.lost,
                if trial.pass { "pass" } else { "FAIL" }
            )?;
        }
        Ok(())
    }
}

/// Runs the whole experiment: the noise floor, then each kind from
/// j_start, raising j by `j_growth_percent` until a trial loses steps or j
/// reaches the most the kind can make. Needs a finished homing, which set
/// the first end and the span. STOP works in every slow move; it is not
/// checked during a trial's plan, which lasts about a second.
pub fn run(
    axis: &mut Axis,
    delay: &mut Delay,
    monitor: &mut Monitor,
    report: &mut Report,
    wifi: &mut Option<Wifi>,
) -> Result<Outcome, Halt> {
    let (Some(span), Some((reference, _))) = (report.span.filter(|_| report.homed), report.first) else {
        return Err(Halt::NotHomed);
    };
    report.experiment = ExperimentReport { started: true, ..ExperimentReport::default() };
    monitor.line(format_args!(
        "jump-limit experiment: {CONDITIONS}; Δt {TRIAL_DT_MS} ms, top {TRIAL_TOP} pulses/s"
    ));

    // 1. The noise floor: slow returns to the reference switch.
    let (mut low, mut high, mut sum) = (i32::MAX, i32::MIN, 0i64);
    for repeat in 0..NOISE_REPEATS {
        if repeat > 0 {
            slow(axis, delay, wifi, report, Direction::Plus, NOISE_AWAY_PULSES)?;
        }
        let count = back_to(axis, delay, wifi, report, reference)?;
        monitor.line(format_args!("noise floor {}: {} tripped at pulse {count}", repeat + 1, reference.name()));
        low = low.min(count);
        high = high.max(count);
        sum += i64::from(count);
    }
    let reference = Reference {
        switch: reference,
        count: (sum / i64::from(NOISE_REPEATS)) as i32,
        noise: low.abs_diff(high),
    };
    report.experiment.reference = Some(reference);
    monitor.line(format_args!(
        "noise floor: reference pulse {}, spread {} pulses",
        reference.count, reference.noise
    ));

    // 2–4. Each kind, j raised step by step.
    let centre = (span / 2) as i32;
    let room = i64::from(span / 2) - i64::from(END_MARGIN_PULSES);
    for (index, kind) in Kind::ALL.into_iter().enumerate() {
        let most = kind.j_max();
        let mut j = J_START;
        while j <= most {
            let Some(plan) = kind.plan(j) else {
                report.experiment.kinds[index].no_room = Some(j);
                break;
            };
            let (lowest, highest) = plan.excursion();
            if lowest < -room || highest > room {
                monitor.line(format_args!("{} at j {j}: the plan needs {lowest}..{highest} pulses from the centre, more than the rail has", kind.name()));
                report.experiment.kinds[index].no_room = Some(j);
                break;
            }
            let (c_a, c_b) = kind.rates(j);
            let mut worst = 0;
            let mut failed = false;
            for number in 1..=TRIALS_PER_J {
                report.experiment.now = Some((kind, j, number));
                let to_go = centre - axis.position();
                let towards = if to_go < 0 { Direction::Minus } else { Direction::Plus };
                slow(axis, delay, wifi, report, towards, to_go.unsigned_abs())?;
                delay_ms(delay, SETTLE_MS);
                axis.run_profile(&plan)?;
                delay_ms(delay, SETTLE_MS);
                let count = back_to(axis, delay, wifi, report, reference.switch)?;
                let lost = count - reference.count;
                let pass = lost.unsigned_abs() <= reference.noise;
                axis.set_position(reference.count);
                worst = worst.max(lost.unsigned_abs());
                report.experiment.trials += 1;
                report.experiment.last = Some(Trial { kind, j, number, lost, pass });
                monitor.line(format_args!(
                    "trial {} j {j} (c_a {c_a}, c_b {c_b}, Δt {TRIAL_DT_MS} ms) {number}/{TRIALS_PER_J}: switch at {count}, lost {lost}: {}",
                    kind.name(),
                    if pass { "pass" } else { "FAIL" }
                ));
                // The trial is over and the cart rests at the reference
                // switch: the one moment its record goes to the bench
                // station ([report], wifi.rs `post_trial`).
                if let Some(wifi) = wifi.as_mut() {
                    let mut record = crate::buffer::Buffer::<96>::new();
                    let written = write!(
                        record,
                        "{},{j},{number},{c_a},{c_b},{TRIAL_DT_MS},{count},{lost},{}",
                        kind.name(),
                        if pass { "pass" } else { "fail" }
                    );
                    let text = core::str::from_utf8(record.as_bytes()).unwrap_or("");
                    if written.is_err() || !wifi.post_trial(delay, text) {
                        monitor.line(format_args!("trial record not taken by the bench station at {REPORT_HOST}:{REPORT_PORT}"));
                    }
                }
                if !pass {
                    failed = true;
                    break;
                }
            }
            let result = &mut report.experiment.kinds[index];
            if failed {
                result.failed = Some((j, worst));
                break;
            }
            result.passed = Some(j);
            if j == most {
                result.at_least = true;
                break;
            }
            let raised = (u64::from(j) * u64::from(100 + J_GROWTH_PERCENT) / 100).min(u64::from(most)) as u32;
            j = raised.max(j + 1);
        }
        report.experiment.kinds[index].done = true;
        report.experiment.now = None;
        let mut line = crate::buffer::Buffer::<160>::new();
        let _ = report.experiment.write_kind(&mut line, index);
        monitor.line(format_args!("{}", core::str::from_utf8(line.as_bytes()).unwrap_or("")));
    }

    let limit = report.experiment.jump_limit();
    match limit {
        JumpLimit::Measured(m) => monitor.line(format_args!("m = {m} pulses/s², measured: write it into __synthetic__/trainer/python/settings.toml [drive] jump_limit")),
        JumpLimit::AtLeast(m) => monitor.line(format_args!("m = at least {m} pulses/s²: no slip found")),
        JumpLimit::Below(m) => monitor.line(format_args!("m is below j_start, {m}: lower j_start and run again")),
        JumpLimit::Incomplete => monitor.line(format_args!("m: not every kind finished")),
    }
    Ok(Outcome::Measured(limit))
}

impl ExperimentReport {
    /// One kind's line, for the log.
    fn write_kind(&self, out: &mut impl Write, index: usize) -> fmt::Result {
        let result = self.kinds[index];
        write!(out, "{}: ", Kind::ALL[index].name())?;
        match (result.passed, result.failed, result.no_room) {
            (_, _, Some(j)) => write!(out, "ended by no room on the rail at j {j}"),
            (Some(j), None, None) if result.at_least => write!(out, "at least {j}"),
            (Some(j), Some((failed, lost)), None) => write!(out, "limit {j}; lost {lost} at {failed}"),
            (None, Some((failed, lost)), None) => write!(out, "below {failed}; lost {lost} there"),
            (Some(j), None, None) => write!(out, "passed up to {j}"),
            (None, None, None) => write!(out, "j_start is above what it can make"),
        }
    }
}

/// A slow move of `pulses` in `towards`, with STOP checked against the
/// experiment's progress so far. A switch tripping on the way halts:
/// the moves stay well clear of both ends.
fn slow(
    axis: &mut Axis,
    delay: &mut Delay,
    wifi: &mut Option<Wifi>,
    report: &Report,
    towards: Direction,
    pulses: u32,
) -> Result<(), Halt> {
    let progress = report.experiment;
    let mut stop = |delay: &mut Delay| {
        wifi.as_mut().is_some_and(|wifi| wifi.stop_requested(delay, Action::Experiment, Some(&progress)))
    };
    match axis.slow(delay, towards, pulses, &mut stop)? {
        Some(switch) => Err(Halt::SwitchOnTheWay { switch, position: axis.position() }),
        None => Ok(()),
    }
}

/// A slow seek in Minus to the reference switch: returns the count at
/// which it tripped. Another switch tripping means the cart is lost.
fn back_to(
    axis: &mut Axis,
    delay: &mut Delay,
    wifi: &mut Option<Wifi>,
    report: &Report,
    reference: Switch,
) -> Result<i32, Halt> {
    let progress = report.experiment;
    let mut stop = |delay: &mut Delay| {
        wifi.as_mut().is_some_and(|wifi| wifi.stop_requested(delay, Action::Experiment, Some(&progress)))
    };
    let (_, switch) = axis.seek(delay, Direction::Minus, &mut stop)?;
    if switch != reference {
        return Err(Halt::WrongReference { switch });
    }
    Ok(axis.position())
}

fn delay_ms(delay: &mut Delay, ms: u32) {
    embedded_hal::delay::DelayNs::delay_ms(delay, ms);
}
