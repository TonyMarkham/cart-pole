//! The page's requests to the webserver, which passes them on to
//! `calibrate_service.py`.

pub(crate) mod refusal;

// ---------------------------------------------------------------------------------------------- //

use crate::{
    constants::{STATE_ROUTE, STATUS_ROUTE},
    error::{app_error::AppError, app_result::AppResult},
    lens_status::{LensStatus, lens_result::LensResult},
    poll_settings::PollSettings,
    request::refusal::Refusal,
};

use js_sys::Promise;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{RequestCache, RequestInit, Response, Window};

/// How often to look, from the webserver's `/calibrate/state`.
pub(crate) async fn poll_ms(window: &Window) -> AppResult<u32> {
    let text = answer(window, STATE_ROUTE, "GET").await?;
    Ok(serde_json::from_str::<PollSettings>(&text)?.poll_ms)
}

/// One look at the measurement.
pub(crate) async fn status(window: &Window) -> AppResult<LensStatus> {
    let text = answer(window, STATUS_ROUTE, "GET").await?;
    Ok(serde_json::from_str(&text)?)
}

/// One of the buttons' requests; only whether it worked matters.
pub(crate) async fn post(window: &Window, route: &str) -> AppResult<()> {
    answer(window, route, "POST").await.map(|_| ())
}

/// A request whose answer is a sentence to show.
pub(crate) async fn post_saying(window: &Window, route: &str) -> AppResult<String> {
    let text = answer(window, route, "POST").await?;
    Ok(serde_json::from_str::<Refusal>(&text).map_or(text, |said| said.message))
}

/// Solves the lens from the kept stills, and returns the result.
pub(crate) async fn solve(window: &Window, route: &str) -> AppResult<LensResult> {
    let text = answer(window, route, "POST").await?;
    Ok(serde_json::from_str(&text)?)
}

/// The body of a request that worked, or the refusal's own words.
async fn answer(window: &Window, route: &str, method: &str) -> AppResult<String> {
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
    if response.ok() {
        Ok(text)
    } else {
        let words = serde_json::from_str::<Refusal>(&text)
            .map_or_else(|_| text.clone(), |refusal| refusal.message);
        Err(AppError::answer(words))
    }
}

/// The browser's only timer is `setTimeout`; this is the interop it needs.
pub(crate) async fn sleep(window: &Window, milliseconds: u32) -> AppResult<()> {
    let milliseconds = i32::try_from(milliseconds).unwrap_or(i32::MAX);
    let mut timer = Ok(0);
    let promise = Promise::new(&mut |resolve, _reject| {
        timer =
            window.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, milliseconds);
    });
    timer?;
    JsFuture::from(promise).await?;
    Ok(())
}
