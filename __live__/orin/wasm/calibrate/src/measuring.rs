//! The modal's steps: watch the live detections while holding the pole in the pose,
//! Calibrate now, then keep or retry. Only `calibrate_service.py` runs the camera and
//! measures; this asks and shows.

use crate::{
    board,
    constants::{
        FIT_ACCEPT_ROUTE, FIT_ROUTE, LIVE_START_ROUTE, LIVE_STATUS_ROUTE, LIVE_STOP_ROUTE,
        MEASURE_ACCEPT_ROUTE, MEASURE_CANCEL_ROUTE, MEASURE_ROUTE, MEASURE_STATUS_ROUTE,
    },
    error::result::AppResult,
    fit_report::FitReport,
    live_status::LiveStatus,
    measure_status::MeasureStatus,
    page::Page,
    pole::Pole,
    report,
    travel::Travel,
};

use serde::Deserialize;
use std::rc::Rc;
use wasm_bindgen::{JsCast, closure::Closure};
use wasm_bindgen_futures::spawn_local;
use web_sys::{Event, HtmlButtonElement};

/// What the webserver or the service says when it refuses something.
#[derive(Deserialize)]
struct Refusal {
    message: String,
}

/// The modal's buttons, and the page's button that opens it without a move.
pub(crate) fn wire(page: &Rc<Page>) -> AppResult<()> {
    let modal = &page.modal;
    on_click(&modal.calibrate, page, calibrate_now)?;
    on_click(&modal.done, page, done)?;
    on_click(&modal.retry, page, retry)?;
    on_click(&modal.cancel, page, cancel)?;
    on_click(&modal.close, page, |page| page.modal.dismiss())?;
    on_click(&modal.accept_fit, page, accept_fit)?;
    for (stop, pole, button) in page.pose_buttons() {
        let (stop, pole) = (*stop, *pole);
        let clicked = Rc::clone(page);
        let click = Closure::<dyn FnMut()>::new(move || open_pose(Rc::clone(&clicked), stop, pole));
        button.add_event_listener_with_callback("click", click.as_ref().unchecked_ref())?;
        // The buttons last as long as the page, so this is never to be freed.
        click.forget();
    }
    on_click(page.measure_button(), page, |page| {
        let stop = page.session.borrow().stop_here();
        if let Some(stop) = stop {
            open(page, stop);
        }
    })?;
    // The Esc key would otherwise close the modal behind the buttons' backs.
    let esc = Closure::<dyn FnMut(Event)>::new(|event: Event| event.prevent_default());
    modal
        .dialog
        .add_event_listener_with_callback("cancel", esc.as_ref().unchecked_ref())?;
    esc.forget();
    Ok(())
}

fn on_click(button: &HtmlButtonElement, page: &Rc<Page>, action: fn(Rc<Page>)) -> AppResult<()> {
    let page = Rc::clone(page);
    let click = Closure::<dyn FnMut()>::new(move || action(Rc::clone(&page)));
    button.add_event_listener_with_callback("click", click.as_ref().unchecked_ref())?;
    // The buttons last as long as the page, so this is never to be freed.
    click.forget();
    Ok(())
}

/// The modal for this stop's first pose not yet measured, or its first pose when
/// all are: Measure this stop's window.
fn open(page: Rc<Page>, stop: Travel) {
    spawn_local(async move {
        let pole = match read_status(&page).await {
            Ok(status) => unmeasured(stop, &status.measured).unwrap_or(Pole::Hanging),
            Err(_) => Pole::Hanging,
        };
        open_pose(page, stop, pole);
    });
}

/// The modal for one pose, on the live view. The service's limits and the kept gap
/// come with its status, so a service that is not running shows at once.
pub(crate) fn open_pose(page: Rc<Page>, stop: Travel, pole: Pole) {
    spawn_local(async move {
        let status = match read_status(&page).await {
            Ok(status) => status,
            Err(message) => {
                page.modal.show_problem(&message);
                if !page.modal.dialog.open() {
                    let _ = page.modal.dialog.show_modal();
                }
                return;
            }
        };
        if let Err(error) = page.modal.open(stop, pole, &status) {
            report(&error);
            return;
        }
        let generation = page.modal.generation();
        if let Err(message) = post(&page, LIVE_START_ROUTE).await {
            page.modal.show_problem(&message);
            return;
        }
        watch_live(&page, generation).await;
    });
}

/// Shows the live view until the modal moves on. Looking also keeps the view on:
/// the service stops it once no page has looked for a while.
async fn watch_live(page: &Rc<Page>, generation: u32) {
    while page.modal.generation() == generation {
        let status = read_live(page).await;
        if page.modal.generation() != generation {
            return;
        }
        let shown = match status {
            Ok(status) => page.modal.show_live(&status),
            Err(message) => {
                page.modal.show_problem(&message);
                return;
            }
        };
        if let Err(error) = shown {
            report(&error);
            return;
        }
        if let Err(error) = board::sleep(&page.window, page.modal.poll_ms()).await {
            report(&error);
            return;
        }
    }
}

