/// What the modal is showing. Each has its own part of `index.html`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum ModalScreen {
    /// Asks the person to steady the pole.
    Steady,
    /// Asks how many images to measure from.
    Form,
    /// The progress bar and the slide show.
    Running,
    /// How the stop's measurement went.
    Summary,
    /// The matrix's fit, once all three stops are measured.
    Fit,
    /// Something went wrong, in words.
    Problem,
}
