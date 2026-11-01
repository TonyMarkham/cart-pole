use crate::constants::{CENTRE_ROUTE, IDLER_ROUTE, MOTOR_ROUTE};

use serde::Deserialize;

/// One of the three moves this page can ask for, and the stop it ends at. The
/// names are the ones the webserver and `calibrate_service.py` use.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Travel {
    Motor,
    Idler,
    Centre,
}

impl Travel {
    pub(crate) fn route(self) -> &'static str {
        match self {
            Travel::Motor => MOTOR_ROUTE,
            Travel::Idler => IDLER_ROUTE,
            Travel::Centre => CENTRE_ROUTE,
        }
    }

    /// The stop's name in the measuring routes.
    pub(crate) fn key(self) -> &'static str {
        match self {
            Travel::Motor => "motor",
            Travel::Idler => "idler",
            Travel::Centre => "centre",
        }
    }

    /// The table's name for this move.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Travel::Motor => "Motor end",
            Travel::Idler => "Idler end",
            Travel::Centre => "Centre",
        }
    }

    /// The move, finishing the sentence "Moving …".
    pub(crate) fn heading(self) -> &'static str {
        match self {
            Travel::Motor => "towards the motor end",
            Travel::Idler => "towards the idler end",
            Travel::Centre => "to the centre",
        }
    }
}
