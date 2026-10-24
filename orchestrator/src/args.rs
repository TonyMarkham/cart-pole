use std::path::PathBuf;

use clap::Parser;

use crate::control::Control;

/// The synthetic loop's orchestrator (orchestrator-plan.md): steps MuJoCo
/// (the MJCF --model names) and sends each capture state to the synthetic
/// Orin, which triggers the Unity player and detects the markers
/// (synthetic-loop-dia.md). The trainer launches it with all of its
/// arguments, from its settings.toml.
#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Args {
    /// The MJCF to load, relative to the working directory.
    #[arg(long)]
    pub model: PathBuf,

    /// The MJCF's cart slide joint name; also the cart's wire key in the
    /// pose message (synthetic-loop-dia.md, Contracts).
    #[arg(long)]
    pub cart_joint: String,

    /// The MJCF's pole hinge joint name; also the pole's wire key.
    #[arg(long)]
    pub pole_joint: String,

    /// The MJCF's cart actuator name.
    #[arg(long)]
    pub actuator: String,

    /// The control socket's port on 127.0.0.1 (synthetic-loop-dia.md,
    /// 4–5): the orchestrator listens, the synthetic Orin connects.
    #[arg(long, default_value_t = 9000)]
    pub orin_port: u16,

    /// The frame socket's port on 127.0.0.1 (6–7): the synthetic Orin
    /// listens on it (--frames-listen), the Unity player connects.
    #[arg(long, default_value_t = 9001)]
    pub unity_port: u16,

    /// The run's seed (synthetic-loop-dia.md, 4): the source of the
    /// latency draws (SplitMix64), and passed to the synthetic Orin for its
    /// pool's pre-warm gaps. The trainer passes its settings.toml
    /// [orchestrator] seed.
    #[arg(long)]
    pub seed: u64,

    /// The Orin latency's lower bound, ms: each observation's latency is a
    /// seeded RND(--rnd-delay-min, --rnd-delay-max) (synthetic-loop-dia.md),
    /// and the synthetic Orin's pre-warm gaps come from the same band (4).
    /// The trainer passes its settings.toml [orchestrator] rnd_delay_min.
    #[arg(long)]
    pub rnd_delay_min: f64,

    /// The Orin latency's upper bound, ms; the trainer passes its
    /// rnd_delay_max. 0 ≤ --rnd-delay-min ≤ --rnd-delay-max.
    #[arg(long)]
    pub rnd_delay_max: f64,

    /// The built Unity player to launch with
    /// `-orin ws://127.0.0.1:<unity-port>` (6).
    #[arg(long, required_unless_present_any = ["no_launch", "no_render"])]
    pub player: Option<PathBuf>,

    /// Launch no player: editor Play mode is the camera, connecting to
    /// the synthetic Orin's frame socket itself.
    #[arg(long)]
    pub no_launch: bool,

    /// Seconds to wait for the synthetic Orin to connect, and then for
    /// Unity to register with it, covering each one's startup.
    #[arg(long, default_value_t = 120)]
    pub connect_timeout: u64,

    /// Seconds to wait for the synthetic Orin's reply to each pose.
    #[arg(long, default_value_t = 10)]
    pub reply_timeout: u64,

    /// --no-render only: the number of states in a physics-only run. With
    /// the Orin, the trainer's stop ends the run (18).
    #[arg(long, default_value_t = 60, requires = "no_render")]
    pub frames: u32,

    /// --no-render only: the cart command, a target cart speed — zero,
    /// speed:<m_s>, or sine:<amp_m_s>,<period_s> (m/s; seconds of
    /// simulated time). sine moves the cart ±amp·period/2π m about its
    /// start. With the Orin, the trainer's command drives the cart (18).
    #[arg(long, default_value = "zero", requires = "no_render")]
    pub control: Control,

    /// The pole's start angle in radians, at rest (0 = hanging,
    /// π = upright). The cart always starts at the rail's center
    /// (synthetic-loop-dia.md, 2), so it has no start value.
    #[arg(long, default_value_t = 0.0, allow_hyphen_values = true)]
    pub start: f64,

    /// No Orin, no player, no socket — physics and states.csv only.
    #[arg(long)]
    pub no_render: bool,

    /// Launch the synthetic Orin (synthetic-orin-plan.md) with this
    /// command — program and arguments, e.g. `--orin uv run --project
    /// orin python orin/src/synthetic_orin.py --engine <file>` —
    /// appending `--orchestrator
    /// ws://127.0.0.1:<orin-port> --frames-listen 127.0.0.1:<unity-port>
    /// --out <out> --seed <seed> --rnd-delay-min <min> --rnd-delay-max
    /// <max>` (4). Required unless --no-render. Must
    /// come last: every argument after it is the command.
    #[arg(
        long,
        num_args = 1..,
        allow_hyphen_values = true,
        required_unless_present = "no_render",
        conflicts_with = "no_render"
    )]
    pub orin: Option<Vec<String>>,

    /// The run's folder: states.csv here; the synthetic Orin writes the
    /// frames and observations.jsonl beside it.
    #[arg(long, default_value = "target/orchestrator-frames")]
    pub out: PathBuf,
}
