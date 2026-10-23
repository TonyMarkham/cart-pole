use std::{net::SocketAddr, path::PathBuf};

use clap::Parser;

use crate::control::Control;

/// Sim-loop orchestrator stub: sends the Unity player joint states over a
/// WebSocket and checks the frames it returns. The states come from a
/// scripted sweep, one fixed pose, or (with --physics) MuJoCo stepping
/// the model named in physics.toml. Joint names come from physics.toml.
#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Args {
    /// Address the WebSocket server listens on; the player is told to
    /// connect to ws://<listen>.
    #[arg(long, default_value = "127.0.0.1:9000")]
    pub listen: SocketAddr,

    /// The built Unity player to launch with `-orchestrator ws://<listen>`.
    #[arg(long, required_unless_present_any = ["no_launch", "no_render"])]
    pub player: Option<PathBuf>,

    /// Launch nothing; wait for any client (editor Play mode).
    #[arg(long)]
    pub no_launch: bool,

    /// Seconds to wait for the player to connect, covering its startup.
    #[arg(long, default_value_t = 120)]
    pub connect_timeout: u64,

    /// Seconds to wait for each frame after sending its state.
    #[arg(long, default_value_t = 10)]
    pub reply_timeout: u64,

    /// Number of states: sweep states, or physics frames (ignored with
    /// --pose).
    #[arg(long, default_value_t = 60)]
    pub frames: u32,

    /// Send one state instead of the sweep: cart meters, pole radians
    /// (e.g. --pose 0.2,0.5 or --pose -0.2,0.5).
    #[arg(long, value_delimiter = ',', allow_hyphen_values = true, conflicts_with = "physics")]
    pub pose: Option<Vec<f64>>,

    /// Step MuJoCo (the model in physics.toml) between frames and send
    /// its qpos; writes states.csv beside the frames.
    #[arg(long)]
    pub physics: bool,

    /// Physics mode: the cart command, a target cart speed — zero,
    /// speed:<m_s>, or sine:<amp_m_s>,<period_s> (m/s; seconds of
    /// simulated time). sine moves the cart ±amp·period/2π m about its
    /// start.
    #[arg(long, default_value = "zero", requires = "physics")]
    pub control: Control,

    /// Physics mode: the start pose, cart meters and pole radians, at
    /// rest (0 = hanging).
    #[arg(long, value_delimiter = ',', allow_hyphen_values = true, requires = "physics")]
    pub start: Option<Vec<f64>>,

    /// Physics mode: simulated time between frames, in milliseconds; a
    /// whole number of physics steps. 20 is a placeholder for the Pi's
    /// frame interval T (loop-plan.md).
    #[arg(long, default_value_t = 20, requires = "physics")]
    pub frame_interval_ms: u32,

    /// Physics mode: no player, no socket — physics and states.csv only.
    #[arg(long, requires = "physics")]
    pub no_render: bool,

    /// Physics mode: run each returned frame through the detector
    /// (ObjectDetection.toml, beside physics.toml) and append its four
    /// slots and confidences to the frame's states.csv row.
    #[arg(long, requires = "physics", conflicts_with = "no_render")]
    pub detect: bool,

    /// Directory the returned frames (and states.csv) are written into.
    #[arg(long, default_value = "target/orchestrator-frames")]
    pub out: PathBuf,
}
