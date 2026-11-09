//! The page's requests to the webserver, which passes Run, STOP and the
//! status on to the brain, sends the board the command 0 for STOP, and sends
//! the board Re-centre's `MoveToCentre` and its `Stop` as the calibrate
//! page's moves.

pub(crate) mod refusal;

// ---------------------------------------------------------------------------------------------- //

use crate::{
    constants::{CENTRE_ROUTE, MOVE_STOP_ROUTE, START_ROUTE, STATUS_ROUTE, STOP_ROUTE},
    error::{app_error::AppError, app_result::AppResult},
    look::Look,
    move_answer::MoveAnswer,
    request::refusal::Refusal,
    stop_answer::StopAnswer,
};

use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{RequestCache, RequestInit, Response, Window};

/// One look at the brain and the board. Never fails: a missing answer is
/// itself what the page shows.
pub(crate) async fn look(window: &Window) -> Look {
    match answer(window, STATUS_ROUTE, "GET").await {
        Ok(text) => match serde_json::from_str(&text) {
            Ok(status) => Look::Status(status),
            Err(error) => Look::NoWebserver(format!("its answer could not be read: {error}")),
        },
        Err(AppError::Answer { message }) => Look::NoBrain(message),
        Err(error) => Look::NoWebserver(error.to_string()),
    }
}

/// Run: Ok once the run has started, or the brain's reason it would not.
pub(crate) async fn start(window: &Window) -> AppResult<()> {
    answer(window, START_ROUTE, "POST").await.map(|_| ())
}

/// STOP: what the webserver did, read whatever the HTTP status, since it
/// answers 502 when either step failed and says which in the same shape.
pub(crate) async fn stop(window: &Window) -> AppResult<StopAnswer> {
    let (_, text) = fetch(window, STOP_ROUTE, "POST").await?;
    serde_json::from_str(&text).map_err(|_| AppError::answer(text))
}

/// Re-centre: the board's answer once the cart has stopped, read whatever
/// the HTTP status, which is the board's own `status`; the webserver answers
/// in the same shape when the board gave none.
pub(crate) async fn centre(window: &Window) -> AppResult<MoveAnswer> {
    let (_, text) = fetch(window, CENTRE_ROUTE, "POST").await?;
    serde_json::from_str(&text).map_err(|_| AppError::answer(text))
}

/// The calibrate page's Stop, which ends a move to the centre. The board
/// drops it at any other time.
pub(crate) async fn stop_move(window: &Window) -> AppResult<()> {
    answer(window, MOVE_STOP_ROUTE, "POST").await.map(|_| ())
}

/// The body of a request that worked, or the refusal's own words.
async fn answer(window: &Window, route: &str, method: &str) -> AppResult<String> {
    let (ok, text) = fetch(window, route, method).await?;
    if ok {
        Ok(text)
    } else {
        let words = serde_json::from_str::<Refusal>(&text)
            .map_or_else(|_| text.clone(), |refusal| refusal.message);
        Err(AppError::answer(words))
    }
}

/// One request: whether its status was 2xx, and its body. Never cached, so
/// each look is the state now.
async fn fetch(window: &Window, route: &str, method: &str) -> AppResult<(bool, String)> {
    let init = RequestInit::new();
    init.set_method(method);
    init.set_cache(RequestCache::NoStore);
    let response: Response = JsFuture::from(window.fetch_with_str_and_init(route, &init))
        .await?
        .dyn_into()?;
    let text = JsFuture::from(response.text()?)
        .await?
        .as_string()
        .unwrap_or_default();
    Ok((response.ok(), text))
}
