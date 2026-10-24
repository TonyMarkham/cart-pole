use crate::orin_reply::PoolRow;

use anyhow::{Context, Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, Write};

/// 17: one instance's pool to the trainer, the same rows as its Orin's
/// reply (14) without `failed`: a failed pool goes to the shutdown and
/// never reaches the trainer.
#[derive(Serialize)]
pub struct PoolEntry<'a> {
    pub instance: usize,
    pub id: u32,
    pub pool: &'a [PoolRow],
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
            (Next::Continue | Next::Stop, Some(_)) => bail!("instance {k}: only a reset carries a seed"),
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
        TrainerLink { line: String::new() }
    }

    /// 17, then 18: sends the advanced instances' pools and waits for the
    /// answers. Every MuJoCo stays frozen at its arrival step meanwhile, so
    /// the wait has no timeout: the trainer's time is not simulated time.
    /// Nothing else runs during it, so the blocking read is done in place.
    pub fn exchange(&mut self, pools: &[PoolEntry], pending: &[bool]) -> Result<Vec<Answer>> {
        let message = serde_json::to_string(&PoolsMessage { pools })?;
        let mut stdout = std::io::stdout().lock();
        writeln!(stdout, "{message}").and_then(|()| stdout.flush()).context("sending the pools to the trainer")?;
        drop(stdout);

        self.line.clear();
        let read = tokio::task::block_in_place(|| std::io::stdin().lock().read_line(&mut self.line))
            .context("reading the trainer's answers")?;
        if read == 0 {
            bail!("the trainer closed the link before answering");
        }
        parse_answers(self.line.trim_end(), pending)
            .map_err(|error| anyhow!("the trainer's answers, {:?}: {error}", self.line.trim_end()))
    }
}

pub fn parse_answers(line: &str, pending: &[bool]) -> Result<Vec<Answer>> {
    let answers = serde_json::from_str::<AnswersMessage>(line)?.answers;
    check_answers(&answers, pending)?;
    Ok(answers)
}
