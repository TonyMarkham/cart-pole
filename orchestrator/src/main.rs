mod args;
mod control;
mod joint_qpos;
mod latency;
mod orin_reply;
mod pose_message;
mod registered;
#[cfg(test)]
mod tests;

use crate::{
    args::Args,
    control::Control,
    joint_qpos::JointQpos,
    latency::Latency,
    orin_reply::{OrinReply, POOL_ENTRIES, PoolRow},
    pose_message::PoseMessage,
    registered::Registered,
};

use anyhow::{Context, Result, anyhow, bail};
use arduino::{Config, Direction, Drv8825Signals, RIG, SyntheticArduino, encode_command};
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
    let seed = args.seed;
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
    csv: BufWriter<File>,
    // Sent each new command once, at the arrival step (19).
    arduino: SyntheticArduino,
    meters_per_pulse: f64,
    names: Names,
    latency: Latency,
}

// One observation's capture state: simulated time and the pose (9–10,
// then each arrival step's).
struct Capture {
    time: f64,
    pose: Pose,
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
        // The reset zeroes ctrl: the cart is at rest until the first
        // command acts, at the first arrival step.
        physics.reset(start)?;
        let meters_per_pulse = meters_per_pulse(&RIG);
        // Created with RIG alone (3): the synthetic Arduino starts at the
        // rail's center too, where MuJoCo's cart was just reset.
        let arduino = SyntheticArduino::new(RIG);

        let path = args.out.join("states.csv");
        let file = File::create(&path).with_context(|| format!("creating {}", path.display()))?;
        let mut csv = BufWriter::new(file);
        writeln!(csv, "id,time,cart,pole,left_x,left_y,right_x,right_y,pivot_x,pivot_y,end_x,end_y")?;
        println!(
            "physics: model {}\n         start cart {:+.3} m pole {:.3} rad, control {:?}, seed {} -> {}",
            config.model.display(),
            start.cart,
            start.pole,
            args.control,
            args.seed,
            path.display()
        );
        Ok(Simulation {
            physics,
            control: args.control,
            csv,
            arduino,
            meters_per_pulse,
            names: config.names,
            latency: Latency::new(args.seed),
        })
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

    // 19–21, at the arrival step: the new command. The scripted speed at
    // this step stands in for the trainer's answer; it reaches the
    // firmware as the packet the trainer will send (pulses/s, rounded).
    // The firmware's DRV8825 signals become the cart speed, set as ctrl
    // once, here. Returns that speed.
    fn command(&mut self) -> Result<f64> {
        let speed = self.control.at(self.physics.time());
        // `as` saturates, so an absurd speed becomes the i32 limit, which
        // the firmware clamps to the motor's top rate.
        let command = (speed / self.meters_per_pulse).round() as i32;
        let signals = self.arduino.receive(encode_command(command));
        let velocity = cart_velocity(signals, self.meters_per_pulse);
        self.physics.set_ctrl(velocity)?;
        Ok(velocity)
    }

    // The truth log's row for one observation (synthetic-loop-dia.md,
    // Truth log): its capture time and true pose, beside what the Orin
    // observed in that frame, the pool's newest entry — empty without the
    // Orin (--no-render). `{}` writes a float as the shortest text that
    // reads back to the same number, so identical runs give identical
    // files.
    fn log(&mut self, id: u32, capture: &Capture, newest: Option<&PoolRow>) -> Result<()> {
        let observed = match newest {
            Some(row) => row[1..].iter().map(|v| v.to_string()).collect::<Vec<_>>().join(","),
            None => ",".repeat(7),
        };
        writeln!(self.csv, "{id},{},{},{},{observed}", capture.time, capture.pose.cart, capture.pose.pole)?;
        Ok(())
    }

    // MuJoCo warnings seen during the run.
    fn finish(&mut self) -> Result<usize> {
        self.csv.flush().context("writing states.csv")?;
        Ok(self.physics.warnings())
    }
}

// One step pulse's cart travel: belt travel per revolution ÷ pulses per
// revolution (RIG: 96,000 µm / 6,400 = 15 µm).
fn meters_per_pulse(config: &Config) -> f64 {
    f64::from(config.belt_um_per_rev) / 1e6 / f64::from(config.pulses_per_rev())
}

// 21: cart velocity = STEP rate × meters per pulse, m/s, signed by DIR
// (RIG's top rate: 45,333 × 15 µm = 0.679995 m/s, inside ctrlrange).
fn cart_velocity(signals: Drv8825Signals, meters_per_pulse: f64) -> f64 {
    let speed = f64::from(signals.step_rate) * meters_per_pulse;
    match signals.dir {
        Direction::Positive => speed,
        Direction::Negative => -speed,
    }
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

// One observation per pass (synthetic-loop-dia.md, the loop): at the
// capture, draw the Orin's latency and cache the arrival time; send the
// pose (11); step MuJoCo on the old command up to the arrival step while
// the Orin works (15–16); wait there for the reply (the freeze, 14); then
// set the new command at the arrival step, which is the next capture. The
// run fails when the Orin reports failed or if MuJoCo warned (P3).
// Without the Orin (--no-render), the same clock runs with no
// observations.
async fn run(mut orin: Option<&mut WebSocketStream<TcpStream>>, args: &Args, sim: &mut Simulation) -> Result<()> {
    let reply_timeout = Duration::from_secs(args.reply_timeout);
    let mut capture = sim.capture();
    for id in 0..args.frames {
        let started = Instant::now();
        let latency_ms = sim.latency.next_ms();
        let arrival = capture.time + latency_ms / 1000.0;
        if let Some(orin) = orin.as_deref_mut() {
            send_pose(orin, id, &capture, &sim.names).await?;
        }
        sim.step_to(arrival);
        let newest = match orin.as_deref_mut() {
            Some(orin) => Some(receive_pool(orin, id, reply_timeout).await?),
            None => None,
        };
        // The truth log (the note before 17), then the new command (19–21).
        sim.log(id, &capture, newest.as_ref())?;
        let speed = sim.command()?;

        let next = sim.capture();
        let mut line = format!(
            "observation {id:06}  t {:.3} s  latency {latency_ms:.3} ms, arrival step t {:.3} s (gap {:.0} ms)  cart {:+.3} m  pole {:.3} rad ({:.1}°)  ctrl {speed:+.3} m/s",
            capture.time,
            next.time,
            (next.time - capture.time) * 1000.0,
            capture.pose.cart,
            capture.pose.pole,
            capture.pose.pole.to_degrees()
        );
        if let Some(newest) = newest {
            line += &format!(
                "  orin {:.1} ms  newest L({:.3}, {:.3}) R({:.3}, {:.3}) P({:.3}, {:.3}) E({:.3}, {:.3})",
                started.elapsed().as_secs_f64() * 1000.0,
                newest[1],
                newest[2],
                newest[3],
                newest[4],
                newest[5],
                newest[6],
                newest[7],
                newest[8]
            );
        }
        println!("{line}");
        capture = next;
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

// 14: the Orin's one reply, the pool; returns its newest entry. A reply
// for another id, a pool of the wrong shape, or failed ends the run.
async fn receive_pool(orin: &mut WebSocketStream<TcpStream>, id: u32, timeout: Duration) -> Result<PoolRow> {
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
    Ok(reply.pool[0])
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
