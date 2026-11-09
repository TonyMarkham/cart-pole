//! The routes that only pass a page's request on to a program on the Orin's own
//! address: `calibrate_service.py`, which runs the camera, the detector and the
//! fits (the measurements, the live view and the lens measurement), and the
//! brain, which runs the balance loop (`run`).

pub(crate) mod lens;
pub(crate) mod live;
pub(crate) mod measure;

// ---------------------------------------------------------------------------------------------- //

use crate::settings::calibration_settings::CalibrationSettings;

use axum::{
    Json,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use serde_json::{Value, json};
use std::time::Duration;

/// Whether a measurement is running. A service that is not running is not
/// measuring, and the moves do not depend on it.
pub(crate) async fn is_measuring(calibration: &CalibrationSettings) -> bool {
    match ask(
        &calibration.service,
        calibration.timeout_ms,
        false,
        "/busy".to_string(),
    )
    .await
    {
        Ok((_, body)) => serde_json::from_slice::<Value>(&body)
            .is_ok_and(|answer| answer["busy"].as_bool() == Some(true)),
        Err(_) => false,
    }
}

/// Passes one request on to `calibrate_service.py`, and its answer back
/// unchanged. A service that gives no answer is answered 502 here, in the same
/// shape.
pub(crate) async fn forward(
    calibration: &CalibrationSettings,
    post: bool,
    path: String,
) -> Response {
    forward_to(
        &calibration.service,
        calibration.timeout_ms,
        post,
        path,
        "the calibration service",
    )
    .await
}

/// Passes one request on to the program at `base`, and its answer back
/// unchanged. One that gives no answer is answered 502 here, naming it as
/// `who`, in the same shape.
pub(crate) async fn forward_to(
    base: &str,
    timeout_ms: u64,
    post: bool,
    path: String,
    who: &str,
) -> Response {
    match ask(base, timeout_ms, post, path).await {
        Ok((status, body)) => {
            (status, [(header::CONTENT_TYPE, "application/json")], body).into_response()
        }
        Err(reason) => refused(
            StatusCode::BAD_GATEWAY,
            &format!("{who} did not answer, so it may not be running: {reason}"),
        ),
    }
}

/// A refusal of the webserver's own, in the shape the service refuses in.
pub(crate) fn refused(status: StatusCode, message: &str) -> Response {
    (status, Json(json!({ "message": message }))).into_response()
}

/// One request to the program at `base`, on a thread of its own because the
/// HTTP client blocks. Its 4xx answers are answers, not failures.
pub(crate) async fn ask(
    base: &str,
    timeout_ms: u64,
    post: bool,
    path: String,
) -> Result<(StatusCode, Vec<u8>), String> {
    let url = format!("{base}{path}");
    let timeout = Duration::from_millis(timeout_ms);
    tokio::task::spawn_blocking(move || {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(timeout))
            .build()
            .into();
        let sent = if post {
            agent.post(&url).send_empty()
        } else {
            agent.get(&url).call()
        };
        let mut response = sent.map_err(|error| error.to_string())?;
        let status =
            StatusCode::from_u16(response.status().as_u16()).map_err(|error| error.to_string())?;
        let body = response
            .body_mut()
            .read_to_vec()
            .map_err(|error| error.to_string())?;
        Ok((status, body))
    })
    .await
    .map_err(|error| error.to_string())?
}
