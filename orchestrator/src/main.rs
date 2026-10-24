mod args;
mod control;
mod joint_qpos;
mod latency;
mod orin_reply;
mod pose_message;
mod registered;
#[cfg(test)]
mod tests;
mod trainer_link;
mod window;

use crate::{
    args::Args,
    control::Control,
    joint_qpos::JointQpos,
    latency::Latency,
    orin_reply::{OrinReply, POOL_ENTRIES, PoolRow},
    pose_message::PoseMessage,
    registered::{Registered, ResetMessage},
    trainer_link::{Next, PoolEntry, TrainerLink},
    window::{Pixels, grid_position},
};

use anyhow::{Context, Result, anyhow, bail};
use arduino::{Config, Direction, Drv8825Signals, RIG, SyntheticArduino, encode_command};
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

// How long a launched process (player, synthetic Orin) gets to quit on its
// own after its socket closes before it is killed.
const CLIENT_EXIT_GRACE: Duration = Duration::from_secs(5);

// Everything the orchestrator prints goes to stderr (eprintln!): stdout is
// the trainer link alone (synthetic-loop-dia.md, Trainer link).
#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let count = usize::from(args.instances);
    tokio::fs::create_dir_all(&args.out)
        .await
        .with_context(|| format!("creating {}", args.out.display()))?;

    if args.no_render {
        if count != 1 {
            bail!("--no-render runs one instance, got --instances {count}");
        }
        // 2–3: MuJoCo loaded and reset, the synthetic Arduino created.
        let mut sim = Simulation::new(&args, &instance_folder(&args.out, 0)?, args.seed, 0)?;
        return run_physics(&args, &mut sim);
    }

    let ports = instance_ports(&args, count)?;
    let command = args.orin.as_deref().ok_or_else(|| anyhow!("--orin is required unless --no-render"))?;
    let connect_timeout = Duration::from_secs(args.connect_timeout);

    // 2–5 for every instance (synthetic-loop-dia.md, Training instances):
    // MuJoCo and the synthetic Arduino, then the synthetic Orin, which
    // pre-warms its pool from the instance's seed, opens the frame socket,
    // then connects to the instance's control socket. Every Orin is
    // launched before any connection is awaited, so they start together.
    let mut launched = Vec::with_capacity(count);
    for (k, &(orin_port, unity_port)) in ports.iter().enumerate() {
        let out = instance_folder(&args.out, k)?;
        let seed = args.seed.wrapping_add(k as u64);
        let sim = Simulation::new(&args, &out, seed, k)?;
        let (listener, url) = bind(orin_port, &format!("[{k}] control socket (synthetic Orin)")).await?;
        let frames_listen = SocketAddr::from((Ipv4Addr::LOCALHOST, unity_port));
        let orin_child = launch_orin(k, command, &url, frames_listen, &out, seed, &args)?;
        launched.push((sim, listener, orin_child, frames_listen));
    }
    let mut instances = Vec::with_capacity(count);
    for (k, (sim, listener, mut orin_child, frames_listen)) in launched.into_iter().enumerate() {
        let who = format!("synthetic Orin {k}");
        let stream = accept(&listener, &mut orin_child, &who, connect_timeout).await?;
        let orin = accept_async(stream).await.with_context(|| format!("{who} WebSocket handshake"))?;
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
                let place = args
                    .window_columns
                    .zip(args.window_step)
                    .map(|(columns, step)| grid_position(instance.k, columns, step, args.window_origin));
                Some(launch(instance.k, player, &instance.frames_url, place)?)
            }
            _ => {
                eprintln!("[{}] --no-launch: enter Play mode in the editor; it connects to {}", instance.k, instance.frames_url);
                None
            }
        };
    }

    // 7–8: each Unity connects to its Orin, and the Orin says so.
    for instance in &mut instances {
        wait_registered(&mut instance.orin, &mut instance.orin_child, instance.player.as_mut(), connect_timeout)
            .await
            .with_context(|| format!("instance {}", instance.k))?;
        eprintln!("[{}] Unity has registered with the synthetic Orin", instance.k);
    }

    let result = run(&mut instances, &args).await;

    // 22–25 for every instance: closing the control socket ends the run for
    // the Orin, which closes the frame socket, so the player quits, then the
    // Orin. Every socket is closed before any exit is awaited, so the
    // instances shut down together. A failed close only means the Orin is
    // already gone.
    let mut children = Vec::with_capacity(instances.len());
    for mut instance in instances {
        let _ = instance.orin.close(None).await;
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
            args.orin_port.checked_add(offset).ok_or_else(|| anyhow!("instance {k}'s control port passes 65535"))?,
            args.unity_port.checked_add(offset).ok_or_else(|| anyhow!("instance {k}'s frame port passes 65535"))?,
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
    // Sent each new command once, at the arrival step (19).
    arduino: SyntheticArduino,
    meters_per_pulse: f64,
    names: Names,
    latency: Latency,
    // The start pose every episode begins at (2, and R2 at each reset).
    start: Pose,
    // The latency band, ms, kept for reseeding at a reset (R4).
    band: (f64, f64),
    // 0 for the instance's first episode, one more at each reset (R1).
    episode: u32,
}

