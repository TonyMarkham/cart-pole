use crate::{
    constants::{
        MODAL_BOXES_ID, MODAL_CANCEL_ID, MODAL_CLOSE_ID, MODAL_COUNT_ERROR_ID, MODAL_COUNT_HELP_ID,
        MODAL_COUNT_ID, MODAL_DONE_ID, MODAL_FIT_ID, MODAL_FIT_TEXT_ID, MODAL_FORM_ID,
        MODAL_GAP_ERROR_ID, MODAL_GAP_ID, MODAL_GAP_ROW_ID, MODAL_ID, MODAL_IMAGE_ID,
        MODAL_NEXT_ID, MODAL_PROBLEM_ID, MODAL_PROBLEM_TEXT_ID, MODAL_PROGRESS_ID,
        MODAL_PROGRESS_TEXT_ID, MODAL_RETRY_ID, MODAL_RUNNING_ID, MODAL_START_ID, MODAL_STEADY_ID,
        MODAL_SUMMARY_ID, MODAL_SUMMARY_TEXT_ID, MODAL_TITLE_ID,
    },
    error::result::AppResult,
    fit_report::FitReport,
    measure_status::MeasureStatus,
    measure_summary::MeasureSummary,
    modal_screen::ModalScreen,
    page::{button, element},
    render,
    travel::Travel,
};

use std::cell::Cell;
use wasm_bindgen::JsCast;
use web_sys::{
    Document, Element, HtmlButtonElement, HtmlDialogElement, HtmlImageElement, HtmlInputElement,
};

/// The modal's elements, and the one measurement it is about.
pub(crate) struct Modal {
    pub(crate) dialog: HtmlDialogElement,
    title: Element,
    screens: [(ModalScreen, Element); 6],
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
    pub(crate) next: HtmlButtonElement,
    pub(crate) start: HtmlButtonElement,
    pub(crate) done: HtmlButtonElement,
    pub(crate) retry: HtmlButtonElement,
    pub(crate) cancel: HtmlButtonElement,
    pub(crate) close: HtmlButtonElement,
    stop: Cell<Option<Travel>>,
    /// Changes whenever the modal moves on, so a look at a measurement that was
    /// on its way when that happened knows to do nothing.
    generation: Cell<u32>,
    limits: Cell<(u32, u32)>,
    /// The gap typed for this stop, once NEXT has accepted it.
    gap_mm: Cell<Option<f64>>,
    poll_ms: Cell<u32>,
}

impl Modal {
    pub(crate) fn find(document: &Document) -> AppResult<Self> {
        Ok(Self {
            dialog: element(document, MODAL_ID)?.dyn_into()?,
            title: element(document, MODAL_TITLE_ID)?,
            screens: [
                (ModalScreen::Steady, element(document, MODAL_STEADY_ID)?),
                (ModalScreen::Form, element(document, MODAL_FORM_ID)?),
                (ModalScreen::Running, element(document, MODAL_RUNNING_ID)?),
                (ModalScreen::Summary, element(document, MODAL_SUMMARY_ID)?),
                (ModalScreen::Fit, element(document, MODAL_FIT_ID)?),
                (ModalScreen::Problem, element(document, MODAL_PROBLEM_ID)?),
            ],
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
            next: button(document, MODAL_NEXT_ID)?,
            start: button(document, MODAL_START_ID)?,
            done: button(document, MODAL_DONE_ID)?,
            retry: button(document, MODAL_RETRY_ID)?,
            cancel: button(document, MODAL_CANCEL_ID)?,
            close: button(document, MODAL_CLOSE_ID)?,
            stop: Cell::new(None),
            generation: Cell::new(0),
            limits: Cell::new((0, 0)),
            gap_mm: Cell::new(None),
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

    pub(crate) fn generation(&self) -> u32 {
        self.generation.get()
    }

    pub(crate) fn limits(&self) -> (u32, u32) {
        self.limits.get()
    }

    /// Marks whatever was under way as over.
    pub(crate) fn move_on(&self) -> u32 {
        let generation = self.generation.get().wrapping_add(1);
        self.generation.set(generation);
        generation
    }

    /// Opens on the first screen. A person cannot press what is behind it.
    pub(crate) fn open(&self, stop: Travel) -> AppResult<()> {
        self.move_on();
        self.stop.set(Some(stop));
        self.title.set_text_content(Some(stop.name()));
        self.gap_mm.set(None);
        self.gap.set_value("");
        self.gap_error.set_text_content(None);
        // Only the two ends have a rail support beside them to measure to.
        if stop == Travel::Centre {
            let _ = self.gap_row.set_attribute("hidden", "");
        } else {
            let _ = self.gap_row.remove_attribute("hidden");
        }
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
    }

    /// The form, with the box at its smallest allowed number.
    pub(crate) fn show_form(&self, limits: (u32, u32)) {
        self.limits.set(limits);
        self.count.set_value(&limits.0.to_string());
        self.count_help.set_text_content(Some(&format!(
            "How many images to measure from, {} to {}. Only an image with both Pivot and End in it counts.",
            limits.0, limits.1
        )));
        self.count_error.set_text_content(None);
        self.start.set_disabled(false);
        self.show(ModalScreen::Form);
    }

    /// Fills the box with this end's last gap, unless something has been typed.
    pub(crate) fn offer_gap(&self, gap_mm: f64) {
        if self.gap.value().is_empty() {
            self.gap.set_value(&gap_mm.to_string());
        }
    }

    pub(crate) fn typed_gap(&self) -> String {
        self.gap.value()
    }

    pub(crate) fn say_gap_error(&self, message: &str) {
        self.gap_error.set_text_content(Some(message));
    }

    pub(crate) fn keep_gap(&self, gap_mm: Option<f64>) {
        self.gap_error.set_text_content(None);
        self.gap_mm.set(gap_mm);
    }

    pub(crate) fn gap_mm(&self) -> Option<f64> {
        self.gap_mm.get()
    }

    pub(crate) fn typed_count(&self) -> String {
        self.count.value()
    }

    pub(crate) fn say_count_error(&self, message: &str) {
        self.count_error.set_text_content(Some(message));
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
