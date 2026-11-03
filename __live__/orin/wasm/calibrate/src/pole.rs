use crate::travel::Travel;

/// What the pole does in one of the seven poses (`fit.py`'s `POSES`, from
/// `synthetic-loop-dia.md`, *The seven poses*). The hanging poses alone cover only
/// the rail and the hanging pole, while a swinging End reaches the pole's length past
/// them on either side and above, so at each end it is also held out and up.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Pole {
    Hanging,
    /// Level, pointing away from the rail's centre.
    Horizontal,
    Upright,
}

impl Pole {
    /// The poses at a stop, in the order the window offers them. The centre has only
    /// the hanging one.
    pub(crate) fn at(stop: Travel) -> &'static [Pole] {
        match stop {
            Travel::Centre => &[Pole::Hanging],
            Travel::Motor | Travel::Idler => &[Pole::Hanging, Pole::Horizontal, Pole::Upright],
        }
    }

    /// The pole's name in the measuring routes.
    pub(crate) fn key(self) -> &'static str {
        match self {
            Pole::Hanging => "hanging",
            Pole::Horizontal => "horizontal",
            Pole::Upright => "upright",
        }
    }

    /// The pose's name in `calibrate_service.py`'s lists: "idler upright".
    pub(crate) fn pose_key(self, stop: Travel) -> String {
        format!("{} {}", stop.key(), self.key())
    }

    /// The pose, finishing "Idler end, …". The idler's end is on the left as the
    /// camera sees it, so held out there the pole points left.
    pub(crate) fn name(self, stop: Travel) -> &'static str {
        match (self, stop) {
            (Pole::Hanging, _) => "pole hanging",
            (Pole::Horizontal, Travel::Idler) => "pole level, pointing left",
            (Pole::Horizontal, _) => "pole level, pointing right",
            (Pole::Upright, _) => "pole straight up",
        }
    }

    /// What the person does before pressing Calibrate now.
    pub(crate) fn instruction(self, stop: Travel) -> &'static str {
        match (self, stop) {
            (Pole::Hanging, _) => {
                "Steady the pole with your hand until it hangs still, then get clear of the \
                 camera's view. Press Calibrate now when it is steady."
            }
            (Pole::Horizontal, Travel::Idler) => {
                "Hold the pole level, pointing left as the camera sees it, away from the rail's \
                 centre. Hold it by its rod so your hand covers neither marker, keep the rest of \
                 you out of the camera's view, and keep it still until the measurement has ended. \
                 Press Calibrate now when it is level."
            }
            (Pole::Horizontal, _) => {
                "Hold the pole level, pointing right as the camera sees it, away from the rail's \
                 centre. Hold it by its rod so your hand covers neither marker, keep the rest of \
                 you out of the camera's view, and keep it still until the measurement has ended. \
                 Press Calibrate now when it is level."
            }
            (Pole::Upright, _) => {
                "Hold the pole straight up, above the cart. Hold it by its rod so your hand covers \
                 neither marker, keep the rest of you out of the camera's view, and keep it still \
                 until the measurement has ended. Press Calibrate now when it is straight up."
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn there_are_seven_poses_and_the_centre_has_one() {
        let count: usize = [Travel::Idler, Travel::Motor, Travel::Centre]
            .into_iter()
            .map(|stop| Pole::at(stop).len())
            .sum();
        assert_eq!(count, 7);
        assert_eq!(Pole::at(Travel::Centre), &[Pole::Hanging]);
    }

    #[test]
    fn the_keys_are_the_services() {
        assert_eq!(Pole::Upright.pose_key(Travel::Idler), "idler upright");
        assert_eq!(Pole::Hanging.pose_key(Travel::Centre), "centre hanging");
        assert_eq!(Pole::Horizontal.pose_key(Travel::Motor), "motor horizontal");
    }

    #[test]
    fn held_out_the_pole_points_away_from_the_centre() {
        assert!(Pole::Horizontal.name(Travel::Idler).ends_with("left"));
        assert!(Pole::Horizontal.name(Travel::Motor).ends_with("right"));
    }
}