// One observation's capture state: simulated time and the pose (9–10,
// then each arrival step's).
struct Capture {
    time: f64,
    pose: Pose,
}

impl Simulation {
    fn new(args: &Args, out: &Path, seed: u64, k: usize) -> Result<Simulation> {
        // The cart always starts at the rail's center, 50% (2): only the
        // pole's angle is the run's to choose.
        let start = Pose { cart: 0.0, pole: args.start };
        // The MJCF and its names come only from the command line
        // (the trainer's settings.toml [mjcf]).
        let config = PhysicsConfig {
            model: args.model.clone(),
            names: Names {
                cart_joint: args.cart_joint.clone(),
                pole_joint: args.pole_joint.clone(),
                actuator: args.actuator.clone(),
            },
        };
        let mut physics = Physics::load(&config)?;
        // The reset zeroes ctrl: the cart is at rest until the first
        // command acts, at the first arrival step.
        physics.reset(start)?;
        let meters_per_pulse = meters_per_pulse(&RIG);
        // Created with RIG alone (3): the synthetic Arduino starts at the
        // rail's center too, where MuJoCo's cart was just reset.
        let arduino = SyntheticArduino::new(RIG);

        let path = out.join("states.csv");
        let file = File::create(&path).with_context(|| format!("creating {}", path.display()))?;
        let mut csv = BufWriter::new(file);
        writeln!(csv, "episode,id,time,cart,pole,left_x,left_y,right_x,right_y,pivot_x,pivot_y,end_x,end_y")?;
        let driver = match args.no_render {
            true => format!("--control {:?}", args.control),
            false => "the trainer's commands".to_string(),
        };
        eprintln!(
            "[{k}] physics: model {}\n         start cart {:+.3} m pole {:.3} rad, driven by {driver}, seed {seed}, latency RND({}, {}) ms -> {}",
            config.model.display(),
            start.cart,
            start.pole,
            args.rnd_delay_min,
            args.rnd_delay_max,
            path.display()
        );
        Ok(Simulation {
            physics,
            control: args.control,
            csv,
            arduino,
            meters_per_pulse,
            names: config.names,
            latency: Latency::new(seed, args.rnd_delay_min, args.rnd_delay_max)?,
            start,
            band: (args.rnd_delay_min, args.rnd_delay_max),
            episode: 0,
        })
    }

    // R1–R4 (synthetic-loop-dia.md, Reset in place): the next episode, back
    // at the start pose, t = 0, ctrl 0, with the latency draws reseeded from
    // its seed, so it repeats a fresh run with that seed. The synthetic
    // Arduino holds only RIG, so it has nothing to reset (R3).
    fn reset(&mut self, seed: u64) -> Result<()> {
        self.physics.reset(self.start)?;
        self.latency = Latency::new(seed, self.band.0, self.band.1)?;
        self.episode += 1;
        Ok(())
    }

    fn capture(&self) -> Capture {
        Capture { time: self.physics.time(), pose: self.physics.pose() }
    }

