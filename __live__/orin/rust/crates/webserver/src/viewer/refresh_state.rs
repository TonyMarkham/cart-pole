use crate::arduino::arduino_state::ArduinoState;

use axum::body::Bytes;
use tokio::sync::watch;

/// The refresh route's state: the viewer's image channel, where the fresh
/// frame goes, and the rig's state, for the calibration service's address
/// and for whether a run holds the camera.
#[derive(Clone)]
pub(crate) struct RefreshState {
    pub(crate) images: watch::Sender<Option<Bytes>>,
    pub(crate) rig: ArduinoState,
}