/// The first of this stop's poses with no kept measurement.
fn unmeasured(stop: Travel, measured: &[String]) -> Option<Pole> {
    Pole::at(stop)
        .iter()
        .copied()
        .find(|pole| !measured.contains(&pole.pose_key(stop)))
}

/// A number of millimetres inside the limit, or what to tell the person. Written
/// with a point and digits only, so a slip cannot pass for a different number.
fn parse_gap(typed: &str, most: f64) -> Result<f64, String> {
    let typed = typed.trim();
    let words = format!("Measure the gap and enter it in millimetres, from 0 to {most}.");
    if typed.is_empty()
        || !typed
            .bytes()
            .all(|byte| byte.is_ascii_digit() || byte == b'.')
    {
        return Err(words);
    }
    match typed.parse::<f64>() {
        Ok(gap) if (0.0..=most).contains(&gap) => Ok(gap),
        _ => Err(words),
    }
}

/// A whole number inside the limits, or what to tell the person.
fn parse_count(typed: &str, (low, high): (u32, u32)) -> Result<u32, String> {
    let typed = typed.trim();
    let words = format!("Enter a whole number from {low} to {high}.");
    if typed.is_empty() || !typed.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(words);
    }
    match typed.parse::<u32>() {
        Ok(count) if (low..=high).contains(&count) => Ok(count),
        _ => Err(words),
    }
}

/// Calibrate now: the person has the pole in the pose and the gap typed, if this
/// stop has one. The measurement takes the camera from the live view by itself.
fn calibrate_now(page: Rc<Page>) {
    let modal = &page.modal;
    let Some(stop) = modal.stop() else {
        return;
    };
    modal.clear_errors();
    let gap_mm = match stop {
        Travel::Centre => None,
        Travel::Motor | Travel::Idler => match parse_gap(&modal.typed_gap(), modal.max_gap_mm()) {
            Ok(gap_mm) => Some(gap_mm),
            Err(words) => {
                modal.say_gap_error(&words);
                return;
            }
        },
    };
    let count = match parse_count(&modal.typed_count(), modal.limits()) {
        Ok(count) => count,
        Err(words) => {
            modal.say_count_error(&words);
            return;
        }
    };
    modal.calibrate.set_disabled(true);
    spawn_local(async move {
        let mut route = format!(
            "{MEASURE_ROUTE}?stop={}&pole={}&count={count}",
            stop.key(),
            page.modal.pole().key()
        );
        if let Some(gap_mm) = gap_mm {
            route.push_str(&format!("&gap_mm={gap_mm}"));
        }
        match post(&page, &route).await {
            Ok(_) => {
                let generation = page.modal.move_on();
                page.modal.show_running(count);
                watch(&page, generation).await;
            }
            Err(message) => {
                page.modal.calibrate.set_disabled(false);
                page.modal.say_count_error(&message);
            }
        }
    });
}

/// Follows the measurement until it ends or the modal moves on.
async fn watch(page: &Rc<Page>, generation: u32) {
    while page.modal.generation() == generation {
        let status = read_status(page).await;
        if page.modal.generation() != generation {
            return;
        }
        let shown = match status {
            Ok(status) => show_status(page, &status),
            Err(message) => {
                page.modal.show_problem(&message);
                return;
            }
        };
        match shown {
            Ok(true) => {}
            Ok(false) => return,
            Err(error) => {
                report(&error);
                return;
            }
        }
        if let Err(error) = board::sleep(&page.window, page.modal.poll_ms()).await {
            report(&error);
            return;
        }
    }
}

/// Shows one look at the measurement, and says whether it is still running.
fn show_status(page: &Rc<Page>, status: &MeasureStatus) -> AppResult<bool> {
    let modal = &page.modal;
    match (status.state.as_str(), &status.summary) {
        ("settling" | "measuring", _) => {
            modal.update_running(status)?;
            Ok(true)
        }
        ("done", Some(summary)) => {
            modal.show_summary(summary)?;
            Ok(false)
        }
        ("failed", _) => {
            modal.show_problem(&status.message);
            Ok(false)
        }
        _ => {
            modal.show_problem(
                "The measurement has ended without a result. Press Cancel, then measure again.",
            );
            Ok(false)
        }
    }
}

/// Keeps this pose's result. With all seven poses kept, the fit runs; otherwise the
/// modal closes, and the stop's pose buttons choose the next.
fn done(page: Rc<Page>) {
    spawn_local(async move {
        if let Err(message) = post(&page, MEASURE_ACCEPT_ROUTE).await {
            page.modal.show_problem(&message);
            return;
        }
        match read_status(&page).await {
            Ok(status) if status.measured.len() == status.poses.len() => fit(&page).await,
            Ok(_) => page.modal.dismiss(),
            Err(message) => page.modal.show_problem(&message),
        }
    });
}

