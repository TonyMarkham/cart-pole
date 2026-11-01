mod args;
mod control;
mod joint_qpos;
mod orin_reply;
mod pole_message;
mod pose_message;
mod registered;
mod sites_message;
#[cfg(test)]
mod tests;
mod timing;
mod trainer_link;
mod window;

use crate::{
    args::Args,
    control::Control,
    joint_qpos::JointQpos,
    orin_reply::{OrinReply, POOL_ENTRIES, PoolRow},
    pole_message::PoleMessage,
    pose_message::PoseMessage,
    registered::{Registered, ResetMessage},
    sites_message::SitesMessage,
    timing::Timing,
    trainer_link::{Next, PoolEntry, RunValues, TrainerLink, run_line},
    window::{Pixels, grid_position, window_size},
};

use anyhow::{Context, Result, anyhow, bail};
use arduino::{Config, Direction, Dm542Signals, SyntheticArduino, encode_command};
use clap::Parser;
use futures_util::{SinkExt, StreamExt};
use physics::{Names, Physics, PhysicsConfig, Pose};
use std::{
    collections::HashSet,
    fs::File,
    io::{BufWriter, Write},
    net::{Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
    process::{ExitStatus, Stdio},
    time::{Duration, Instant},
};
use tokio::{
    net::{TcpListener, TcpStream},
    process::{Child, Command},
};
use tokio_tungstenite::{WebSocketStream, accept_async, tungstenite::Message};

// The most training instances in one run (synthetic-loop-dia.md, Training
// instances), --instance-rows × --instance-columns.
const INSTANCES_MAX: usize = 12;

// How long a launched process (player, synthetic Orin) gets to quit on its
// own after its socket closes before it is killed.
const CLIENT_EXIT_GRACE: Duration = Duration::from_secs(5);

// Everything the orchestrator prints goes to stderr (eprintln!): stdout is
// the trainer link alone (synthetic-loop-dia.md, Trainer link).
#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    if args.run_values {
        let physics = Physics::load(&physics_config(&args))?;
        let config = drive_config(
            args.full_steps_per_rev,
            args.microsteps,
            args.belt_um_per_rev,
            args.jump_limit,
            physics.max_speed(),
        )?;
        println!("{}", run_line(&run_values(&physics, &config))?);
        return Ok(());
    }
    let count = usize::from(args.instance_rows) * usize::from(args.instance_columns);
    if count > INSTANCES_MAX {
        bail!(
            "--instance-rows {} × --instance-columns {} is {count} instances; at most {INSTANCES_MAX}",
            args.instance_rows,
            args.instance_columns
        );
    }
    // Worked out here so that a screen too small refuses the run before
    // anything launches.
    let size = screen_window_size(&args)?;
    tokio::fs::create_dir_all(&args.out)
        .await
        .with_context(|| format!("creating {}", args.out.display()))?;

    if args.no_render {
        if count != 1 {
            bail!("--no-render runs one instance, got {count}");
        }
        // 2–3: MuJoCo loaded and reset, the synthetic Arduino created.
        let mut sim = Simulation::new(&args, &instance_folder(&args.out, 0)?, args.seed, 0)?;
        return run_physics(&args, &mut sim);
    }

    let ports = instance_ports(&args, count)?;
    let command = args
        .orin
        .as_deref()
        .ok_or_else(|| anyhow!("--orin is required unless --no-render"))?;
    let connect_timeout = Duration::from_secs(args.connect_timeout);

    // 2–5 for every instance (synthetic-loop-dia.md, Training instances):
    // MuJoCo and the synthetic Arduino, then the synthetic Orin, which
    // pre-warms its pool at the gaps it is given, opens the frame socket,
    // then connects to the instance's control socket. Every Orin is
    // launched before any connection is awaited, so they start together.
    let mut launched = Vec::with_capacity(count);
    for (k, &(orin_port, unity_port)) in ports.iter().enumerate() {
        let out = instance_folder(&args.out, k)?;
        let seed = args.seed.wrapping_add(k as u64);
        let sim = Simulation::new(&args, &out, seed, k)?;
        let (listener, url) =
            bind(orin_port, &format!("[{k}] control socket (synthetic Orin)")).await?;
        let frames_listen = SocketAddr::from((Ipv4Addr::LOCALHOST, unity_port));
        let orin_child = launch_orin(k, command, &url, frames_listen, &out, &sim.pool_gaps)?;
        launched.push((sim, listener, orin_child, frames_listen));
    }
    let mut instances = Vec::with_capacity(count);
    for (k, (mut sim, listener, mut orin_child, frames_listen)) in launched.into_iter().enumerate()
    {
        let who = format!("synthetic Orin {k}");
        let stream = accept(&listener, &mut orin_child, &who, connect_timeout).await?;
        let orin = accept_async(stream)
            .await
            .with_context(|| format!("{who} WebSocket handshake"))?;
        eprintln!("[{k}] synthetic Orin connected");
        let capture = sim.capture();
        instances.push(Instance {
            k,
            sim,
            orin,
            orin_child,
            player: None,
            frames_url: format!("ws://{frames_listen}"),
            capture,
            id: 0,
            latency_ms: 0.0,
            arrival: 0.0,
            next_capture: 0.0,
            sent: Instant::now(),
            pool: Vec::new(),
            line: String::new(),
        });
    }

    // 6: each player, pointed at its Orin's frame socket — or, with
    // --no-launch, editor Play mode connects there itself.
    for instance in &mut instances {
        instance.player = match (&args.player, args.no_launch) {
            (Some(player), false) => {
                let window = size.map(|size| {
                    (
                        grid_position(instance.k, args.instance_columns, size, args.window_origin),
                        size,
                    )
                });
                Some(launch(instance.k, player, &instance.frames_url, window)?)
            }
            _ => {
                eprintln!(
                    "[{}] --no-launch: enter Play mode in the editor; it connects to {}",
                    instance.k, instance.frames_url
                );
                None
            }
        };
    }

    // 7–8: each Unity connects to its Orin, and the Orin says so.
    for instance in &mut instances {
        wait_registered(
            &mut instance.orin,
            &mut instance.orin_child,
            instance.player.as_mut(),
            connect_timeout,
        )
        .await
        .with_context(|| format!("instance {}", instance.k))?;
        eprintln!(
            "[{}] Unity has registered with the synthetic Orin",
            instance.k
        );
    }

    let result = run(&mut instances, &args).await;

    // 22–25 for every instance: closing the control socket ends the run for
    // the Orin, which closes the frame socket, so the player quits, then the
    // Orin. Every close frame is sent before any answer is awaited, and
    // every socket is closed before any exit is awaited, so the instances
    // shut down together. A failed close only means the Orin is already
    // gone.
    for instance in &mut instances {
        let _ = instance.orin.close(None).await;
    }
    let mut children = Vec::with_capacity(instances.len());
    for mut instance in instances {
        await_close(&mut instance.orin, CLIENT_EXIT_GRACE).await;
        // The server ends the TCP connection (RFC 6455 7.1.1); the Orin's
        // client waits for that, up to its 10 s close timeout, before it
        // exits, so the socket is dropped before the exit is awaited.
        drop(instance.orin);
        children.push((instance.k, instance.orin_child, instance.player));
    }
    for (k, orin_child, player) in children {
        stop(orin_child, &format!("[{k}] synthetic Orin")).await?;
        if let Some(player) = player {
            stop(player, &format!("[{k}] player")).await?;
        }
    }
    result
}

