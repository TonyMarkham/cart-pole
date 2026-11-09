pub(crate) mod capture;

// ---------------------------------------------------------------------------------------------- //

use crate::{
    args::Args,
    control::Control,
    drive::{cart_velocity, drive_config, meters_per_pulse, ramp_tick},
    orin::orin_reply::PoolRow,
    schedule::Schedule,
    simulation::capture::Capture,
    timing::{band::Band, bands::Bands},
    trainer_link::run_values::RunValues,
    walk::world::World,
};

use anyhow::{Context, Result, bail};
use arduino::{Config, Dm542Signals, LimitSwitches, SyntheticArduino, encode_command};
use physics::{Names, Physics, PhysicsConfig, Pose};
use std::{
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

pub(crate) struct Simulation {
    pub(crate) physics: Physics,
    pub(crate) control: Control,
    pub(crate) csv: BufWriter<File>,
    // Ticked once every timestep, given the command that landed on it
    // (19–21): the ramp and the limit switches.
    pub(crate) arduino: SyntheticArduino,
    pub(crate) meters_per_pulse: f64,
    pub(crate) names: Names,
    // The random timings' bands, checked at load (Timing).
    pub(crate) bands: Bands,
    pub(crate) max_episode_steps: u32,
    // One timestep, ms: the grid every scheduled time rounds to.
    pub(crate) timestep_ms: f64,
    // A limit switch is closed when its strike-face gap is at most this, m
    // (The rail's ends).
    pub(crate) stop_gap: f64,
    // The start pose every episode begins at (2, and R2 at each reset).
    pub(crate) start: Pose,
    // 0 for the instance's first episode, one more at each reset (R1).
    pub(crate) episode: u32,
}

impl Simulation {
    pub(crate) fn new(args: &Args, out: &Path, seed: u64, k: usize) -> Result<Simulation> {
        // The cart always starts at the rail's center, 50% (2): only the
        // pole's angle is the run's to choose.
        let start = Pose {
            cart: 0.0,
            pole: args.start,
        };
        let physics_config = physics_config(args);
        let mut physics = Physics::load(&physics_config)?;
        // The reset zeroes ctrl: the cart is at rest until the first
        // command lands.
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
        // reset, and at rest, as MuJoCo's ctrl 0, not in FAULT.
        let arduino = SyntheticArduino::new(config, tick);
        // The tick is the timestep in whole µs, so this is exact: 2.0 ms.
        let timestep_ms = f64::from(tick) / 1000.0;
        let bands = bands(args);
        bands.check(timestep_ms)?;
        if !(args.calibration_stop_gap.is_finite() && args.calibration_stop_gap >= 0.0) {
            bail!(
                "--calibration-stop-gap is a gap of 0 m or more, got {}",
                args.calibration_stop_gap
            );
        }

        let path = out.join("states.csv");
        let file = File::create(&path).with_context(|| format!("creating {}", path.display()))?;
        let mut csv = BufWriter::new(file);
        writeln!(csv, "episode,id,time,cart,pole,pivot_x,pivot_z,end_x,end_z")?;
        let driver = match args.no_render {
            true => format!("--control {:?}", args.control),
            false => "the trainer's commands".to_string(),
        };
        let band = |b: Band| format!("RND({}, {})", b.min, b.max);
        eprintln!(
            "[{k}] physics: model {}\n         start cart {:+.3} m pole {:.3} rad, driven by {driver}, seed {seed}, stretches {}, {} and {} ms, wait {} ms, 1 in {} lost, {} steps an episode, switches at {} m, {} m a pulse, top pulse rate {} pulses/s, jump limit {} pulses/s² ({} a {} µs tick) -> {}",
            physics_config.model.display(),
            start.cart,
            start.pole,
            band(bands.frame_ready),
            band(bands.to_uno),
            band(bands.change_motor),
            band(bands.next_frame_ready),
            bands.drop_one_message_in,
            args.max_episode_steps,
            args.calibration_stop_gap,
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
            bands,
            max_episode_steps: args.max_episode_steps,
            timestep_ms,
            stop_gap: args.calibration_stop_gap,
            start,
            episode: 0,
        })
    }

    // The episode's schedule from its seed (Timing, The schedule).
    pub(crate) fn schedule(&self, seed: u64) -> Schedule {
        Schedule::make(seed, &self.bands, self.max_episode_steps, self.timestep_ms)
    }

    // R1–R3 (synthetic-loop-dia.md, Reset in place): the next episode, back
    // at the start pose, t = 0, ctrl 0; the synthetic Arduino back to rest
    // and out of FAULT. The schedule (R4) is the caller's, from the seed.
    pub(crate) fn reset(&mut self) -> Result<()> {
        self.physics.reset(self.start)?;
        self.arduino.reset();
        self.episode += 1;
        Ok(())
    }

    pub(crate) fn capture(&mut self) -> Capture {
        Capture {
            time: self.physics.time(),
            pose: self.physics.pose(),
            sites: self.physics.sites().into(),
            pole: self.physics.pole().into(),
        }
    }

    // --no-render's stand-in for the trainer's command: the scripted speed
    // at this time, as the packet the trainer would send (pulses/s,
    // rounded). `as` saturates, so an absurd speed becomes the i32 limit,
    // which the firmware clamps to the motor's top rate.
    pub(crate) fn scripted_command(&self) -> i32 {
        let speed = self.control.at(self.physics.time());
        (speed / self.meters_per_pulse).round() as i32
    }

    // 21: ctrl = STEP rate × metres per pulse, signed by DIR.
    pub(crate) fn drive(&mut self, signals: Dm542Signals) -> Result<f64> {
        let velocity = cart_velocity(signals, self.meters_per_pulse);
        self.physics.set_ctrl(velocity)?;
        Ok(velocity)
    }

    // The truth log's row for one capture (synthetic-loop-dia.md, Truth
    // log): its episode, capture time and true pose, beside what the Orin
    // observed in that picture, the pool's newest entry — empty without the
    // Orin (--no-render). `{}` writes a float as the shortest text that
    // reads back to the same number, so identical runs give identical
    // files.
    pub(crate) fn log(
        &mut self,
        id: u32,
        capture: &Capture,
        newest: Option<&PoolRow>,
    ) -> Result<()> {
        let observed = match newest {
            Some(row) => row[1..]
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(","),
            None => ",".repeat(3),
        };
        writeln!(
            self.csv,
            "{},{id},{},{},{},{observed}",
            self.episode, capture.time, capture.pose.cart, capture.pose.pole
        )?;
        Ok(())
    }

    // MuJoCo warnings seen during the run.
    pub(crate) fn finish(&mut self) -> Result<usize> {
        self.csv.flush().context("writing states.csv")?;
        Ok(self.physics.warnings())
    }
}

