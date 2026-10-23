mod args;
mod control;
mod joint_qpos;
mod jpeg_info;
mod state_message;
mod sweep;

use crate::{args::Args, control::Control, joint_qpos::JointQpos, jpeg_info::JpegInfo, state_message::StateMessage};

use anyhow::{Context, Result, anyhow, bail};
use clap::Parser;
use futures_util::{SinkExt, StreamExt};
use physics::{Names, Physics, PhysicsConfig, Pose};
use std::{
    fs::File,
    io::{BufWriter, Write},
    path::Path,
    time::Duration,
};
use tokio::{
    net::{TcpListener, TcpStream},
    process::{Child, Command},
};
use tokio_tungstenite::{WebSocketStream, accept_async, tungstenite::Message};

// The frame contract (loop-plan.md): half the IMX296's native 1456×1088.
const FRAME_WIDTH: u16 = 728;
const FRAME_HEIGHT: u16 = 544;

// How long a launched player gets to quit on its own after the socket
// closes before it is killed.
const PLAYER_EXIT_GRACE: Duration = Duration::from_secs(5);

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    // Every mode takes the wire keys from physics.toml's [names] — the
    // same names physics mode reads qpos by.
    let config = PhysicsConfig::load()?;
    tokio::fs::create_dir_all(&args.out)
        .await
        .with_context(|| format!("creating {}", args.out.display()))?;
    let mut source = Source::new(&args, &config)?;

    if args.no_render {
        return run(None, &args, &config.names, &mut source).await;
    }

    let listener = TcpListener::bind(args.listen)
        .await
        .with_context(|| format!("binding {}", args.listen))?;
    let url = format!("ws://{}", args.listen);
    println!("listening on {url}");

    let mut child = match (&args.player, args.no_launch) {
        (Some(player), false) => Some(launch(player, &url)?),
        _ => {
            println!("--no-launch: waiting for a client (enter Play mode in the editor)");
            None
        }
    };

    let stream = accept(&listener, child.as_mut(), Duration::from_secs(args.connect_timeout)).await?;
    let mut ws = accept_async(stream).await.context("WebSocket handshake")?;
    println!("player connected");

    let result = run(Some(&mut ws), &args, &config.names, &mut source).await;
    // Closing tells the player the run is over; a failed close only
    // means the player is already gone.
    let _ = ws.close(None).await;
    if let Some(child) = child {
        stop(child).await?;
    }
    result
}

// Where the states come from: a fixed list (the sweep or --pose), or
// MuJoCo stepping between frames.
enum Source {
    Poses(Vec<Pose>),
    Physics(Simulation),
}

struct Simulation {
    physics: Physics,
    control: Control,
    steps_per_frame: u32,
    csv: BufWriter<File>,
}

// One frame's state at its capture. Physics mode adds the simulated time
// and the command set at this capture, held until the next one.
struct Capture {
    pose: Pose,
    time_and_ctrl: Option<(f64, f64)>,
}

impl Source {
    fn new(args: &Args, config: &PhysicsConfig) -> Result<Source> {
        if !args.physics {
            let poses = match args.pose.as_deref() {
                Some([cart, pole]) => vec![Pose { cart: *cart, pole: *pole }],
                Some(other) => bail!("--pose takes exactly two values (cart, pole), got {}", other.len()),
                None => (0..args.frames).map(|i| sweep::sweep(i, args.frames)).collect(),
            };
            return Ok(Source::Poses(poses));
        }

        let start = match args.start.as_deref() {
            None => Pose { cart: 0.0, pole: 0.0 },
            Some([cart, pole]) => Pose { cart: *cart, pole: *pole },
            Some(other) => bail!("--start takes exactly two values (cart, pole), got {}", other.len()),
        };
        let mut physics = Physics::load(config)?;
        physics.reset(start)?;
        let steps_per_frame = steps_per_frame(args.frame_interval_ms, physics.timestep())?;

        let path = args.out.join("states.csv");
        let file = File::create(&path).with_context(|| format!("creating {}", path.display()))?;
        let mut csv = BufWriter::new(file);
        writeln!(csv, "id,time,ctrl,cart,pole")?;
        println!(
            "physics: model {}\n         start cart {:+.3} m pole {:.3} rad, control {:?}, {} ms = {steps_per_frame} steps per frame -> {}",
            config.model.display(),
            start.cart,
            start.pole,
            args.control,
            args.frame_interval_ms,
            path.display()
        );
        Ok(Source::Physics(Simulation { physics, control: args.control, steps_per_frame, csv }))
    }