// Instance k's ports: --orin-port + 2k and --unity-port + 2k
// (synthetic-loop-dia.md, Training instances). Every port of the group must
// be its own, so neither the connection order nor the messages have to tell
// the participants apart.
fn instance_ports(args: &Args, count: usize) -> Result<Vec<(u16, u16)>> {
    let mut ports = Vec::with_capacity(count);
    let mut seen = HashSet::new();
    for k in 0..count {
        let offset = u16::try_from(2 * k)?;
        let pair = (
            args.orin_port
                .checked_add(offset)
                .ok_or_else(|| anyhow!("instance {k}'s control port passes 65535"))?,
            args.unity_port
                .checked_add(offset)
                .ok_or_else(|| anyhow!("instance {k}'s frame port passes 65535"))?,
        );
        if !seen.insert(pair.0) || !seen.insert(pair.1) {
            bail!(
                "instance {k}'s ports {} and {} clash with another instance's: with --orin-port {} and --unity-port {}, every port must be its own",
                pair.0,
                pair.1,
                args.orin_port,
                args.unity_port
            );
        }
        ports.push(pair);
    }
    Ok(ports)
}

// Instance k's folder, the run's --out / instance_k/.
fn instance_folder(out: &Path, k: usize) -> Result<PathBuf> {
    let folder = out.join(format!("instance_{k}"));
    std::fs::create_dir_all(&folder).with_context(|| format!("creating {}", folder.display()))?;
    Ok(folder)
}

// MuJoCo, the synthetic Arduino between the command and MuJoCo
// (synthetic-arduino-plan.md), and the truth log: one instance's world.
struct Simulation {
    physics: Physics,
    control: Control,
    csv: BufWriter<File>,
    // Sent each new command at the arrival step (19), and ticked after
    // every other step: the ramp (the acceleration ceiling).
    arduino: SyntheticArduino,
    meters_per_pulse: f64,
    names: Names,
    timing: Timing,
    // The current episode's pool's six pre-warm gaps, ms: its first draws,
    // so a reset repeats a fresh launch with its seed.
    pool_gaps: [f64; POOL_ENTRIES],
    // The start pose every episode begins at (2, and R2 at each reset).
    start: Pose,
    // 0 for the instance's first episode, one more at each reset (R1).
    episode: u32,
}

// One observation's capture state: simulated time, the pose, the twelve
// sites' true positions and the pole's true state, all from one MuJoCo
// step (9–10, then each capture step's). Held while MuJoCo steps on, so the
// sites and pole state that go to the trainer (17) show the instant the
// frame shows.
struct Capture {
    time: f64,
    pose: Pose,
    sites: SitesMessage,
    pole: PoleMessage,
}

