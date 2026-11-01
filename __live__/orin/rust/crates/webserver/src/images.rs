use crate::constants::PNG_SIGNATURE;

use axum::{body::Bytes, extract::State, http::StatusCode};
use tokio::sync::watch;

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