    fn len(&self, frames: u32) -> Result<u32> {
        match self {
            Source::Poses(poses) => u32::try_from(poses.len()).context("too many poses"),
            Source::Physics(_) => Ok(frames),
        }
    }

    fn capture(&self, index: usize) -> Result<Capture> {
        match self {
            Source::Poses(poses) => {
                let pose = *poses.get(index).ok_or_else(|| anyhow!("no pose {index}"))?;
                Ok(Capture { pose, time_and_ctrl: None })
            }
            Source::Physics(sim) => {
                let time = sim.physics.time();
                Ok(Capture {
                    pose: sim.physics.pose(),
                    time_and_ctrl: Some((time, sim.control.at(time))),
                })
            }
        }
    }

    // After frame `id` is rendered: log the truth, apply the command,
    // and step to the next capture (D = 0: the command takes effect at
    // the capture it was decided at).
    fn advance(&mut self, id: u32, capture: &Capture) -> Result<()> {
        let (Source::Physics(sim), Some((time, ctrl))) = (self, capture.time_and_ctrl) else {
            return Ok(());
        };
        // `{}` writes an f64 as the shortest text that reads back to the
        // same number, so identical runs give identical files.
        writeln!(sim.csv, "{id},{time},{ctrl},{},{}", capture.pose.cart, capture.pose.pole)?;
        sim.physics.set_ctrl(ctrl)?;
        sim.physics.step(sim.steps_per_frame);
        Ok(())
    }

