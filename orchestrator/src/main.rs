mod args;
mod control;
mod joint_qpos;
mod orin_reply;
mod pose_message;
mod registered;

use crate::{
    args::Args, control::Control, joint_qpos::JointQpos, orin_reply::OrinReply, pose_message::PoseMessage,
    registered::Registered,
};

use anyhow::{Context, Result, anyhow, bail};
use arduino::{Config, RIG, SyntheticArduino, encode_command};
use clap::Parser;
use futures_util::{SinkExt, StreamExt};
use physics::{Names, Physics, PhysicsConfig, Pose};
use std::{
    fs::File,
    io::{BufWriter, Write},
    net::{Ipv4Addr, SocketAddr},
    path::Path,
    process::ExitStatus,
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

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    tokio::fs::create_dir_all(&args.out)
        .await
        .with_context(|| format!("creating {}", args.out.display()))?;
    // 2–3: MuJoCo loaded and reset, the synthetic Arduino created.
    let mut sim = Simulation::new(&args)?;

    if args.no_render {
        return run(None, &args, &mut sim).await;
    }

    // Each socket has its own port, so neither the connection order nor
    // the messages have to tell the participants apart.
    if args.orin_port == args.unity_port {
        bail!("--orin-port and --unity-port are both {}; each socket needs its own port", args.orin_port);
    }
    let command = args.orin.as_deref().ok_or_else(|| anyhow!("--orin is required unless --no-render"))?;
    let seed = args.seed.ok_or_else(|| anyhow!("--orin needs --seed (the Orin's pre-warm gaps)"))?;
    let connect_timeout = Duration::from_secs(args.connect_timeout);

    // 4–5: the synthetic Orin first. It pre-warms its pool from the seed,
    // opens the frame socket, then connects to the control socket.
    let (orin_listener, orin_url) = bind(args.orin_port, "control socket (synthetic Orin)").await?;
    let frames_listen = SocketAddr::from((Ipv4Addr::LOCALHOST, args.unity_port));
    let mut orin_child = launch_orin(command, &orin_url, frames_listen, &args.out, seed)?;
    let stream = accept(&orin_listener, &mut orin_child, "synthetic Orin", connect_timeout).await?;
    let mut orin = accept_async(stream).await.context("synthetic Orin WebSocket handshake")?;
    println!("synthetic Orin connected");

    // 6: the player, pointed at the Orin's frame socket — or, with
    // --no-launch, editor Play mode connects there itself.
    let frames_url = format!("ws://{frames_listen}");
    let mut player = match (&args.player, args.no_launch) {
        (Some(player), false) => Some(launch(player, &frames_url)?),
        _ => {
            println!("--no-launch: enter Play mode in the editor; it connects to {frames_url}");
            None
        }
    };

    // 7–8: Unity connects to the Orin, and the Orin says so.
    wait_registered(&mut orin, &mut orin_child, player.as_mut(), connect_timeout).await?;
    println!("Unity has registered with the synthetic Orin");

    let result = run(Some(&mut orin), &args, &mut sim).await;

    // 22–25: closing the control socket ends the run for the Orin, which
    // closes the frame socket, so the player quits, then the Orin. A
    // failed close only means the Orin is already gone.
    let _ = orin.close(None).await;
    // The server ends the TCP connection (RFC 6455 7.1.1); the Orin's
    // client waits for that, up to its 10 s close timeout, before it
    // exits, so the socket is dropped before the exit is awaited.
    drop(orin);
    stop(orin_child, "synthetic Orin").await?;
    if let Some(player) = player {
        stop(player, "player").await?;
    }
    result
}

// MuJoCo, the synthetic Arduino between the command and MuJoCo
// (synthetic-arduino-plan.md), and the truth log.
struct Simulation {
    physics: Physics,
    control: Control,
    steps_per_frame: u32,
    csv: BufWriter<File>,
    // Ticked once per physics step.
    arduino: SyntheticArduino,
    step_micros: u32,
    meters_per_pulse: f64,
    names: Names,
}

// One observation's state at its capture: simulated time, the pose, and
// the command set at this capture, held until the next one.
struct Capture {
    time: f64,
    pose: Pose,
    ctrl: f64,
}

