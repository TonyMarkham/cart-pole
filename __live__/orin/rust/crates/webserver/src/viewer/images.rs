use crate::constants::PNG_SIGNATURE;

use axum::{body::Bytes, extract::State, http::StatusCode};
use tokio::sync::watch;

/// Takes one PNG and hands it to every viewer, replacing the one before. Only
/// the first 8 bytes are checked: anything else is answered 415.
pub(crate) async fn push_image(
    State(images): State<watch::Sender<Option<Bytes>>>,
    image: Bytes,
) -> StatusCode {
    if !image.starts_with(PNG_SIGNATURE) {
        return StatusCode::UNSUPPORTED_MEDIA_TYPE;
    }

    images.send_replace(Some(image));

    StatusCode::NO_CONTENT
}