    // MuJoCo warnings seen during the run (0 for a fixed list).
    fn finish(&mut self) -> Result<usize> {
        match self {
            Source::Poses(_) => Ok(0),
            Source::Physics(sim) => {
                sim.csv.flush().context("writing states.csv")?;
                Ok(sim.physics.warnings())
            }
        }
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

// kill_on_drop: an early error return drops the handle, and the player
// must not outlive the orchestrator that launched it.
fn launch(player: &Path, url: &str) -> Result<Child> {
    println!("launching {} -orchestrator {url}", player.display());
    Command::new(player)
        .arg("-orchestrator")
        .arg(url)
        .kill_on_drop(true)
        .spawn()
        .with_context(|| format!("launching {}", player.display()))
}

// Waits for the first connection; a launched player that exits before
// connecting fails the wait at once instead of after the timeout.
async fn accept(listener: &TcpListener, child: Option<&mut Child>, timeout: Duration) -> Result<TcpStream> {
    let wait = async {
        match child {
            Some(child) => tokio::select! {
                accepted = listener.accept() => Ok(accepted?.0),
                status = child.wait() => Err(anyhow!("player exited before connecting: {status:?}")),
            },
            None => Ok(listener.accept().await?.0),
        }
    };
    tokio::time::timeout(timeout, wait)
        .await
        .map_err(|_| anyhow!("no connection within {} s", timeout.as_secs()))?
}

// Lockstep: take one state, send it, wait for its one frame, check and
// save it, then advance. Every frame is checked; the run fails at the
// end if any broke the size contract or MuJoCo warned (P3). Without a
// socket (--no-render) only the states are produced.
async fn run(
    mut ws: Option<&mut WebSocketStream<TcpStream>>,
    args: &Args,
    names: &Names,
    source: &mut Source,
) -> Result<()> {
    let reply_timeout = Duration::from_secs(args.reply_timeout);
    let count = source.len(args.frames)?;
    let mut broken = 0usize;
    for id in 0..count {
        let capture = source.capture(usize::try_from(id)?)?;
        let pose = capture.pose;
        let mut state = format!(
            "cart {:+.3} m  pole {:.3} rad ({:.1}°)",
            pose.cart,
            pose.pole,
            pose.pole.to_degrees()
        );
        if let Some((time, ctrl)) = capture.time_and_ctrl {
            state = format!("t {time:.3} s  ctrl {ctrl:+.3} m/s  {state}");
        }

        match ws.as_deref_mut() {
            Some(ws) => {
                if !render(ws, id, pose, names, reply_timeout, &args.out, &state).await? {
                    broken += 1;
                }
            }
            None => println!("state {id:06}  {state}"),
        }
        source.advance(id, &capture)?;
    }

    let warnings = source.finish()?;
    if broken > 0 {
        bail!("{broken} of {count} frames broke the 728x544 JPEG contract");
    }
    if warnings > 0 {
        bail!("MuJoCo warned {warnings} time(s); the states cannot be trusted");
    }
    match ws {
        Some(_) => println!("all {count} frames 728x544 -> {}", args.out.display()),
        None => println!("{count} states -> {}", args.out.display()),
    }
    Ok(())
}

// Sends one state, saves the frame that comes back, and prints its
// header. Returns whether the frame met the 728×544 JPEG contract.
async fn render(
    ws: &mut WebSocketStream<TcpStream>,
    id: u32,
    pose: Pose,
    names: &Names,
    reply_timeout: Duration,
    out: &Path,
    state: &str,
) -> Result<bool> {
    let message = StateMessage {
        id,
        joints: vec![
            JointQpos { name: names.cart_joint.clone(), qpos: pose.cart },
            JointQpos { name: names.pole_joint.clone(), qpos: pose.pole },
        ],
    };
    ws.send(Message::text(serde_json::to_string(&message)?))
        .await
        .with_context(|| format!("sending state {id}"))?;
    let jpeg = next_frame(ws, reply_timeout)
        .await
        .with_context(|| format!("frame {id}"))?;

    let path = out.join(format!("frame_{id:06}.jpg"));
    tokio::fs::write(&path, &jpeg)
        .await
        .with_context(|| format!("writing {}", path.display()))?;

    match JpegInfo::read(&jpeg) {
        Some(info) => {
            let ok = info.width == FRAME_WIDTH && info.height == FRAME_HEIGHT;
            println!(
                "frame {id:06}  {}x{}  {} ch  {} bytes  {state}{}",
                info.width,
                info.height,
                info.channels,
                jpeg.len(),
                if ok { "" } else { "  <- expected 728x544" }
            );
            Ok(ok)
        }
        None => {
            println!("frame {id:06}  not a JPEG ({} bytes)  {state}", jpeg.len());
            Ok(false)
        }
    }
}

// The next binary message is the frame. Text from the player is
// informational (logged, not a reply); ping/pong are answered by the
// WebSocket layer itself.
async fn next_frame(ws: &mut WebSocketStream<TcpStream>, timeout: Duration) -> Result<Vec<u8>> {
    loop {
        let message = tokio::time::timeout(timeout, ws.next())
            .await
            .map_err(|_| anyhow!("no reply within {} s", timeout.as_secs()))?
            .ok_or_else(|| anyhow!("player disconnected"))??;
        match message {
            Message::Binary(data) => return Ok(data.to_vec()),
            Message::Text(text) => println!("player: {}", text.as_str()),
            Message::Close(frame) => bail!("player closed the socket: {frame:?}"),
            _ => {}
        }
    }
}

// The player quits on its own when the socket closes; it is killed only
// if it is still running after the grace period.
async fn stop(mut child: Child) -> Result<()> {
    match tokio::time::timeout(PLAYER_EXIT_GRACE, child.wait()).await {
        Ok(status) => println!("player exited: {}", status?),
        Err(_) => {
            child.kill().await.context("killing the player")?;
            println!("player killed after {} s", PLAYER_EXIT_GRACE.as_secs());
        }
    }
    Ok(())
}
