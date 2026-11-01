use crate::constants::{CENTRE_ROUTE, IDLER_ROUTE, MOTOR_ROUTE};

/// One of the three moves this page can ask for.
#[derive(Clone, Copy, Debug, PartialEq)]
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