    // 15–16: MuJoCo steps on the old command, one fixed deltatime at a
    // time, until a step's time is later than the arrival time: that step
    // is the arrival step, where it then waits (the freeze) for the
    // Orin's reply. ctrl is untouched here: the old command's speed holds
    // until the new one is set at the arrival step.
    fn step_to(&mut self, arrival: f64) {
        while self.physics.time() <= arrival {
            self.physics.step(1);
        }
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
    // whose DRV8825 signals become the cart speed, set as ctrl once, here.
    // Returns that speed.
    fn command(&mut self, command: i32) -> Result<f64> {
        let signals = self.arduino.receive(encode_command(command));
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
            Some(row) => row[1..].iter().map(|v| v.to_string()).collect::<Vec<_>>().join(","),
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
    // across the instance's episodes), its latency and arrival time, when
    // its pose went out, and the pool its Orin replied with.
    capture: Capture,
    id: u32,
    latency_ms: f64,
    arrival: f64,
    sent: Instant,
    pool: Vec<PoolRow>,
    line: String,
}

impl Instance {
    // 9–11: at the capture, draw the Orin's latency, cache the arrival
    // time, and send the pose.
    async fn send(&mut self) -> Result<()> {
        self.sent = Instant::now();
        self.latency_ms = self.sim.latency.next_ms();
        self.arrival = self.capture.time + self.latency_ms / 1000.0;
        send_pose(&mut self.orin, self.id, &self.capture, &self.sim.names).await
    }

    // 14–16: step MuJoCo on the old command up to the arrival step while the
    // Orin works, then wait there for the reply (the freeze), and log the
    // truth row (the note before 17).
    async fn collect(&mut self, timeout: Duration) -> Result<()> {
        self.sim.step_to(self.arrival);
        self.pool = receive_pool(&mut self.orin, self.id, timeout).await?;
        let orin_ms = self.sent.elapsed().as_secs_f64() * 1000.0;
        self.sim.log(self.id, &self.capture, Some(&self.pool[0]))?;
        let n = &self.pool[0];
        self.line = format!(
            "[{}] observation {:06}  t {:.3} s  latency {:.3} ms, arrival step t {:.3} s (gap {:.0} ms)  cart {:+.3} m  pole {:.3} rad ({:.1}°)  orin {orin_ms:.1} ms  newest L({:.3}, {:.3}) R({:.3}, {:.3}) P({:.3}, {:.3}) E({:.3}, {:.3})",
            self.k,
            self.id,
            self.capture.time,
            self.latency_ms,
            self.sim.physics.time(),
            (self.sim.physics.time() - self.capture.time) * 1000.0,
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

// One round per pass (synthetic-loop-dia.md, Training instances): every
// advanced instance sends its pose first, so their Orins detect at the
// same time, then each steps to its arrival step and collects its reply;
// their pools go to the trainer in one line, and its answers, for any
// pending instances, say which advance next: on continue, the command is
// set at the arrival step, which is the next capture (19–21); on reset,
// the instance goes back to the start in place (R1–R7) and its next
// capture is the new episode's first; on stop, the loop ends for the
// shutdown (22). Instances not answered stay frozen at their arrival step.
// The run fails when an Orin reports failed or if a MuJoCo warned (P3).
async fn run(instances: &mut [Instance], args: &Args) -> Result<()> {
    let reply_timeout = Duration::from_secs(args.reply_timeout);
    let mut link = TrainerLink::new();
    let mut pending = vec![false; instances.len()];
    let mut advance: Vec<usize> = (0..instances.len()).collect();
    loop {
        for &k in &advance {
            instances[k].send().await.with_context(|| format!("instance {k}"))?;
        }
        for &k in &advance {
            instances[k].collect(reply_timeout).await.with_context(|| format!("instance {k}"))?;
            pending[k] = true;
        }
        let asked = Instant::now();
        let answers = {
            let pools: Vec<PoolEntry> = advance
                .iter()
                .map(|&k| PoolEntry { instance: k, id: instances[k].id, pool: &instances[k].pool })
                .collect();
            link.exchange(&pools, &pending)?
        };
        let trainer_ms = asked.elapsed().as_secs_f64() * 1000.0;
        for &k in &advance {
            eprintln!("{}  trainer {trainer_ms:.1} ms", instances[k].line);
        }
        if answers.iter().any(|answer| answer.next == Next::Stop) {
            let observations: u32 = instances.iter().map(|instance| instance.id + 1).sum();
            eprintln!("the trainer stopped the run after {observations} observations -> {}", args.out.display());
            break;
        }
        advance.clear();
        for answer in &answers {
            let instance = &mut instances[answer.instance];
            match answer.next {
                // 19–21: the new command acts from the arrival step, the
                // next capture.
                Next::Continue => {
                    let speed = instance.sim.command(answer.command)?;
                    eprintln!(
                        "[{}] observation {:06}  command {} pulses/s, ctrl {speed:+.3} m/s",
                        instance.k, instance.id, answer.command
                    );
                }
                // R1–R7: the instance goes back to the start in place; its
                // next capture is the new episode's first, with ids
                // counting on.
                Next::Reset => {
                    let seed = answer.seed.ok_or_else(|| anyhow!("a reset without a seed reached the run"))?;
                    instance.sim.reset(seed)?;
                    reset_orin(&mut instance.orin, seed, reply_timeout)
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
        bail!("MuJoCo warned ({}); the states cannot be trusted", warned.join(", "));
    }
    Ok(())
}

// --no-render: no Orin, no pool and no trainer; the same clock runs
// --frames states of one instance on the scripted --control.
fn run_physics(args: &Args, sim: &mut Simulation) -> Result<()> {
    let mut capture = sim.capture();
    for id in 0..args.frames {
        let latency_ms = sim.latency.next_ms();
        sim.step_to(capture.time + latency_ms / 1000.0);
        sim.log(id, &capture, None)?;
        let command = sim.scripted_command();
        let speed = sim.command(command)?;
        eprintln!(
            "observation {id:06}  t {:.3} s  latency {latency_ms:.3} ms, arrival step t {:.3} s (gap {:.0} ms)  cart {:+.3} m  pole {:.3} rad ({:.1}°)  command {command} pulses/s, ctrl {speed:+.3} m/s",
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

// One step pulse's cart travel: belt travel per revolution ÷ pulses per
// revolution (RIG: 96,000 µm / 6,400 = 15 µm).
fn meters_per_pulse(config: &Config) -> f64 {
    f64::from(config.belt_um_per_rev) / 1e6 / f64::from(config.pulses_per_rev())
}

// 21: cart velocity = STEP rate × meters per pulse, m/s, signed by DIR
// (RIG's top rate: 56,666 × 15 µm = 0.84999 m/s, inside ctrlrange).
fn cart_velocity(signals: Drv8825Signals, meters_per_pulse: f64) -> f64 {
    let speed = f64::from(signals.step_rate) * meters_per_pulse;
    match signals.dir {
        Direction::Positive => speed,
        Direction::Negative => -speed,
    }
}

// 6: the player, told its Orin's frame socket and, with a grid, its
// window's place (synthetic-loop-dia.md, Training instances, The windows).
// kill_on_drop: an early error return drops the handle, and the player
// must not outlive the orchestrator that launched it.
fn launch(k: usize, player: &Path, frames_url: &str, place: Option<Pixels>) -> Result<Child> {
    let mut command = Command::new(player);
    command.arg("-orin").arg(frames_url);
    let mut shown = format!("{} -orin {frames_url}", player.display());
    if let Some(Pixels { x, y }) = place {
        command.arg("-window-position").arg(format!("{x},{y}"));
        shown.push_str(&format!(" -window-position {x},{y}"));
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
// address, the instance's folder, its seed and the latency band (its bounds
// as the shortest text that reads back to the same f64); kill_on_drop as
// for the player.
fn launch_orin(
    k: usize,
    command: &[String],
    url: &str,
    frames_listen: SocketAddr,
    out: &Path,
    seed: u64,
    args: &Args,
) -> Result<Child> {
    let (program, rest) = command.split_first().ok_or_else(|| anyhow!("--orin needs a command"))?;
    let (min, max) = (args.rnd_delay_min, args.rnd_delay_max);
    eprintln!(
        "[{k}] launching synthetic Orin: {} --orchestrator {url} --frames-listen {frames_listen} --out {} --seed {seed} --rnd-delay-min {min} --rnd-delay-max {max}",
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
        .arg("--seed")
        .arg(seed.to_string())
        .arg("--rnd-delay-min")
        .arg(min.to_string())
        .arg("--rnd-delay-max")
        .arg(max.to_string())
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
async fn accept(listener: &TcpListener, child: &mut Child, who: &str, timeout: Duration) -> Result<TcpStream> {
    let wait = async {
        tokio::select! {
            accepted = listener.accept() => Ok(accepted?.0),
            status = child.wait() => Err(anyhow!("{who} exited before connecting: {status:?}")),
        }
    };
    tokio::time::timeout(timeout, wait)
        .await
        .map_err(|_| anyhow!("no connection from the {who} within {} s", timeout.as_secs()))?
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
    tokio::time::timeout(timeout, wait)
        .await
        .map_err(|_| anyhow!("Unity did not register with the synthetic Orin within {} s", timeout.as_secs()))?
}

// 11: one capture state to the synthetic Orin.
async fn send_pose(orin: &mut WebSocketStream<TcpStream>, id: u32, capture: &Capture, names: &Names) -> Result<()> {
    let message = PoseMessage {
        id,
        time: capture.time,
        joints: vec![
            JointQpos { name: names.cart_joint.clone(), qpos: capture.pose.cart },
            JointQpos { name: names.pole_joint.clone(), qpos: capture.pose.pole },
        ],
    };
    orin.send(Message::text(serde_json::to_string(&message)?))
        .await
        .with_context(|| format!("sending pose {id} to the synthetic Orin"))
}

// 14: the Orin's one reply, the pool, newest first. A reply for another
// id, a pool of the wrong shape, or failed ends the run.
async fn receive_pool(orin: &mut WebSocketStream<TcpStream>, id: u32, timeout: Duration) -> Result<Vec<PoolRow>> {
    let reply = loop {
        let message = tokio::time::timeout(timeout, orin.next())
            .await
            .map_err(|_| anyhow!("no reply from the synthetic Orin for pose {id} within {} s", timeout.as_secs()))?
            .ok_or_else(|| anyhow!("synthetic Orin disconnected"))??;
        match message {
            Message::Text(text) => {
                break serde_json::from_str::<OrinReply>(text.as_str())
                    .with_context(|| format!("synthetic Orin reply {:?}", text.as_str()))?;
            }
            Message::Binary(_) => bail!("synthetic Orin sent binary data for pose {id}, expected its reply"),
            Message::Close(frame) => bail!("synthetic Orin closed the socket: {frame:?}"),
            _ => {}
        }
    };
    if reply.id != id {
        bail!("synthetic Orin replied for pose {}, expected {id}", reply.id);
    }
    // The pool's shape (Contracts): 6 entries, newest first, the newest at
    // age 0 and every older one strictly older.
    if reply.pool.len() != POOL_ENTRIES {
        bail!("pool {id} has {} entries, expected {POOL_ENTRIES}", reply.pool.len());
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

// R5–R7: tell the Orin to pre-warm its pool afresh from the seed, and wait
// for its {"type": "reset"}, so the next pose meets the new episode's pool.
async fn reset_orin(orin: &mut WebSocketStream<TcpStream>, seed: u64, timeout: Duration) -> Result<()> {
    let message = ResetMessage { kind: "reset", seed };
    orin.send(Message::text(serde_json::to_string(&message)?))
        .await
        .context("sending the reset to the synthetic Orin")?;
    loop {
        let message = tokio::time::timeout(timeout, orin.next())
            .await
            .map_err(|_| anyhow!("no reset reply from the synthetic Orin within {} s", timeout.as_secs()))?
            .ok_or_else(|| anyhow!("synthetic Orin disconnected during the reset"))??;
        match message {
            Message::Text(text) => {
                let reply = serde_json::from_str::<Registered>(text.as_str())
                    .with_context(|| format!("synthetic Orin reset reply {:?}", text.as_str()))?;
                if reply.kind != "reset" {
                    bail!("synthetic Orin sent {:?}, expected its reset reply", text.as_str());
                }
                return Ok(());
            }
            Message::Binary(_) => bail!("synthetic Orin sent binary data, expected its reset reply"),
            Message::Close(frame) => bail!("synthetic Orin closed the socket during the reset: {frame:?}"),
            _ => {}
        }
    }
}

// A launched process quits on its own when its socket closes; it is
// killed only if it is still running after the grace period.
async fn stop(mut child: Child, who: &str) -> Result<()> {
    match tokio::time::timeout(CLIENT_EXIT_GRACE, child.wait()).await {
        Ok(status) => eprintln!("{who} exited: {}", status?),
        Err(_) => {
            child.kill().await.with_context(|| format!("killing the {who}"))?;
            eprintln!("{who} killed after {} s", CLIENT_EXIT_GRACE.as_secs());
        }
    }
    Ok(())
}
