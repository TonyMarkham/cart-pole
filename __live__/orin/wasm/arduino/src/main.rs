//! The Arduino page: draws the rig's UNO R4 WiFi, marks the pins the rig uses,
//! and shows what each reads now, about once a second. The Orin has no monitor
//! and the board should run without a USB cable, so this is how wiring faults
//! are seen.

mod constants;
mod error;
mod page;
mod pins;
mod render;
mod request;
mod settings;
#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------------------------- //

use crate::{
    constants::{POLL_ELEMENT_ID, SETTINGS_ROUTE},
    error::{app_error::AppError, app_result::AppResult},
    page::{Page, element},
    settings::Settings,
};

use js_sys::Date;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;
use web_sys::console;

fn main() {
    spawn_local(async {
        if let Err(error) = run().await {
            report(&error);
        }
    });
}

/// Finds the page's parts and the settings, then reads the pins until the
/// page closes.
async fn run() -> AppResult<()> {
    let window = web_sys::window().ok_or_else(|| AppError::missing("window"))?;
    let document = window
        .document()
        .ok_or_else(|| AppError::missing("document"))?;
    let page = Page::find(window, &document)?;
    page.show(None)?;

    let settings = match request::fetch_text(&page.window, SETTINGS_ROUTE).await {
        Ok(text) => serde_json::from_str::<Settings>(&text)?,
        Err(error) => {
            page.say(
                "warn",
                "The webserver didn't answer. Refresh this page to try again.",
            );
            return Err(error);
        }
    };
    element(&document, POLL_ELEMENT_ID)?.set_text_content(Some(&settings.poll_ms.to_string()));

    // One reading at a time, with the pause between them, so a slow answer
    // never piles readings up.
    loop {
        match request::read_levels(&page.window).await {
            Ok(levels) => {
                page.show(Some(&levels))?;
                page.say(
                    "ok",
                    &format!(
                        "Reading from the board at {}, taken at {}.",
                        settings.address,
                        time_now()
                    ),
                );
            }
            Err(error) => {
                page.show(None)?;
                page.say(
                    "warn",
                    &format!(
                        "No answer from the board at {}. It may be off, running other firmware, at another \
                         address (settings.toml), or its answer may have been lost on the WiFi, in which case \
                         the next reading will show. Reason: {error}",
                        settings.address
                    ),
                );
            }
        }
        request::sleep(&page.window, settings.poll_ms).await?;
    }
}

/// The clock, for saying when a reading was taken.
fn time_now() -> String {
    let now = Date::new_0();
    format!(
        "{:02}:{:02}:{:02}",
        now.get_hours(),
        now.get_minutes(),
        now.get_seconds()
    )
}

/// A page has nowhere else to say what went wrong: the browser's console.
fn report(error: &AppError) {
    console::error_1(&JsValue::from_str(&error.to_string()));
}
