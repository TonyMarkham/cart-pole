use crate::run_status::RunStatus;

/// One look at `/run/status`: the status, or which program gave none and its
/// words. The webserver answers 502 when the brain does not answer it; a
/// webserver that does not answer gives the page nothing at all.
#[derive(Debug)]
pub(crate) enum Look {
    Status(RunStatus),
    NoBrain(String),
    NoWebserver(String),
}
