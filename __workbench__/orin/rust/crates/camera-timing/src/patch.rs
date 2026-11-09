use crate::constants::{FRAME_HEIGHT, FRAME_WIDTH};

/// The LED's patch of the frame: a square around the pixel that changed most
/// between the LED off and on, found from the run's own calibration frames,
/// so nobody has to say where the LED sits in the picture. Its level is the
/// patch's mean grey, 0 to 255.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Patch {
    /// The patch's columns and rows: `left..right`, `top..bottom`.
    pub(crate) left: usize,
    pub(crate) right: usize,
    pub(crate) top: usize,
    pub(crate) bottom: usize,
}

impl Patch {
    /// The square of `half` pixels either side of the pixel where `on` is
    /// brightest over `off`, cut at the frame's edges. `off` and `on` are mean
    /// frames, one value per pixel, top row first.
    pub(crate) fn find(off: &[f64], on: &[f64], half: usize) -> Patch {
        let brightest = off
            .iter()
            .zip(on)
            .map(|(dark, lit)| lit - dark)
            .enumerate()
            .max_by(|(_, a), (_, b)| a.total_cmp(b))
            .map_or(0, |(index, _)| index);
        let (row, column) = (brightest / FRAME_WIDTH, brightest % FRAME_WIDTH);
        Patch {
            left: column.saturating_sub(half),
            right: (column + half + 1).min(FRAME_WIDTH),
            top: row.saturating_sub(half),
            bottom: (row + half + 1).min(FRAME_HEIGHT),
        }
    }

    /// The patch's mean over `values`, one per pixel, top row first.
    pub(crate) fn level<T: Copy + Into<f64>>(&self, values: &[T]) -> f64 {
        let mut sum = 0.0;
        let mut count = 0.0;
        for row in self.top..self.bottom {
            for column in self.left..self.right {
                if let Some(&value) = values.get(row * FRAME_WIDTH + column) {
                    sum += value.into();
                    count += 1.0;
                }
            }
        }
        if count > 0.0 { sum / count } else { 0.0 }
    }
}
