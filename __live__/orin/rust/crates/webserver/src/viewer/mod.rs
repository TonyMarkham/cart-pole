//! The viewer's routes: `/image` takes a PNG from `image-push`,
//! `/image/refresh` takes a fresh camera frame from the live view, and `/ws`
//! pushes the latest image to every open viewer page. Only the latest image is
//! kept, in a `watch` channel, so a viewer that opens late still gets it.

pub(crate) mod base64;
pub(crate) mod images;
pub(crate) mod refresh;
pub(crate) mod refresh_state;
pub(crate) mod ws;
