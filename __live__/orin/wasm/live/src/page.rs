use crate::{
    constants::{BOXES_ID, BUTTON_ID, IMAGE_ID, ROWS_ID, STATUS_ID},
    error::{AppError, result::AppResult},
    live_status::LiveStatus,
    render,
};

use std::cell::Cell;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlButtonElement, HtmlImageElement, Window};

/// The elements `index.html` leaves for this program to fill, and whether the
/// view was on at the last look.
pub(crate) struct Page {
    pub(crate) window: Window,
    pub(crate) button: HtmlButtonElement,
    status: Element,
    image: HtmlImageElement,
    boxes: Element,
    rows: Element,
    on: Cell<bool>,
}

impl Page {
    pub(crate) fn find(window: Window, document: &Document) -> AppResult<Self> {
        Ok(Self {
            button: element(document, BUTTON_ID)?.dyn_into()?,
            status: element(document, STATUS_ID)?,
            image: element(document, IMAGE_ID)?.dyn_into()?,
            boxes: element(document, BOXES_ID)?,
            rows: element(document, ROWS_ID)?,
            on: Cell::new(false),
            window,
        })
    }

    pub(crate) fn is_on(&self) -> bool {
        self.on.get()
    }

    /// One look at the view.
    pub(crate) fn show(&self, status: &LiveStatus) -> AppResult<()> {
        self.on.set(status.on);
        self.button
            .set_text_content(Some(if status.on { "Stop" } else { "Start" }));
        self.button.set_disabled(false);
        let line = if status.on {
            format!(
                "Live: {:.0} frames a second detected, {} so far.",
                status.per_s, status.frames
            )
        } else if status.message.is_empty() {
            "Off. Press Start to see the camera.".to_string()
        } else {
            format!("Off: {}. Press Start to see the camera.", status.message)
        };
        self.say(&line);
        if status.on {
            if let Some(frame) = &status.frame {
                self.image
                    .set_src(&format!("data:image/jpeg;base64,{frame}"));
            }
            self.boxes.set_inner_html(&render::boxes(status)?);
            self.rows.set_inner_html(&render::rows(status)?);
        }
        Ok(())
    }

    /// Set as text, so nothing in a message can become part of the page.
    pub(crate) fn say(&self, message: &str) {
        self.status.set_text_content(Some(message));
    }
}

fn element(document: &Document, id: &str) -> AppResult<Element> {
    document
        .get_element_by_id(id)
        .ok_or_else(|| AppError::missing(id))
}
