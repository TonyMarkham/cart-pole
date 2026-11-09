//! The viewer page: opens the webserver's WebSocket and shows each image it
//! pushes, the latest replacing the one before. The Orin has no monitor, so
//! this is where a picture made on it is seen. Its Refresh button asks the
//! webserver for a fresh camera frame, which arrives over the same socket
//! (`real-orin-plan.md`, O12).

mod constants;
mod error;
mod refresher;

// ---------------------------------------------------------------------------------------------- //

use crate::{
    constants::{CLICK_EVENT, IMAGE_ELEMENT_ID, REFRESH_BUTTON_ID, STATUS_ELEMENT_ID, WS_ROUTE},
    error::{app_error::AppError, app_result::AppResult},
    refresher::Refresher,
};

use std::rc::Rc;
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use wasm_bindgen_futures::spawn_local;
use web_sys::{Blob, HtmlImageElement, MessageEvent, Url, WebSocket, console};

fn main() {
    if let Err(error) = run() {
        report(&error);
    }
}

/// Finds the image and opens the socket, each message one image, then wires
/// the Refresh button.
fn run() -> AppResult<()> {
    let window = web_sys::window().ok_or_else(|| AppError::missing("window"))?;
    let document = window
        .document()
        .ok_or_else(|| AppError::missing("document"))?;
    let image: HtmlImageElement = document
        .get_element_by_id(IMAGE_ELEMENT_ID)
        .ok_or_else(|| AppError::missing(IMAGE_ELEMENT_ID))?
        .dyn_into()
        .map_err(JsValue::from)?;

    let host = window.location().host()?;
    let socket = WebSocket::new(&format!("ws://{host}{WS_ROUTE}"))?;

    let on_message = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
        if let Err(error) = show_image(&image, &event) {
            report(&error);
        }
    });
    socket.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
    on_message.forget();

    let refresher = Rc::new(Refresher {
        button: document
            .get_element_by_id(REFRESH_BUTTON_ID)
            .ok_or_else(|| AppError::missing(REFRESH_BUTTON_ID))?
            .dyn_into()
            .map_err(JsValue::from)?,
        status: document
            .get_element_by_id(STATUS_ELEMENT_ID)
            .ok_or_else(|| AppError::missing(STATUS_ELEMENT_ID))?,
        window,
    });
    let clicked = Rc::clone(&refresher);
    let on_click = Closure::<dyn FnMut()>::new(move || {
        let refresher = Rc::clone(&clicked);
        spawn_local(async move { refresher.refresh().await });
    });
    refresher
        .button
        .add_event_listener_with_callback(CLICK_EVENT, on_click.as_ref().unchecked_ref())?;
    // The button lasts as long as the page, so this is never to be freed.
    on_click.forget();

    Ok(())
}

/// Shows one pushed image, and frees the one before: each is a blob URL, which
/// the browser keeps until it is revoked.
fn show_image(image: &HtmlImageElement, event: &MessageEvent) -> AppResult<()> {
    let blob: Blob = event.data().dyn_into()?;
    let url = Url::create_object_url_with_blob(&blob)?;

    let previous = image.src();
    image.set_src(&url);

    if !previous.is_empty() {
        Url::revoke_object_url(&previous)?;
    }

    Ok(())
}

/// A page has nowhere else to say what went wrong: the browser's console.
fn report(error: &AppError) {
    console::error_1(&JsValue::from_str(&error.to_string()));
}
