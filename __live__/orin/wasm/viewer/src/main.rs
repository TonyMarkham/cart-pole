mod constants;
mod error;

// ---------------------------------------------------------------------------------------------- //

pub(crate) use crate::constants::{IMAGE_ELEMENT_ID, WS_ROUTE};

// ---------------------------------------------------------------------------------------------- //

use crate::error::{AppError, result::AppResult};

use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use web_sys::{Blob, HtmlImageElement, MessageEvent, Url, WebSocket, console};

fn main() {
    if let Err(error) = run() {
        report(&error);
    }
}

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

    Ok(())
}

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

fn report(error: &AppError) {
    console::error_1(&JsValue::from_str(&error.to_string()));
}
