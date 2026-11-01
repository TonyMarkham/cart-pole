use crate::{
    await_close, cart_velocity, drive_config, meters_per_pulse,
    pole_message::PoleMessage,
    ramp_tick,
    registered::ResetMessage,
    sites_message::SitesMessage,
    timing::Timing,
    trainer_link::{Next, PoolEntry, RunValues, parse_answers, parse_request, run_line},
    window::{Pixels, grid_position, window_size},
};
use anyhow::Result;
use arduino::{Config, Direction, Dm542Signals};
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
    assert!(parse_answers(&line(stop), &[true, true]).is_ok());
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
    assert!(parse_answers(&line(&answer(1)), &[true, true, true]).is_ok());
    assert!(
        parse_answers(&line(&answer(1)), &[true, false, true]).is_err(),
        "not pending"
    );
    assert!(
        parse_answers(&line(&answer(3)), &[true, true, true]).is_err(),
        "no such instance"
    );
    assert!(
        parse_answers(
            &line(&format!("{}, {}", answer(0), answer(0))),
            &[true, true]
        )
        .is_err(),
        "twice"
    );
    assert!(parse_answers(&line(""), &[true]).is_err(), "no answer");
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
            parse_answers(&line(answers), &pending).is_err(),
            "{answers} was accepted"
        );
    }
    assert!(
        parse_answers(r#"{"command": 0, "next": "continue"}"#, &[true]).is_err(),
        "the one-instance line"
    );
}

// SplitMix64's published outputs for seed 0 (the reference
// implementation's first values; e.g. Vigna's splitmix64.c), so the
// generator is checked against the algorithm, not against itself.
#[test]
fn splitmix64_matches_the_reference_for_seed_0() {
    let mut timing = Timing::new(0, (8.0, 15.0), (20.0, 30.0)).unwrap();
    assert_eq!(timing.next_u64(), 0xE220_A839_7B1D_CDAF);
    assert_eq!(timing.next_u64(), 0x6E78_9E6A_A1B9_65F4);
    assert_eq!(timing.next_u64(), 0x06C4_5D18_8009_454F);
}

#[test]
fn latencies_lie_in_the_band() {
    for (min, max) in [(8.0, 15.0), (2.5, 4.0)] {
        let mut timing = Timing::new(42, (min, max), (20.0, 30.0)).unwrap();
        for _ in 0..10_000 {
            let ms = timing.next_latency_ms();
            assert!(
                (min..max).contains(&ms),
                "{ms} ms is outside [{min}, {max})"
            );
        }
    }
}

// A gap is 1000 ÷ a rate in [min, max): at 20–30 Hz, above 33.33 ms and at
// most 50 ms.
#[test]
fn gaps_follow_the_command_rate() {
    let mut timing = Timing::new(42, (8.0, 15.0), (20.0, 30.0)).unwrap();
    for _ in 0..10_000 {
        let ms = timing.next_gap_ms();
        assert!(
            1000.0 / 30.0 < ms && ms <= 50.0,
            "{ms} ms is outside (33.33, 50]"
        );
    }
}

// The band only scales u, the first seed-0 output's top 53 bits / 2^53:
// at 8-15 ms that is 8 + 7u = 14.183 ms; at 20-30 Hz, 20 + 10u = 28.833 Hz,
// a gap of 34.682 ms. A band of zero width gives its bound.
#[test]
fn the_band_scales_the_same_draw() {
    let timing = |latency, rate| Timing::new(0, latency, rate).unwrap();
    let u = (0xE220_A839_7B1D_CDAF_u64 >> 11) as f64 / 9_007_199_254_740_992.0;
    assert_eq!(
        timing((8.0, 15.0), (20.0, 30.0)).next_latency_ms(),
        8.0 + 7.0 * u
    );
    assert!((timing((8.0, 15.0), (20.0, 30.0)).next_latency_ms() - 14.183).abs() < 5e-4);
    assert_eq!(timing((10.0, 10.0), (20.0, 30.0)).next_latency_ms(), 10.0);
    assert_eq!(
        timing((8.0, 15.0), (20.0, 30.0)).next_gap_ms(),
        1000.0 / (20.0 + 10.0 * u)
    );
    assert!((timing((8.0, 15.0), (20.0, 30.0)).next_gap_ms() - 34.682).abs() < 5e-4);
    assert_eq!(timing((8.0, 15.0), (25.0, 25.0)).next_gap_ms(), 40.0);
}

// Each capture's two draws come from one stream, so the latency and the gap
// are different numbers: two generators seeded alike would tie them.
#[test]
fn the_latency_and_the_gap_are_drawn_in_turn_from_one_stream() {
    let mut timing = Timing::new(0, (0.0, 1.0), (1000.0, 2000.0)).unwrap();
    let mut outputs = Timing::new(0, (8.0, 15.0), (20.0, 30.0)).unwrap();
    let unit = |output: u64| (output >> 11) as f64 / 9_007_199_254_740_992.0;
    assert_eq!(timing.next_latency_ms(), unit(outputs.next_u64()));
    assert_eq!(
        timing.next_gap_ms(),
        1000.0 / (1000.0 + 1000.0 * unit(outputs.next_u64()))
    );
}

