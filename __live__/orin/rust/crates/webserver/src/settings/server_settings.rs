use serde::Deserialize;

/// `[server]`: where the server listens, and the largest image it takes.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ServerSettings {
    /// The address and port to listen on: `0.0.0.0:8080` listens on port 8080
    /// of every network the Orin is on.
    pub(crate) address: String,
    /// The largest PNG `/image` accepts, bytes. A larger one is answered 413.
    pub(crate) max_image_bytes: usize,
}
