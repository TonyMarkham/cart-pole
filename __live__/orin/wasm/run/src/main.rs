//! The run page: Run and STOP for the balance loop on the rig, and the
//! brain's and the board's state. The brain runs the loop and the webserver
//! passes the page's presses on to it; this page only asks and shows
//! (`__documentation__/real-loop-dia.md`, *The run and idle states*; `real-orin-plan.md`, O9).

mod board_answer;
mod constants;
mod error;
mod look;
mod move_answer;
mod page;
mod request;
mod run_rule;
mod run_status;
mod stop_answer;
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

/// Finds the page's parts, wires the two buttons, then takes one look.
async fn run() -> AppResult<()> {
    let window = web_sys::window().ok_or_else(|| AppError::missing("window"))?;
    let document = window
        .document()
        .ok_or_else(|| AppError::missing("document"))?;
    let page = Rc::new(Page::find(window, &document)?);

    let for_run = Rc::clone(&page);
    let run_click =
        Closure::<dyn FnMut()>::new(move || spawn_local(press_run(Rc::clone(&for_run))));
    page.run
        .add_event_listener_with_callback("click", run_click.as_ref().unchecked_ref())?;
    // The buttons last as long as the page, so these are never to be freed.
    run_click.forget();

    let for_stop = Rc::clone(&page);
    let stop_click =
        Closure::<dyn FnMut()>::new(move || spawn_local(press_stop(Rc::clone(&for_stop))));
    page.stop
        .add_event_listener_with_callback("click", stop_click.as_ref().unchecked_ref())?;
    stop_click.forget();

    let for_centre = Rc::clone(&page);
    let centre_click =
        Closure::<dyn FnMut()>::new(move || spawn_local(press_centre(Rc::clone(&for_centre))));
    page.centre
        .add_event_listener_with_callback("click", centre_click.as_ref().unchecked_ref())?;
    centre_click.forget();

    page.show(&request::look(&page.window).await);
    Ok(())
}

/// Run, then a fresh look, so the page shows what came of it.
async fn press_run(page: Rc<Page>) {
    page.set_pending(Some("Asking the brain to start a run."));
    page.say("Run pressed: asking the brain to start.");
    match request::start(&page.window).await {
        Ok(()) => page.say("Run started."),
        Err(AppError::Answer { message }) => page.say(&format!("Run refused: {message}")),
        Err(error) => {
            page.say("Run: the Orin's webserver did not answer.");
            report(&error);
        }
    }
    page.set_pending(None);
    page.show(&request::look(&page.window).await);
}

/// Re-centre, which answers only once the cart has stopped, then a fresh
/// look.
async fn press_centre(page: Rc<Page>) {
    page.set_pending(Some(
        "The cart is moving to the centre. STOP ends the move.",
    ));
    page.say("Re-centre pressed: the cart moves to the centre at the board's seek speed.");
    match request::centre(&page.window).await {
        Ok(answer) => page.say(&answer.words()),
        Err(AppError::Answer { message }) => page.say(&format!("Re-centre: {message}")),
        Err(error) => {
            page.say("Re-centre: the Orin's webserver did not answer. Look at the cart.");
            report(&error);
        }
    }
    page.set_pending(None);
    page.show(&request::look(&page.window).await);
}

/// STOP, then a fresh look. It also sends the board `Stop`, on its own, so
/// it ends a Re-centre without waiting for the run's stop: during a move the
/// board takes only `Stop`, and drops it at any other time (`__documentation__/real-loop-dia.md`,
/// *The run and idle states*, Re-centre). STOP is never disabled, so a second
/// press while the first is on its way sends both again, which is harmless:
/// the board is already bringing the cart to rest.
async fn press_stop(page: Rc<Page>) {
    page.say("STOP pressed: asking the brain for idle, then sending the board the command 0.");
    let for_move = Rc::clone(&page);
    spawn_local(async move {
        if let Err(error) = request::stop_move(&for_move.window).await {
            report(&error);
        }
    });
    match request::stop(&page.window).await {
        Ok(answer) => page.say(&answer.words()),
        Err(AppError::Answer { message }) => page.say(&format!("STOP: {message}")),
        Err(error) => {
            page.say("STOP: the Orin's webserver did not answer. Use the e-stop.");
            report(&error);
        }
    }
    page.show(&request::look(&page.window).await);
}

/// A page has nowhere else to say what went wrong: the browser's console.
fn report(error: &AppError) {
    console::error_1(&JsValue::from_str(&error.to_string()));
}
