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
use object_detection::{Detector, DetectorConfig, MarkerFrame, SLOTS};
use physics::{Names, Physics, PhysicsConfig, Pose};
use std::{
    fs::File,
    io::{BufWriter, Write},
    path::Path,
    time::{Duration, Instant},
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
    detection: Option<Box<Detection>>,
}

// --detect: the detector and what it found over the run.
struct Detection {
    detector: Detector,
    frames: usize,
    found: [usize; SLOTS],
    seconds: f64,
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

        // The detector's config sits beside physics.toml, at the repo
        // root, whatever the current directory.
        let detection = match args.detect {
            false => None,
            true => {
                let folder = config
                    .path
                    .parent()
                    .ok_or_else(|| anyhow!("{} has no parent", config.path.display()))?;
                let detector = Detector::load(DetectorConfig::load(&folder.join("ObjectDetection.toml"))?)?;
                Some(Box::new(Detection { detector, frames: 0, found: [0; SLOTS], seconds: 0.0 }))
            }
        };

        let path = args.out.join("states.csv");
        let file = File::create(&path).with_context(|| format!("creating {}", path.display()))?;
        let mut csv = BufWriter::new(file);
        write!(csv, "id,time,ctrl,cart,pole")?;
        if let Some(detection) = &detection {
            let classes: Vec<String> = detection.detector.config().classes.iter().map(|c| c.to_lowercase()).collect();
            for class in &classes {
                write!(csv, ",{class}_x,{class}_y")?;
            }
            for class in &classes {
                write!(csv, ",{class}_conf")?;
            }
        }
        writeln!(csv)?;
        println!(
            "physics: model {}\n         start cart {:+.3} m pole {:.3} rad, control {:?}, {} ms = {steps_per_frame} steps per frame -> {}",
            config.model.display(),
            start.cart,
            start.pole,
            args.control,
            args.frame_interval_ms,
            path.display()
        );
        Ok(Source::Physics(Simulation { physics, control: args.control, steps_per_frame, csv, detection }))
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

    // With --detect: frame `id`'s marker frame, printed with the time the
    // detection took. None without --detect.
    fn detect(&mut self, id: u32, jpeg: &[u8]) -> Result<Option<MarkerFrame>> {
        let Source::Physics(Simulation { detection: Some(detection), .. }) = self else {
            return Ok(None);
        };
        let started = Instant::now();
        let marker_frame = detection
            .detector
            .detect(jpeg, usize::from(FRAME_WIDTH), usize::from(FRAME_HEIGHT))
            .with_context(|| format!("detecting frame {id}"))?;
        let seconds = started.elapsed().as_secs_f64();
        detection.frames += 1;
        detection.seconds += seconds;
        let mut line = format!("detect {id:06}  {:.0} ms", seconds * 1000.0);
        for (slot, name) in detection.detector.config().classes.iter().enumerate() {
            let (x, y) = marker_frame.slots[slot];
            line += &format!("  {name} {x:+.3} {y:+.3} ({:.2})", marker_frame.confidences[slot]);
            if marker_frame.detected(slot) {
                detection.found[slot] += 1;
            }
        }
        println!("{line}");
        Ok(Some(marker_frame))
    }

    // After frame `id` is rendered: log the truth (and, with --detect,
    // what the detector saw), apply the command, and step to the next
    // capture (D = 0: the command takes effect at the capture it was
    // decided at).
    fn advance(&mut self, id: u32, capture: &Capture, seen: Option<&MarkerFrame>) -> Result<()> {
        let (Source::Physics(sim), Some((time, ctrl))) = (self, capture.time_and_ctrl) else {
            return Ok(());
        };
        // `{}` writes a float as the shortest text that reads back to the
        // same number, so identical runs give identical files.
        write!(sim.csv, "{id},{time},{ctrl},{},{}", capture.pose.cart, capture.pose.pole)?;
        if sim.detection.is_some() {
            match seen {
                Some(marker_frame) => {
                    for (x, y) in marker_frame.slots {
                        write!(sim.csv, ",{x},{y}")?;
                    }
                    for conf in marker_frame.confidences {
                        write!(sim.csv, ",{conf}")?;
                    }
                }
                // A frame that broke the contract was not detected; its
                // fields stay empty (never -1, which means "not found").
                None => write!(sim.csv, "{}", ",".repeat(3 * SLOTS))?,
            }
        }
        writeln!(sim.csv)?;
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
                if let Some(detection) = &sim.detection {
                    let mut line = format!(
                        "detected in {} frames (mean {:.0} ms):",
                        detection.frames,
                        detection.seconds * 1000.0 / detection.frames.max(1) as f64
                    );
                    for (name, found) in detection.detector.config().classes.iter().zip(detection.found) {
                        line += &format!("  {name} {found}");
                    }
                    println!("{line}");
                }
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

        let mut seen = None;
        match ws.as_deref_mut() {
            Some(ws) => match render(ws, id, pose, names, reply_timeout, &args.out, &state).await? {
                Some(jpeg) => seen = source.detect(id, &jpeg)?,
                None => broken += 1,
            },
            None => println!("state {id:06}  {state}"),
        }
        source.advance(id, &capture, seen.as_ref())?;
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
// header. Returns the frame when it met the 728×544 JPEG contract.
async fn render(
    ws: &mut WebSocketStream<TcpStream>,
    id: u32,
    pose: Pose,
    names: &Names,
    reply_timeout: Duration,
    out: &Path,
    state: &str,
) -> Result<Option<Vec<u8>>> {
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
            Ok(ok.then_some(jpeg))
        }
        None => {
            println!("frame {id:06}  not a JPEG ({} bytes)  {state}", jpeg.len());
            Ok(None)
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
