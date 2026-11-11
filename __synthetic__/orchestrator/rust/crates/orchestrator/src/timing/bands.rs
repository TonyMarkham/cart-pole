use crate::timing::band::Band;

use anyhow::{Result, bail};

/// The random timings' bands and the loss, the trainer's settings.toml
/// [orchestrator] (user, 2026-11-08), each from its flag of the same name.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bands {
    /// Stretch 1, capture → picture ready: --frame-ready-min/-max.
    pub frame_ready: Band,
    /// Stretch 2, picture ready → send: --to-uno-min/-max.
    pub to_uno: Band,
    /// Stretch 3, send → motor change: --change-motor-min/-max.
    pub change_motor: Band,
    /// The wait, send → the next step's picture ready:
    /// --next-frame-ready-min/-max.
    pub next_frame_ready: Band,
    /// One command in this many is lost: --drop-one-message-in, at least 1.
    pub drop_one_message_in: u32,
}

impl Bands {
    /// Fails unless every band is finite with 0 ≤ min ≤ max, the loss is at
    /// least 1, and the bands keep the orders the walk relies on
    /// (__documentation__/synthetic-loop-dia.md, *The schedule*, *Captures stay in order*).
    /// Each order is an exact gap of at least one timestep, which two
    /// roundings to the nearest timestep can never bring to 0:
    /// - each stretch's min: a step's own timings never share a timestep;
    /// - frame_ready.min − next_frame_ready.max: the next step's capture
    ///   comes before this step's send;
    /// - frame_ready.min + to_uno.min + next_frame_ready.min −
    ///   frame_ready.max: captures stay in order (15.5 ms today).
    pub fn check(&self, timestep_ms: f64) -> Result<()> {
        for (name, band) in [
            ("frame-ready", self.frame_ready),
            ("to-uno", self.to_uno),
            ("change-motor", self.change_motor),
            ("next-frame-ready", self.next_frame_ready),
        ] {
            let Band { min, max } = band;
            if !(min.is_finite() && max.is_finite() && 0.0 <= min && min <= max) {
                bail!(
                    "the {name} band is 0 ≤ --{name}-min ≤ --{name}-max ms, finite; got {min} and {max}"
                );
            }
        }
        if self.drop_one_message_in < 1 {
            bail!("--drop-one-message-in is at least 1");
        }
        for (name, min) in [
            ("--frame-ready-min", self.frame_ready.min),
            ("--to-uno-min", self.to_uno.min),
            ("--change-motor-min", self.change_motor.min),
        ] {
            if min < timestep_ms {
                bail!(
                    "{name} {min} ms is under the {timestep_ms} ms timestep: a step's own timings could share a timestep"
                );
            }
        }
        let before_send = self.frame_ready.min - self.next_frame_ready.max;
        if before_send < timestep_ms {
            bail!(
                "--frame-ready-min {} − --next-frame-ready-max {} = {before_send} ms is under the {timestep_ms} ms timestep: the next capture could come after this step's send",
                self.frame_ready.min,
                self.next_frame_ready.max
            );
        }
        let spacing = self.frame_ready.min + self.to_uno.min + self.next_frame_ready.min
            - self.frame_ready.max;
        if spacing < timestep_ms {
            bail!(
                "the shortest gap between captures, --frame-ready-min + --to-uno-min + --next-frame-ready-min − --frame-ready-max = {spacing} ms, is under the {timestep_ms} ms timestep: captures could fall out of order"
            );
        }
        Ok(())
    }
}
