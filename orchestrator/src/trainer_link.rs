use crate::orin_reply::PoolRow;

use anyhow::{Context, Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, Write};

/// 17: the pool to the trainer, one stdout line, the same rows as the
/// Orin's reply (14) without `failed`: a failed pool goes to the shutdown
/// and never reaches the trainer.
#[derive(Serialize)]
struct PoolMessage<'a> {
    id: u32,
    pool: &'a [PoolRow],
}

/// 18: the trainer's answer, one stdin line. serde rejects a command that
/// is not an integer or does not fit an i32, so a bad answer fails the run
/// here instead of reaching the synthetic Arduino.
#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Answer {
    pub command: i32,
    #[serde(rename = "continue")]
    pub keep_going: bool,
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

    /// 17, then 18: sends the pool and waits for the answer. MuJoCo stays
    /// frozen at the arrival step meanwhile, so the wait has no timeout:
    /// the trainer's time is not simulated time. Nothing else runs during
    /// it, so the blocking read is done in place.
    pub fn exchange(&mut self, id: u32, pool: &[PoolRow]) -> Result<Answer> {
        let message = serde_json::to_string(&PoolMessage { id, pool })?;
        let mut stdout = std::io::stdout().lock();
        writeln!(stdout, "{message}").and_then(|()| stdout.flush()).context("sending the pool to the trainer")?;
        drop(stdout);

        self.line.clear();
        let read = tokio::task::block_in_place(|| std::io::stdin().lock().read_line(&mut self.line))
            .context("reading the trainer's answer")?;
        if read == 0 {
            bail!("the trainer closed the link before answering pool {id}");
        }
        serde_json::from_str::<Answer>(self.line.trim_end())
            .map_err(|error| anyhow!("the trainer's answer to pool {id}, {:?}: {error}", self.line.trim_end()))
    }
}