impl Simulation {
    fn new(args: &Args, out: &Path, seed: u64, k: usize) -> Result<Simulation> {
        // The cart always starts at the rail's center, 50% (2): only the
        // pole's angle is the run's to choose.
        let start = Pose {
            cart: 0.0,
            pole: args.start,
        };
        let physics_config = physics_config(args);
        let mut physics = Physics::load(&physics_config)?;
        // The reset zeroes ctrl: the cart is at rest until the first
        // command acts, at the first arrival step.
        physics.reset(start)?;
        let config = drive_config(
            args.full_steps_per_rev,
            args.microsteps,
            args.belt_um_per_rev,
            args.jump_limit,
            physics.max_speed(),
        )?;
        let meters_per_pulse = meters_per_pulse(&config);
        let tick = ramp_tick(&config, physics.timestep())?;
        // Created with its Config and the tick (3): the synthetic Arduino
        // starts at the rail's center too, where MuJoCo's cart was just
        // reset, and at rest, as MuJoCo's ctrl 0.
        let arduino = SyntheticArduino::new(config, tick);
        let mut timing = Timing::new(
            seed,
            (args.rnd_delay_min, args.rnd_delay_max),
            (args.command_rate_min, args.command_rate_max),
        )?;
        timing.check_commands_act(physics.timestep())?;
        let pool_gaps = timing.next_pool_gaps();

        let path = out.join("states.csv");
        let file = File::create(&path).with_context(|| format!("creating {}", path.display()))?;
        let mut csv = BufWriter::new(file);
        writeln!(
            csv,
            "episode,id,time,cart,pole,left_x,left_y,right_x,right_y,pivot_x,pivot_y,end_x,end_y"
        )?;
        let driver = match args.no_render {
            true => format!("--control {:?}", args.control),
            false => "the trainer's commands".to_string(),
        };
        eprintln!(
            "[{k}] physics: model {}\n         start cart {:+.3} m pole {:.3} rad, driven by {driver}, seed {seed}, latency RND({}, {}) ms, command rate RND({}, {}) Hz, {} m a pulse, top pulse rate {} pulses/s, jump limit {} pulses/s² ({} a {} µs tick) -> {}",
            physics_config.model.display(),
            start.cart,
            start.pole,
            args.rnd_delay_min,
            args.rnd_delay_max,
            args.command_rate_min,
            args.command_rate_max,
            meters_per_pulse,
            arduino.config().max_rate(),
            config.jump_limit,
            config.ramp_limit(tick),
            tick,
            path.display()
        );
        Ok(Simulation {
            physics,
            control: args.control,
            csv,
            arduino,
            meters_per_pulse,
            names: physics_config.names,
            timing,
            pool_gaps,
            start,
            episode: 0,
        })
    }

    // R1–R4 (synthetic-loop-dia.md, Reset in place): the next episode, back
    // at the start pose, t = 0, ctrl 0, with the timing draws reseeded from
    // its seed and the new pool's gaps drawn, so it repeats a fresh run with
    // that seed. The synthetic Arduino goes back to rest, as MuJoCo's ctrl 0
    // (R3).
    fn reset(&mut self, seed: u64) -> Result<()> {
        self.physics.reset(self.start)?;
        self.arduino.reset();
        self.timing = self.timing.reseeded(seed);
        self.pool_gaps = self.timing.next_pool_gaps();
        self.episode += 1;
        Ok(())
    }

    fn capture(&mut self) -> Capture {
        Capture {
            time: self.physics.time(),
            pose: self.physics.pose(),
            sites: self.physics.sites().into(),
            pole: self.physics.pole().into(),
        }
    }

    // 15–16: MuJoCo steps on the old command, one fixed deltatime at a
    // time, until a step's time is later than the arrival time: that step
    // is the arrival step, where it then waits (the freeze) for the
    // Orin's reply. After every step but the arrival step the synthetic
    // Arduino ticks the ramp and ctrl follows it; the arrival step's tick
    // comes with the new command (19–21), so every step runs on one tick.
    fn step_to(&mut self, arrival: f64) -> Result<()> {
        while self.physics.time() <= arrival {
            self.physics.step(1);
            if self.physics.time() <= arrival {
                let signals = self.arduino.tick();
                self.drive(signals)?;
            }
        }
        Ok(())
    }

    // The note after 21: nothing happens between commands. MuJoCo steps on
    // the new command, one fixed deltatime at a time, to the first step
    // later than the next capture's time: that step is the next capture.
    // The ramp ticks after every step, this last one included, so every
    // step runs on one tick. A tick changes ctrl alone, so the capture
    // reads the state that step left.
    fn step_to_capture(&mut self, next_capture: f64) -> Result<()> {
        while self.physics.time() <= next_capture {
            self.physics.step(1);
            let signals = self.arduino.tick();
            self.drive(signals)?;
        }
        Ok(())
    }

    // --no-render's stand-in for the trainer's command: the scripted speed
    // at this step, as the packet the trainer would send (pulses/s,
    // rounded). `as` saturates, so an absurd speed becomes the i32 limit,
    // which the firmware clamps to the motor's top rate.
    fn scripted_command(&self) -> i32 {
        let speed = self.control.at(self.physics.time());
        (speed / self.meters_per_pulse).round() as i32
    }

    // 19–21, at the arrival step: the new command goes to the firmware,
    // which takes the arrival step's tick; its DM542 signals become the
    // cart speed. Returns that speed.
    fn command(&mut self, command: i32) -> Result<f64> {
        let signals = self.arduino.receive(encode_command(command));
        self.drive(signals)
    }

