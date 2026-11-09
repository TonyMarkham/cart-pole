use crate::{
    constants::{POST_METHOD, REFRESH_ROUTE, REFUSAL_MESSAGE, STATUS_REFRESHING},
    error::{app_error::AppError, app_result::AppResult},
};

use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Element, HtmlButtonElement, RequestCache, RequestInit, Response, Window};

/// The Refresh button and the text beside it. A click asks the webserver for
/// a fresh camera frame; the frame itself comes over the viewer's socket
/// (`real-orin-plan.md`, O12).
pub(crate) struct Refresher {
    pub(crate) window: Window,
    pub(crate) button: HtmlButtonElement,
    pub(crate) status: Element,
}

impl Refresher {
    /// Asks for the frame. The button is off while it waits, and a refusal's
    /// words are shown beside it.
    pub(crate) async fn refresh(&self) {
        self.button.set_disabled(true);
        self.status.set_text_content(Some(STATUS_REFRESHING));
        let said = match self.post().await {
            Ok(()) => String::new(),
            Err(error) => error.to_string(),
        };
        self.status.set_text_content(Some(&said));
        self.button.set_disabled(false);
    }

    /// One POST to the refresh route; a refusal comes back as its words.
    async fn post(&self) -> AppResult<()> {
        let init = RequestInit::new();
        init.set_method(POST_METHOD);
        init.set_cache(RequestCache::NoStore);
        let response: Response =
            JsFuture::from(self.window.fetch_with_str_and_init(REFRESH_ROUTE, &init))
                .await?
                .dyn_into()?;
        if response.ok() {
            return Ok(());
        }
        let text = JsFuture::from(response.text()?)
            .await?
            .as_string()
            .unwrap_or_default();
        let words = serde_json::from_str::<serde_json::Value>(&text)
            .ok()
            .and_then(|answer| answer[REFUSAL_MESSAGE].as_str().map(str::to_string))
            .unwrap_or(text);
        Err(AppError::answer(words))
    }
}
