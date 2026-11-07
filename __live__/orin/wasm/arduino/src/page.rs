use crate::{
    constants::{BOARD_ELEMENT_ID, CONNECTION_ELEMENT_ID, ROWS_ELEMENT_ID},
    error::{app_error::AppError, app_result::AppResult},
    pins::levels::Levels,
    render,
};

use web_sys::{Document, Element, Window};

/// The elements `index.html` leaves for this program to fill.
pub(crate) struct Page {
    pub(crate) window: Window,
    board: Element,
    rows: Element,
    connection: Element,
}

impl Page {
    pub(crate) fn find(window: Window, document: &Document) -> AppResult<Self> {
        Ok(Self {
            board: element(document, BOARD_ELEMENT_ID)?,
            rows: element(document, ROWS_ELEMENT_ID)?,
            connection: element(document, CONNECTION_ELEMENT_ID)?,
            window,
        })
    }

    /// Draws the board and the table for one reading, or for none.
    pub(crate) fn show(&self, levels: Option<&Levels>) -> AppResult<()> {
        let drawn = render::board(levels).and_then(|board| Ok((board, render::rows(levels)?)));
        let (board, rows) = drawn.map_err(|error| AppError::answer(error.to_string()))?;
        self.board.set_inner_html(&board);
        self.rows.set_inner_html(&rows);
        Ok(())
    }

    /// The line above the drawing. Set as text, so nothing in a message can
    /// become part of the page.
    pub(crate) fn say(&self, class: &str, message: &str) {
        self.connection.set_class_name(class);
        self.connection.set_text_content(Some(message));
    }
}

/// The element `id` in `index.html`; a missing one is this crate's fault.
pub(crate) fn element(document: &Document, id: &str) -> AppResult<Element> {
    document
        .get_element_by_id(id)
        .ok_or_else(|| AppError::missing(id))
}
