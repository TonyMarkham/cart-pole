pub(crate) mod modal_screen;

// ---------------------------------------------------------------------------------------------- //

use crate::{
    board::travel::Travel,
    constants::{
        MODAL_ACCEPT_FIT_ID, MODAL_BOXES_ID, MODAL_CALIBRATE_ID, MODAL_CANCEL_ID, MODAL_CLOSE_ID,
        MODAL_COUNT_ERROR_ID, MODAL_COUNT_HELP_ID, MODAL_COUNT_ID, MODAL_DONE_ID, MODAL_FIT_ID,
        MODAL_FIT_TEXT_ID, MODAL_GAP_ERROR_ID, MODAL_GAP_ID, MODAL_GAP_ROW_ID, MODAL_ID,
        MODAL_IMAGE_ID, MODAL_LIVE_BOXES_ID, MODAL_LIVE_IMAGE_ID, MODAL_LIVE_TEXT_ID,
        MODAL_PROBLEM_ID, MODAL_PROBLEM_TEXT_ID, MODAL_PROGRESS_ID, MODAL_PROGRESS_TEXT_ID,
        MODAL_RETRY_ID, MODAL_RUNNING_ID, MODAL_STEADY_ID, MODAL_STEADY_TEXT_ID, MODAL_SUMMARY_ID,
        MODAL_SUMMARY_TEXT_ID, MODAL_TITLE_ID,
    },
    error::app_result::AppResult,
    measure::{
        fit_report::FitReport, live_status::LiveStatus, measure_status::MeasureStatus,
        measure_summary::MeasureSummary, pole::Pole,
    },
    modal::modal_screen::ModalScreen,
    page::{button, element},
    render,
};

use std::cell::Cell;
use wasm_bindgen::JsCast;
use web_sys::{
    Document, Element, HtmlButtonElement, HtmlDialogElement, HtmlImageElement, HtmlInputElement,
};

/// The modal's elements, and the one pose it is about.
pub(crate) struct Modal {
    pub(crate) dialog: HtmlDialogElement,
    title: Element,
    screens: [(ModalScreen, Element); 5],
    live_image: HtmlImageElement,
    live_boxes: Element,
    live_text: Element,
    steady_text: Element,
    gap_row: Element,
    gap: HtmlInputElement,
    gap_error: Element,
    count: HtmlInputElement,
    count_help: Element,
    count_error: Element,
    progress: Element,
    progress_text: Element,
    image: HtmlImageElement,
    boxes: Element,
    summary: Element,
    fit: Element,
    problem: Element,
    pub(crate) calibrate: HtmlButtonElement,
    pub(crate) done: HtmlButtonElement,
    pub(crate) retry: HtmlButtonElement,
    pub(crate) cancel: HtmlButtonElement,
    pub(crate) close: HtmlButtonElement,
    pub(crate) accept_fit: HtmlButtonElement,
    stop: Cell<Option<Travel>>,
    pole: Cell<Pole>,
    /// Changes whenever the modal moves on, so a look at the camera that was on its
    /// way when that happened knows to do nothing.
    generation: Cell<u32>,
    limits: Cell<(u32, u32)>,
    max_gap_mm: Cell<f64>,
    poll_ms: Cell<u32>,
}