    // 21: ctrl = STEP rate × metres per pulse, signed by DIR.
    fn drive(&mut self, signals: Dm542Signals) -> Result<f64> {
        let velocity = cart_velocity(signals, self.meters_per_pulse);
        self.physics.set_ctrl(velocity)?;
        Ok(velocity)
    }

    // The truth log's row for one observation (synthetic-loop-dia.md,
    // Truth log): its episode, capture time and true pose, beside what the
    // Orin observed in that frame, the pool's newest entry — empty without
    // the Orin (--no-render). `{}` writes a float as the shortest text that
    // reads back to the same number, so identical runs give identical
    // files.
    fn log(&mut self, id: u32, capture: &Capture, newest: Option<&PoolRow>) -> Result<()> {
        let observed = match newest {
            Some(row) => row[1..]
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(","),
            None => ",".repeat(7),
        };
        writeln!(
            self.csv,
            "{},{id},{},{},{},{observed}",
            self.episode, capture.time, capture.pose.cart, capture.pose.pole
        )?;
        Ok(())
    }

    // MuJoCo warnings seen during the run.
    fn finish(&mut self) -> Result<usize> {
        self.csv.flush().context("writing states.csv")?;
        Ok(self.physics.warnings())
    }
}

// One training instance (synthetic-loop-dia.md, Training instances): its
// world, its synthetic Orin and its player, always together, and where its
// current observation stands.
struct Instance {
    k: usize,
    sim: Simulation,
    orin: WebSocketStream<TcpStream>,
    orin_child: Child,
    player: Option<Child>,
    frames_url: String,
    // The current observation: its capture state and id (ids count on
    // across the instance's episodes), its latency, arrival time and the
    // next capture's time, when its pose went out, and the pool its Orin
    // replied with.
    capture: Capture,
    id: u32,
    latency_ms: f64,
    arrival: f64,
    next_capture: f64,
    sent: Instant,
    pool: Vec<PoolRow>,
    line: String,
}

impl Instance {
    // 9–11: at the capture, draw the Orin's latency and the command rate,
    // cache the arrival time and the next capture's time, and send the pose.
    async fn send(&mut self) -> Result<()> {
        self.sent = Instant::now();
        self.latency_ms = self.sim.timing.next_latency_ms();
        self.arrival = self.capture.time + self.latency_ms / 1000.0;
        self.next_capture = self.capture.time + self.sim.timing.next_gap_ms() / 1000.0;
        send_pose(&mut self.orin, self.id, &self.capture, &self.sim.names).await
    }

    // 14–16: step MuJoCo on the old command up to the arrival step while the
    // Orin works, then wait there for the reply (the freeze), and log the
    // truth row (the note before 17).
    async fn collect(&mut self, timeout: Duration) -> Result<()> {
        self.sim.step_to(self.arrival)?;
        self.pool = receive_pool(&mut self.orin, self.id, timeout).await?;
        let orin_ms = self.sent.elapsed().as_secs_f64() * 1000.0;
        self.sim.log(self.id, &self.capture, Some(&self.pool[0]))?;
        let n = &self.pool[0];
        self.line = format!(
            "[{}] observation {:06}  t {:.3} s  latency {:.3} ms, arrival step t {:.3} s, next capture after t {:.3} s  cart {:+.3} m  pole {:.3} rad ({:.1}°)  orin {orin_ms:.1} ms  newest L({:.3}, {:.3}) R({:.3}, {:.3}) P({:.3}, {:.3}) E({:.3}, {:.3})",
            self.k,
            self.id,
            self.capture.time,
            self.latency_ms,
            self.sim.physics.time(),
            self.next_capture,
            self.capture.pose.cart,
            self.capture.pose.pole,
            self.capture.pose.pole.to_degrees(),
            n[1],
            n[2],
            n[3],
            n[4],
            n[5],
            n[6],
            n[7],
            n[8],
        );
        Ok(())
    }
}

