//! The page's requests to the webserver, which passes the live ones on to
//! `calibrate_service.py`.

pub(crate) mod refusal;

// ---------------------------------------------------------------------------------------------- //

use crate::{
    constants::{START_ROUTE, STATE_ROUTE, STATUS_ROUTE, STOP_ROUTE},
    error::{app_error::AppError, app_result::AppResult},
    live_status::LiveStatus,
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

/// One look at the view.
pub(crate) async fn status(window: &Window) -> AppResult<LiveStatus> {
    let text = answer(window, STATUS_ROUTE, "GET").await?;
    Ok(serde_json::from_str(&text)?)
}

/// Starts or stops the view.
pub(crate) async fn switch(window: &Window, on: bool) -> AppResult<()> {
    let route = if on { START_ROUTE } else { STOP_ROUTE };
    answer(window, route, "POST").await.map(|_| ())
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