impl Simulation {
    fn new(args: &Args) -> Result<Simulation> {
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
        physics.reset(start)?;
        let steps_per_frame = steps_per_frame(args.frame_interval_ms, physics.timestep())?;
        let step_micros = step_micros(physics.timestep())?;
        let meters_per_pulse = meters_per_pulse(&RIG);
        // Created with RIG alone (3): the synthetic Arduino starts at the
        // rail's center too, where MuJoCo's cart was just reset.
        let arduino = SyntheticArduino::new(RIG);

        let path = args.out.join("states.csv");
        let file = File::create(&path).with_context(|| format!("creating {}", path.display()))?;
        let mut csv = BufWriter::new(file);
        writeln!(csv, "id,time,ctrl,cart,pole,command")?;
        println!(
            "physics: model {}\n         start cart {:+.3} m pole {:.3} rad, control {:?}, {} ms = {steps_per_frame} steps per frame -> {}",
            config.model.display(),
            start.cart,
            start.pole,
            args.control,
            args.frame_interval_ms,
            path.display()
        );
        Ok(Simulation {
            physics,
            control: args.control,
            steps_per_frame,
            csv,
            arduino,
            step_micros,
            meters_per_pulse,
            names: config.names,
        })
    }

    // 9–10, and each arrival step after: the capture state.
    fn capture(&self) -> Capture {
        let time = self.physics.time();
        Capture { time, pose: self.physics.pose(), ctrl: self.control.at(time) }
    }

    // After observation `id`: log the truth, apply the command, and step
    // to the next capture (D = 0: the command takes effect at the capture
    // it was decided at).
    //
    // The scripted speed stands in for the trainer's command: it is sent
    // to the firmware as the packet the trainer would send (pulses/s,
    // rounded), and each physics step MuJoCo's ctrl is the step signal the
    // firmware emitted in that step, as a speed: pulses × meters per pulse
    // ÷ the step's duration.
    fn advance(&mut self, id: u32, capture: &Capture) -> Result<()> {
        // `as` saturates, so an absurd speed becomes the i32 limit, which
        // the firmware clamps to the motor's top rate.
        let command = (capture.ctrl / self.meters_per_pulse).round() as i32;
        // `{}` writes a float as the shortest text that reads back to the
        // same number, so identical runs give identical files.
        writeln!(
            self.csv,
            "{id},{},{},{},{},{command}",
            capture.time, capture.ctrl, capture.pose.cart, capture.pose.pole
        )?;
        self.arduino.receive(encode_command(command));
        for _ in 0..self.steps_per_frame {
            let signal = self.arduino.tick(self.step_micros);
            let ctrl = f64::from(signal.pulses) * self.meters_per_pulse / (f64::from(signal.micros) / 1e6);
            self.physics.set_ctrl(ctrl)?;
            self.physics.step(1);
        }
        Ok(())
    }

    // MuJoCo warnings seen during the run.
    fn finish(&mut self) -> Result<usize> {
        self.csv.flush().context("writing states.csv")?;
        Ok(self.physics.warnings())
    }
}

// P4: the frame interval is a whole number of physics steps; anything
// else is rejected rather than rounded.
fn steps_per_frame(interval_ms: u32, timestep: f64) -> Result<u32> {
    let ratio = f64::from(interval_ms) / 1000.0 / timestep;
    let steps = ratio.round();
    if steps < 1.0 || (ratio - steps).abs() > 1e-6 || steps > f64::from(u32::MAX) {
        bail!("--frame-interval-ms {interval_ms} is not a whole number of {timestep} s physics steps");
    }
    Ok(steps as u32)
}

// The firmware ticks in whole microseconds; MuJoCo's 2 ms step is 2,000.
fn step_micros(timestep: f64) -> Result<u32> {
    let micros = timestep * 1e6;
    let whole = micros.round();
    if whole < 1.0 || (micros - whole).abs() > 1e-6 || whole > f64::from(u32::MAX) {
        bail!("the physics timestep {timestep} s is not a whole number of microseconds");
    }
    Ok(whole as u32)
}

// One step pulse's cart travel: belt travel per revolution ÷ pulses per
// revolution (RIG: 96,000 µm / 6,400 = 15 µm).
fn meters_per_pulse(config: &Config) -> f64 {
    f64::from(config.belt_um_per_rev) / 1e6 / f64::from(config.pulses_per_rev())
}

