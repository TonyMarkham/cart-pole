//! The `/run` page's routes: Run and STOP, passed on to the brain, and the
//! brain's status (`real-loop-dia.md`, *The run and idle states*; *Contracts*,
//! the brain's requests). The brain alone sends commands during a run; STOP is
//! the one time this server sends the board a command, the command 0.

use crate::{
    arduino::{arduino_state::ArduinoState, exchange},
    constants::{
        BRAIN_IDLE_PATH, BRAIN_RUN_PATH, BRAIN_STATUS_PATH, COMMAND_ZERO, JSON_CONTENT_TYPE,
        STATE_ANSWER_MAX, STATE_REQUEST,
    },
    service::{ask, forward_to, refused},
    settings::{arduino_settings::ArduinoSettings, run_settings::RunSettings},
};

use axum::{
    Json,
    extract::State,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use serde_json::{Value, json};
use std::{io, time::Duration};
use tokio::{net::UdpSocket, time};

/// Run: the brain starts a run, or says why not.
pub(crate) async fn start(State(arduino): State<ArduinoState>) -> Response {
    forward_to(
        &arduino.run.brain,
        arduino.run.timeout_ms,
        true,
        BRAIN_RUN_PATH.to_string(),
        "the brain",
    )
    .await
}

/// STOP: first the brain is asked for idle, so no command follows; only then
/// is the board sent the command 0, several times, since a datagram can be
/// lost. The zeros go out whether or not the brain answered, so a brain that
/// has crashed can't keep the cart moving (`real-loop-dia.md`, *The run and
/// idle states*, two ways out of run). The answer is the brain's, with
/// `zeros_sent` added, or the webserver's own if the brain gave none.
pub(crate) async fn stop(State(arduino): State<ArduinoState>) -> Response {
    let idled = ask(
        &arduino.run.brain,
        arduino.run.timeout_ms,
        true,
        BRAIN_IDLE_PATH.to_string(),
    )
    .await;
    let zeros = send_zeros(&arduino.settings).await;

    let mut answer = match &idled {
        Ok((_, body)) => serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({})),
        Err(reason) => json!({
            "state": "unknown",
            "message": format!("the brain did not answer, so it may not be running: {reason}"),
        }),
    };
    answer["zeros_sent"] = json!(zeros.is_ok());
    if let Err(reason) = &zeros {
        answer["zeros_error"] = json!(reason.to_string());
    }
    let status = match (&idled, &zeros) {
        (Ok((status, _)), Ok(())) => *status,
        _ => StatusCode::BAD_GATEWAY,
    };
    (status, Json(answer)).into_response()
}

/// The brain's state, the last reason, the run's folder and its frames, and,
/// while the brain is idle, the board's answer to `state`: `/run` enables Run
/// only when that answer is `"calibrated"` (`real-open-issues.md`, issue 7,
/// step 2). During a run the board is not asked: the brain alone talks to it
/// then, and the board answers whoever sent the latest datagram.
pub(crate) async fn status(State(arduino): State<ArduinoState>) -> Response {
    let (status, body) = match ask(
        &arduino.run.brain,
        arduino.run.timeout_ms,
        false,
        BRAIN_STATUS_PATH.to_string(),
    )
    .await
    {
        Ok(answer) => answer,
        Err(reason) => {
            return refused(
                StatusCode::BAD_GATEWAY,
                &format!("the brain did not answer, so it may not be running: {reason}"),
            );
        }
    };
    let Ok(brain) = serde_json::from_slice::<Value>(&body) else {
        return (status, [(header::CONTENT_TYPE, JSON_CONTENT_TYPE)], body).into_response();
    };
    let board = if brain["state"].as_str() == Some("idle") {
        Some(board_state(&arduino).await)
    } else {
        None
    };
    (status, Json(with_board(brain, board))).into_response()
}

/// The brain's status with the board's added: `board`, the board's own answer
/// to `state`, or null with `board_error` saying why there is none. `None`
/// means the board was not asked, during a run.
pub(crate) fn with_board(mut brain: Value, board: Option<Result<String, String>>) -> Value {
    brain["board"] = Value::Null;
    match board {
        None => {}
        Some(Ok(text)) => match serde_json::from_str::<Value>(&text) {
            Ok(answer) => brain["board"] = answer,
            Err(_) => {
                brain["board_error"] =
                    json!(format!("The board's answer was not its JSON: {text}"));
            }
        },
        Some(Err(reason)) => brain["board_error"] = json!(reason),
    }
    brain
}

/// The board's answer to `state`, or why there is none, in words for the
/// page. It waits its turn with the `/arduino` readings, and during a
/// calibration move it sends nothing: the board throws datagrams away then.
async fn board_state(arduino: &ArduinoState) -> Result<String, String> {
    let _one_at_a_time = arduino.reading.lock().await;
    if arduino.moving.available_permits() == 0 {
        return Err(
            "A calibration move is under way: the board hears nothing until the cart stops."
                .to_string(),
        );
    }
    exchange(&arduino.settings, STATE_REQUEST, STATE_ANSWER_MAX)
        .await
        .map_err(|reason| {
            format!("The board did not answer: {reason}. It is off, or the message was lost.")
        })
}

/// Whether the brain reports a run. A brain that does not answer is not
/// running, and so not in a run: the camera jobs go ahead.
pub(crate) async fn is_running(run: &RunSettings) -> bool {
    match ask(
        &run.brain,
        run.timeout_ms,
        false,
        BRAIN_STATUS_PATH.to_string(),
    )
    .await
    {
        Ok((_, body)) => serde_json::from_slice::<Value>(&body)
            .is_ok_and(|answer| answer["state"].as_str() == Some("run")),
        Err(_) => false,
    }
}

/// Sends the command 0 `stop_sends` times, `stop_gap_ms` apart, as the
/// calibration's Stop is sent: a datagram can be lost on the WiFi and nothing
/// resends it. The board ramps the cart to rest on the first to arrive, and
/// the rest ask for what it is already doing.
async fn send_zeros(arduino: &ArduinoSettings) -> io::Result<()> {
    let socket = UdpSocket::bind(("0.0.0.0", 0)).await?;
    socket
        .connect((arduino.address.as_str(), arduino.port))
        .await?;
    for send in 0..arduino.stop_sends {
        if send > 0 {
            time::sleep(Duration::from_millis(arduino.stop_gap_ms)).await;
        }
        socket.send(&COMMAND_ZERO).await?;
    }
    Ok(())
}
