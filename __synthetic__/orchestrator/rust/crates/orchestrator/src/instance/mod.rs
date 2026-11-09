pub(crate) mod held;
pub(crate) mod owed;

// ---------------------------------------------------------------------------------------------- //

use crate::{
    instance::{held::Held, owed::Owed},
    orin::{receive_reply, reset_orin, send_pose},
    simulation::Simulation,
    walk::{Walk, event::Event},
};

use anyhow::{Result, bail};
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};
use tokio::{net::TcpStream, process::Child};
use tokio_tungstenite::WebSocketStream;

pub(crate) struct Instance {
    pub(crate) k: usize,
    pub(crate) sim: Simulation,
    pub(crate) walk: Walk,
    pub(crate) orin: WebSocketStream<TcpStream>,
    pub(crate) orin_child: Child,
    pub(crate) player: Option<Child>,
    pub(crate) frames_url: String,
    // Ids count on across the instance's episodes.
    pub(crate) next_id: u32,
    // Captured steps, oldest first, until their sends.
    pub(crate) held: VecDeque<Held>,
    pub(crate) owed: Option<Owed>,
}

impl Instance {
    // The walk on to this instance's next send or a FAULT (synthetic-loop-
    // dia.md, the loop): at each capture, 11–14 and the truth row, MuJoCo
    // waiting there for the reply; at the send, or the FAULT, the line owed
    // is set aside for the round's line (17).
    pub(crate) async fn walk_to_send(&mut self, timeout: Duration) -> Result<()> {
        loop {
            match self.walk.advance(&mut self.sim)? {
                Event::Capture(step) => {
                    let id = self.next_id;
                    self.next_id += 1;
                    let capture = self.sim.capture();
                    let sent = Instant::now();
                    send_pose(&mut self.orin, id, &capture, &self.sim.names).await?;
                    let reply = receive_reply(&mut self.orin, id, timeout).await?;
                    let orin_ms = sent.elapsed().as_secs_f64() * 1000.0;
                    self.sim.log(id, &capture, Some(&reply.pool[0]))?;
                    self.held.push_back(Held {
                        step,
                        id,
                        capture,
                        pool: reply.pool,
                        state: reply.state,
                        orin_ms,
                    });
                }
                Event::Send(step) => {
                    self.owed = Some(Owed {
                        held: self.take_held(step)?,
                        fault: false,
                        at_ms: self.walk.time_ms(),
                    });
                    return Ok(());
                }
                Event::Fault => {
                    self.owed = Some(Owed {
                        held: self.take_held(self.walk.owed())?,
                        fault: true,
                        at_ms: self.walk.time_ms(),
                    });
                    return Ok(());
                }
            }
        }
    }

    // Step `step`'s held capture: the oldest, since captures and sends both
    // go in step order.
    pub(crate) fn take_held(&mut self, step: i64) -> Result<Held> {
        match self.held.pop_front() {
            Some(held) if held.step == step => Ok(held),
            Some(held) => bail!("step {step}'s send found step {}'s capture", held.step),
            None => bail!("step {step}'s send came before its capture"),
        }
    }

    // R1–R7: back to the start in place, with the new episode's schedule
    // from its seed; the captures and commands of the last episode go.
    pub(crate) async fn reset(&mut self, seed: u64, timeout: Duration) -> Result<()> {
        self.sim.reset()?;
        self.walk = Walk::new(self.sim.schedule(seed));
        self.held.clear();
        reset_orin(&mut self.orin, self.walk.schedule().pool_gaps(), timeout).await
    }

    // The round's log line for the line this instance owes.
    pub(crate) fn line(&self) -> String {
        let Some(owed) = &self.owed else {
            return format!("[{}] nothing owed", self.k);
        };
        let held = &owed.held;
        let n = &held.pool[0];
        let tally = self.walk.tally();
        format!(
            "[{}] step {} id {:06}  captured t {:.3} s, {} t {:.3} s  cart {:+.3} m  pole {:.3} rad ({:.1}°)  orin {:.1} ms  newest pivot ({:+.3}, {:+.3}) m, end ({:+.3}, {:+.3}) m  state {}  commands landed {}, gobbled {}, lost {}",
            self.k,
            held.step,
            held.id,
            held.capture.time,
            if owed.fault { "FAULT at" } else { "sent" },
            owed.at_ms / 1000.0,
            held.capture.pose.cart,
            held.capture.pose.pole,
            held.capture.pose.pole.to_degrees(),
            held.orin_ms,
            n[1],
            n[2],
            n[3],
            n[4],
            held.state,
            tally.landed,
            tally.gobbled,
            tally.lost,
        )
    }
}