// 6: the player, told the Orin's frame socket. kill_on_drop: an early
// error return drops the handle, and the player must not outlive the
// orchestrator that launched it.
fn launch(player: &Path, frames_url: &str) -> Result<Child> {
    println!("launching {} -orin {frames_url}", player.display());
    Command::new(player)
        .arg("-orin")
        .arg(frames_url)
        .kill_on_drop(true)
        .spawn()
        .with_context(|| format!("launching {}", player.display()))
}

// The synthetic Orin (synthetic-loop-dia.md, 4): the given program and
// arguments, plus the control socket's address, the frame socket's
// address, the run's folder and the seed; kill_on_drop as for the player.
fn launch_orin(command: &[String], url: &str, frames_listen: SocketAddr, out: &Path, seed: u64) -> Result<Child> {
    let (program, rest) = command.split_first().ok_or_else(|| anyhow!("--orin needs a command"))?;
    println!(
        "launching synthetic Orin: {} --orchestrator {url} --frames-listen {frames_listen} --out {} --seed {seed}",
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
        .kill_on_drop(true)
        .spawn()
        .with_context(|| format!("launching {program}"))
}

// The loop runs on this workstation, so every socket is on 127.0.0.1.
async fn bind(port: u16, what: &str) -> Result<(TcpListener, String)> {
    let address = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    let listener = TcpListener::bind(address)
        .await
        .with_context(|| format!("binding the {what} on {address}"))?;
    let url = format!("ws://{address}");
    println!("{what}: listening on {url}");
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

// Lockstep: take the capture state, send it to the Orin, wait for its one
// reply, then advance. The run fails on an error reply (O3) or if MuJoCo
// warned (P3). Without the Orin (--no-render) only the states are
// produced.
async fn run(mut orin: Option<&mut WebSocketStream<TcpStream>>, args: &Args, sim: &mut Simulation) -> Result<()> {
    let reply_timeout = Duration::from_secs(args.reply_timeout);
    for id in 0..args.frames {
        let capture = sim.capture();
        let state = format!(
            "t {:.3} s  ctrl {:+.3} m/s  cart {:+.3} m  pole {:.3} rad ({:.1}°)",
            capture.time,
            capture.ctrl,
            capture.pose.cart,
            capture.pose.pole,
            capture.pose.pole.to_degrees()
        );
        match orin.as_deref_mut() {
            Some(orin) => observe(orin, id, &capture, &sim.names, reply_timeout, &state).await?,
            None => println!("state {id:06}  {state}"),
        }
        sim.advance(id, &capture)?;
    }

    let warnings = sim.finish()?;
    if warnings > 0 {
        bail!("MuJoCo warned {warnings} time(s); the states cannot be trusted");
    }
    match orin {
        Some(_) => println!("all {} observations -> {}", args.frames, args.out.display()),
        None => println!("{} states -> {}", args.frames, args.out.display()),
    }
    Ok(())
}

// 11 and 14: sends one capture state to the synthetic Orin and waits for
// its one reply. A reply for another id, or an error reply (O3), fails the
// run.
async fn observe(
    orin: &mut WebSocketStream<TcpStream>,
    id: u32,
    capture: &Capture,
    names: &Names,
    timeout: Duration,
    state: &str,
) -> Result<()> {
    let started = Instant::now();
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
        .with_context(|| format!("sending pose {id} to the synthetic Orin"))?;
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
    if let Some(error) = reply.error {
        bail!("synthetic Orin rejected frame {id}: {error}");
    }
    println!("observation {id:06}  {state}  orin {:.1} ms", started.elapsed().as_secs_f64() * 1000.0);
    Ok(())
}

// A launched process quits on its own when its socket closes; it is
// killed only if it is still running after the grace period.
async fn stop(mut child: Child, who: &str) -> Result<()> {
    match tokio::time::timeout(CLIENT_EXIT_GRACE, child.wait()).await {
        Ok(status) => println!("{who} exited: {}", status?),
        Err(_) => {
            child.kill().await.with_context(|| format!("killing the {who}"))?;
            println!("{who} killed after {} s", CLIENT_EXIT_GRACE.as_secs());
        }
    }
    Ok(())
}