// First the run's values (synthetic-loop-dia.md, Contracts), then one
// round per pass (synthetic-loop-dia.md, Training instances): every
// advanced instance sends its pose first, so their Orins detect at the
// same time, then each steps to its arrival step and collects its reply;
// their pools go to the trainer in one line, and its answers, for any
// pending instances, say which advance next: on continue, the command is
// set at the arrival step (19–21) and MuJoCo steps on it to the next
// capture, nothing else happening between; on reset,
// the instance goes back to the start in place (R1–R7) and its next
// capture is the new episode's first; on stop, the loop ends for the
// shutdown (22). Instances not answered stay frozen at their arrival step.
// The run fails when an Orin reports failed or if a MuJoCo warned (P3).
async fn run(instances: &mut [Instance], args: &Args) -> Result<()> {
    let reply_timeout = Duration::from_secs(args.reply_timeout);
    let mut link = TrainerLink::new();
    // The run's values, once, before the first pools line (the note before
    // the loop). Every instance loads the same --model and drive figures, so
    // instance 0's are the run's.
    let first = &instances[0].sim;
    link.answer_request(&run_values(&first.physics, first.arduino.config()))?;
    let mut pending = vec![false; instances.len()];
    let mut advance: Vec<usize> = (0..instances.len()).collect();
    loop {
        for &k in &advance {
            instances[k]
                .send()
                .await
                .with_context(|| format!("instance {k}"))?;
        }
        for &k in &advance {
            instances[k]
                .collect(reply_timeout)
                .await
                .with_context(|| format!("instance {k}"))?;
            pending[k] = true;
        }
        let asked = Instant::now();
        let answers = {
            let pools: Vec<PoolEntry> = advance
                .iter()
                .map(|&k| PoolEntry {
                    instance: k,
                    id: instances[k].id,
                    time: instances[k].capture.time,
                    pool: &instances[k].pool,
                    sites: &instances[k].capture.sites,
                    pole: &instances[k].capture.pole,
                })
                .collect();
            link.exchange(&pools, &pending)?
        };
        let trainer_ms = asked.elapsed().as_secs_f64() * 1000.0;
        for &k in &advance {
            eprintln!("{}  trainer {trainer_ms:.1} ms", instances[k].line);
        }
        if answers.iter().any(|answer| answer.next == Next::Stop) {
            let observations: u32 = instances.iter().map(|instance| instance.id + 1).sum();
            eprintln!(
                "the trainer stopped the run after {observations} observations -> {}",
                args.out.display()
            );
            break;
        }
        advance.clear();
        for answer in &answers {
            let instance = &mut instances[answer.instance];
            match answer.next {
                // 19–21: the new command acts from the arrival step, on to
                // the next capture.
                Next::Continue => {
                    let speed = instance.sim.command(answer.command)?;
                    instance.sim.step_to_capture(instance.next_capture)?;
                    eprintln!(
                        "[{}] observation {:06}  command {} pulses/s, ctrl {speed:+.3} m/s",
                        instance.k, instance.id, answer.command
                    );
                }
                // R1–R7: the instance goes back to the start in place; its
                // next capture is the new episode's first, with ids
                // counting on.
                Next::Reset => {
                    let seed = answer
                        .seed
                        .ok_or_else(|| anyhow!("a reset without a seed reached the run"))?;
                    instance.sim.reset(seed)?;
                    reset_orin(&mut instance.orin, instance.sim.pool_gaps, reply_timeout)
                        .await
                        .with_context(|| format!("instance {}", instance.k))?;
                    eprintln!(
                        "[{}] observation {:06}  reset: episode {}, seed {seed}",
                        instance.k, instance.id, instance.sim.episode
                    );
                }
                Next::Stop => unreachable!("a line with a stop ends the loop above"),
            }
            instance.capture = instance.sim.capture();
            instance.id += 1;
            pending[answer.instance] = false;
            advance.push(answer.instance);
        }
        advance.sort_unstable();
    }

    let mut warned = Vec::new();
    for instance in instances.iter_mut() {
        let warnings = instance.sim.finish()?;
        if warnings > 0 {
            warned.push(format!("instance {}: {warnings}", instance.k));
        }
    }
    if !warned.is_empty() {
        bail!(
            "MuJoCo warned ({}); the states cannot be trusted",
            warned.join(", ")
        );
    }
    Ok(())
}

// --no-render: no Orin, no pool and no trainer; the same clock runs
// --frames states of one instance on the scripted --control.
fn run_physics(args: &Args, sim: &mut Simulation) -> Result<()> {
    let mut capture = sim.capture();
    for id in 0..args.frames {
        let latency_ms = sim.timing.next_latency_ms();
        let next_capture = capture.time + sim.timing.next_gap_ms() / 1000.0;
        sim.step_to(capture.time + latency_ms / 1000.0)?;
        sim.log(id, &capture, None)?;
        let command = sim.scripted_command();
        let speed = sim.command(command)?;
        let arrived = sim.physics.time();
        sim.step_to_capture(next_capture)?;
        eprintln!(
            "observation {id:06}  t {:.3} s  latency {latency_ms:.3} ms, arrival step t {arrived:.3} s, next capture t {:.3} s (gap {:.0} ms)  cart {:+.3} m  pole {:.3} rad ({:.1}°)  command {command} pulses/s, ctrl {speed:+.3} m/s",
            capture.time,
            sim.physics.time(),
            (sim.physics.time() - capture.time) * 1000.0,
            capture.pose.cart,
            capture.pose.pole,
            capture.pose.pole.to_degrees()
        );
        capture = sim.capture();
    }
    eprintln!("{} states -> {}", args.frames, args.out.display());
    let warnings = sim.finish()?;
    if warnings > 0 {
        bail!("MuJoCo warned {warnings} time(s); the states cannot be trusted");
    }
    Ok(())
}

// The MJCF and its names come only from the command line (the trainer's
// settings.toml [mjcf]).
fn physics_config(args: &Args) -> PhysicsConfig {
    PhysicsConfig {
        model: args.model.clone(),
        names: Names {
            cart_joint: args.cart_joint.clone(),
            pole_joint: args.pole_joint.clone(),
            actuator: args.actuator.clone(),
            left_site: args.left_site.clone(),
            right_site: args.right_site.clone(),
            pivot_site: args.pivot_site.clone(),
            end_site: args.end_site.clone(),
            target_site: args.target_site.clone(),
            dynamic_target_site: args.dynamic_target_site.clone(),
            cart_site: args.cart_site.clone(),
            cart_target_site: args.cart_target_site.clone(),
            left_rail_site: args.left_rail_site.clone(),
            left_cart_site: args.left_cart_site.clone(),
            right_rail_site: args.right_rail_site.clone(),
            right_cart_site: args.right_cart_site.clone(),
        },
    }
}