async fn fit(page: &Rc<Page>) {
    let modal = &page.modal;
    let answer = post(page, FIT_ROUTE).await;
    let shown = match answer {
        Ok(text) => match serde_json::from_str::<FitReport>(&text) {
            Ok(report) => modal.show_fit(&report),
            Err(error) => {
                modal.show_fit_refused(&error.to_string());
                Ok(())
            }
        },
        Err(message) => {
            modal.show_fit_refused(&message);
            Ok(())
        }
    };
    if let Err(error) = shown {
        report(&error);
    }
}

/// Marks the fit just shown as the one to use, though a point is over the limit:
/// the robot balancing decides whether it is good enough, not the limit.
fn accept_fit(page: Rc<Page>) {
    page.modal.accept_fit.set_disabled(true);
    spawn_local(async move {
        let outcome = match post(&page, FIT_ACCEPT_ROUTE).await {
            Ok(text) => {
                Ok(serde_json::from_str::<Refusal>(&text).map_or(text, |said| said.message))
            }
            Err(message) => Err(message),
        };
        if let Err(error) = page.modal.show_fit_accepted(outcome) {
            report(&error);
        }
    });
}

/// Throws this pose's result away, and goes back to the live view: the pole may
/// have been nudged, so that means holding it in the pose again.
fn retry(page: Rc<Page>) {
    let (Some(stop), pole) = (page.modal.stop(), page.modal.pole()) else {
        return;
    };
    page.modal.move_on();
    spawn_local(async move {
        if let Err(message) = post(&page, MEASURE_CANCEL_ROUTE).await {
            page.modal.show_problem(&message);
            return;
        }
        open_pose(page, stop, pole);
    });
}

/// Closes the modal at once, and ends any measurement or live view behind it. The
/// cart stays where it is, and the pose buttons open the modal again.
fn cancel(page: Rc<Page>) {
    page.modal.dismiss();
    spawn_local(async move {
        for route in [MEASURE_CANCEL_ROUTE, LIVE_STOP_ROUTE] {
            if let Err(message) = post(&page, route).await {
                report(&crate::error::AppError::answer(message));
            }
        }
    });
}

async fn read_live(page: &Rc<Page>) -> Result<LiveStatus, String> {
    let (ok, _, text) = board::request(&page.window, LIVE_STATUS_ROUTE, "GET")
        .await
        .map_err(|error| error.to_string())?;
    if ok {
        serde_json::from_str(&text).map_err(|error| error.to_string())
    } else {
        Err(refusal_words(&text))
    }
}

async fn read_status(page: &Rc<Page>) -> Result<MeasureStatus, String> {
    let (ok, _, text) = board::request(&page.window, MEASURE_STATUS_ROUTE, "GET")
        .await
        .map_err(|error| error.to_string())?;
    if ok {
        serde_json::from_str(&text).map_err(|error| error.to_string())
    } else {
        Err(refusal_words(&text))
    }
}

async fn post(page: &Rc<Page>, route: &str) -> Result<String, String> {
    let (ok, _, text) = board::request(&page.window, route, "POST")
        .await
        .map_err(|error| error.to_string())?;
    if ok {
        Ok(text)
    } else {
        Err(refusal_words(&text))
    }
}

fn refusal_words(text: &str) -> String {
    serde_json::from_str::<Refusal>(text)
        .map_or_else(|_| text.to_string(), |refusal| refusal.message)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIMITS: (u32, u32) = (30, 9_999);

    #[test]
    fn the_box_takes_only_whole_numbers_inside_the_limits() {
        assert_eq!(parse_count("30", LIMITS), Ok(30));
        assert_eq!(parse_count(" 1000 ", LIMITS), Ok(1_000));
        assert_eq!(parse_count("9999", LIMITS), Ok(9_999));
        for typed in [
            "29",
            "0",
            "10000",
            "",
            " ",
            "abc",
            "1e3",
            "30.5",
            "-30",
            "+30",
            "99999999999",
        ] {
            assert!(parse_count(typed, LIMITS).is_err(), "{typed}");
        }
    }

    #[test]
    fn the_gap_takes_only_millimetres_inside_the_limit() {
        assert_eq!(parse_gap("5", 50.0), Ok(5.0));
        assert_eq!(parse_gap(" 5.25 ", 50.0), Ok(5.25));
        assert_eq!(parse_gap("0", 50.0), Ok(0.0));
        assert_eq!(parse_gap("50", 50.0), Ok(50.0));
        for typed in [
            "", " ", "50.1", "-5", "5mm", "5,5", "abc", "1e1", ".", "1.2.3",
        ] {
            assert!(parse_gap(typed, 50.0).is_err(), "{typed}");
        }
    }

    #[test]
    fn a_refusal_gives_its_words() {
        assert_eq!(refusal_words(r#"{"message":"no"}"#), "no");
        assert_eq!(refusal_words("plain"), "plain");
    }
}
