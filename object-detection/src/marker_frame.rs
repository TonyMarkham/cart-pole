use crate::SLOTS;

/// A slot's position when its class was not detected. Outside [0, 1], so
/// it can never be a real position (synthetic-loop-dia.md, Contracts).
pub const NOT_DETECTED: (f32, f32) = (-1.0, -1.0);

/// The 4-slot marker frame (synthetic-loop-dia.md, Contracts): per class, in class order, the
/// best box's center normalized to [0, 1] of the frame (top-left origin),
/// or [`NOT_DETECTED`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MarkerFrame {
    pub slots: [(f32, f32); SLOTS],
    /// Each class's best score, whether or not it reached the threshold —
    /// for logs and the reference comparison; not part of the contract.
    pub confidences: [f32; SLOTS],
}

impl MarkerFrame {
    /// Whether slot `i` holds a detection.
    pub fn detected(&self, i: usize) -> bool {
        self.slots.get(i).is_some_and(|&slot| slot != NOT_DETECTED)
    }
}
