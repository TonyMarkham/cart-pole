/// What the modal is showing. Each has its own part of `index.html`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum ModalScreen {
    /// The live view, while the person holds the pole in the pose; the gap and the
    /// number of images; Calibrate now.
    Steady,
    /// The progress bar and the slide show.
    Running,
    /// How the stop's measurement went.
    Summary,
    /// The matrix's fit, once all three stops are measured.
    Fit,
    /// Something went wrong, in words.
    Problem,
}
