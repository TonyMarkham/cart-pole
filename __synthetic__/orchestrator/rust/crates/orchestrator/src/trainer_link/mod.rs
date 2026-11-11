pub(crate) mod answer;
pub(crate) mod answers_message;
pub(crate) mod next;
pub(crate) mod pole_message;
pub(crate) mod pool_entry;
pub(crate) mod pools_message;
pub(crate) mod request;
pub(crate) mod request_message;
pub(crate) mod run_message;
pub(crate) mod run_values;
pub(crate) mod sites_message;

// ---------------------------------------------------------------------------------------------- //

use crate::trainer_link::{
    answer::Answer, answers_message::AnswersMessage, next::Next, pool_entry::PoolEntry,
    pools_message::PoolsMessage, request_message::RequestMessage, run_message::RunMessage,
    run_values::RunValues,
};

use anyhow::{Context, Result, anyhow, bail};
use std::io::{BufRead, Write};

/// Checks a line's answers against the instances waiting for one: each
/// answers a pending instance, at most once; a reset carries the next
/// episode's seed, and nothing else does; an instance whose episode ended
/// in FAULT (`faulted`) is answered with reset or stop, never continue; a
/// stop is for the whole run, so a line with a stop answers every pending
/// instance with stop.
pub fn check_answers(answers: &[Answer], pending: &[bool], faulted: &[bool]) -> Result<()> {
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
        if answer.next == Next::Continue && faulted.get(k).copied().unwrap_or(false) {
            bail!("instance {k}: its episode ended in FAULT; answer reset or stop");
        }
    }
    let stops = answers.iter().filter(|a| a.next == Next::Stop).count();
    if stops > 0 && (stops != answers.len() || seen != pending) {
        bail!("a stop is for the whole run: every pending instance must be answered with stop");
    }
    Ok(())
}

/// The trainer link (__documentation__/synthetic-loop-dia.md, *Trainer link*): the
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
    /// answers. Every MuJoCo stays frozen at its send meanwhile, so
    /// the wait has no timeout: the trainer's time is not simulated time.
    /// Nothing else runs during it, so the blocking read is done in place.
    pub fn exchange(
        &mut self,
        pools: &[PoolEntry],
        pending: &[bool],
        faulted: &[bool],
    ) -> Result<Vec<Answer>> {
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
        parse_answers(self.line.trim_end(), pending, faulted)
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

pub fn parse_answers(line: &str, pending: &[bool], faulted: &[bool]) -> Result<Vec<Answer>> {
    let answers = serde_json::from_str::<AnswersMessage>(line)?.answers;
    check_answers(&answers, pending, faulted)?;
    Ok(answers)
}