impl Modal {
    pub(crate) fn find(document: &Document) -> AppResult<Self> {
        Ok(Self {
            dialog: element(document, MODAL_ID)?.dyn_into()?,
            title: element(document, MODAL_TITLE_ID)?,
            screens: [
                (ModalScreen::Steady, element(document, MODAL_STEADY_ID)?),
                (ModalScreen::Running, element(document, MODAL_RUNNING_ID)?),
                (ModalScreen::Summary, element(document, MODAL_SUMMARY_ID)?),
                (ModalScreen::Fit, element(document, MODAL_FIT_ID)?),
                (ModalScreen::Problem, element(document, MODAL_PROBLEM_ID)?),
            ],
            live_image: element(document, MODAL_LIVE_IMAGE_ID)?.dyn_into()?,
            live_boxes: element(document, MODAL_LIVE_BOXES_ID)?,
            live_text: element(document, MODAL_LIVE_TEXT_ID)?,
            steady_text: element(document, MODAL_STEADY_TEXT_ID)?,
            gap_row: element(document, MODAL_GAP_ROW_ID)?,
            gap: element(document, MODAL_GAP_ID)?.dyn_into()?,
            gap_error: element(document, MODAL_GAP_ERROR_ID)?,
            count: element(document, MODAL_COUNT_ID)?.dyn_into()?,
            count_help: element(document, MODAL_COUNT_HELP_ID)?,
            count_error: element(document, MODAL_COUNT_ERROR_ID)?,
            progress: element(document, MODAL_PROGRESS_ID)?,
            progress_text: element(document, MODAL_PROGRESS_TEXT_ID)?,
            image: element(document, MODAL_IMAGE_ID)?.dyn_into()?,
            boxes: element(document, MODAL_BOXES_ID)?,
            summary: element(document, MODAL_SUMMARY_TEXT_ID)?,
            fit: element(document, MODAL_FIT_TEXT_ID)?,
            problem: element(document, MODAL_PROBLEM_TEXT_ID)?,
            calibrate: button(document, MODAL_CALIBRATE_ID)?,
            done: button(document, MODAL_DONE_ID)?,
            retry: button(document, MODAL_RETRY_ID)?,
            cancel: button(document, MODAL_CANCEL_ID)?,
            close: button(document, MODAL_CLOSE_ID)?,
            accept_fit: button(document, MODAL_ACCEPT_FIT_ID)?,
            stop: Cell::new(None),
            pole: Cell::new(Pole::Hanging),
            generation: Cell::new(0),
            limits: Cell::new((0, 0)),
            max_gap_mm: Cell::new(0.0),
            poll_ms: Cell::new(0),
        })
    }

    pub(crate) fn set_poll_ms(&self, poll_ms: u32) {
        self.poll_ms.set(poll_ms);
    }

    pub(crate) fn poll_ms(&self) -> u32 {
        self.poll_ms.get()
    }

    pub(crate) fn stop(&self) -> Option<Travel> {
        self.stop.get()
    }

    pub(crate) fn pole(&self) -> Pole {
        self.pole.get()
    }

    pub(crate) fn generation(&self) -> u32 {
        self.generation.get()
    }

    pub(crate) fn limits(&self) -> (u32, u32) {
        self.limits.get()
    }

    pub(crate) fn max_gap_mm(&self) -> f64 {
        self.max_gap_mm.get()
    }

    /// Marks whatever was under way as over.
    pub(crate) fn move_on(&self) -> u32 {
        let generation = self.generation.get().wrapping_add(1);
        self.generation.set(generation);
        generation
    }

    /// Opens on the live view, for one pose, with the service's limits. A person
    /// cannot press what is behind it.
    pub(crate) fn open(&self, stop: Travel, pole: Pole, status: &MeasureStatus) -> AppResult<()> {
        self.move_on();
        self.stop.set(Some(stop));
        self.pole.set(pole);
        self.title
            .set_text_content(Some(&format!("{}, {}", stop.name(), pole.name(stop))));
        self.steady_text
            .set_text_content(Some(pole.instruction(stop)));
        let _ = self.live_image.remove_attribute("src");
        self.live_boxes.set_inner_html("");
        self.live_text
            .set_text_content(Some("Starting the camera."));
        self.gap.set_value("");
        self.gap_error.set_text_content(None);
        // Only the two ends have a rail support beside them to measure to.
        if stop == Travel::Centre {
            let _ = self.gap_row.set_attribute("hidden", "");
        } else {
            let _ = self.gap_row.remove_attribute("hidden");
        }
        if let Some(&gap_mm) = status.gaps_mm.get(stop.key()) {
            self.gap.set_value(&gap_mm.to_string());
        }
        self.max_gap_mm.set(status.max_gap_mm);
        let limits = (status.min_images, status.max_images);
        self.limits.set(limits);
        self.count.set_value(&limits.0.to_string());
        self.count_help.set_text_content(Some(&format!(
            "How many images to measure from, {} to {}. Only an image with both Pivot and End in it counts.",
            limits.0, limits.1
        )));
        self.count_error.set_text_content(None);
        self.calibrate.set_disabled(false);
        self.show(ModalScreen::Steady);
        if !self.dialog.open() {
            self.dialog.show_modal()?;
        }
        Ok(())
    }

    pub(crate) fn dismiss(&self) {
        self.move_on();
        self.stop.set(None);
        self.dialog.close();
    }

    /// Shows one screen, and the buttons that belong to it.
    pub(crate) fn show(&self, screen: ModalScreen) {
        for (name, element) in &self.screens {
            if *name == screen {
                let _ = element.remove_attribute("hidden");
            } else {
                let _ = element.set_attribute("hidden", "");
            }
        }
        // Cancel is on every screen, so no one is ever stuck here. The fit's result
        // is already final, so it has only Close.
        self.cancel.set_hidden(screen == ModalScreen::Fit);
        self.close.set_hidden(screen != ModalScreen::Fit);
        // Shown again by `show_fit` only for a fit someone may still accept.
        self.accept_fit.set_hidden(true);
        self.calibrate.set_hidden(screen != ModalScreen::Steady);
    }

