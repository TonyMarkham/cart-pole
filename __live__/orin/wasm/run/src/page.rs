use crate::{
    constants::{
        BOARD_ID, BRAIN_IDLE, BRAIN_RUN, CENTRE_ID, FRAMES_ID, MESSAGE_ID, RESULT_ID, RUN_DIR_ID,
        RUN_ID, STATE_ID, STOP_ID, WHY_ID,
    },
    error::{app_error::AppError, app_result::AppResult},
    look::Look,
    run_rule,
};

use std::cell::Cell;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlButtonElement, Window};

/// The elements `index.html` leaves for this program, and whether a Run or a
/// Re-centre is on its way, so a look that lands meanwhile can't enable
/// either: the board takes one of them at a time.
pub(crate) struct Page {
    pub(crate) window: Window,
    pub(crate) run: HtmlButtonElement,
    pub(crate) centre: HtmlButtonElement,
    pub(crate) stop: HtmlButtonElement,
    why: Element,
    result: Element,
    state: Element,
    message: Element,
    frames: Element,
    run_dir: Element,
    board: Element,
    pending: Cell<bool>,
}

impl Page {
    pub(crate) fn find(window: Window, document: &Document) -> AppResult<Self> {
        Ok(Self {
            run: element(document, RUN_ID)?.dyn_into()?,
            centre: element(document, CENTRE_ID)?.dyn_into()?,
            stop: element(document, STOP_ID)?.dyn_into()?,
            why: element(document, WHY_ID)?,
            result: element(document, RESULT_ID)?,
            state: element(document, STATE_ID)?,
            message: element(document, MESSAGE_ID)?,
            frames: element(document, FRAMES_ID)?,
            run_dir: element(document, RUN_DIR_ID)?,
            board: element(document, BOARD_ID)?,
            pending: Cell::new(false),
            window,
        })
    }

    /// Marks a Run or a Re-centre as on its way, saying which, or as back
    /// with `None`.
    pub(crate) fn set_pending(&self, doing: Option<&str>) {
        self.pending.set(doing.is_some());
        if let Some(doing) = doing {
            self.run.set_disabled(true);
            self.centre.set_disabled(true);
            text(&self.why, doing);
        }
    }

    /// One look's worth of state. STOP is never touched here: it is always
    /// pressable, whatever the brain or the board says.
    pub(crate) fn show(&self, look: &Look) {
        let why = run_rule::why_not(look);
        // Re-centre follows Run's rule (`__documentation__/real-loop-dia.md`, *The run and idle
        // states*, Re-centre).
        let off = why.is_some() || self.pending.get();
        self.run.set_disabled(off);
        self.centre.set_disabled(off);
        if !self.pending.get() {
            text(
                &self.why,
                why.as_deref()
                    .unwrap_or("The board is calibrated and the brain is idle."),
            );
        }

        match look {
            Look::Status(status) => {
                let state = match status.state.as_str() {
                    BRAIN_RUN => "Running",
                    BRAIN_IDLE => "Idle",
                    other => other,
                };
                text(&self.state, state);
                text(
                    &self.message,
                    if status.message.is_empty() {
                        "None yet."
                    } else {
                        &status.message
                    },
                );
                text(&self.frames, &status.frames.to_string());
                text(
                    &self.run_dir,
                    status.run_dir.as_deref().unwrap_or("None yet."),
                );
                let board = match (&status.board, &status.board_error) {
                    (Some(board), _) => format!("{}: {}", board.result, board.message),
                    (None, Some(reason)) => reason.clone(),
                    (None, None) => {
                        "Not asked during a run: the brain alone talks to the board then."
                            .to_string()
                    }
                };
                text(&self.board, &board);
            }
            Look::NoBrain(words) | Look::NoWebserver(words) => {
                text(&self.state, &format!("Not known: {words}"));
                for unknown in [&self.message, &self.frames, &self.run_dir, &self.board] {
                    text(unknown, "Not known.");
                }
            }
        }
    }

    /// What the last press did.
    pub(crate) fn say(&self, words: &str) {
        text(&self.result, words);
    }
}

/// Set as text, so nothing in a message can become part of the page.
fn text(element: &Element, words: &str) {
    element.set_text_content(Some(words));
}

/// The element `id` in `index.html`; a missing one is this crate's fault.
fn element(document: &Document, id: &str) -> AppResult<Element> {
    document
        .get_element_by_id(id)
        .ok_or_else(|| AppError::missing(id))
}