// The run's values (synthetic-loop-dia.md, Contracts): MuJoCo's, from the
// loaded model, and the top pulse rate, the synthetic Arduino's clamp.
fn run_values(physics: &Physics, config: &Config) -> RunValues {
    let pole = physics.pole_body();
    RunValues {
        cart_top_speed: physics.max_speed(),
        top_pulse_rate: config.max_rate(),
        timestep: physics.timestep(),
        pole_mass: pole.mass,
        pole_com_distance: pole.com_distance,
        pole_inertia: pole.inertia,
        gravity: pole.gravity,
    }
}

// The synthetic Arduino's Config (3): the drive figures and the jump limit
// from the command line (the trainer's [drive]) and the cart top speed,
// MuJoCo's ctrlrange, in whole µm/s rounded to the nearest: a plain floor
// of the float turns 2.01 m/s into 2,009,999 µm/s. The top pulse rate it
// gives, rounded down, must be at least 1 and fit an i32, the command's
// type.
fn drive_config(
    full_steps_per_rev: u32,
    microsteps: u32,
    belt_um_per_rev: u32,
    jump_limit: u32,
    max_speed: f64,
) -> Result<Config> {
    if full_steps_per_rev.checked_mul(microsteps).is_none() {
        bail!(
            "--full-steps-per-rev {full_steps_per_rev} × --microsteps {microsteps} does not fit a u32"
        );
    }
    let um_per_s = (max_speed * 1e6).round();
    if !(1.0..=f64::from(u32::MAX)).contains(&um_per_s) {
        bail!(
            "the cart top speed {max_speed} m/s is not a whole number of µm/s from 1 to {}",
            u32::MAX
        );
    }
    let config = Config {
        full_steps_per_rev,
        microsteps,
        belt_um_per_rev,
        max_speed_um_per_s: um_per_s as u32,
        jump_limit,
    };
    let rate = u64::from(config.max_speed_um_per_s) * u64::from(config.pulses_per_rev())
        / u64::from(belt_um_per_rev);
    if rate < 1 || rate > i32::MAX as u64 {
        bail!(
            "the top pulse rate {rate} pulses/s (cart top speed {max_speed} m/s ÷ metres per pulse) is not from 1 to {}",
            i32::MAX
        );
    }
    Ok(config)
}

// The ramp's tick (A7): MuJoCo's timestep in whole µs, rounded to the
// nearest (0.002 s is 2,000). m × the tick, rounded down, must be at least
// 1 pulse/s, or the sent rate could never move and the cart never start.
fn ramp_tick(config: &Config, timestep: f64) -> Result<u32> {
    let micros = (timestep * 1e6).round();
    if !(1.0..=f64::from(u32::MAX)).contains(&micros) {
        bail!(
            "the timestep {timestep} s is not a whole number of µs from 1 to {}",
            u32::MAX
        );
    }
    let tick = micros as u32;
    if config.ramp_limit(tick) < 1 {
        bail!(
            "--jump-limit {} pulses/s² × the {tick} µs timestep rounds to 0 pulses/s a tick: the cart could never move",
            config.jump_limit
        );
    }
    Ok(tick)
}

// One step pulse's cart travel: belt travel per revolution ÷ pulses per
// revolution, the trainer's [drive].
fn meters_per_pulse(config: &Config) -> f64 {
    f64::from(config.belt_um_per_rev) / 1e6 / f64::from(config.pulses_per_rev())
}

// 21: cart velocity = STEP rate × meters per pulse, m/s, signed by DIR. The
// STEP rate is at most the top pulse rate, rounded down from the cart top
// speed, so the velocity stays inside ctrlrange.
fn cart_velocity(signals: Dm542Signals, meters_per_pulse: f64) -> f64 {
    let speed = f64::from(signals.step_rate) * meters_per_pulse;
    match signals.dir {
        Direction::Positive => speed,
        Direction::Negative => -speed,
    }
}

// Each window's size in the grid that fills the screen, when the arguments
// give a screen (synthetic-loop-dia.md, Training instances, The windows).
fn screen_window_size(args: &Args) -> Result<Option<Pixels>> {
    let (Some(x), Some(y)) = (args.screen_width, args.screen_height) else {
        return Ok(None);
    };
    let size = window_size(Pixels { x, y }, args.instance_rows, args.instance_columns)
        .map_err(|error| anyhow!(error))?;
    Ok(Some(size))
}

// 6: the player, told its Orin's frame socket and, with a grid, its
// borderless window's place and size (synthetic-loop-dia.md, Training
// instances, The windows). kill_on_drop: an early error return drops the
// handle, and the player must not outlive the orchestrator that launched
// it.
fn launch(
    k: usize,
    player: &Path,
    frames_url: &str,
    window: Option<(Pixels, Pixels)>,
) -> Result<Child> {
    let mut command = Command::new(player);
    command.arg("-orin").arg(frames_url);
    let mut shown = format!("{} -orin {frames_url}", player.display());
    if let Some((
        Pixels { x, y },
        Pixels {
            x: width,
            y: height,
        },
    )) = window
    {
        command
            .arg("-popupwindow")
            .arg("-window-position")
            .arg(format!("{x},{y}"))
            .arg("-screen-width")
            .arg(width.to_string())
            .arg("-screen-height")
            .arg(height.to_string());
        shown.push_str(&format!(
            " -popupwindow -window-position {x},{y} -screen-width {width} -screen-height {height}"
        ));
    }
    eprintln!("[{k}] launching {shown}");
    command
        .stdin(Stdio::null())
        .stdout(child_stdout())
        .kill_on_drop(true)
        .spawn()
        .with_context(|| format!("launching {}", player.display()))
}

