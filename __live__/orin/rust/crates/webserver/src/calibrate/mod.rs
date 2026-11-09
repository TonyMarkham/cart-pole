//! The `/calibrate` page's moves: each sends the board one datagram and waits
//! for its answer, which comes only once the cart has stopped. What the answers
//! say about the rail is kept in `RailState`, for the measurements.

pub(crate) mod centre_query;
pub(crate) mod rail_state;
pub(crate) mod stop;

// ---------------------------------------------------------------------------------------------- //

use crate::{
    arduino::arduino_state::ArduinoState,
    calibrate::{centre_query::CentreQuery, stop::Stop},
    constants::{
        CENTRE_REQUEST, IDLER_REQUEST, MILLIS_PER_SECOND, MOTOR_REQUEST, MOVE_ANSWER_MAX,
        STOP_REQUEST,
    },
    run::is_running,
    service::is_measuring,
    settings::arduino_settings::ArduinoSettings,
};

use axum::{
    Json,
    extract::{Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use serde_json::{Value, json};
use std::{io, time::Duration};
use tokio::{net::UdpSocket, time};

/// A seek to the idler's end: the board drives the cart there until its switch
/// trips.
pub(crate) async fn idler(State(arduino): State<ArduinoState>) -> Response {
    let wait_ms = arduino.settings.move_timeout_ms;
    travel(arduino, IDLER_REQUEST.to_string(), wait_ms, Stop::Idler).await
}

/// A seek to the motor's end: the board drives the cart there until its switch
/// trips.
pub(crate) async fn motor(State(arduino): State<ArduinoState>) -> Response {
    let wait_ms = arduino.settings.move_timeout_ms;
    travel(arduino, MOTOR_REQUEST.to_string(), wait_ms, Stop::Motor).await
}

/// A move to the centre: with no speed the board uses its `seek_speed`. A speed
/// lengthens the wait by half the rail at that speed, because the board answers
/// only once the cart has stopped.
pub(crate) async fn centre(
    State(arduino): State<ArduinoState>,
    Query(query): Query<CentreQuery>,
) -> Response {
    let settings = &arduino.settings;
    let crossing_ms = query
        .speed
        .as_deref()
        .and_then(|speed| speed.parse::<u64>().ok())
        .filter(|&speed| speed > 0)
        .map_or(0, |speed| {
            settings.half_span_pulses.saturating_mul(MILLIS_PER_SECOND) / speed
        });
    let wait_ms = settings.move_timeout_ms.saturating_add(crossing_ms);
    let request = match &query.speed {
        Some(speed) => format!("{CENTRE_REQUEST} {speed}"),
        None => CENTRE_REQUEST.to_string(),
    };
    travel(arduino, request, wait_ms, Stop::Centre).await
}

/// Asks the board to end whichever move is under way. It takes no permit, because
/// a move holds that, and it sends even if the server knows of no move: a server
/// restarted mid-move has forgotten the one under way. The board does not answer a
/// stop. The move's own request gets the answer, `stopped`, so 200 here means only
/// that the datagrams left.
pub(crate) async fn stop(State(arduino): State<ArduinoState>) -> Response {
    match send_stops(&arduino.settings).await {
        Ok(()) => answer_of_ours(StatusCode::OK, "sent", "the stop was sent".to_string()),
        Err(reason) => answer_of_ours(StatusCode::BAD_GATEWAY, "not_sent", reason.to_string()),
    }
}

/// Sends `stop_sends` Stops, `stop_gap_ms` apart: a datagram can be lost on the
/// WiFi, and nothing resends it.
async fn send_stops(arduino: &ArduinoSettings) -> io::Result<()> {
    let socket = UdpSocket::bind(("0.0.0.0", 0)).await?;
    socket
        .connect((arduino.address.as_str(), arduino.port))
        .await?;
    for send in 0..arduino.stop_sends {
        if send > 0 {
            time::sleep(Duration::from_millis(arduino.stop_gap_ms)).await;
        }
        socket.send(STOP_REQUEST).await?;
    }
    Ok(())
}

/// One move: sends its datagram and returns the board's JSON unchanged, with its
/// `status` as the real HTTP status. The move runs in its own task, because a
/// page closed mid-move must not make the server think the cart has stopped: the
/// permit is held until the board answers or the wait runs out. It waits for any
/// reading on its way to finish first, and no new one starts once it holds the
/// permit (`pins`), so the datagram never lands while the board is answering. A
/// measurement needs the cart still, so a move is refused during one; and the
/// brain alone moves the cart during a run, so a move is refused then too
/// (`real-open-issues.md`, issue 2, step 5).
async fn travel(arduino: ArduinoState, request: String, wait_ms: u64, stop: Stop) -> Response {
    let Ok(moving) = arduino.moving.clone().try_acquire_owned() else {
        return answer_of_ours(
            StatusCode::CONFLICT,
            "busy",
            "a move is under way, and the board would throw this one away".to_string(),
        );
    };
    if is_running(&arduino.run).await {
        return answer_of_ours(
            StatusCode::CONFLICT,
            "busy",
            "a run is going, and the brain alone moves the cart: STOP it first".to_string(),
        );
    }
    if is_measuring(&arduino.calibration).await {
        return answer_of_ours(
            StatusCode::CONFLICT,
            "busy",
            "a measurement is under way, and it needs the cart still".to_string(),
        );
    }
    let settings = arduino.settings.clone();
    let reading = arduino.reading.clone();
    let rail = arduino.rail.clone();
    let asked = tokio::spawn(async move {
        let _until_it_ends = moving;
        drop(reading.lock().await);
        let asked = ask(&settings, &request, wait_ms).await;
        let mut rail = rail.lock().await;
        match &asked {
            Ok((_, answer)) => rail.note(stop, answer),
            Err(_) => rail.forget(),
        }
        asked
    })
    .await;

    match asked {
        Ok(Ok((status, answer))) => {
            (status, [(header::CONTENT_TYPE, "application/json")], answer).into_response()
        }
        Ok(Err(reason)) if reason.kind() == io::ErrorKind::TimedOut => answer_of_ours(
            StatusCode::GATEWAY_TIMEOUT,
            "no_answer",
            format!(
                "no answer within {wait_ms} ms. The answer may have been lost on the WiFi, so the \
                 cart may have moved: look at the switches, and at the cart"
            ),
        ),
        Ok(Err(reason)) => answer_of_ours(StatusCode::BAD_GATEWAY, "no_answer", reason.to_string()),
        Err(reason) => answer_of_ours(
            StatusCode::INTERNAL_SERVER_ERROR,
            "no_answer",
            reason.to_string(),
        ),
    }
}

/// The status in a move's answer, or None if this datagram is not one.
pub(crate) fn move_status(datagram: &[u8]) -> Option<StatusCode> {
    let status = serde_json::from_slice::<Value>(datagram).ok()?["status"].as_u64()?;
    StatusCode::from_u16(u16::try_from(status).ok()?).ok()
}

/// An answer the webserver gives itself, because the board gave none or was not
/// asked, with the same five keys as the board's, so the page reads one shape.
fn answer_of_ours(status: StatusCode, result: &str, message: String) -> Response {
    let answer = json!({
        "status": status.as_u16(),
        "result": result,
        "message": message,
        "pin": null,
        "pulses": 0,
    });
    (status, Json(answer)).into_response()
}

/// A new socket for each move, so a late answer to an earlier one goes nowhere
/// rather than passing for this one. A datagram that is not a move's answer is
/// skipped, and the wait goes on: treating it as the answer would end the wait
/// while the cart is still moving, with the buttons back on and STOP off.
async fn ask(
    arduino: &ArduinoSettings,
    request: &str,
    wait_ms: u64,
) -> io::Result<(StatusCode, Vec<u8>)> {
    let socket = UdpSocket::bind(("0.0.0.0", 0)).await?;
    socket
        .connect((arduino.address.as_str(), arduino.port))
        .await?;
    socket.send(request.as_bytes()).await?;

    let mut answer = [0; MOVE_ANSWER_MAX];
    let until_answered = async {
        loop {
            let length = socket.recv(&mut answer).await?;
            if let Some(status) = move_status(&answer[..length]) {
                return Ok::<_, io::Error>((status, answer[..length].to_vec()));
            }
        }
    };
    time::timeout(Duration::from_millis(wait_ms), until_answered)
        .await
        .map_err(|_| io::Error::from(io::ErrorKind::TimedOut))?
}
