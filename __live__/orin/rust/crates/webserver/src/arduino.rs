use crate::{
    constants::{PINS_ANSWER_MAX, PINS_REQUEST},
    settings::ArduinoSettings,
};

use axum::{
    Json,
    extract::State,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use std::{io, sync::Arc, time::Duration};
use tokio::{
    net::UdpSocket,
    sync::{Mutex, Semaphore},
    time,
};

/// The pages' route state: the settings, a lock held for each reading, and the
/// one permit a move holds until it has ended. The board answers whoever sent
/// the latest datagram, so a datagram sent while it is answering another takes
/// that answer. The lock and the permit keep this server's exchanges with the
/// board one at a time. Worked failure, 2026-11-01: a move sent during a reading
/// was handed the pin levels as its answer.
#[derive(Clone)]
pub(crate) struct ArduinoState {
    pub(crate) settings: ArduinoSettings,
    pub(crate) reading: Arc<Mutex<()>>,
    pub(crate) moving: Arc<Semaphore>,
}

impl ArduinoState {
    pub(crate) fn new(settings: ArduinoSettings) -> Self {
        Self {
            settings,
            reading: Arc::new(Mutex::new(())),
            moving: Arc::new(Semaphore::new(1)),
        }
    }
}

/// One reading of the pins, for the page: the Arduino's JSON unchanged, or
/// 502 with the reason it gave none. The browser asks here because a browser
/// cannot send a datagram, and the board wakes only for one. One request is
/// one reading, so with no page open the board sleeps undisturbed. During a
/// move the board throws datagrams away, so this says so without sending one.
/// The check comes after the lock: a move takes its permit and then waits for
/// this lock, so a reading that holds the lock and sees no move is finished
/// before any move is sent.
pub(crate) async fn pins(State(arduino): State<ArduinoState>) -> Response {
    let _one_at_a_time = arduino.reading.lock().await;
    if arduino.moving.available_permits() == 0 {
        let reason = "the cart is moving, and the board hears nothing until it stops";
        return (StatusCode::SERVICE_UNAVAILABLE, reason).into_response();
    }

    match read_pins(&arduino.settings).await {
        Ok(levels) => ([(header::CONTENT_TYPE, "application/json")], levels).into_response(),
        Err(reason) => (StatusCode::BAD_GATEWAY, reason.to_string()).into_response(),
    }
}

/// `[arduino]` from settings.toml, for the page: its address to show, and how
/// long to pause between readings.
pub(crate) async fn settings(State(arduino): State<ArduinoState>) -> Json<ArduinoSettings> {
    Json(arduino.settings)
}

/// A new socket for each reading, so a late answer to an earlier one goes
/// nowhere rather than passing for this one.
async fn read_pins(arduino: &ArduinoSettings) -> io::Result<String> {
    let socket = UdpSocket::bind(("0.0.0.0", 0)).await?;
    socket.connect((arduino.address.as_str(), arduino.port)).await?;
    socket.send(PINS_REQUEST).await?;

    let mut answer = [0; PINS_ANSWER_MAX];
    let wait = Duration::from_millis(arduino.timeout_ms);
    let length = time::timeout(wait, socket.recv(&mut answer))
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, format!("no answer within {} ms", arduino.timeout_ms)))??;

    Ok(String::from_utf8_lossy(&answer[..length]).into_owned())
}