#[test]
fn a_band_outside_its_limits_is_refused() {
    let rate = (20.0, 30.0);
    assert!(Timing::new(0, (15.0, 8.0), rate).is_err());
    assert!(Timing::new(0, (-1.0, 8.0), rate).is_err());
    assert!(Timing::new(0, (8.0, f64::INFINITY), rate).is_err());
    assert!(Timing::new(0, (f64::NAN, 8.0), rate).is_err());
    let latency = (8.0, 15.0);
    assert!(Timing::new(0, latency, (30.0, 20.0)).is_err());
    assert!(
        Timing::new(0, latency, (0.0, 30.0)).is_err(),
        "a rate of 0 is no gap at all"
    );
    assert!(Timing::new(0, latency, (20.0, f64::INFINITY)).is_err());
    assert!(Timing::new(0, latency, (f64::NAN, 30.0)).is_err());
}

// The shortest gap must leave a command at least one step: at 8–15 ms and
// a 2 ms timestep, 1000 ÷ the top rate ≥ 17 ms, a rate of at most 58.82 Hz.
#[test]
fn a_rate_too_fast_for_the_latency_is_refused() {
    let timing = |top| Timing::new(0, (8.0, 15.0), (20.0, top)).unwrap();
    assert!(timing(30.0).check_commands_act(0.002).is_ok());
    assert!(timing(58.8).check_commands_act(0.002).is_ok());
    assert!(timing(59.0).check_commands_act(0.002).is_err());
}

#[test]
fn the_same_seed_gives_the_same_draws() {
    let mut a = Timing::new(7, (8.0, 15.0), (20.0, 30.0)).unwrap();
    let mut b = Timing::new(7, (8.0, 15.0), (20.0, 30.0)).unwrap();
    for _ in 0..100 {
        assert_eq!(a.next_latency_ms().to_bits(), b.next_latency_ms().to_bits());
        assert_eq!(a.next_gap_ms().to_bits(), b.next_gap_ms().to_bits());
    }
    assert_eq!(
        a.next_pool_gaps().map(f64::to_bits),
        b.next_pool_gaps().map(f64::to_bits)
    );
}

// R4: a reseeded generator keeps its bands and repeats a fresh one, so an
// episode after a reset draws what a fresh launch with its seed would.
#[test]
fn a_reseed_repeats_a_fresh_start() {
    let mut used = Timing::new(1, (8.0, 15.0), (20.0, 30.0)).unwrap();
    used.next_pool_gaps();
    used.next_latency_ms();
    let mut reseeded = used.reseeded(7);
    let mut fresh = Timing::new(7, (8.0, 15.0), (20.0, 30.0)).unwrap();
    assert_eq!(
        reseeded.next_pool_gaps().map(f64::to_bits),
        fresh.next_pool_gaps().map(f64::to_bits)
    );
    assert_eq!(
        reseeded.next_latency_ms().to_bits(),
        fresh.next_latency_ms().to_bits()
    );
}

// R5 on the wire: the new pool's six gaps, no seed: the Orin draws nothing.
#[test]
fn a_reset_carries_the_pools_gaps() -> Result<()> {
    let message = ResetMessage {
        kind: "reset",
        gaps: [40.0, 41.0, 42.0, 43.0, 44.0, 45.5],
    };
    assert_eq!(
        serde_json::to_string(&message)?,
        r#"{"type":"reset","gaps":[40.0,41.0,42.0,43.0,44.0,45.5]}"#
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
    let pool = [[0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8]];
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
    };
    assert_eq!(
        serde_json::to_string(&entry)?,
        concat!(
            r#"{"instance":2,"id":7,"time":0.014,"pool":[[0.0,0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8]],"sites":{"#,
            r#""left":[-0.51,-0.0765,0.025],"right":[0.495,-0.07665,0.025],"pivot":[-0.01,-0.132,0.082],"#,
            r#""end":[-0.01,-0.132,-0.368],"target":[-0.01,-0.132,0.532],"dynamic_target":[-0.01,-0.132,0.532],"#,
            r#""cart":[-0.01,0.0,0.05],"cart_target":[-0.01,0.0,0.05],"left_rail":[-0.485,-0.045,0.05],"#,
            r#""left_cart":[-0.045,-0.045,0.05],"right_rail":[0.47,-0.045,0.05],"right_cart":[0.025,-0.045,0.05]},"#,
            r#""pole":{"speed":-1.5,"angle":7.0,"com":[0.1,0.2,0.3],"hinge":[0.4,0.5,0.6]}}"#
        )
    );
    Ok(())
}
