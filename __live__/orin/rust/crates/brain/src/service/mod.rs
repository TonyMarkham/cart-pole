//! The brain's requests, on the Orin's own address, which the webserver
//! forwards the page's buttons to (`real-loop-dia.md`, *Contracts*, the
//! brain's requests). Each answers the brain's status; a refused Run answers
//! it with 409 and the reason in `message`.

use crate::run::brain::Brain;

use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;

/// `POST /run`: starts a run, or says why not.
pub(crate) async fn run(State(brain): State<Brain>) -> Response {
    let started = tokio::task::spawn_blocking({
        let brain = brain.clone();
        move || brain.start()
    })
    .await;
    match started {
        Ok(Ok(())) => (StatusCode::OK, Json(brain.status())).into_response(),
        Ok(Err(reason)) => {
            let mut status = brain.status();
            status.message = reason;
            (StatusCode::CONFLICT, Json(status)).into_response()
        }
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "message": error.to_string() })),
        )
            .into_response(),
    }
}

/// `POST /idle`: ends the run, answering once no command will follow.
pub(crate) async fn idle(State(brain): State<Brain>) -> Response {
    let stopped = tokio::task::spawn_blocking({
        let brain = brain.clone();
        move || brain.stop()
    })
    .await;
    match stopped {
        Ok(()) => (StatusCode::OK, Json(brain.status())).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "message": error.to_string() })),
        )
            .into_response(),
    }
}

/// `GET /status`: the state, the last reason, the run's folder and its frames.
pub(crate) async fn status(State(brain): State<Brain>) -> Response {
    (StatusCode::OK, Json(brain.status())).into_response()
}
