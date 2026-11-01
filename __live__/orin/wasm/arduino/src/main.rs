mod constants;
mod error;
mod pins;
mod render;

// ---------------------------------------------------------------------------------------------- //

pub(crate) use crate::constants::{
    BOARD_ELEMENT_ID, CONNECTION_ELEMENT_ID, PINS_ROUTE, POLL_ELEMENT_ID, ROWS_ELEMENT_ID,
    SETTINGS_ROUTE,
};

// ---------------------------------------------------------------------------------------------- //

use crate::{
    error::{AppError, result::AppResult},
    pins::Levels,
};

use js_sys::{Date, Promise};
use serde::Deserialize;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{Document, Element, RequestCache, RequestInit, Response, Window, console};

/// The part of the webserver's `/arduino/settings` this page uses.
#[derive(Deserialize)]
struct Settings {
    address: String,
    poll_ms: u32,
}

/// The elements `index.html` leaves for this program to fill.
struct Page {
    window: Window,
    board: Element,
    rows: Element,
    connection: Element,
}

fn main() {
    spawn_local(async {
        if let Err(error) = run().await {
            report(&error);
        }
    });
}

async fn run() -> AppResult<()> {
    let window = web_sys::window().ok_or_else(|| AppError::missing("window"))?;
    let document = window
        .document()
        .ok_or_else(|| AppError::missing("document"))?;
    let page = Page {
        board: element(&document, BOARD_ELEMENT_ID)?,
        rows: element(&document, ROWS_ELEMENT_ID)?,
        connection: element(&document, CONNECTION_ELEMENT_ID)?,
        window,
    };
    page.show(None)?;

    let settings = match fetch_text(&page.window, SETTINGS_ROUTE).await {
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
        match read_levels(&page.window).await {
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
        sleep(&page.window, settings.poll_ms).await?;
    }
}

impl Page {
    /// Draws the board and the table for one reading, or for none.
    fn show(&self, levels: Option<&Levels>) -> AppResult<()> {
        let drawn = render::board(levels).and_then(|board| Ok((board, render::rows(levels)?)));
        let (board, rows) = drawn.map_err(|error| AppError::answer(error.to_string()))?;
        self.board.set_inner_html(&board);
        self.rows.set_inner_html(&rows);
        Ok(())
    }

    /// The line above the drawing. Set as text, so nothing in a message can
    /// become part of the page.
    fn say(&self, class: &str, message: &str) {
        self.connection.set_class_name(class);
        self.connection.set_text_content(Some(message));
    }
}

fn element(document: &Document, id: &str) -> AppResult<Element> {
    document
        .get_element_by_id(id)
        .ok_or_else(|| AppError::missing(id))
}

async fn read_levels(window: &Window) -> AppResult<Levels> {
    Ok(serde_json::from_str(
        &fetch_text(window, PINS_ROUTE).await?,
    )?)
}

/// The body of one GET, never from the browser's cache, because a reading has
/// to be a new one. A response that isn't a success becomes an error holding
/// what the webserver said.
async fn fetch_text(window: &Window, route: &str) -> AppResult<String> {
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
async fn sleep(window: &Window, milliseconds: u32) -> AppResult<()> {
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

fn time_now() -> String {
    let now = Date::new_0();
    format!(
        "{:02}:{:02}:{:02}",
        now.get_hours(),
        now.get_minutes(),
        now.get_seconds()
    )
}

fn report(error: &AppError) {
    console::error_1(&JsValue::from_str(&error.to_string()));
}