impl World for Simulation {
    // 15–16: one timestep, then the switches from the strike faces there.
    fn move_on(&mut self) -> Result<LimitSwitches> {
        self.physics.step(1);
        let sites = self.physics.sites();
        Ok(limit_switches(
            sites.left_cart[0] - sites.left_rail[0],
            sites.right_rail[0] - sites.right_cart[0],
            self.stop_gap,
        ))
    }

    // 19–21: the landed command, if any, then the tick; ctrl follows.
    fn tick(&mut self, landed: Option<i32>, switches: LimitSwitches) -> Result<bool> {
        if let Some(command) = landed {
            self.arduino.receive(encode_command(command));
        }
        let answer = self.arduino.tick(switches);
        self.drive(answer.signals)?;
        Ok(answer.fault)
    }
}

pub(crate) fn limit_switches(left_gap: f64, right_gap: f64, stop_gap: f64) -> LimitSwitches {
    LimitSwitches {
        left: left_gap <= stop_gap,
        right: right_gap <= stop_gap,
    }
}

pub(crate) fn bands(args: &Args) -> Bands {
    let band = |min, max| Band { min, max };
    Bands {
        frame_ready: band(args.frame_ready_min, args.frame_ready_max),
        to_uno: band(args.to_uno_min, args.to_uno_max),
        change_motor: band(args.change_motor_min, args.change_motor_max),
        next_frame_ready: band(args.next_frame_ready_min, args.next_frame_ready_max),
        drop_one_message_in: args.drop_one_message_in,
    }
}

pub(crate) fn physics_config(args: &Args) -> PhysicsConfig {
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

pub(crate) fn run_values(physics: &Physics, config: &Config) -> RunValues {
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
