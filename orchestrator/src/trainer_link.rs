use crate::{orin_reply::PoolRow, pole_message::PoleMessage, sites_message::SitesMessage};

use anyhow::{Context, Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, Write};

/// What the trainer's first stdin line asks for: the run's values.
#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum Request {
    Run,
}

/// The trainer's first stdin line, `{"request": "run"}`
/// (synthetic-loop-dia.md, Contracts, the run's values).
#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct RequestMessage {
    #[allow(dead_code)]
    request: Request,
}

/// The run's values, from the model MuJoCo loaded and the drive figures:
/// the cart top speed in m/s (the actuator's ctrlrange), the top pulse
/// rate in pulses/s (the synthetic Arduino's clamp), the timestep in s,
/// and the pole's body (synthetic-loop-dia.md, Contracts, the pole): its
/// mass in kg, its centre of mass's distance from the hinge axis in m, its
/// inertia about that axis in kg·m², and gravity in m/s².
#[derive(Serialize, Debug, Clone, Copy, PartialEq)]
pub struct RunValues {
    pub cart_top_speed: f64,
    pub top_pulse_rate: i32,
    pub timestep: f64,
    pub pole_mass: f64,
    pub pole_com_distance: f64,
    pub pole_inertia: f64,
    pub gravity: f64,
}

/// The answer, the orchestrator's first stdout line:
/// `{"run": {"cart_top_speed": v, "top_pulse_rate": r, "timestep": s,
/// "pole_mass": m, "pole_com_distance": l, "pole_inertia": I, "gravity":
/// g}}`.
#[derive(Serialize)]
struct RunMessage<'a> {
    run: &'a RunValues,
}

/// 17: one instance's pool to the trainer, the same rows as its Orin's
/// reply (14) without `failed`: a failed pool goes to the shutdown and
/// never reaches the trainer. `time` is the capture's MuJoCo time in s,
/// and `sites` and `pole` its true site positions and pole state, for the
/// reward and the record alone.
#[derive(Serialize)]
pub struct PoolEntry<'a> {
    pub instance: usize,
    pub id: u32,
    pub time: f64,
    pub pool: &'a [PoolRow],
    pub sites: &'a SitesMessage,
    pub pole: &'a PoleMessage,
}

/// 17: the stdout line, one entry per instance advanced, in instance
/// order (synthetic-loop-dia.md, Training instances).
#[derive(Serialize)]
struct PoolsMessage<'a> {
    pools: &'a [PoolEntry<'a>],
}

/// What the trainer wants after an instance's pool (18): go on with the
/// command, reset the instance in place for its next episode
/// (synthetic-loop-dia.md, *Reset in place*), or stop the run.
#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Next {
    Continue,
    Reset,
    Stop,
}

/// 18: one instance's answer: `{"instance": k, "command": c, "next":
/// "continue" | "reset" | "stop"}`, plus `"seed": S` on a reset only.
/// serde rejects a command that is not an integer or does not fit an i32,
/// and a seed outside a u64, so a bad answer fails the run here instead of
/// reaching the synthetic Arduino.
#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(deny_unknown_fields)]
pub struct Answer {
    pub instance: usize,
    pub command: i32,
    pub next: Next,
    pub seed: Option<u64>,
}

/// 18: the stdin line.
#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct AnswersMessage {
    answers: Vec<Answer>,
}

/// Checks a line's answers against the instances waiting for one: each
/// answers a pending instance, at most once; a reset carries the next
/// episode's seed, and nothing else does; a stop is for the whole run, so
/// a line with a stop answers every pending instance with stop.
pub fn check_answers(answers: &[Answer], pending: &[bool]) -> Result<()> {
    if answers.is_empty() {
        bail!("the line answers no instance");
    }
    let mut seen = vec![false; pending.len()];
    for answer in answers {
        let k = answer.instance;
        if k >= pending.len() || !pending[k] {
            bail!("instance {k} has no pool waiting for an answer");
        }
        if seen[k] {
            bail!("instance {k} is answered twice");
        }
        seen[k] = true;
        match (answer.next, answer.seed) {
            (Next::Reset, None) => bail!("instance {k}: a reset needs the next episode's seed"),
            (Next::Continue | Next::Stop, Some(_)) => {
                bail!("instance {k}: only a reset carries a seed")
            }
            _ => {}
        }
    }
    let stops = answers.iter().filter(|a| a.next == Next::Stop).count();
    if stops > 0 && (stops != answers.len() || seen != pending) {
        bail!("a stop is for the whole run: every pending instance must be answered with stop");
    }
    Ok(())
}

/// The trainer link (synthetic-loop-dia.md, *Trainer link*): the
/// orchestrator's stdin and stdout, the pipes of the trainer that launched
/// it. stdout carries these lines and nothing else; every other line the
/// orchestrator or its children print goes to stderr.
pub struct TrainerLink {
    line: String,
}

impl TrainerLink {
    pub fn new() -> TrainerLink {
        TrainerLink {
            line: String::new(),
        }
    }

    /// Before the first pools line, once per run: reads the trainer's
    /// request for the run's values and answers it. A first line that is
    /// not the request fails the run.
    pub fn answer_request(&mut self, values: &RunValues) -> Result<()> {
        self.line.clear();
        let read =
            tokio::task::block_in_place(|| std::io::stdin().lock().read_line(&mut self.line))
                .context("reading the trainer's request")?;
        if read == 0 {
            bail!("the trainer closed the link before its request for the run's values");
        }
        parse_request(self.line.trim_end()).map_err(|error| {
            anyhow!("the trainer's request, {:?}: {error}", self.line.trim_end())
        })?;
        let mut stdout = std::io::stdout().lock();
        writeln!(stdout, "{}", run_line(values)?)
            .and_then(|()| stdout.flush())
            .context("sending the run's values to the trainer")
    }

    /// 17, then 18: sends the advanced instances' pools and waits for the
    /// answers. Every MuJoCo stays frozen at its arrival step meanwhile, so
    /// the wait has no timeout: the trainer's time is not simulated time.
    /// Nothing else runs during it, so the blocking read is done in place.
    pub fn exchange(&mut self, pools: &[PoolEntry], pending: &[bool]) -> Result<Vec<Answer>> {
        let message = serde_json::to_string(&PoolsMessage { pools })?;
        let mut stdout = std::io::stdout().lock();
        writeln!(stdout, "{message}")
            .and_then(|()| stdout.flush())
            .context("sending the pools to the trainer")?;
        drop(stdout);

        self.line.clear();
        let read =
            tokio::task::block_in_place(|| std::io::stdin().lock().read_line(&mut self.line))
                .context("reading the trainer's answers")?;
        if read == 0 {
            bail!("the trainer closed the link before answering");
        }
        parse_answers(self.line.trim_end(), pending)
            .map_err(|error| anyhow!("the trainer's answers, {:?}: {error}", self.line.trim_end()))
    }
}

pub fn parse_request(line: &str) -> Result<()> {
    serde_json::from_str::<RequestMessage>(line)?;
    Ok(())
}

pub fn run_line(values: &RunValues) -> Result<String> {
    Ok(serde_json::to_string(&RunMessage { run: values })?)
}

pub fn parse_answers(line: &str, pending: &[bool]) -> Result<Vec<Answer>> {
    let answers = serde_json::from_str::<AnswersMessage>(line)?.answers;
    check_answers(&answers, pending)?;
    Ok(answers)
}
