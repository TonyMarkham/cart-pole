use crate::{
    board::travel::Travel,
    constants::{
        CENTRE_BUTTON_ID, IDLER_BUTTON_ID, MEASURE_BUTTON_ID, MOTOR_BUTTON_ID, NEXT_ELEMENT_ID,
        POSE_CENTRE_HANGING_ID, POSE_IDLER_HANGING_ID, POSE_IDLER_HORIZONTAL_ID,
        POSE_IDLER_UPRIGHT_ID, POSE_MOTOR_HANGING_ID, POSE_MOTOR_HORIZONTAL_ID,
        POSE_MOTOR_UPRIGHT_ID, RAIL_ELEMENT_ID, RESULT_ELEMENT_ID, ROWS_ELEMENT_ID,
        SPEED_ELEMENT_ID, STATUS_ELEMENT_ID, STOP_BUTTON_ID, WHERE_ELEMENT_ID,
    },
    error::{app_error::AppError, app_result::AppResult},
    measure::pole::Pole,
    modal::Modal,
    render,
    session::Session,
    tone::Tone,
};

use js_sys::Date;
use std::cell::RefCell;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlButtonElement, HtmlInputElement, Window};

/// The elements `index.html` leaves for this program to fill, and what is known.
pub(crate) struct Page {
    pub(crate) window: Window,
    pub(crate) session: RefCell<Session>,
    pub(crate) modal: Modal,
    status: Element,
    rail: Element,
    place: Element,
    next: Element,
    result: Element,
    rows: Element,
    speed: HtmlInputElement,
    buttons: [(Travel, HtmlButtonElement); 3],
    stop: HtmlButtonElement,
    measure: HtmlButtonElement,
    /// Each stop's pose buttons, which open the modal for that pose.
    poses: [(Travel, Pole, HtmlButtonElement); 7],
    /// What was last written to `rail` and `rows`. Writing the drawing again
    /// restarts the belt's animation, and the table's text could not be selected.
    drawn: RefCell<(String, String)>,
}

impl Page {
    pub(crate) fn find(window: Window, document: &Document) -> AppResult<Self> {
        Ok(Self {
            session: RefCell::new(Session::new()),
            modal: Modal::find(document)?,
            status: element(document, STATUS_ELEMENT_ID)?,
            rail: element(document, RAIL_ELEMENT_ID)?,
            place: element(document, WHERE_ELEMENT_ID)?,
            next: element(document, NEXT_ELEMENT_ID)?,
            result: element(document, RESULT_ELEMENT_ID)?,
            rows: element(document, ROWS_ELEMENT_ID)?,
            speed: element(document, SPEED_ELEMENT_ID)?.dyn_into()?,
            buttons: [
                (Travel::Motor, button(document, MOTOR_BUTTON_ID)?),
                (Travel::Idler, button(document, IDLER_BUTTON_ID)?),
                (Travel::Centre, button(document, CENTRE_BUTTON_ID)?),
            ],
            stop: button(document, STOP_BUTTON_ID)?,
            measure: button(document, MEASURE_BUTTON_ID)?,
            poses: [
                (
                    Travel::Idler,
                    Pole::Hanging,
                    button(document, POSE_IDLER_HANGING_ID)?,
                ),
                (
                    Travel::Idler,
                    Pole::Horizontal,
                    button(document, POSE_IDLER_HORIZONTAL_ID)?,
                ),
                (
                    Travel::Idler,
                    Pole::Upright,
                    button(document, POSE_IDLER_UPRIGHT_ID)?,
                ),
                (
                    Travel::Motor,
                    Pole::Hanging,
                    button(document, POSE_MOTOR_HANGING_ID)?,
                ),
                (
                    Travel::Motor,
                    Pole::Horizontal,
                    button(document, POSE_MOTOR_HORIZONTAL_ID)?,
                ),
                (
                    Travel::Motor,
                    Pole::Upright,
                    button(document, POSE_MOTOR_UPRIGHT_ID)?,
                ),
                (
                    Travel::Centre,
                    Pole::Hanging,
                    button(document, POSE_CENTRE_HANGING_ID)?,
                ),
            ],
            drawn: RefCell::new((String::new(), String::new())),
            window,
        })
    }

    pub(crate) fn buttons(&self) -> &[(Travel, HtmlButtonElement); 3] {
        &self.buttons
    }

    pub(crate) fn stop_button(&self) -> &HtmlButtonElement {
        &self.stop
    }

    pub(crate) fn measure_button(&self) -> &HtmlButtonElement {
        &self.measure
    }

    pub(crate) fn pose_buttons(&self) -> &[(Travel, Pole, HtmlButtonElement); 7] {
        &self.poses
    }

    /// The typed speed, for a centring. None when the box is empty.
    pub(crate) fn speed(&self) -> Option<String> {
        let typed = self.speed.value();
        let typed = typed.trim();
        (!typed.is_empty()).then(|| typed.to_string())
    }

    /// Brings everything on the page into line with what is known. Every line is
    /// set as text, so nothing in a message can become part of the page.
    pub(crate) fn draw(&self) -> AppResult<()> {
        let session = self.session.borrow();

        let (tone, status) = session.status(Date::now());
        say(&self.status, tone, &status);
        self.place
            .set_text_content(Some(session.place().sentence()));
        self.next.set_text_content(Some(&session.next_step()));
        match session.last_result() {
            Some((tone, result)) => say(&self.result, tone, &result),
            None => self.result.set_text_content(None),
        }
        for (_, button) in &self.buttons {
            button.set_disabled(session.is_busy());
        }
        self.speed.set_disabled(session.is_busy());
        self.stop.set_disabled(!session.is_busy());
        self.measure
            .set_disabled(session.is_busy() || session.stop_here().is_none());
        // A pose can only be measured where the last move left the cart: nothing here
        // moves the cart to it.
        for (stop, _, button) in &self.poses {
            button.set_disabled(session.is_busy() || session.stop_here() != Some(*stop));
        }

        let rail = render::rail(&session)?;
        let rows = render::rows(&session.entries)?;
        let mut drawn = self.drawn.borrow_mut();
        if drawn.0 != rail {
            self.rail.set_inner_html(&rail);
            drawn.0 = rail;
        }
        if drawn.1 != rows {
            self.rows.set_inner_html(&rows);
            drawn.1 = rows;
        }
        Ok(())
    }
}

/// One line of the page, in its colour. Set as text, so nothing in a message
/// can become part of the page.
fn say(line: &Element, tone: Tone, message: &str) {
    line.set_class_name(&format!("line {}", tone.class()));
    line.set_text_content(Some(message));
}

/// The element `id` in `index.html`; a missing one is this crate's fault.
pub(crate) fn element(document: &Document, id: &str) -> AppResult<Element> {
    document
        .get_element_by_id(id)
        .ok_or_else(|| AppError::missing(id))
}

/// The button `id` in `index.html`.
pub(crate) fn button(document: &Document, id: &str) -> AppResult<HtmlButtonElement> {
    Ok(element(document, id)?.dyn_into()?)
}
