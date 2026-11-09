use crate::trainer_link::{pole_message::PoleMessage, sites_message::SitesMessage};

use physics::Pose;

pub(crate) struct Capture {
    pub(crate) time: f64,
    pub(crate) pose: Pose,
    pub(crate) sites: SitesMessage,
    pub(crate) pole: PoleMessage,
}
