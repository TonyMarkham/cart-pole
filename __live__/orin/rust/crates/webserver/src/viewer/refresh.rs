use crate::{
    constants::{
        JSON_CONTENT_TYPE, LIVE_STATUS_FRAME, LIVE_STATUS_ON, SERVICE_LIVE_START,
        SERVICE_LIVE_STATUS, SERVICE_LIVE_STOP,
    },
    run::is_running,
    service::{ask, refused},
    viewer::{base64, refresh_state::RefreshState},
};

use axum::{
    body::Bytes,
    extract::State,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use serde_json::Value;
use std::time::{Duration, Instant};

/// Puts one fresh camera frame on every open viewer (`real-orin-plan.md`,
/// O12). `calibrate_service.py` holds the camera, so the frame comes from its
/// live view: started if it was off, its status asked every `poll_ms` until
/// it carries a frame, then stopped again if it was off. The service clears
/// its frame when the view starts, so the first frame after a start was
/// taken since it. The frame is a JPEG, which the viewer shows as it does a
/// PNG. Refused while the brain reports a run, as `/live/start` is (O10).
pub(crate) async fn refresh(State(state): State<RefreshState>) -> Response {
    if is_running(&state.rig.run).await {
        return refused(
            StatusCode::CONFLICT,
            "a run is going, and the brain has the camera",
        );
    }
    let calibration = &state.rig.calibration;
    let was_on = match status(&state).await {
        Ok(status) => status[LIVE_STATUS_ON].as_bool() == Some(true),
        Err(reason) => return refused(StatusCode::BAD_GATEWAY, &reason),
    };
    if !was_on {
        match ask(
            &calibration.service,
            calibration.timeout_ms,
            true,
            SERVICE_LIVE_START.to_string(),
        )
        .await
        {
            Ok((answer, _)) if answer.is_success() => {}
            Ok((answer, body)) => {
                return (answer, [(header::CONTENT_TYPE, JSON_CONTENT_TYPE)], body).into_response();
            }
            Err(reason) => return refused(StatusCode::BAD_GATEWAY, &reason),
        }
    }

    let frame = wait_for_frame(&state).await;

    if !was_on {
        // The frame is what was asked for; a stop that fails leaves the view
        // to stop itself once no page looks at it.
        let _stopped = ask(
            &calibration.service,
            calibration.timeout_ms,
            true,
            SERVICE_LIVE_STOP.to_string(),
        )
        .await;
    }
    match frame {
        Ok(jpeg) => {
            state.images.send_replace(Some(Bytes::from(jpeg)));
            StatusCode::NO_CONTENT.into_response()
        }
        Err(reason) => refused(StatusCode::GATEWAY_TIMEOUT, &reason),
    }
}

/// The live view's status, as JSON.
async fn status(state: &RefreshState) -> Result<Value, String> {
    let calibration = &state.rig.calibration;
    let (_, body) = ask(
        &calibration.service,
        calibration.timeout_ms,
        false,
        SERVICE_LIVE_STATUS.to_string(),
    )
    .await
    .map_err(|reason| format!("the calibration service did not answer: {reason}"))?;
    serde_json::from_slice(&body).map_err(|error| format!("the live view's status: {error}"))
}

/// Asks the status every `poll_ms` until it carries a frame, for at most
/// `refresh_timeout_ms`, and returns the frame's JPEG bytes.
async fn wait_for_frame(state: &RefreshState) -> Result<Vec<u8>, String> {
    let calibration = &state.rig.calibration;
    let deadline = Instant::now() + Duration::from_millis(calibration.refresh_timeout_ms);
    while Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(calibration.poll_ms)).await;
        let status = status(state).await?;
        if let Some(text) = status[LIVE_STATUS_FRAME].as_str() {
            return base64::decode(text)
                .ok_or_else(|| "the live view's frame is not base64".to_string());
        }
    }
    Err(format!(
        "no frame from the live view within {} ms",
        calibration.refresh_timeout_ms
    ))
}
