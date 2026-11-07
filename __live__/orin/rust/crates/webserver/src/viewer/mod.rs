//! The viewer's two routes: `/image` takes a PNG from `image-push`, and `/ws`
//! pushes the latest one to every open viewer page. Only the latest image is
//! kept, in a `watch` channel, so a viewer that opens late still gets it.

pub(crate) mod images;
pub(crate) mod ws;
