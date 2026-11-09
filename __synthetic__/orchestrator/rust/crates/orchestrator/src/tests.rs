use crate::{
    calibration::{fit_message::FitMessage, measure_message::MeasureMessage, stops},
    drive::{cart_velocity, drive_config, meters_per_pulse, ramp_tick},
    orin::{
        await_close, derived_state::DerivedState, joint_qpos::JointQpos, orin_reply::OrinReply,
        reset_message::ResetMessage,
    },
    schedule::{Schedule, nearest_timestep, step_times::StepTimes},
    simulation::limit_switches,
    timing::{Timing, band::Band, bands::Bands, step_picks::StepPicks},
    trainer_link::{
        next::Next, parse_answers, parse_request, pole_message::PoleMessage, pool_entry::PoolEntry,
        run_line, run_values::RunValues, sites_message::SitesMessage,
    },
    walk::{Walk, event::Event, tally::Tally, world::World},
    window::{Pixels, grid_position, window_size},
};

use anyhow::Result;
use arduino::{Config, Direction, Dm542Signals, LimitSwitches};
use futures_util::{SinkExt, StreamExt};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio_tungstenite::{accept_async, connect_async, tungstenite::Message};

fn line(answers: &str) -> String {
    format!(r#"{{"answers": [{answers}]}}"#)
}

// The answers (synthetic-loop-dia.md, Training instances and Contracts,
// 18): per instance, continue with the command, reset with the next
// episode's seed, or stop.
#[test]
fn an_answer_is_continue_reset_or_stop() {
    let all = [true, true, true];
    let answers = parse_answers(
        &line(
            r#"{"instance": 0, "command": -26667, "next": "continue"},
               {"instance": 2, "command": 0, "next": "reset", "seed": 18446744073709551615}"#,
        ),
        &all,
        &[],
    )
    .unwrap();
    assert_eq!(
        (answers[0].instance, answers[0].command, answers[0].next),
        (0, -26_667, Next::Continue)
    );
    assert_eq!(
        (answers[1].instance, answers[1].next, answers[1].seed),
        (2, Next::Reset, Some(u64::MAX))
    );
    let stop = r#"{"instance": 0, "command": 0, "next": "stop"}, {"instance": 1, "command": 0, "next": "stop"}"#;
    assert!(parse_answers(&line(stop), &[true, true], &[]).is_ok());
}

// The run's values (synthetic-loop-dia.md, Contracts): the trainer's first
// line asks for them, and the orchestrator's first line answers with the
// seven values it is given.
#[test]
fn the_request_is_the_run_and_the_answer_holds_its_values() -> Result<()> {
    parse_request(r#"{"request": "run"}"#)?;
    for bad in [
        r#"{"request": "go"}"#,
        r#"{"request": "run", "seed": 1}"#,
        r#"{}"#,
        r#"{"answers": [{"instance": 0, "command": 0, "next": "continue"}]}"#, // an answer before the request
        "run",
    ] {
        assert!(parse_request(bad).is_err(), "{bad} was accepted");
    }
    let values = RunValues {
        cart_top_speed: 0.36,
        top_pulse_rate: 24_000,
        timestep: 0.002,
        pole_mass: 0.402,
        pole_com_distance: 0.155,
        pole_inertia: 0.0222,
        gravity: 9.81,
    };
    assert_eq!(
        run_line(&values)?,
        concat!(
            r#"{"run":{"cart_top_speed":0.36,"top_pulse_rate":24000,"timestep":0.002,"#,
            r#""pole_mass":0.402,"pole_com_distance":0.155,"pole_inertia":0.0222,"gravity":9.81}}"#
        )
    );
    Ok(())
}

// Only pending instances are answered, each once: after instance 1 alone
// was reset, it is pending again while the others still are.
#[test]
fn a_line_may_answer_some_pending_instances() {
    let answer = |k| format!(r#"{{"instance": {k}, "command": 5, "next": "continue"}}"#);
    assert!(parse_answers(&line(&answer(1)), &[true, true, true], &[]).is_ok());
    assert!(
        parse_answers(&line(&answer(1)), &[true, false, true], &[]).is_err(),
        "not pending"
    );
    assert!(
        parse_answers(&line(&answer(3)), &[true, true, true], &[]).is_err(),
        "no such instance"
    );
    assert!(
        parse_answers(
            &line(&format!("{}, {}", answer(0), answer(0))),
            &[true, true],
            &[]
        )
        .is_err(),
        "twice"
    );
    assert!(parse_answers(&line(""), &[true], &[]).is_err(), "no answer");
}

// An instance whose episode ended in FAULT (synthetic-loop-dia.md, The
// rail's ends) is answered with reset or stop; a continue fails the run.
// Instance 1 faulted, instance 0 did not.
#[test]
fn a_fault_is_answered_with_reset_or_stop() {
    let pending = [true, true];
    let faulted = [false, true];
    let answers = |next: &str| {
        line(&format!(
            r#"{{"instance": 0, "command": 5, "next": "continue"}}, {{"instance": 1, "command": 0, "next": {next}}}"#
        ))
    };
    assert!(parse_answers(&answers(r#""reset", "seed": 9"#), &pending, &faulted).is_ok());
    assert!(parse_answers(&answers(r#""continue""#), &pending, &faulted).is_err());
    let stop = r#"{"instance": 0, "command": 0, "next": "stop"}, {"instance": 1, "command": 0, "next": "stop"}"#;
    assert!(parse_answers(&line(stop), &pending, &faulted).is_ok());
}

#[test]
fn a_bad_answer_is_refused() {
    let pending = [true, true];
    for answers in [
        r#"{"instance": 0, "command": 0, "next": "reset"}"#, // a reset needs a seed
        r#"{"instance": 0, "command": 0, "next": "continue", "seed": 1}"#, // only a reset carries one
        r#"{"instance": 0, "command": 0, "next": "pause"}"#,
        r#"{"instance": 0, "command": 0, "continue": true}"#, // the old answer
        r#"{"instance": 0, "command": 0, "next": "reset", "seed": -1}"#,
        r#"{"instance": 0, "command": 2147483648, "next": "continue"}"#,
        r#"{"command": 0, "next": "continue"}"#, // no instance
        // A stop is for the whole run: every pending instance, all stop.
        r#"{"instance": 0, "command": 0, "next": "stop"}"#,
        r#"{"instance": 0, "command": 0, "next": "stop"}, {"instance": 1, "command": 0, "next": "continue"}"#,
    ] {
        assert!(
            parse_answers(&line(answers), &pending, &[]).is_err(),
            "{answers} was accepted"
        );
    }
    assert!(
        parse_answers(r#"{"command": 0, "next": "continue"}"#, &[true], &[]).is_err(),
        "the one-instance line"
    );
}

// SplitMix64's published outputs for seed 0 (the reference
// implementation's first values; e.g. Vigna's splitmix64.c), so the
// generator is checked against the algorithm, not against itself.
#[test]
fn splitmix64_matches_the_reference_for_seed_0() {
    let mut timing = Timing::new(0);
    assert_eq!(timing.next_u64(), 0xE220_A839_7B1D_CDAF);
    assert_eq!(timing.next_u64(), 0x6E78_9E6A_A1B9_65F4);
    assert_eq!(timing.next_u64(), 0x06C4_5D18_8009_454F);
}

// u for each output: its top 53 bits ÷ 2^53.
fn unit(output: u64) -> f64 {
    (output >> 11) as f64 / 9_007_199_254_740_992.0
}

// The random timings' bands today (settings.toml [orchestrator], user
// 2026-11-08).
fn todays_bands() -> Bands {
    Bands {
        frame_ready: Band {
            min: 43.0,
            max: 48.0,
        },
        to_uno: Band {
            min: 20.5,
            max: 27.8,
        },
        change_motor: Band {
            min: 3.6,
            max: 123.3,
        },
        next_frame_ready: Band {
            min: 0.0,
            max: 16.7,
        },
        drop_one_message_in: 3_000,
    }
}

// A pick is min + (max − min)·u: at 43–48 ms the first seed-0 output's u,
// about 0.88331 (0xE220A839 ÷ 2^32), gives 43 + 5 × 0.88331 = 47.417 ms. A
// band of zero width gives its bound.
#[test]
fn a_pick_scales_u_into_its_band() {
    let band = |min, max| Band { min, max };
    assert_eq!(
        Timing::new(0).pick(band(43.0, 48.0)),
        43.0 + 5.0 * unit(0xE220_A839_7B1D_CDAF)
    );
    assert!((Timing::new(0).pick(band(43.0, 48.0)) - 47.417).abs() < 5e-4);
    assert_eq!(Timing::new(0).pick(band(10.0, 10.0)), 10.0);
}

// The drop: 1 + ⌊N·u⌋ coming out 1, lost exactly when u < 1 ÷ N. Seed 0's
// first two u are about 0.88331 and 0.43153 (0x6E789E6A ÷ 2^32): at N = 2,
// 1 + ⌊1.767⌋ = 2, kept, then 1 + ⌊0.863⌋ = 1, lost. At N = 1 every command
// is lost.
#[test]
fn a_command_is_lost_when_its_drop_comes_out_1() {
    let mut timing = Timing::new(0);
    assert!(!timing.pick_lost(2));
    assert!(timing.pick_lost(2));
    let mut timing = Timing::new(0);
    assert!((0..100).all(|_| timing.pick_lost(1)));
}

// The order of the picks (synthetic-loop-dia.md, The schedule): stretch 1,
// stretch 2, stretch 3, the wait, the drop, each the next output of one
// stream. With every band 0–1 ms, each pick is that output's u.
#[test]
fn a_steps_five_picks_come_in_order_from_one_stream() {
    let unit_band = Band { min: 0.0, max: 1.0 };
    let bands = Bands {
        frame_ready: unit_band,
        to_uno: unit_band,
        change_motor: unit_band,
        next_frame_ready: unit_band,
        drop_one_message_in: 2,
    };
    let picks = Timing::new(0).step_picks(&bands);
    let mut outputs = Timing::new(0);
    let u: [f64; 5] = std::array::from_fn(|_| unit(outputs.next_u64()));
    assert_eq!(
        [
            picks.frame_ready,
            picks.to_uno,
            picks.change_motor,
            picks.next_frame_ready
        ],
        [u[0], u[1], u[2], u[3]]
    );
    assert_eq!(picks.lost, u[4] < 0.5);
}

// Today's bands keep the walk's orders at a 2 ms timestep (15.5 ms between
// captures at the shortest, 26.3 ms from the next capture to this send), and
// a band that breaks one is refused.
#[test]
fn bands_that_break_the_walks_orders_are_refused() {
    assert!(todays_bands().check(2.0).is_ok());
    let with = |change: fn(&mut Bands)| {
        let mut bands = todays_bands();
        change(&mut bands);
        bands.check(2.0)
    };
    assert!(
        with(|b| b.to_uno = Band {
            min: 27.8,
            max: 20.5
        })
        .is_err(),
        "inverted"
    );
    assert!(with(|b| b.next_frame_ready.min = -1.0).is_err(), "negative");
    assert!(
        with(|b| b.change_motor.max = f64::NAN).is_err(),
        "not a number"
    );
    assert!(with(|b| b.drop_one_message_in = 0).is_err(), "never 1 in 0");
    assert!(
        with(|b| b.change_motor.min = 1.9).is_err(),
        "a motor change could share its send's timestep"
    );
    // 43 − 41.5 = 1.5 ms: the next capture could come after this send.
    assert!(with(|b| b.next_frame_ready.max = 41.5).is_err());
    // 43 + 20.5 + 0 − 70 = −6.5 ms: captures could fall out of order;
    // 43 + 20.5 + 0 − 61.5 = 2.0 ms exactly is one timestep, allowed.
    assert!(with(|b| b.frame_ready.max = 70.0).is_err());
    assert!(with(|b| b.frame_ready.max = 61.5).is_ok());
}

// Halfway goes later (towards +∞), negative times included.
#[test]
fn a_time_rounds_to_the_nearest_timestep_halfway_later() {
    assert_eq!(nearest_timestep(69.0, 2.0), 35); // 70 ms
    assert_eq!(nearest_timestep(69.25, 2.0), 35); // 70 ms
    assert_eq!(nearest_timestep(68.9, 2.0), 34); // 68 ms
    assert_eq!(nearest_timestep(-33.0, 2.0), -16); // −32 ms
    assert_eq!(nearest_timestep(-33.1, 2.0), -17); // −34 ms
    assert_eq!(nearest_timestep(0.0, 2.0), 0);
}

// One step's picks, for the hand-made schedules below.
fn picks(frame_ready: f64, to_uno: f64, change_motor: f64, next_frame_ready: f64) -> StepPicks {
    StepPicks {
        frame_ready,
        to_uno,
        change_motor,
        next_frame_ready,
        lost: false,
    }
}

// The diagram's worked example (The schedule): picks 45.37, 23.88 and
// 51.27 ms give step 0's picture ready 45.37 → 46 ms, send 69.25 → 70 ms and
// motor change 120.52 → 120 ms; with the wait 8.35 ms and step 1's stretch 1
// 45.40 ms, step 1's capture is 69.25 + 8.35 − 45.40 = 32.20 → 32 ms.
#[test]
fn the_diagrams_worked_example() {
    let plain = picks(45.0, 24.0, 51.0, 8.0);
    let mut all = vec![plain; 5];
    all.push(picks(45.37, 23.88, 51.27, 8.35));
    all.push(picks(45.40, 24.0, 51.0, 8.0));
    let schedule = Schedule::from_picks(&all, 2.0);
    let step = |k| *schedule.step(k).unwrap();
    assert_eq!(
        step(0),
        StepTimes {
            capture: 0,
            picture_ready: 23,
            send: 35,
            motor_change: Some(60),
        }
    );
    assert_eq!(step(1).capture, 16);
}

// A schedule with every time worked by hand, timestep 2 ms. Steps −5 to 1,
// each stretch 1 45, stretch 2 24, stretch 3 51 ms, the wait 8 ms, but step
// −1's wait 9 and step −2's 12, and step 1 lost. Picture ready: step 0's is
// 45 (→ 46 ms, halfway later), step 1's 45 + 24 + 8 = 77 (→ 78); going
// back, step −1's 45 − 24 − 9 = 12, −2's 12 − 24 − 12 = −24, −3's −56, −4's
// −88, −5's −120. Captures, picture ready − 45: 0, 32; −33 → −32 (halfway
// later), −69 → −68, −101 → −100, −133 → −132, −165 → −164. Sends,
// picture ready + 24: step 0's 69 → 70, step 1's 101 → 102. Step 0's motor
// change 69 + 51 = 120.
fn hand_schedule() -> Schedule {
    let plain = picks(45.0, 24.0, 51.0, 8.0);
    let mut all = vec![plain; 7];
    all[3].next_frame_ready = 12.0; // step −2
    all[4].next_frame_ready = 9.0; // step −1
    all[6].lost = true; // step 1
    Schedule::from_picks(&all, 2.0)
}

#[test]
fn every_time_is_the_running_total_rounded() {
    let schedule = hand_schedule();
    let step = |k| *schedule.step(k).unwrap();
    assert_eq!(
        step(0),
        StepTimes {
            capture: 0,
            picture_ready: 23,
            send: 35,
            motor_change: Some(60),
        }
    );
    assert_eq!(
        step(1),
        StepTimes {
            capture: 16,
            picture_ready: 39,
            send: 51,
            motor_change: None,
        }
    );
    let captures: Vec<i64> = (-5..=1).map(|k| step(k).capture * 2).collect();
    assert_eq!(captures, [-164, -132, -100, -68, -32, 0, 32]);
    assert!(schedule.step(-6).is_none() && schedule.step(2).is_none());
    assert_eq!(schedule.last_step(), 1);
}

// The pre-warm (The initial pool): d₁ = 0 − (−32) = 32, d₂ = −32 − (−68) =
// 36, then 32, 32, 32, and d₆ = d₅.
#[test]
fn the_pre_warm_gaps_are_the_pre_warm_steps_captures() {
    assert_eq!(
        hand_schedule().pool_gaps(),
        [32.0, 36.0, 32.0, 32.0, 32.0, 32.0]
    );
}

// The schedule's length follows max_episode_steps: 5 + 500 + 15 = 520
// steps, −5 to 514.
#[test]
fn the_schedule_holds_5_plus_the_episode_plus_15_steps() {
    let schedule = Schedule::make(0, &todays_bands(), 500, 2.0);
    assert_eq!(schedule.last_step(), 514);
    assert!(schedule.step(-5).is_some() && schedule.step(514).is_some());
    assert!(schedule.step(-6).is_none() && schedule.step(515).is_none());
}

// R4 and Determinism: the same seed gives the same schedule; another seed
// another one.
#[test]
fn the_same_seed_gives_the_same_schedule() {
    let make = |seed| Schedule::make(seed, &todays_bands(), 500, 2.0);
    assert_eq!(make(7), make(7));
    assert_ne!(make(7), make(8));
}

// With today's bands, on many seeds, the orders the walk relies on hold
// after rounding (The schedule, Captures stay in order): captures 14 to 50
// ms apart, the next capture before this send, sends in order, each motor
// change after its send; and the pre-warm gaps are captures' gaps.
#[test]
fn todays_bands_keep_the_orders_on_every_seed() {
    for seed in 0..200 {
        let schedule = Schedule::make(seed, &todays_bands(), 500, 2.0);
        let step = |k| *schedule.step(k).unwrap();
        for k in -5..514 {
            let (this, next) = (step(k), step(k + 1));
            let spacing = (next.capture - this.capture) * 2;
            assert!(
                (14..=50).contains(&spacing),
                "seed {seed} step {k}: {spacing} ms"
            );
            assert!(next.capture < this.send, "seed {seed} step {k}");
            assert!(next.send > this.send, "seed {seed} step {k}");
            assert!(this.picture_ready > this.capture && this.send > this.picture_ready);
            if let Some(motor_change) = this.motor_change {
                assert!(motor_change > this.send, "seed {seed} step {k}");
            }
        }
        for gap in schedule.pool_gaps() {
            assert!((14.0..=50.0).contains(&gap), "seed {seed}: gap {gap} ms");
        }
    }
}

// A stand-in for MuJoCo and the synthetic Arduino: it counts timesteps,
// records each tick's landed command, and closes the left switch from
// `trip_at` on, faulting as the synthetic Arduino does.
#[derive(Default)]
struct StandIn {
    now: i64,
    ticks: u32,
    landed: Vec<(i64, i32)>,
    trip_at: Option<i64>,
    fault: bool,
}

impl World for StandIn {
    fn move_on(&mut self) -> Result<LimitSwitches> {
        self.now += 1;
        Ok(LimitSwitches {
            left: self.trip_at.is_some_and(|t| self.now >= t),
            right: false,
        })
    }

    fn tick(&mut self, landed: Option<i32>, switches: LimitSwitches) -> Result<bool> {
        self.ticks += 1;
        if let Some(command) = landed {
            self.landed.push((self.now, command));
        }
        self.fault |= switches.tripped();
        Ok(self.fault)
    }
}

// The walk's schedule, worked by hand at 2 ms (every pick but stretch 3 as
// in `hand_schedule`'s plain steps, so step k's picture ready is 45 + 32k
// ms): captures at 32k ms, timestep 16k; sends at 69 + 32k ms, rounded
// halfway later, timestep 35 + 16k. Stretch 3: step 0's 100 ms (motor
// change 169 → 170 ms, timestep 85), step 1's 20 ms (121 → 122, timestep
// 61), step 2 lost, the rest 51 ms (step 3's 216 → timestep 108). So step 1
// lands at 61 before step 0, which is gobbled at 85.
fn walk_schedule() -> Schedule {
    let plain = picks(45.0, 24.0, 51.0, 8.0);
    let mut all = vec![plain; 5 + 8];
    all[5].change_motor = 100.0; // step 0
    all[6].change_motor = 20.0; // step 1
    all[7].lost = true; // step 2
    Schedule::from_picks(&all, 2.0)
}

// The walk (orchestrator-plan.md, O4, O8): every event on its timestep, in
// order; step 1's command lands at 61, step 0's is gobbled at 85, step 2's
// is lost, step 3's lands at 108.
#[test]
fn the_walk_lands_gobbles_and_loses_commands_on_their_timesteps() -> Result<()> {
    let mut world = StandIn::default();
    let mut walk = Walk::new(walk_schedule());
    let mut events = Vec::new();
    while events.len() < 14 {
        let event = walk.advance(&mut world)?;
        events.push((world.now, event));
        if let Event::Send(step) = event {
            walk.command(step, 100 * (step as i32 + 1))?;
        }
    }
    use Event::{Capture, Send};
    assert_eq!(
        events,
        [
            (0, Capture(0)),
            (16, Capture(1)),
            (32, Capture(2)),
            (35, Send(0)),
            (48, Capture(3)),
            (51, Send(1)),
            (64, Capture(4)),
            (67, Send(2)),
            (80, Capture(5)),
            (83, Send(3)),
            (96, Capture(6)),
            (99, Send(4)),
            (112, Capture(7)),
            (115, Send(5)),
        ]
    );
    // Commands 100 × (step + 1): step 1's 200, then step 3's 400.
    assert_eq!(world.landed, [(61, 200), (108, 400)]);
    assert_eq!(
        walk.tally(),
        Tally {
            landed: 2,
            gobbled: 1,
            lost: 1,
        }
    );
    // One tick a timestep: 115 moved on, 114 ticked; 115's tick comes after
    // its send.
    assert_eq!((world.now, world.ticks), (115, 114));
    assert_eq!(walk.time_ms(), 230.0);
    Ok(())
}

// Two motor changes on one timestep (Events on a shared timestep): in step
// order, both land, and the tick is given the newer step's command. Step
// 0's stretch 3 of 52 ms puts its motor change at 69 + 52 = 121 → 122 ms,
// timestep 61, where step 1's 20 ms puts its own.
#[test]
fn motor_changes_on_one_timestep_land_in_step_order() -> Result<()> {
    let plain = picks(45.0, 24.0, 51.0, 8.0);
    let mut all = vec![plain; 5 + 4];
    all[5].change_motor = 52.0;
    all[6].change_motor = 20.0;
    let mut world = StandIn::default();
    let mut walk = Walk::new(Schedule::from_picks(&all, 2.0));
    while world.now < 61 {
        if let Event::Send(step) = walk.advance(&mut world)? {
            walk.command(step, 100 * (step as i32 + 1))?;
        }
    }
    assert_eq!(world.landed, [(61, 200)]);
    assert_eq!(walk.tally().landed, 2);
    assert_eq!(walk.tally().gobbled, 0);
    Ok(())
}

// FAULT (The rail's ends): the switch closes at timestep 90, after step 3's
// send (83): the walk stops there, owes step 4's line, whose capture (64)
// was taken, and moves no further.
#[test]
fn a_fault_ends_the_walk_and_owes_the_next_steps_line() -> Result<()> {
    let mut world = StandIn {
        trip_at: Some(90),
        ..StandIn::default()
    };
    let mut walk = Walk::new(walk_schedule());
    let mut captured = Vec::new();
    loop {
        match walk.advance(&mut world)? {
            Event::Capture(step) => captured.push(step),
            Event::Send(step) => walk.command(step, 0)?,
            Event::Fault => break,
        }
    }
    assert_eq!(world.now, 90);
    assert_eq!(walk.owed(), 4);
    assert_eq!(captured, [0, 1, 2, 3, 4, 5]);
    assert!(walk.advance(&mut world).is_err());
    Ok(())
}

// An episode that needs a step past its schedule fails, and a command
// without its send is refused.
#[test]
fn the_walk_never_runs_past_its_schedule() -> Result<()> {
    let mut walk = Walk::new(hand_schedule()); // steps −5 to 1
    let mut world = StandIn::default();
    assert!(walk.command(0, 5).is_err(), "no send yet");
    let mut sends = 0;
    let error = loop {
        match walk.advance(&mut world) {
            Ok(Event::Send(step)) => {
                walk.command(step, 5)?;
                sends += 1;
            }
            Ok(_) => {}
            Err(error) => break error,
        }
    };
    assert_eq!(sends, 2);
    assert!(error.to_string().contains("outran its schedule"), "{error}");
    Ok(())
}

// The limit switches (The rail's ends): closed when the strike-face gap is
// at most the stop gap, 5 mm.
#[test]
fn a_switch_closes_within_the_stop_gap() {
    let left = |gap| limit_switches(gap, 0.4, 0.005).left;
    assert!(!left(0.006));
    assert!(left(0.005));
    assert!(left(0.004));
    assert!(left(-0.001), "past the base");
    assert!(limit_switches(0.4, 0.002, 0.005).right);
    assert_eq!(limit_switches(0.4, 0.4, 0.005), LimitSwitches::OPEN);
}

// R5 on the wire: the new pool's six gaps, no seed: the Orin picks nothing.
#[test]
fn a_reset_carries_the_pools_gaps() -> Result<()> {
    let message = ResetMessage {
        kind: "reset",
        gaps: hand_schedule().pool_gaps(),
    };
    assert_eq!(
        serde_json::to_string(&message)?,
        r#"{"type":"reset","gaps":[32.0,36.0,32.0,32.0,32.0,32.0]}"#
    );
    Ok(())
}

// synthetic-arduino-plan.md Done-when 4: ctrl = STEP rate × meters per
// pulse, m/s, signed by DIR. At this test's own Config, 96,000 µm ÷ (200 ×
// 32) = 15 µm a pulse: 26,667 × 15 µm = 0.400005; 56,666 × 15 µm = 0.84999.
#[test]
fn cart_velocity_is_step_rate_times_15_um_signed_by_dir() {
    let config = Config {
        full_steps_per_rev: 200,
        microsteps: 32,
        belt_um_per_rev: 96_000,
        max_speed_um_per_s: 850_000,
        jump_limit: 1,
    };
    let m = meters_per_pulse(&config);
    assert_eq!(m, 15e-6);
    let velocity = |dir, step_rate| cart_velocity(Dm542Signals { dir, step_rate }, m);
    assert!((velocity(Direction::Positive, 26_667) - 0.400_005).abs() < 1e-12);
    assert!((velocity(Direction::Negative, 26_667) + 0.400_005).abs() < 1e-12);
    assert!((velocity(Direction::Negative, 56_666) + 0.849_99).abs() < 1e-12);
    assert_eq!(velocity(Direction::Positive, 0), 0.0);
}

// The synthetic Arduino's Config takes the cart top speed to the nearest
// µm/s (2.01 m/s is 2,010,000, not a float floor's 2,009,999), and its top
// pulse rate is the cart top speed ÷ meters per pulse: 0.36 m/s ÷ 15 µm =
// 24,000. A top pulse rate of 0, or past an i32, is refused.
#[test]
fn the_drive_config_takes_the_cart_top_speed_from_mujoco() -> Result<()> {
    let config = drive_config(200, 32, 96_000, 1, 2.01)?;
    assert_eq!(config.max_speed_um_per_s, 2_010_000);
    let config = drive_config(200, 32, 96_000, 1, 0.36)?;
    assert_eq!(
        (config.max_speed_um_per_s, config.max_rate()),
        (360_000, 24_000)
    );
    assert!(
        drive_config(200, 32, 96_000, 1, 0.000_01).is_err(),
        "rounds to a top pulse rate of 0"
    );
    assert!(
        drive_config(200, 32, 1, 1, 1.0).is_err(),
        "a top pulse rate past an i32"
    );
    assert!(drive_config(200, 32, 96_000, 1, f64::NAN).is_err());
    Ok(())
}

// The ramp's tick is MuJoCo's timestep in µs: 0.002 s is 2,000, where the
// rig's m, 1,210,937 pulses/s², gives 2,421 pulses/s a tick. A jump limit
// whose tick rounds to 0 pulses/s (499 × 2 ms = 0.998) is refused: the
// cart could never move.
#[test]
fn the_ramp_ticks_once_a_timestep() -> Result<()> {
    let config = drive_config(200, 32, 96_000, 1_210_937, 0.72)?;
    let tick = ramp_tick(&config, 0.002)?;
    assert_eq!((tick, config.ramp_limit(tick)), (2_000, 2_421));
    assert!(ramp_tick(&drive_config(200, 32, 96_000, 499, 0.72)?, 0.002).is_err());
    assert!(ramp_tick(&config, 0.0).is_err());
    Ok(())
}

// The windows' grid (synthetic-loop-dia.md, Training instances, The
// windows): 4 columns of 960×752 cells from (0, 0) put instances 0–3 on
// the top row and 4–5 below, from the left.
#[test]
fn the_windows_fill_the_grid_row_by_row() {
    let step = Pixels { x: 960, y: 752 };
    let origin = Pixels { x: 0, y: 0 };
    let places: Vec<(i32, i32)> = (0..6)
        .map(|k| grid_position(k, 4, step, origin))
        .map(|p| (p.x, p.y))
        .collect();
    assert_eq!(
        places,
        [(0, 0), (960, 0), (1920, 0), (2880, 0), (0, 752), (960, 752)]
    );
    assert_eq!(
        grid_position(5, 4, step, Pixels { x: -100, y: 40 }),
        Pixels { x: 860, y: 792 }
    );
}

// Filling the screen (synthetic-loop-dia.md, Training instances, The
// windows): 3840×2112 in 2 rows and 3 columns is windows of 3840 ÷ 3 = 1280 by
// 2112 ÷ 2 = 1056. The last instance's window ends exactly at the screen's
// corner.
#[test]
fn the_windows_fill_the_screen() {
    let screen = Pixels { x: 3840, y: 2112 };
    let size = window_size(screen, 2, 3).unwrap();
    assert_eq!(size, Pixels { x: 1280, y: 1056 });
    let last = grid_position(5, 3, size, Pixels { x: 0, y: 0 });
    assert_eq!((last.x + size.x, last.y + size.y), (3840, 2112));
    // 4 columns × 3 rows, and 1 × 1, env_check's one window.
    assert_eq!(
        window_size(screen, 3, 4).unwrap(),
        Pixels { x: 960, y: 704 }
    );
    assert_eq!(window_size(screen, 1, 1).unwrap(), screen);
    // A division that does not come out whole rounds down: 1000 ÷ 3 = 333.
    assert_eq!(
        window_size(Pixels { x: 1000, y: 1000 }, 3, 3).unwrap(),
        Pixels { x: 333, y: 333 }
    );
}

// A window under 1 pixel leaves nothing to watch.
#[test]
fn a_screen_too_small_for_the_grid_is_refused() {
    assert!(window_size(Pixels { x: 3840, y: 1 }, 2, 3).is_err()); // 1 ÷ 2 = 0 tall
    assert!(window_size(Pixels { x: 3840, y: 2 }, 2, 3).is_ok()); // 1 tall
    assert!(window_size(Pixels { x: 2, y: 2112 }, 2, 3).is_err()); // 2 ÷ 3 = 0 wide
}

#[test]
fn a_window_place_is_x_comma_y() {
    assert_eq!("960,-8".parse::<Pixels>(), Ok(Pixels { x: 960, y: -8 }));
    assert!("960".parse::<Pixels>().is_err());
    assert!("9.5,0".parse::<Pixels>().is_err());
}

// The close handshake (synthetic-loop-dia.md, the note at 22): a ping still
// unread when the server closes must not cost the client the close frame.
// The client pings, as the Orin's keepalive does; the server reads nothing
// until it has sent its close, as at the end of a run. The client must get
// the close frame and then the connection's end, with no reset.
#[tokio::test]
async fn the_close_frame_reaches_a_client_whose_ping_is_unread() -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let url = format!("ws://{}", listener.local_addr()?);
    let client = tokio::spawn(async move {
        let (mut ws, _) = connect_async(url).await?;
        ws.send(Message::Ping(Default::default())).await?;
        let mut closed = false;
        while let Some(message) = ws.next().await {
            closed |= message?.is_close();
        }
        anyhow::Ok(closed)
    });
    let (stream, _) = listener.accept().await?;
    let mut ws = accept_async(stream).await?;
    // Long enough for the ping to arrive and sit unread.
    tokio::time::sleep(Duration::from_millis(200)).await;
    ws.close(None).await?;
    await_close(&mut ws, Duration::from_secs(5)).await;
    drop(ws);
    assert!(client.await??, "the client never received the close frame");
    Ok(())
}

// The calibration's stops: each end stop is the stop gap short of its base,
// and the centre is halfway between them. A cart with 0.440 m to the left base
// and 0.445 m to the right, at a 5 mm gap, stands at -0.435, 0.0025 and 0.440.
#[test]
fn the_stops_stand_the_gap_short_of_each_base() -> Result<()> {
    let (left, centre, right) = stops(0.440, 0.445, 0.005)?;
    assert!((left + 0.435).abs() < 1e-12);
    assert!((right - 0.440).abs() < 1e-12);
    assert!((centre - 0.0025).abs() < 1e-12);
    Ok(())
}

// A stop past a base, or no room at all, would measure a pose the rig cannot
// reach.
#[test]
fn a_stop_gap_outside_the_rail_is_refused() {
    assert!(stops(0.440, 0.445, 0.441).is_err());
    assert!(stops(0.440, 0.445, -0.001).is_err());
    assert!(stops(0.440, 0.445, f64::NAN).is_err());
}

// C6 and C8 on the wire.
#[test]
fn a_measure_carries_the_pose_and_its_true_points() -> Result<()> {
    let measure = MeasureMessage {
        kind: "measure",
        pose: "left up",
        joints: vec![JointQpos {
            name: "cart".to_string(),
            qpos: -0.5,
        }],
        pivot: [-0.5, 0.25],
        end: [-0.5, 0.75],
    };
    assert_eq!(
        serde_json::to_string(&measure)?,
        r#"{"type":"measure","pose":"left up","joints":[{"name":"cart","qpos":-0.5}],"pivot":[-0.5,0.25],"end":[-0.5,0.75]}"#
    );
    assert_eq!(
        serde_json::to_string(&FitMessage { kind: "fit" })?,
        r#"{"type":"fit"}"#
    );
    Ok(())
}

// 14: the Orin's reply carries the five numbers, null where a marker was not
// seen, as a frame with Pivot but no earlier Pivot has no speed.
#[test]
fn a_reply_carries_the_five_numbers() -> Result<()> {
    let reply: OrinReply = serde_json::from_str(
        r#"{"id":7,"pool":[],"failed":false,"state":{"cart":0.1,"cos_theta":-1.0,"sin_theta":0.0,"cart_speed":null,"pole_speed":-2.5}}"#,
    )?;
    assert_eq!(
        reply.state,
        DerivedState {
            cart: Some(0.1),
            cos_theta: Some(-1.0),
            sin_theta: Some(0.0),
            cart_speed: None,
            pole_speed: Some(-2.5),
        }
    );
    Ok(())
}

// A reply without them, or with a number the orchestrator does not know, fails
// the run at the first frame rather than losing the values quietly.
#[test]
fn a_reply_with_the_wrong_numbers_is_refused() {
    let without = serde_json::from_str::<OrinReply>(r#"{"id":7,"pool":[],"failed":false}"#);
    assert!(without.is_err());
    let unknown = serde_json::from_str::<OrinReply>(
        r#"{"id":7,"pool":[],"failed":false,"state":{"angle":1.0}}"#,
    );
    assert!(unknown.is_err());
}

// The log line prints every number, and says which were not seen.
#[test]
fn the_five_numbers_print_with_their_units() {
    let state = DerivedState {
        cart: Some(0.1),
        cos_theta: Some(-1.0),
        sin_theta: None,
        cart_speed: None,
        pole_speed: Some(-2.5),
    };
    assert_eq!(
        state.to_string(),
        "cart +0.100 m, cos -1.000, sin none, cart speed none m/s, pole speed -2.500 rad/s"
    );
}

// 17: a pool entry carries its capture's MuJoCo time, its twelve sites
// beside the pool, each a world [x, y, z] in meters, keyed by role, and its
// pole state (synthetic-loop-dia.md, Contracts, the sites and the pole).
// The pole's numbers are arbitrary: this checks the message's shape.
#[test]
fn a_pool_entry_carries_its_captures_time_sites_and_pole() -> Result<()> {
    let sites = SitesMessage {
        left: [-0.51, -0.0765, 0.025],
        right: [0.495, -0.07665, 0.025],
        pivot: [-0.01, -0.132, 0.082],
        end: [-0.01, -0.132, -0.368],
        target: [-0.01, -0.132, 0.532],
        dynamic_target: [-0.01, -0.132, 0.532],
        cart: [-0.01, 0.0, 0.05],
        cart_target: [-0.01, 0.0, 0.05],
        left_rail: [-0.485, -0.045, 0.05],
        left_cart: [-0.045, -0.045, 0.05],
        right_rail: [0.47, -0.045, 0.05],
        right_cart: [0.025, -0.045, 0.05],
    };
    let pool = [[0.0, 0.1, 0.2, 0.3, 0.4]];
    let pole = PoleMessage {
        speed: -1.5,
        angle: 7.0,
        com: [0.1, 0.2, 0.3],
        hinge: [0.4, 0.5, 0.6],
    };
    let entry = PoolEntry {
        instance: 2,
        id: 7,
        time: 0.014,
        pool: &pool,
        sites: &sites,
        pole: &pole,
        fault: false,
    };
    assert_eq!(
        serde_json::to_string(&entry)?,
        concat!(
            r#"{"instance":2,"id":7,"time":0.014,"pool":[[0.0,0.1,0.2,0.3,0.4]],"sites":{"#,
            r#""left":[-0.51,-0.0765,0.025],"right":[0.495,-0.07665,0.025],"pivot":[-0.01,-0.132,0.082],"#,
            r#""end":[-0.01,-0.132,-0.368],"target":[-0.01,-0.132,0.532],"dynamic_target":[-0.01,-0.132,0.532],"#,
            r#""cart":[-0.01,0.0,0.05],"cart_target":[-0.01,0.0,0.05],"left_rail":[-0.485,-0.045,0.05],"#,
            r#""left_cart":[-0.045,-0.045,0.05],"right_rail":[0.47,-0.045,0.05],"right_cart":[0.025,-0.045,0.05]},"#,
            r#""pole":{"speed":-1.5,"angle":7.0,"com":[0.1,0.2,0.3],"hinge":[0.4,0.5,0.6]},"fault":false}"#
        )
    );
    Ok(())
}
