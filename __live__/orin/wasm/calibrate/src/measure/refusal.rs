use serde::Deserialize;

/// What the webserver or the service says when it refuses something.
#[derive(Deserialize)]
pub(crate) struct Refusal {
    pub(crate) message: String,
}
