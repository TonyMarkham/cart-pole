//! The page's one way of asking the webserver anything, and its timer.

use crate::error::app_result::AppResult;

use js_sys::Promise;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{RequestCache, RequestInit, Response, Window};

/// One request, never from the browser's cache: whether it worked, its status,
/// and its body.
pub(crate) async fn request(
    window: &Window,
    route: &str,
    method: &str,
) -> AppResult<(bool, u16, String)> {
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
    Ok((response.ok(), response.status(), text))
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
