//! The lens measurement: a red box over the live picture, with the board's shape
//! dashed inside it, shows where and how to hold the printed board; once the board
//! has been held there for a moment, that still is kept and the box moves on, first
//! flat across the whole picture, then with the board turned. Solve then measures
//! the lens's bend from them. `calibrate_service.py` runs the
//! camera, finds the board, keeps the stills and solves; this page asks and shows.

mod bar;
mod constants;
mod error;
mod lens_status;
mod page;
mod poll_settings;
mod render;
mod request;
#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------------------------- //

use crate::{
    constants::{RESTART_ROUTE, SKIP_ROUTE, SOLVE_ROUTE, START_ROUTE, STILL_ROUTE, STOP_ROUTE},
    error::{app_error::AppError, app_result::AppResult},
    page::Page,
};

use std::rc::Rc;
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlButtonElement, console};

fn main() {
    spawn_local(async {
        if let Err(error) = run().await {
            report(&error);
        }
    });
}

/// Finds the page's parts, wires the buttons, then looks at the measurement
/// until the page closes.
async fn run() -> AppResult<()> {
    let window = web_sys::window().ok_or_else(|| AppError::missing("window"))?;
    let document = window
        .document()
        .ok_or_else(|| AppError::missing("document"))?;
    let page = Rc::new(Page::find(window, &document)?);
    let poll_ms = request::poll_ms(&page.window).await?;

    on_click(&page.toggle, &page, toggle)?;
    on_click(&page.skip, &page, |page| post(page, SKIP_ROUTE))?;
    on_click(&page.restart, &page, restart)?;
    on_click(&page.solve, &page, solve)?;
    on_click(&page.picture, &page, save_picture)?;

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

/// Runs `action` on each click of `button`.
fn on_click(button: &HtmlButtonElement, page: &Rc<Page>, action: fn(Rc<Page>)) -> AppResult<()> {
    let page = Rc::clone(page);
    let click = Closure::<dyn FnMut()>::new(move || action(Rc::clone(&page)));
    button.add_event_listener_with_callback("click", click.as_ref().unchecked_ref())?;
    // The buttons last as long as the page, so this is never to be freed.
    click.forget();
    Ok(())
}

/// Start when off, Stop when on. The next look shows what came of it.
fn toggle(page: Rc<Page>) {
    page.toggle.set_disabled(true);
    let route = if page.is_on() {
        STOP_ROUTE
    } else {
        START_ROUTE
    };
    post(page, route);
}

/// Posts one button's request; a refusal is shown, and Start/Stop comes back.
fn post(page: Rc<Page>, route: &'static str) {
    spawn_local(async move {
        if let Err(error) = request::post(&page.window, route).await {
            page.say(&error.to_string());
            page.toggle.set_disabled(false);
        }
    });
}

/// Forgets every kept still, after asking, because they cannot be got back.
fn restart(page: Rc<Page>) {
    let sure = page
        .window
        .confirm_with_message("Forget every kept still and start again from the first position?")
        .unwrap_or(false);
    if sure {
        post(page, RESTART_ROUTE);
    }
}

/// Solves the lens from the kept stills, and shows the result.
fn solve(page: Rc<Page>) {
    page.solve.set_disabled(true);
    page.say_result("Solving.");
    spawn_local(async move {
        match request::solve(&page.window, SOLVE_ROUTE).await {
            Ok(result) => page.show_result(&result),
            Err(error) => page.say_result(&error.to_string()),
        }
        page.solve.set_disabled(false);
    });
}

/// Saves the current frame on the Orin, drawn as this page shows it, so it can be
/// looked at from elsewhere. It keeps no still for the lens and does not move the
/// box: only holding the board in place does that.
fn save_picture(page: Rc<Page>) {
    spawn_local(async move {
        match request::post_saying(&page.window, STILL_ROUTE).await {
            Ok(said) => page.say_result(&format!(
                "Picture {said} (for looking at only: it is not a kept still)."
            )),
            Err(error) => page.say_result(&error.to_string()),
        }
    });
}

/// A page has nowhere else to say what went wrong: the browser's console.
fn report(error: &AppError) {
    console::error_1(&JsValue::from_str(&error.to_string()));
}
