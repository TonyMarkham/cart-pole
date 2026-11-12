mod args;
mod calibration;
mod control;
mod drive;
mod instance;
mod orin;
mod process;
mod schedule;
mod simulation;
#[cfg(test)]
mod tests;
mod timing;
mod trainer_link;
mod walk;
mod window;

// ---------------------------------------------------------------------------------------------- //

use crate::{
    args::Args,
    calibration::{
        calibration_pose::CalibrationPose, calibration_poses, fit_message::FitMessage,
        fitted_reply::FittedReply, measure_message::MeasureMessage, measured_reply::MeasuredReply,
    },
    drive::drive_config,
    instance::Instance,
    orin::{
        await_close, joint_qpos::JointQpos, launch_orin, pool_gaps_argument, receive_message,
        wait_registered,
    },
    process::{CLIENT_EXIT_GRACE, accept, bind, frames_url, start_player, stop},
    simulation::{Simulation, capture::Capture, physics_config, run_values},
    trainer_link::{TrainerLink, next::Next, pool_entry::PoolEntry, run_line},
    walk::{Walk, event::Event},
    window::{Pixels, window_size},
};

use anyhow::{Context, Result, anyhow, bail};
use clap::Parser;
use futures_util::{SinkExt, future::try_join_all};
use physics::Physics;
use std::{
    collections::{HashSet, VecDeque},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use tokio::net::TcpStream;
use tokio_tungstenite::{WebSocketStream, accept_async, tungstenite::Message};

// The most training instances in one run (__documentation__/synthetic-loop-dia.md, Training
// instances), --instance-rows × --instance-columns.
const INSTANCES_MAX: usize = 12;

// How long a launched process (player, synthetic Orin) gets to quit on its
// own after its socket closes before it is killed.

// Everything the orchestrator prints goes to stderr (eprintln!): stdout is
// the trainer link alone (__documentation__/synthetic-loop-dia.md, Trainer link).
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

    // C1–C10: the matrix, before any training instance starts. A failed
    // calibration ends the run here: without the matrix, no pool has metres.
    let homography = args.out.join("homography.json");
    calibrate(&args, ports[0], command, &homography, size, connect_timeout).await?;
    let homography_argument = homography.display().to_string();

    // 2–6 for every instance (__documentation__/synthetic-loop-dia.md, Training instances):
    // MuJoCo and the synthetic Arduino, then the player, listening on the
    // instance's frame socket, then the synthetic Orin, which pre-warms its
    // pool at the gaps it is given and connects to the instance's control
    // socket. The player starts first because only a listening player can be
    // connected to; the Orin retries until it is. Every player and Orin is
    // launched before any connection is awaited, so they start together.
    let mut launched = Vec::with_capacity(count);
    for (k, &(orin_port, unity_port)) in ports.iter().enumerate() {
        let out = instance_folder(&args.out, k)?;
        let seed = args.seed.wrapping_add(k as u64);
        let sim = Simulation::new(&args, &out, seed, k)?;
        // The first episode's schedule, from its seed (the note before 4).
        let walk = Walk::new(sim.schedule(seed));
        let (listener, url) =
            bind(orin_port, &format!("[{k}] control socket (synthetic Orin)")).await?;
        let player = start_player(&args, k, unity_port, size)?;
        let gaps = pool_gaps_argument(&walk.schedule().pool_gaps());
        let orin_child = launch_orin(
            k,
            command,
            &url,
            &frames_url(unity_port),
            &out,
            &["--pool-gaps", &gaps, "--homography", &homography_argument],
        )?;
        launched.push((sim, walk, listener, orin_child, player));
    }
    let mut instances = Vec::with_capacity(count);
    for (k, (sim, walk, listener, mut orin_child, player)) in launched.into_iter().enumerate() {
        let who = format!("synthetic Orin {k}");
        let stream = accept(&listener, &mut orin_child, &who, connect_timeout).await?;
        let orin = accept_async(stream)
            .await
            .with_context(|| format!("{who} WebSocket handshake"))?;
        eprintln!("[{k}] synthetic Orin connected");
        instances.push(Instance {
            k,
            sim,
            walk,
            orin,
            orin_child,
            player,
            next_id: 0,
            held: VecDeque::new(),
            owed: None,
        });
    }

    // 7–8: each Orin connects to its player, and says so.
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
            "[{}] the synthetic Orin has connected to the player",
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

// The calibration (__documentation__/synthetic-loop-dia.md, Calibration, C1–C10): one Orin and
// one player, on instance 0's ports, find the matrix and write it to
// `homography`, then exit, so the training instances start with nothing
// left over from it: no window, no render count, no MuJoCo state.
async fn calibrate(
    args: &Args,
    (orin_port, unity_port): (u16, u16),
    command: &[String],
    homography: &Path,
    size: Option<Pixels>,
    connect_timeout: Duration,
) -> Result<()> {
    // C1–C2: nothing steps, so the poses cost MuJoCo nothing and the pole
    // cannot fall.
    let poses = {
        let mut physics = Physics::load(&physics_config(args))?;
        calibration_poses(&mut physics, args.calibration_stop_gap)?
    };
    let out = instance_folder(&args.out, 0)?;
    let (listener, url) = bind(orin_port, "the calibration's control socket").await?;
    // C3–C4: the player first, listening, then the Orin, which connects to it.
    let mut player = start_player(args, 0, unity_port, size)?;
    let file = homography.display().to_string();
    let mut orin_child = launch_orin(
        0,
        command,
        &url,
        &frames_url(unity_port),
        &out,
        &["--calibrate", &file],
    )?;
    let stream = accept(
        &listener,
        &mut orin_child,
        "calibrating synthetic Orin",
        connect_timeout,
    )
    .await?;
    let mut orin = accept_async(stream)
        .await
        .context("calibrating synthetic Orin WebSocket handshake")?;
    // C5
    wait_registered(&mut orin, &mut orin_child, player.as_mut(), connect_timeout)
        .await
        .context("the calibration")?;
    eprintln!(
        "calibration: the synthetic Orin has connected to the player, measuring {} poses",
        poses.len()
    );
    let names = (args.cart_joint.as_str(), args.pole_joint.as_str());
    let result = measure_and_fit(&mut orin, &poses, names, connect_timeout, homography).await;
    // C10: as 22–25.
    let _ = orin.close(None).await;
    await_close(&mut orin, CLIENT_EXIT_GRACE).await;
    drop(orin);
    stop(orin_child, "calibrating synthetic Orin").await?;
    if let Some(player) = player {
        stop(player, "calibrating player").await?;
    }
    result.context("the calibration")
}

// C6–C9: every pose, then the fit. `not ok` fails the run: the file was not
// written.
async fn measure_and_fit(
    orin: &mut WebSocketStream<TcpStream>,
    poses: &[CalibrationPose],
    (cart_joint, pole_joint): (&str, &str),
    timeout: Duration,
    homography: &Path,
) -> Result<()> {
    for pose in poses {
        let message = MeasureMessage {
            kind: "measure",
            pose: pose.name,
            joints: vec![
                JointQpos {
                    name: cart_joint.to_string(),
                    qpos: pose.cart,
                },
                JointQpos {
                    name: pole_joint.to_string(),
                    qpos: pose.pole,
                },
            ],
            pivot: pose.pivot,
            end: pose.end,
        };
        orin.send(Message::text(serde_json::to_string(&message)?))
            .await
            .with_context(|| format!("sending pose {}", pose.name))?;
        let reply: MeasuredReply = receive_message(orin, timeout, pose.name).await?;
        if reply.kind != "measured" || reply.pose != pose.name {
            bail!("the Orin answered pose {} with {reply:?}", pose.name);
        }
        eprintln!(
            "calibration: {}: {} images from {} renders",
            pose.name, reply.images, reply.renders
        );
    }
    let message = FitMessage { kind: "fit" };
    orin.send(Message::text(serde_json::to_string(&message)?))
        .await
        .context("asking for the fit")?;
    let reply: FittedReply = receive_message(orin, timeout, "the fit").await?;
    if reply.kind != "fitted" {
        bail!("the Orin answered the fit with {reply:?}");
    }
    if !reply.ok {
        bail!(
            "the fit failed: its worst point is {} px from where the matrix puts it, lens number {}",
            reply.worst_px,
            reply.lens_k
        );
    }
    eprintln!(
        "calibration: worst point {:.3} px, lens number {:+.3} -> {}",
        reply.worst_px,
        reply.lens_k,
        homography.display()
    );
    Ok(())
}

// One text message from the Orin, read as `T`. Its going away fails this at
// once, with the reason in its own output on stderr.

// Instance k's ports: --orin-port + 2k and --unity-port + 2k
// (__documentation__/synthetic-loop-dia.md, Training instances). Every port of the group must
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
// (synthetic-arduino-plan.md), and the truth log: one instance's world, the
// `World` its episode's walk moves through.

// One capture state: simulated time, the pose, the twelve sites' true
// positions and the pole's true state, all from one MuJoCo timestep (9–10,
// then each capture's). Held until its step's send, so the sites and pole
// state that go to the trainer (17) show the instant the picture shows.

// The two limit switches (__documentation__/synthetic-loop-dia.md, The rail's ends): each is
// closed when its strike-face gap, the cart's clearance to that base in m,
// is at most the stop gap.

// The random timings' bands from their flags (Timing).

// One training instance (__documentation__/synthetic-loop-dia.md, Training instances): its
// world and its episode's walk, its synthetic Orin and its player, always
// together, the captures waiting for their sends, and the line it owes.

// First the run's values (__documentation__/synthetic-loop-dia.md, Contracts), then one
// round per pass (__documentation__/synthetic-loop-dia.md, Training instances): every
// advanced instance walks at the same time to its next send or a FAULT,
// each waiting at its captures for its own Orin; their owed lines go to the
// trainer in one line, and its answers, for any pending instances, say
// which advance next: on continue, the command goes in flight (the gobble
// rule); on reset, the instance goes back to the start in place (R1–R7)
// with its new schedule; on stop, the loop ends for the shutdown (22).
// Instances not answered stay frozen at their send. The run fails when an
// Orin reports failed, a FAULT is answered with continue, or a MuJoCo
// warned (P3).
async fn run(instances: &mut [Instance], args: &Args) -> Result<()> {
    let reply_timeout = Duration::from_secs(args.reply_timeout);
    let mut link = TrainerLink::new();
    // The run's values, once, before the first pools line (the note before
    // the loop). Every instance loads the same --model and drive figures, so
    // instance 0's are the run's.
    let first = &instances[0].sim;
    link.answer_request(&run_values(&first.physics, first.arduino.config()))?;
    let mut pending = vec![false; instances.len()];
    let mut faulted = vec![false; instances.len()];
    let mut advance: Vec<usize> = (0..instances.len()).collect();
    loop {
        try_join_all(
            instances
                .iter_mut()
                .filter(|instance| advance.contains(&instance.k))
                .map(|instance| async move {
                    let k = instance.k;
                    instance
                        .walk_to_send(reply_timeout)
                        .await
                        .with_context(|| format!("instance {k}"))
                }),
        )
        .await?;
        for &k in &advance {
            pending[k] = true;
            faulted[k] = instances[k].owed.as_ref().is_some_and(|owed| owed.fault);
        }
        let asked = Instant::now();
        let answers = {
            let pools = advance
                .iter()
                .map(|&k| {
                    let owed = instances[k]
                        .owed
                        .as_ref()
                        .ok_or_else(|| anyhow!("instance {k} advanced and owes no line"))?;
                    Ok(PoolEntry {
                        instance: k,
                        id: owed.held.id,
                        time: owed.held.capture.time,
                        pool: &owed.held.pool,
                        sites: &owed.held.capture.sites,
                        pole: &owed.held.capture.pole,
                        fault: owed.fault,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            link.exchange(&pools, &pending, &faulted)?
        };
        let trainer_ms = asked.elapsed().as_secs_f64() * 1000.0;
        for &k in &advance {
            eprintln!("{}  trainer {trainer_ms:.1} ms", instances[k].line());
        }
        if answers.iter().any(|answer| answer.next == Next::Stop) {
            let captures: u32 = instances.iter().map(|instance| instance.next_id).sum();
            eprintln!(
                "the trainer stopped the run after {captures} captures -> {}",
                args.out.display()
            );
            break;
        }
        advance.clear();
        for answer in &answers {
            let instance = &mut instances[answer.instance];
            let owed = instance
                .owed
                .take()
                .ok_or_else(|| anyhow!("instance {} owes no line", answer.instance))?;
            match answer.next {
                // The step's command goes in flight, to land or be gobbled
                // at its motor change, or lost.
                Next::Continue => {
                    instance.walk.command(owed.held.step, answer.command)?;
                    eprintln!(
                        "[{}] step {} command {} pulses/s",
                        instance.k, owed.held.step, answer.command
                    );
                }
                // R1–R7: the instance goes back to the start in place; its
                // walk starts again at the new episode's t = 0, with ids
                // counting on.
                Next::Reset => {
                    let seed = answer
                        .seed
                        .ok_or_else(|| anyhow!("a reset without a seed reached the run"))?;
                    instance
                        .reset(seed, reply_timeout)
                        .await
                        .with_context(|| format!("instance {}", instance.k))?;
                    eprintln!(
                        "[{}] reset: episode {}, seed {seed}",
                        instance.k, instance.sim.episode
                    );
                }
                Next::Stop => unreachable!("a line with a stop ends the loop above"),
            }
            pending[answer.instance] = false;
            faulted[answer.instance] = false;
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

// --no-render: no Orin, no pool and no trainer; the same schedule and walk
// run --frames steps of one instance on the scripted --control, its command
// taken at each send. A FAULT ends the run.
fn run_physics(args: &Args, sim: &mut Simulation) -> Result<()> {
    let mut walk = Walk::new(sim.schedule(args.seed));
    let mut id = 0;
    let mut sends = 0;
    while sends < args.frames {
        match walk.advance(sim)? {
            Event::Capture(step) => {
                let capture = sim.capture();
                sim.log(id, &capture, None)?;
                eprintln!(
                    "step {step} id {id:06}  captured t {:.3} s  cart {:+.3} m  pole {:.3} rad ({:.1}°)",
                    capture.time,
                    capture.pose.cart,
                    capture.pose.pole,
                    capture.pose.pole.to_degrees()
                );
                id += 1;
            }
            Event::Send(step) => {
                let command = sim.scripted_command();
                walk.command(step, command)?;
                eprintln!(
                    "step {step}  sent t {:.3} s  command {command} pulses/s",
                    walk.time_ms() / 1000.0
                );
                sends += 1;
            }
            Event::Fault => {
                eprintln!(
                    "FAULT at t {:.3} s: a limit switch tripped",
                    walk.time_ms() / 1000.0
                );
                break;
            }
        }
    }
    let tally = walk.tally();
    eprintln!(
        "{sends} steps, {id} captures, commands landed {}, gobbled {}, lost {} -> {}",
        tally.landed,
        tally.gobbled,
        tally.lost,
        args.out.display()
    );
    let warnings = sim.finish()?;
    if warnings > 0 {
        bail!("MuJoCo warned {warnings} time(s); the states cannot be trusted");
    }
    Ok(())
}

// The MJCF and its names come only from the command line (the trainer's
// settings.toml [mjcf]).

// The run's values (__documentation__/synthetic-loop-dia.md, Contracts): MuJoCo's, from the
// loaded model, and the top pulse rate, the synthetic Arduino's clamp.

// The synthetic Arduino's Config (3): the drive figures and the jump limit
// from the command line (the trainer's [drive]) and the cart top speed,
// MuJoCo's ctrlrange, in whole µm/s rounded to the nearest: a plain floor
// of the float turns 2.01 m/s into 2,009,999 µm/s. The top pulse rate it
// gives, rounded down, must be at least 1 and fit an i32, the command's
// type.

// The ramp's tick (A7): MuJoCo's timestep in whole µs, rounded to the
// nearest (0.002 s is 2,000). m × the tick, rounded down, must be at least
// 1 pulse/s, or the sent rate could never move and the cart never start.

// One step pulse's cart travel: belt travel per revolution ÷ pulses per
// revolution, the trainer's [drive].

// 21: cart velocity = STEP rate × meters per pulse, m/s, signed by DIR. The
// STEP rate is at most the top pulse rate, rounded down from the cart top
// speed, so the velocity stays inside ctrlrange.

// Each window's size in the grid that fills the screen, when the arguments
// give a screen (__documentation__/synthetic-loop-dia.md, Training instances, The windows).
fn screen_window_size(args: &Args) -> Result<Option<Pixels>> {
    let (Some(x), Some(y)) = (args.screen_width, args.screen_height) else {
        return Ok(None);
    };
    let size = window_size(Pixels { x, y }, args.instance_rows, args.instance_columns)
        .map_err(|error| anyhow!(error))?;
    Ok(Some(size))
}

// Instance k's player, or with --no-launch the note that editor Play mode
// connects itself. Its window has instance k's place in the grid.

// 6: the player, told its Orin's frame socket and, with a grid, its
// borderless window's place and size (__documentation__/synthetic-loop-dia.md, Training
// instances, The windows). kill_on_drop: an early error return drops the
// handle, and the player must not outlive the orchestrator that launched
// it.

// The pool's six pre-warm gaps for the Orin's command line, each as the
// shortest text that reads back to the same f64.

// The synthetic Orin (__documentation__/synthetic-loop-dia.md, 5 and C4): the given program and
// arguments, plus the control socket's address, the frame socket's address,
// the instance's folder and what it runs as, `role`; kill_on_drop as for the
// player.

// A child's stdout is the orchestrator's stderr, and its stdin is closed:
// both of the orchestrator's own pipes are the trainer link, and a child
// must neither write into it nor read from it.

// The loop runs on this workstation, so every socket is on 127.0.0.1.

// Waits for the next connection; a launched process that exits before
// connecting fails the wait at once instead of after the timeout.

// 8: the Orin's {"type": "registered"}, its first message. The Orin or the
// player exiting first fails the wait at once instead of after the
// timeout.

// 11: one capture state to the synthetic Orin.

// 14: the Orin's one reply, the pool, newest first, and the five numbers. A
// reply for another id, a pool of the wrong shape, or failed ends the run.

// R5–R7: tell the Orin to pre-warm its pool afresh at these gaps, and wait
// for its {"type": "reset"}, so the next pose meets the new episode's pool.

// 22: the close is a handshake (RFC 6455 7.1.1). After sending its close
// frame the server reads until the client's close frame answers it, and
// only then ends the TCP connection. A socket dropped with data unread
// (the Orin's keepalive ping, every 20 s) is reset, and the reset discards
// the close frame on the Orin's side before it reads it: the Orin then
// exits 1 (trainer-observations.md, run 1 at 600,000). Whatever else
// arrives first is passed over; an error or the socket's end also ends
// the wait, and so does the timeout, if the Orin never answers.

// A launched process quits on its own when its socket closes; it is
// killed only if it is still running after the grace period.
