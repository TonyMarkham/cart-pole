//! The page's requests to the webserver, which asks the board.

use crate::{
    constants::PINS_ROUTE,
    error::{app_error::AppError, app_result::AppResult},
    pins::levels::Levels,
};

use js_sys::Promise;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{RequestCache, RequestInit, Response, Window};

/// One reading of the pins, which the webserver asks of the board at that
/// moment.
pub(crate) async fn read_levels(window: &Window) -> AppResult<Levels> {
    Ok(serde_json::from_str(
        &fetch_text(window, PINS_ROUTE).await?,
    )?)
}

/// The body of one GET, never from the browser's cache, because a reading has
/// to be a new one. A response that isn't a success becomes an error holding
/// what the webserver said.
pub(crate) async fn fetch_text(window: &Window, route: &str) -> AppResult<String> {
    let init = RequestInit::new();
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
        Err(AppError::answer(text))
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
