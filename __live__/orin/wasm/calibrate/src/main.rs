//! The calibration page: finds the rail's two ends and its centre with the
//! real cart, then measures the pole in the seven poses the camera's matrix is
//! fitted from. Each move button moves the real cart. The board and
//! `calibrate_service.py` do the work; this page asks and shows.

mod board;
mod constants;
mod error;
mod measure;
mod modal;
mod number;
mod page;
mod render;
mod request;
mod session;
#[cfg(test)]
mod tests;
mod tone;

// ---------------------------------------------------------------------------------------------- //

use crate::{
    board::travel::Travel,
    error::{app_error::AppError, app_result::AppResult},
    page::Page,
};

use js_sys::Date;
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

/// Finds the page's parts, learns what the webserver knows, wires the
/// buttons, then reads the switches until the page closes.
async fn run() -> AppResult<()> {
    let window = web_sys::window().ok_or_else(|| AppError::missing("window"))?;
    let document = window
        .document()
        .ok_or_else(|| AppError::missing("document"))?;
    let page = Rc::new(Page::find(window, &document)?);
    page.draw()?;

    let settings = board::settings(&page.window).await?;
    page.session.borrow_mut().address = settings.address;
    let state = board::server_state(&page.window).await?;
    page.session
        .borrow_mut()
        .restore(state.last_stop, state.span);
    page.modal.set_poll_ms(state.poll_ms);
    measure::wire(&page)?;

    for (travel, button) in page.buttons() {
        let travel = *travel;
        let clicked = Rc::clone(&page);
        let click = Closure::<dyn FnMut()>::new(move || start(Rc::clone(&clicked), travel));
        button.add_event_listener_with_callback("click", click.as_ref().unchecked_ref())?;
        // The buttons last as long as the page, so this is never to be freed.
        click.forget();
    }

    let clicked = Rc::clone(&page);
    let click = Closure::<dyn FnMut()>::new(move || stop(Rc::clone(&clicked)));
    page.stop_button()
        .add_event_listener_with_callback("click", click.as_ref().unchecked_ref())?;
    click.forget();

    // One reading at a time, with the pause between them, so a slow answer never
    // piles readings up. None during a move: the board would throw them away.
    loop {
        let moving = page.session.borrow().is_moving();
        if !moving {
            let reading = board::read_switches(&page.window).await;
            page.session.borrow_mut().read(reading, time_now());
        }
        page.draw()?;
        request::sleep(&page.window, settings.poll_ms).await?;
    }
}

/// A button's press: starts the move, unless one is under way, and takes in how
/// it ended. `Session` is never borrowed across a wait.
fn start(page: Rc<Page>, travel: Travel) {
    if !page.session.borrow_mut().begin(travel, Date::now()) {
        return;
    }
    let speed = match travel {
        Travel::Centre => page.speed(),
        Travel::Motor | Travel::Idler => None,
    };
    if let Err(error) = page.draw() {
        report(&error);
    }
    spawn_local(async move {
        let answer = board::ask_for_move(&page.window, travel, speed.as_deref()).await;
        page.session
            .borrow_mut()
            .finish(answer, Date::now(), time_now());
        // At once, so the drawing shows where the cart stopped, not a pause later.
        // Nothing opens by itself: the pose buttons and Measure this stop do that.
        let reading = board::read_switches(&page.window).await;
        page.session.borrow_mut().read(reading, time_now());
        if let Err(error) = page.draw() {
            report(&error);
        }
    });
}

/// The STOP button's press. How the move ended comes through its own request, in
/// `start`, or for a move this page did not start, through the next good reading.
fn stop(page: Rc<Page>) {
    spawn_local(async move {
        let sent = board::ask_to_stop(&page.window).await;
        page.session.borrow_mut().stop_asked(sent.err());
        if let Err(error) = page.draw() {
            report(&error);
        }
    });
}

/// The clock, for the table's times.
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
pub(crate) fn report(error: &AppError) {
    console::error_1(&JsValue::from_str(&error.to_string()));
}
