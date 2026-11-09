use crate::{
    calibrate::rail_state::RailState,
    settings::{
        arduino_settings::ArduinoSettings, calibration_settings::CalibrationSettings,
        run_settings::RunSettings,
    },
};

use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};

/// The pages' route state: the settings, a lock held for each reading, and the
/// one permit a move holds until it has ended. The board answers whoever sent
/// the latest datagram, so a datagram sent while it is answering another takes
/// that answer. The lock and the permit keep this server's exchanges with the
/// board one at a time. Worked failure, 2026-11-01: a move sent during a reading
/// was handed the pin levels as its answer.
#[derive(Clone)]
pub(crate) struct ArduinoState {
    pub(crate) settings: ArduinoSettings,
    pub(crate) calibration: CalibrationSettings,
    pub(crate) run: RunSettings,
    pub(crate) reading: Arc<Mutex<()>>,
    pub(crate) moving: Arc<Semaphore>,
    pub(crate) rail: Arc<Mutex<RailState>>,
}

impl ArduinoState {
    pub(crate) fn new(
        settings: ArduinoSettings,
        calibration: CalibrationSettings,
        run: RunSettings,
    ) -> Self {
        Self {
            settings,
            calibration,
            run,
            reading: Arc::new(Mutex::new(())),
            moving: Arc::new(Semaphore::new(1)),
            rail: Arc::new(Mutex::new(RailState::default())),
        }
    }
}
