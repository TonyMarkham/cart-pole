//! The page's requests about the board: its settings, what the webserver
//! remembers of the rail, a reading of the switches, and the moves.

pub(crate) mod answer;
pub(crate) mod reading;
pub(crate) mod server_state;
pub(crate) mod settings;
pub(crate) mod switches;
pub(crate) mod travel;

// ---------------------------------------------------------------------------------------------- //

use crate::{
    board::{
        answer::Answer, reading::Reading, server_state::ServerState, settings::Settings,
        travel::Travel,
    },
    constants::{PINS_ROUTE, SETTINGS_ROUTE, STATE_ROUTE, STATUS_SERVICE_UNAVAILABLE, STOP_ROUTE},
    error::{app_error::AppError, app_result::AppResult},
    request::request,
};

use js_sys::encode_uri_component;
use web_sys::Window;

/// `[arduino]` from the webserver's settings.toml.
pub(crate) async fn settings(window: &Window) -> AppResult<Settings> {
    let (ok, _, text) = request(window, SETTINGS_ROUTE, "GET").await?;
    if ok {
        Ok(serde_json::from_str(&text)?)
    } else {
        Err(AppError::answer(text))
    }
}

/// What the webserver remembers about the rail.
pub(crate) async fn server_state(window: &Window) -> AppResult<ServerState> {
    let (ok, _, text) = request(window, STATE_ROUTE, "GET").await?;
    if ok {
        Ok(serde_json::from_str(&text)?)
    } else {
        Err(AppError::answer(text))
    }
}

/// One reading of the switches.
pub(crate) async fn read_switches(window: &Window) -> Reading {
    match request(window, PINS_ROUTE, "GET").await {
        Ok((true, _, text)) => match serde_json::from_str(&text) {
            Ok(switches) => Reading::Levels(switches),
            Err(error) => Reading::Failed(error.to_string()),
        },
        Ok((_, STATUS_SERVICE_UNAVAILABLE, _)) => Reading::MovingElsewhere,
        Ok((_, _, text)) => Reading::Failed(text),
        Err(error) => Reading::Failed(error.to_string()),
    }
}

/// Asks for one move and waits for how it ended. A POST, because it does
/// something, and a browser never repeats a POST by itself. `speed` is the
/// words typed on the page, for the board to accept or refuse.
pub(crate) async fn ask_for_move(window: &Window, travel: Travel, speed: Option<&str>) -> Answer {
    let route = match speed {
        Some(speed) => format!(
            "{}?speed={}",
            travel.route(),
            String::from(encode_uri_component(speed))
        ),
        None => travel.route().to_string(),
    };
    match request(window, &route, "POST").await {
        Ok((_, _, text)) => serde_json::from_str(&text).unwrap_or_else(|_| {
            Answer::lost(format!("the webserver's answer made no sense: {text}"))
        }),
        Err(error) => Answer::lost(format!(
            "the webserver did not answer, so the cart may still have moved: {error}"
        )),
    }
}

/// Asks the webserver to send the board a stop. `Ok` means only that it left: the
/// move's own request gets the answer, once the cart has stopped.
pub(crate) async fn ask_to_stop(window: &Window) -> AppResult<()> {
    let (ok, _, text) = request(window, STOP_ROUTE, "POST").await?;
    if ok {
        Ok(())
    } else {
        Err(AppError::answer(text))
    }
}
