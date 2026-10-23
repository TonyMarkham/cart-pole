use std::{f64::consts::TAU, str::FromStr};

/// The scripted cart command for physics mode — a stand-in for the
/// policy (bring-up stages 1–2, `loop-plan.md`). The value is a target
/// cart speed in m/s, sign = direction; the orchestrator sends it to the
/// synthetic Arduino as the Orin's packet would carry it, in pulses/s
/// (speed ÷ meters per pulse, rounded), and the firmware clamps it to the
/// motor's top rate.
#[derive(Debug, Clone, Copy)]
pub enum Control {
    /// `zero`: command no motion (speed 0).
    Zero,
    /// `speed:<m_s>`: one constant speed.
    Speed(f64),
    /// `sine:<amp_m_s>,<period_s>`: speed `amp · cos(2π t / period)`, so
    /// the cart's position follows a sine centered on its start, of
    /// amplitude `amp · period / 2π` meters (the integral of the speed).
    /// A sine speed would integrate to `1 − cos` and drift to one side.
    Sine { amplitude: f64, period: f64 },
}

impl Control {
    /// The command at simulated time `t` seconds.
    pub fn at(self, t: f64) -> f64 {
        match self {
            Control::Zero => 0.0,
            Control::Speed(speed) => speed,
            Control::Sine { amplitude, period } => amplitude * (TAU * t / period).cos(),
        }
    }
}

impl FromStr for Control {
    type Err = String;

    fn from_str(text: &str) -> Result<Control, String> {
        let number = |part: &str| {
            part.trim()
                .parse::<f64>()
                .ok()
                .filter(|value| value.is_finite())
                .ok_or_else(|| format!("{part:?} is not a finite number"))
        };
        match text.split_once(':') {
            None if text == "zero" => Ok(Control::Zero),
            Some(("speed", speed)) => Ok(Control::Speed(number(speed)?)),
            Some(("sine", rest)) => {
                let (amplitude, period) = rest
                    .split_once(',')
                    .ok_or_else(|| "sine takes <amp_m_s>,<period_s>".to_string())?;
                let period = number(period)?;
                if period <= 0.0 {
                    return Err(format!("sine period must be positive, got {period}"));
                }
                Ok(Control::Sine { amplitude: number(amplitude)?, period })
            }
            _ => Err(format!("{text:?}: expected zero, speed:<m_s>, or sine:<amp_m_s>,<period_s>")),
        }
    }
}
