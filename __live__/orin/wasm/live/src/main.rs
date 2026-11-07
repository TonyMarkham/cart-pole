//! The live view: the camera's picture with a box on every marker the detector
//! finds, for aiming the camera so the whole rig, and the pole held level at
//! each end, are in the picture. `calibrate_service.py` runs the camera and the
//! detector; this page only asks and shows.

mod constants;
mod error;
mod live_status;
mod page;
mod poll_settings;
mod render;
mod request;
#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------------------------- //

use crate::{
    error::{app_error::AppError, app_result::AppResult},
    page::Page,
};

use std::rc::Rc;
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use wasm_bindgen_futures::spawn_local;
use web_sys::console;

fn main() {
    spawn_local(async {
        if let Err(error) = run().await {
            report(&error);
        }
    });
}

/// Finds the page's parts, wires the button, then looks at the view until
/// the page closes.
async fn run() -> AppResult<()> {
    let window = web_sys::window().ok_or_else(|| AppError::missing("window"))?;
    let document = window
        .document()
        .ok_or_else(|| AppError::missing("document"))?;
    let page = Rc::new(Page::find(window, &document)?);
    let poll_ms = request::poll_ms(&page.window).await?;

    let clicked = Rc::clone(&page);
    let click = Closure::<dyn FnMut()>::new(move || toggle(Rc::clone(&clicked)));
    page.button
        .add_event_listener_with_callback("click", click.as_ref().unchecked_ref())?;
    // The button lasts as long as the page, so this is never to be freed.
    click.forget();

    // One look at a time, with the pause between them, so a slow answer never
    // piles looks up. Looking is also what keeps the view on: the service stops it
    // by itself once no page has looked for a while.
    loop {
        match request::status(&page.window).await {
            Ok(status) => page.show(&status)?,
            Err(error) => page.say(&error.to_string()),
        }
        request::sleep(&page.window, poll_ms).await?;
    }
}

/// Start when off, Stop when on. The next look shows what came of it.
fn toggle(page: Rc<Page>) {
    let on = !page.is_on();
    page.button.set_disabled(true);
    spawn_local(async move {
        if let Err(error) = request::switch(&page.window, on).await {
            page.say(&error.to_string());
            page.button.set_disabled(false);
        }
    });
}

/// A page has nowhere else to say what went wrong: the browser's console.
fn report(error: &AppError) {
    console::error_1(&JsValue::from_str(&error.to_string()));
}
