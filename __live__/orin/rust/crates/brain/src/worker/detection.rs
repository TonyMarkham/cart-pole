use crate::constants::{DETECTION_FLOATS, NOT_DETECTED};

/// One class's box as the worker answers it: the centre and size normalised to
/// the frame, 0 to 1, and the model's confidence. A marker not found is
/// (−1, −1) with a size of 0 and the confidence the model gave its best
/// candidate, under the threshold (`__documentation__/real-loop-dia.md`, *Contracts*, the
/// brain's worker).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Detection {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) width: f64,
    pub(crate) height: f64,
    pub(crate) confidence: f64,
}

impl Detection {
    /// Five little-endian `f32`, in the order above.
    pub(crate) fn from_bytes(bytes: &[u8; DETECTION_FLOATS * 4]) -> Detection {
        let mut values = [0f64; DETECTION_FLOATS];
        for (value, chunk) in values.iter_mut().zip(bytes.as_chunks::<4>().0) {
            *value = f64::from(f32::from_le_bytes(*chunk));
        }
        Detection {
            x: values[0],
            y: values[1],
            width: values[2],
            height: values[3],
            confidence: values[4],
        }
    }

    /// Whether the marker was found: a found centre is never negative.
    pub(crate) fn found(self) -> bool {
        self.x != NOT_DETECTED && self.y != NOT_DETECTED
    }

    /// The five numbers for the log's `boxes`.
    pub(crate) fn as_array(self) -> [f64; DETECTION_FLOATS] {
        [self.x, self.y, self.width, self.height, self.confidence]
    }
}