    /// One look at the live view: the frame, every marker's box, and how it is going.
    pub(crate) fn show_live(&self, status: &LiveStatus) -> AppResult<()> {
        if !status.on {
            let line = if status.message.is_empty() {
                "The live view is off.".to_string()
            } else {
                format!("The live view is off: {}.", status.message)
            };
            self.live_text.set_text_content(Some(&line));
            return Ok(());
        }
        if let Some(frame) = &status.frame {
            self.live_image
                .set_src(&format!("data:image/jpeg;base64,{frame}"));
        }
        self.live_boxes.set_inner_html(&render::live_boxes(status)?);
        self.live_text
            .set_text_content(Some(&render::live_line(status)));
        Ok(())
    }

    pub(crate) fn typed_gap(&self) -> String {
        self.gap.value()
    }

    pub(crate) fn say_gap_error(&self, message: &str) {
        self.gap_error.set_text_content(Some(message));
    }

    pub(crate) fn typed_count(&self) -> String {
        self.count.value()
    }

    pub(crate) fn say_count_error(&self, message: &str) {
        self.count_error.set_text_content(Some(message));
    }

    pub(crate) fn clear_errors(&self) {
        self.gap_error.set_text_content(None);
        self.count_error.set_text_content(None);
    }

    /// The progress screen, starting empty.
    pub(crate) fn show_running(&self, target: u32) {
        let _ = self.image.remove_attribute("src");
        self.boxes.set_inner_html("");
        let _ = self.progress.set_attribute("max", &target.to_string());
        let _ = self.progress.set_attribute("value", "0");
        self.progress_text
            .set_text_content(Some("Letting the camera settle."));
        self.show(ModalScreen::Running);
    }

    pub(crate) fn update_running(&self, status: &MeasureStatus) -> AppResult<()> {
        if status.state == "settling" {
            return Ok(());
        }
        let _ = self
            .progress
            .set_attribute("value", &status.count.to_string());
        self.progress_text
            .set_text_content(Some(&format!("{} / {}", status.count, status.target)));
        if let Some(frame) = &status.frame {
            self.image
                .set_src(&format!("data:image/jpeg;base64,{frame}"));
        }
        if let Some(boxes) = &status.boxes {
            self.boxes
                .set_inner_html(&render::boxes(boxes, status.width, status.height)?);
        }
        Ok(())
    }

    pub(crate) fn show_summary(&self, summary: &MeasureSummary) -> AppResult<()> {
        self.summary.set_inner_html(&render::summary(summary)?);
        self.show(ModalScreen::Summary);
        Ok(())
    }

    pub(crate) fn show_fit(&self, report: &FitReport) -> AppResult<()> {
        self.title.set_text_content(Some("The fit"));
        self.fit.set_inner_html(&render::fit(report)?);
        self.show(ModalScreen::Fit);
        self.accept_fit.set_disabled(false);
        self.accept_fit
            .set_hidden(report.within_limit || report.accepted);
        Ok(())
    }

    /// What came of pressing Accept this fit, said under the fit.
    pub(crate) fn show_fit_accepted(&self, outcome: Result<String, String>) -> AppResult<()> {
        let (class, words) = match outcome {
            Ok(said) => {
                self.accept_fit.set_hidden(true);
                ("good", said)
            }
            Err(said) => {
                self.accept_fit.set_disabled(false);
                ("bad", said)
            }
        };
        let note = self
            .dialog
            .owner_document()
            .and_then(|document| document.create_element("p").ok());
        if let Some(note) = note {
            note.set_class_name(class);
            // As text, so nothing in the answer can become part of the page.
            note.set_text_content(Some(&words));
            self.fit.append_child(&note)?;
        }
        Ok(())
    }

    /// A fit that cannot run yet. Everything measured so far is kept.
    pub(crate) fn show_fit_refused(&self, message: &str) {
        self.title.set_text_content(Some("The fit"));
        self.fit.set_text_content(Some(message));
        self.show(ModalScreen::Fit);
    }

    /// Set as text, so nothing in a message can become part of the page.
    pub(crate) fn show_problem(&self, message: &str) {
        self.problem.set_text_content(Some(message));
        self.show(ModalScreen::Problem);
    }
}