// The synthetic Orin (synthetic-loop-dia.md, 4): the given program and
// arguments, plus the control socket's address, the frame socket's
// address, the instance's folder and its pool's six pre-warm gaps (each as
// the shortest text that reads back to the same f64); kill_on_drop as for
// the player.
fn launch_orin(
    k: usize,
    command: &[String],
    url: &str,
    frames_listen: SocketAddr,
    out: &Path,
    pool_gaps: &[f64; POOL_ENTRIES],
) -> Result<Child> {
    let (program, rest) = command
        .split_first()
        .ok_or_else(|| anyhow!("--orin needs a command"))?;
    let gaps = pool_gaps
        .iter()
        .map(|gap| gap.to_string())
        .collect::<Vec<_>>()
        .join(",");
    eprintln!(
        "[{k}] launching synthetic Orin: {} --orchestrator {url} --frames-listen {frames_listen} --out {} --pool-gaps {gaps}",
        command.join(" "),
        out.display()
    );
    Command::new(program)
        .args(rest)
        .arg("--orchestrator")
        .arg(url)
        .arg("--frames-listen")
        .arg(frames_listen.to_string())
        .arg("--out")
        .arg(out)
        .arg("--pool-gaps")
        .arg(gaps)
        .stdin(Stdio::null())
        .stdout(child_stdout())
        .kill_on_drop(true)
        .spawn()
        .with_context(|| format!("launching {program}"))
}

// A child's stdout is the orchestrator's stderr, and its stdin is closed:
// both of the orchestrator's own pipes are the trainer link, and a child
// must neither write into it nor read from it.
fn child_stdout() -> Stdio {
    Stdio::from(std::io::stderr())
}

// The loop runs on this workstation, so every socket is on 127.0.0.1.
async fn bind(port: u16, what: &str) -> Result<(TcpListener, String)> {
    let address = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    let listener = TcpListener::bind(address)
        .await
        .with_context(|| format!("binding the {what} on {address}"))?;
    let url = format!("ws://{address}");
    eprintln!("{what}: listening on {url}");
    Ok((listener, url))
}

// Waits for the next connection; a launched process that exits before
// connecting fails the wait at once instead of after the timeout.
async fn accept(
    listener: &TcpListener,
    child: &mut Child,
    who: &str,
    timeout: Duration,
) -> Result<TcpStream> {
    let wait = async {
        tokio::select! {
            accepted = listener.accept() => Ok(accepted?.0),
            status = child.wait() => Err(anyhow!("{who} exited before connecting: {status:?}")),
        }
    };
    tokio::time::timeout(timeout, wait).await.map_err(|_| {
        anyhow!(
            "no connection from the {who} within {} s",
            timeout.as_secs()
        )
    })?
}

// 8: the Orin's {"type": "registered"}, its first message. The Orin or the
// player exiting first fails the wait at once instead of after the
// timeout.
async fn wait_registered(
    orin: &mut WebSocketStream<TcpStream>,
    orin_child: &mut Child,
    player: Option<&mut Child>,
    timeout: Duration,
) -> Result<()> {
    let wait = async {
        let player_exit = async {
            match player {
                Some(player) => player.wait().await,
                None => std::future::pending::<std::io::Result<ExitStatus>>().await,
            }
        };
        tokio::pin!(player_exit);
        loop {
            tokio::select! {
                message = orin.next() => {
                    let message = message.ok_or_else(|| anyhow!("synthetic Orin disconnected before Unity registered"))??;
                    match message {
                        Message::Text(text) => {
                            let registered = serde_json::from_str::<Registered>(text.as_str())
                                .with_context(|| format!("synthetic Orin message {:?}", text.as_str()))?;
                            if registered.kind != "registered" {
                                bail!("synthetic Orin sent {:?} before Unity registered", text.as_str());
                            }
                            return Ok(());
                        }
                        Message::Binary(_) => bail!("synthetic Orin sent binary data before Unity registered"),
                        Message::Close(frame) => bail!("synthetic Orin closed the socket before Unity registered: {frame:?}"),
                        _ => {}
                    }
                }
                status = orin_child.wait() => bail!("synthetic Orin exited before Unity registered: {status:?}"),
                status = &mut player_exit => bail!("the player exited before registering with the synthetic Orin: {status:?}"),
            }
        }
    };
    tokio::time::timeout(timeout, wait).await.map_err(|_| {
        anyhow!(
            "Unity did not register with the synthetic Orin within {} s",
            timeout.as_secs()
        )
    })?
}

