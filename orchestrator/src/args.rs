use std::path::PathBuf;

use clap::Parser;

use crate::{control::Control, window::Pixels};

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

    /// The MJCF's site at the Left marker's centre
    /// (synthetic-loop-dia.md, Contracts, the sites).
    #[arg(long)]
    pub left_site: String,

    /// The MJCF's site at the Right marker's centre.
    #[arg(long)]
    pub right_site: String,

    /// The MJCF's site at the Pivot marker's centre.
    #[arg(long)]
    pub pivot_site: String,

    /// The MJCF's site at the End marker's centre.
    #[arg(long)]
    pub end_site: String,

    /// The MJCF's Target site: fixed where End sits with the cart centred
    /// and the pole upright.
    #[arg(long)]
    pub target_site: String,

    /// The MJCF's Dynamic Target site: on the cart, where End sits with the
    /// pole upright wherever the cart is.
    #[arg(long)]
    pub dynamic_target_site: String,

    /// The MJCF's Cart site: on the cart, at the slider joint.
    #[arg(long)]
    pub cart_site: String,

    /// The MJCF's Cart Target site: fixed where Cart sits with the cart
    /// centred.
    #[arg(long)]
    pub cart_target_site: String,

    /// The MJCF's site where the cart would strike the left base.
    #[arg(long)]
    pub left_rail_site: String,

    /// The MJCF's site on the cart's face that would strike the left base.
    #[arg(long)]
    pub left_cart_site: String,

    /// The MJCF's site where the cart would strike the right base.
    #[arg(long)]
    pub right_rail_site: String,

    /// The MJCF's site on the cart's face that would strike the right base.
    #[arg(long)]
    pub right_cart_site: String,

    /// Motor full steps per pulley revolution: the trainer's settings.toml
    /// [drive] full_steps_per_rev. With --microsteps and --belt-um-per-rev
    /// it gives the metres per step pulse (synthetic-loop-dia.md, Contracts,
    /// the cart speed).
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..))]
    pub full_steps_per_rev: u32,

    /// The driver's microstep setting, as its denominator (1/32 is 32):
    /// [drive] microsteps.
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..))]
    pub microsteps: u32,

    /// Belt travel per pulley revolution, µm: [drive] belt_um_per_rev.
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..))]
    pub belt_um_per_rev: u32,

    /// m, the jump limit, pulses/s²: [drive] jump_limit, the fastest the
    /// synthetic Arduino's ramp changes the sent rate (synthetic-loop-dia.md,
    /// The acceleration ceiling).
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..))]
    pub jump_limit: u32,

    /// The training instances' rows (synthetic-loop-dia.md, Training
    /// instances): --instance-rows × --instance-columns instances, at most
    /// 12, each its own MuJoCo, synthetic Arduino, synthetic Orin and Unity
    /// player. The trainer passes its settings.toml [orchestrator]
    /// instance_rows.
    #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u16).range(1..=12))]
    pub instance_rows: u16,

    /// The training instances' columns: the trainer passes its
    /// instance_columns.
    #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u16).range(1..=12))]
    pub instance_columns: u16,

    /// Instance 0's control socket port on 127.0.0.1 (synthetic-loop-dia.md,
    /// 4–5): the orchestrator listens, the synthetic Orin connects.
    /// Instance k listens on --orin-port + 2k.
    #[arg(long, default_value_t = 9000)]
    pub orin_port: u16,

    /// Instance 0's frame socket port on 127.0.0.1 (6–7): the synthetic Orin
    /// listens on it (--frames-listen), the Unity player connects.
    /// Instance k's is --unity-port + 2k.
    #[arg(long, default_value_t = 9001)]
    pub unity_port: u16,

    /// Instance 0's first episode's seed (synthetic-loop-dia.md, 4): the
    /// source of its timing draws (SplitMix64); instance k's is --seed + k.
    /// Later episodes' seeds come with their resets. The trainer passes its
    /// settings.toml [orchestrator] seed.
    #[arg(long)]
    pub seed: u64,

    /// The Orin latency's lower bound, ms: each observation's latency is a
    /// seeded RND(--rnd-delay-min, --rnd-delay-max) (synthetic-loop-dia.md).
    /// The trainer passes its settings.toml [orchestrator] rnd_delay_min.
    #[arg(long)]
    pub rnd_delay_min: f64,

    /// The Orin latency's upper bound, ms; the trainer passes its
    /// rnd_delay_max. 0 ≤ --rnd-delay-min ≤ --rnd-delay-max.
    #[arg(long)]
    pub rnd_delay_max: f64,

    /// The command rate's lower bound, Hz: each capture's next capture comes
    /// 1000 ÷ a seeded RND(--command-rate-min, --command-rate-max) ms later,
    /// and nothing happens between (synthetic-loop-dia.md); the pool's
    /// pre-warm gaps come from the same band (4). The trainer passes its gym
    /// file's [env] command_rate_min.
    #[arg(long)]
    pub command_rate_min: f64,

    /// The command rate's upper bound, Hz; the trainer passes its
    /// command_rate_max. 0 < --command-rate-min ≤ --command-rate-max, and
    /// 1000 ÷ this at least --rnd-delay-max + one timestep.
    #[arg(long)]
    pub command_rate_max: f64,

    /// The built Unity player to launch with
    /// `-orin ws://127.0.0.1:<unity-port>` (6).
    #[arg(long, required_unless_present_any = ["no_launch", "no_render"])]
    pub player: Option<PathBuf>,

    /// Launch no player: editor Play mode is the camera, connecting to
    /// the synthetic Orin's frame socket itself.
    #[arg(long)]
    pub no_launch: bool,

    /// The width in pixels the players' borderless windows fill, as a grid
    /// of --instance-rows × --instance-columns (synthetic-loop-dia.md,
    /// Training instances, The windows): width = ⌊this ÷ columns⌋, and
    /// instance k's player is launched with `-popupwindow`, `-screen-width`
    /// width and `-window-position x,y`, x = origin x + (k mod columns) ×
    /// width, y = origin y + ⌊k ÷ columns⌋ × height. Without it, the windows
    /// open where Windows puts them. The trainer passes its screen_width.
    #[arg(long, value_parser = clap::value_parser!(i32).range(1..), requires = "screen_height")]
    pub screen_width: Option<i32>,

    /// The height in pixels the windows fill, the part above the taskbar:
    /// `-screen-height` is ⌊this ÷ rows⌋. The trainer passes its
    /// screen_height.
    #[arg(long, value_parser = clap::value_parser!(i32).range(1..), requires = "screen_width")]
    pub screen_height: Option<i32>,

    /// The grid's top-left corner, x,y pixels from the top-left corner of
    /// the display the windows open on.
    #[arg(long, default_value = "0,0", allow_hyphen_values = true)]
    pub window_origin: Pixels,

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

    /// Load the model, print the run's values line the trainer's request
    /// is answered with (synthetic-loop-dia.md, Contracts, the run's
    /// values) on stdout, and exit: no Orin, no player, no socket, no
    /// folder. MuJoCo's values for a test that needs them without the
    /// loop; the rest of the command line is read but not used.
    #[arg(long)]
    pub run_values: bool,

    /// Launch the synthetic Orin (synthetic-orin-plan.md) with this
    /// command — program and arguments, e.g. `--orin uv run --project
    /// orin python orin/src/synthetic_orin.py --engine <file>` —
    /// appending `--orchestrator
    /// ws://127.0.0.1:<orin-port> --frames-listen 127.0.0.1:<unity-port>
    /// --out <out> --pool-gaps <d1,...,d6>` (4). Required unless
    /// --no-render. Must come last: every argument after it is the command.
    #[arg(
        long,
        num_args = 1..,
        allow_hyphen_values = true,
        required_unless_present = "no_render",
        conflicts_with = "no_render"
    )]
    pub orin: Option<Vec<String>>,

    /// The run's folder: instance k's states.csv in instance_k/, where its
    /// synthetic Orin writes the frames and observations.jsonl beside it.
    #[arg(long, default_value = "target/orchestrator-frames")]
    pub out: PathBuf,
}
