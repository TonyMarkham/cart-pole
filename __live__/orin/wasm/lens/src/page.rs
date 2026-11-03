use crate::{
    bar::Bar,
    constants::{
        HOLD_ID, IMAGE_ID, OVERLAY_ID, PICTURE_ID, PROGRESS_ID, RESTART_ID, RESULT_ID, SKIP_ID,
        SOLVE_ID, STATUS_ID, TOGGLE_ID, TURN_ID,
    },
    error::{AppError, result::AppResult},
    lens_result::LensResult,
    lens_status::LensStatus,
    render,
};

use std::cell::Cell;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlButtonElement, HtmlImageElement, Window};

/// The elements `index.html` leaves for this program to fill, and whether the
/// view was on at the last look.
pub(crate) struct Page {
    pub(crate) window: Window,
    pub(crate) toggle: HtmlButtonElement,
    pub(crate) skip: HtmlButtonElement,
    pub(crate) restart: HtmlButtonElement,
    pub(crate) solve: HtmlButtonElement,
    pub(crate) picture: HtmlButtonElement,
    status: Element,
    image: HtmlImageElement,
    overlay: Element,
    result: Element,
    progress: Bar,
    turn: Bar,
    hold: Bar,
    on: Cell<bool>,
}

impl Page {
    pub(crate) fn find(window: Window, document: &Document) -> AppResult<Self> {
        Ok(Self {
            toggle: element(document, TOGGLE_ID)?.dyn_into()?,
            skip: element(document, SKIP_ID)?.dyn_into()?,
            restart: element(document, RESTART_ID)?.dyn_into()?,
            solve: element(document, SOLVE_ID)?.dyn_into()?,
            picture: element(document, PICTURE_ID)?.dyn_into()?,
            status: element(document, STATUS_ID)?,
            image: element(document, IMAGE_ID)?.dyn_into()?,
            overlay: element(document, OVERLAY_ID)?,
            result: element(document, RESULT_ID)?,
            progress: Bar::find(document, PROGRESS_ID)?,
            turn: Bar::find(document, TURN_ID)?,
            hold: Bar::find(document, HOLD_ID)?,
            on: Cell::new(false),
            window,
        })
    }

    pub(crate) fn is_on(&self) -> bool {
        self.on.get()
    }

    /// One look at the measurement.
    pub(crate) fn show(&self, status: &LensStatus) -> AppResult<()> {
        self.on.set(status.on);
        self.toggle
            .set_text_content(Some(if status.on { "Stop" } else { "Start" }));
        self.toggle.set_disabled(false);
        self.skip.set_disabled(!status.on || status.rect.is_none());
        self.picture.set_disabled(!status.on);
        self.solve.set_disabled(status.kept < status.min_views);
        self.say(&render::line(status));
        self.progress.show(Some(&render::progress(status)))?;
        self.turn.show(render::turn(status).as_ref())?;
        self.hold.show(Some(&render::hold(status)))?;
        if status.on {
            if let Some(frame) = &status.frame {
                self.image
                    .set_src(&format!("data:image/jpeg;base64,{frame}"));
            }
            self.overlay.set_inner_html(&render::overlay(status)?);
        }
        if let Some(result) = &status.result {
            self.show_result(result);
        }
        Ok(())
    }

    pub(crate) fn show_result(&self, result: &LensResult) {
        self.result.set_text_content(Some(&render::result(result)));
    }

    /// Set as text, so nothing in a message can become part of the page.
    pub(crate) fn say(&self, message: &str) {
        self.status.set_text_content(Some(message));
    }

    pub(crate) fn say_result(&self, message: &str) {
        self.result.set_text_content(Some(message));
    }
}

pub(crate) fn element(document: &Document, id: &str) -> AppResult<Element> {
    document
        .get_element_by_id(id)
        .ok_or_else(|| AppError::missing(id))
}