// 11: one capture state to the synthetic Orin.
async fn send_pose(
    orin: &mut WebSocketStream<TcpStream>,
    id: u32,
    capture: &Capture,
    names: &Names,
) -> Result<()> {
    let message = PoseMessage {
        id,
        time: capture.time,
        joints: vec![
            JointQpos {
                name: names.cart_joint.clone(),
                qpos: capture.pose.cart,
            },
            JointQpos {
                name: names.pole_joint.clone(),
                qpos: capture.pose.pole,
            },
        ],
    };
    orin.send(Message::text(serde_json::to_string(&message)?))
        .await
        .with_context(|| format!("sending pose {id} to the synthetic Orin"))
}

// 14: the Orin's one reply, the pool, newest first. A reply for another
// id, a pool of the wrong shape, or failed ends the run.
async fn receive_pool(
    orin: &mut WebSocketStream<TcpStream>,
    id: u32,
    timeout: Duration,
) -> Result<Vec<PoolRow>> {
    let reply = loop {
        let message = tokio::time::timeout(timeout, orin.next())
            .await
            .map_err(|_| {
                anyhow!(
                    "no reply from the synthetic Orin for pose {id} within {} s",
                    timeout.as_secs()
                )
            })?
            .ok_or_else(|| anyhow!("synthetic Orin disconnected"))??;
        match message {
            Message::Text(text) => {
                break serde_json::from_str::<OrinReply>(text.as_str())
                    .with_context(|| format!("synthetic Orin reply {:?}", text.as_str()))?;
            }
            Message::Binary(_) => {
                bail!("synthetic Orin sent binary data for pose {id}, expected its reply")
            }
            Message::Close(frame) => bail!("synthetic Orin closed the socket: {frame:?}"),
            _ => {}
        }
    };
    if reply.id != id {
        bail!(
            "synthetic Orin replied for pose {}, expected {id}",
            reply.id
        );
    }
    // The pool's shape (Contracts): 6 entries, newest first, the newest at
    // age 0 and every older one strictly older.
    if reply.pool.len() != POOL_ENTRIES {
        bail!(
            "pool {id} has {} entries, expected {POOL_ENTRIES}",
            reply.pool.len()
        );
    }
    let ages: Vec<f64> = reply.pool.iter().map(|row| row[0]).collect();
    if ages[0] != 0.0 || ages.windows(2).any(|pair| pair[1] >= pair[0]) {
        bail!("pool {id} ages {ages:?} are not 0 then strictly older, newest first");
    }
    // The break after 14: 3 failed frames in the pool fail the run, and
    // training ends (26).
    if reply.failed {
        bail!("the run failed: 3 of pool {id}'s 6 entries are failed frames");
    }
    Ok(reply.pool)
}

// R5–R7: tell the Orin to pre-warm its pool afresh at these gaps, and wait
// for its {"type": "reset"}, so the next pose meets the new episode's pool.
async fn reset_orin(
    orin: &mut WebSocketStream<TcpStream>,
    gaps: [f64; POOL_ENTRIES],
    timeout: Duration,
) -> Result<()> {
    let message = ResetMessage {
        kind: "reset",
        gaps,
    };
    orin.send(Message::text(serde_json::to_string(&message)?))
        .await
        .context("sending the reset to the synthetic Orin")?;
    loop {
        let message = tokio::time::timeout(timeout, orin.next())
            .await
            .map_err(|_| {
                anyhow!(
                    "no reset reply from the synthetic Orin within {} s",
                    timeout.as_secs()
                )
            })?
            .ok_or_else(|| anyhow!("synthetic Orin disconnected during the reset"))??;
        match message {
            Message::Text(text) => {
                let reply = serde_json::from_str::<Registered>(text.as_str())
                    .with_context(|| format!("synthetic Orin reset reply {:?}", text.as_str()))?;
                if reply.kind != "reset" {
                    bail!(
                        "synthetic Orin sent {:?}, expected its reset reply",
                        text.as_str()
                    );
                }
                return Ok(());
            }
            Message::Binary(_) => {
                bail!("synthetic Orin sent binary data, expected its reset reply")
            }
            Message::Close(frame) => {
                bail!("synthetic Orin closed the socket during the reset: {frame:?}")
            }
            _ => {}
        }
    }
}

// 22: the close is a handshake (RFC 6455 7.1.1). After sending its close
// frame the server reads until the client's close frame answers it, and
// only then ends the TCP connection. A socket dropped with data unread
// (the Orin's keepalive ping, every 20 s) is reset, and the reset discards
// the close frame on the Orin's side before it reads it: the Orin then
// exits 1 (trainer-observations.md, run 1 at 600,000). Whatever else
// arrives first is passed over; an error or the socket's end also ends
// the wait, and so does the timeout, if the Orin never answers.
async fn await_close(orin: &mut WebSocketStream<TcpStream>, timeout: Duration) {
    let _ = tokio::time::timeout(timeout, async {
        while let Some(Ok(message)) = orin.next().await {
            if message.is_close() {
                break;
            }
        }
    })
    .await;
}

// A launched process quits on its own when its socket closes; it is
// killed only if it is still running after the grace period.
async fn stop(mut child: Child, who: &str) -> Result<()> {
    match tokio::time::timeout(CLIENT_EXIT_GRACE, child.wait()).await {
        Ok(status) => eprintln!("{who} exited: {}", status?),
        Err(_) => {
            child
                .kill()
                .await
                .with_context(|| format!("killing the {who}"))?;
            eprintln!("{who} killed after {} s", CLIENT_EXIT_GRACE.as_secs());
        }
    }
    Ok(())
}
