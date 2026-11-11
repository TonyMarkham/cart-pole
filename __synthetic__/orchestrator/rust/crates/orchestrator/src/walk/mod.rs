pub(crate) mod event;
pub(crate) mod in_flight;
pub(crate) mod stage;
pub(crate) mod tally;
pub(crate) mod world;

// ---------------------------------------------------------------------------------------------- //

use crate::{
    schedule::Schedule,
    walk::{event::Event, in_flight::InFlight, stage::Stage, tally::Tally, world::World},
};

use anyhow::{Result, anyhow, bail};
use arduino::LimitSwitches;

/// One episode's walk through its schedule (orchestrator-plan.md, O4, O8,
/// O8a). On t = 0, step 0's capture; then on every timestep, in order:
/// MuJoCo moves on; its motor changes in step order, each landing (its step
/// later than the last landed step) or gobbled; its send; the synthetic
/// Arduino's tick; its capture. Rules (each checkable by hand):
/// - every event happens on exactly its scheduled timestep;
/// - one comparison per motor change decides landing or gobbling, and the
///   last to land on a timestep is the command the tick is given;
/// - a lost command has no motor change and so never lands;
/// - after a FAULT the walk does not move.
pub struct Walk {
    schedule: Schedule,
    /// Timesteps since the reset: t = now × the timestep.
    now: i64,
    stage: Stage,
    next_capture: i64,
    next_send: i64,
    /// The step whose send was last made, awaiting `command`.
    sent: Option<i64>,
    in_flight: Vec<InFlight>,
    last_landed: Option<i64>,
    /// The command that landed on this timestep, for its tick.
    landed: Option<i32>,
    switches: LimitSwitches,
    faulted: bool,
    tally: Tally,
}

impl Walk {
    /// At the reset: t = 0, before step 0's capture; nothing in flight, the
    /// last landed step none.
    pub fn new(schedule: Schedule) -> Walk {
        Walk {
            schedule,
            now: 0,
            stage: Stage::Capture,
            next_capture: 0,
            next_send: 0,
            sent: None,
            in_flight: Vec::new(),
            last_landed: None,
            landed: None,
            switches: LimitSwitches::OPEN,
            faulted: false,
            tally: Tally::default(),
        }
    }

    /// Moves the world on to the next event and stops there.
    pub fn advance(&mut self, world: &mut impl World) -> Result<Event> {
        if self.faulted {
            bail!("the episode ended in FAULT: only a reset or a stop goes on");
        }
        loop {
            match self.stage {
                Stage::Capture => {
                    self.stage = Stage::MoveOn;
                    let due = self
                        .schedule
                        .step(self.next_capture)
                        .is_some_and(|times| times.capture == self.now);
                    if due {
                        let n = self.next_capture;
                        self.next_capture += 1;
                        return Ok(Event::Capture(n));
                    }
                }
                Stage::MoveOn => {
                    self.switches = world.move_on()?;
                    self.now += 1;
                    self.land_motor_changes();
                    self.stage = Stage::Send;
                }
                Stage::Send => {
                    self.stage = Stage::Tick;
                    let times = self.schedule.step(self.next_send).ok_or_else(|| {
                        anyhow!(
                            "the episode outran its schedule: step {} is past its last, {} (--max-episode-steps + 14)",
                            self.next_send,
                            self.schedule.last_step()
                        )
                    })?;
                    if times.send == self.now {
                        let n = self.next_send;
                        self.next_send += 1;
                        self.sent = Some(n);
                        return Ok(Event::Send(n));
                    }
                }
                Stage::Tick => {
                    self.stage = Stage::Capture;
                    if world.tick(self.landed.take(), self.switches)? {
                        self.faulted = true;
                        return Ok(Event::Fault);
                    }
                }
            }
        }
    }

    /// The motor changes due on this timestep, in step order, by the
    /// gobble rule (__documentation__/synthetic-loop-dia.md, *The gobble rule*).
    fn land_motor_changes(&mut self) {
        let now = self.now;
        let mut due: Vec<InFlight> = self
            .in_flight
            .iter()
            .copied()
            .filter(|c| c.motor_change == now)
            .collect();
        self.in_flight.retain(|c| c.motor_change != now);
        due.sort_by_key(|c| c.step);
        for change in due {
            if self.last_landed.is_none_or(|last| change.step > last) {
                self.last_landed = Some(change.step);
                self.landed = Some(change.command);
                self.tally.landed += 1;
            } else {
                self.tally.gobbled += 1;
            }
        }
    }

    /// Continue, after `Event::Send(step)`: the step's command is in flight
    /// until its motor change, or lost.
    pub fn command(&mut self, step: i64, command: i32) -> Result<()> {
        if self.sent != Some(step) {
            bail!("step {step}'s command came without its send");
        }
        self.sent = None;
        match self
            .schedule
            .step(step)
            .and_then(|times| times.motor_change)
        {
            Some(motor_change) => self.in_flight.push(InFlight {
                motor_change,
                step,
                command,
            }),
            None => self.tally.lost += 1,
        }
        Ok(())
    }

    /// After a FAULT: the step whose line the trainer is owed, the one after
    /// the last sent. Its capture came before that send (*The schedule*), so
    /// it is already taken.
    pub fn owed(&self) -> i64 {
        self.next_send
    }

    /// The current time, ms from the episode's start.
    pub fn time_ms(&self) -> f64 {
        self.now as f64 * self.schedule.timestep_ms()
    }

    pub fn schedule(&self) -> &Schedule {
        &self.schedule
    }

    pub fn tally(&self) -> Tally {
        self.tally
    }
}
