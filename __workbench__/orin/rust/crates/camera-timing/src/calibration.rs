use crate::{camera::Camera, constants::FRAME_BYTES, led::Led, patch::Patch};

use anyhow::{Result, bail};
use std::time::Duration;

/// Frames skipped after each switch of the LED during calibration, so no
/// frame that was being exposed as it switched is counted as off or on: the
/// camera has at most a frame or two in its pipeline at 60 frames/s, and
/// three is a frame to spare.
const SWITCH_SKIP_FRAMES: u32 = 3;

/// A threshold halfway between two levels: their sum over this.
const HALFWAY: f64 = 2.0;

/// The fewest frames a mean is divided by: with `calibration_frames` set to
/// 0 the mean is of no frames, all zeros, rather than a division by zero.
const AT_LEAST_ONE_FRAME: u32 = 1;

/// Where the LED is in the picture and how bright it is each way, found from
/// `calibration_frames` frames with the LED off and as many with it on.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Calibration {
    pub(crate) patch: Patch,
    /// The patch's mean grey, 0 to 255, with the LED off and on.
    pub(crate) off_level: f64,
    pub(crate) on_level: f64,
}

impl Calibration {
    /// Takes the frames, finds the patch, and refuses an LED whose lit
    /// patch is less than `min_contrast` brighter than its dark one.
    pub(crate) fn take(
        camera: &Camera,
        led: &Led,
        frames: u32,
        half: usize,
        min_contrast: f64,
        timeout: Duration,
    ) -> Result<Calibration> {
        led.off()?;
        skip(camera, SWITCH_SKIP_FRAMES, timeout)?;
        let off = mean_frame(camera, frames, timeout)?;
        led.on()?;
        skip(camera, SWITCH_SKIP_FRAMES, timeout)?;
        let on = mean_frame(camera, frames, timeout)?;
        led.off()?;
        skip(camera, SWITCH_SKIP_FRAMES, timeout)?;

        let patch = Patch::find(&off, &on, half);
        let (off_level, on_level) = (patch.level(&off), patch.level(&on));
        if on_level - off_level < min_contrast {
            bail!(
                "the LED's brightest patch, columns {}..{} rows {}..{}, is only {:.1} grey levels \
                 brighter lit ({on_level:.1}) than dark ({off_level:.1}), under [test] \
                 min_contrast {min_contrast}: is the LED in view, and bright enough?",
                patch.left,
                patch.right,
                patch.top,
                patch.bottom,
                on_level - off_level
            );
        }
        Ok(Calibration {
            patch,
            off_level,
            on_level,
        })
    }

    /// The level above which a frame counts as lit: halfway between dark and
    /// lit, so a flash that lit only part of an exposure still counts once it
    /// gave half the full light.
    pub(crate) fn threshold(&self) -> f64 {
        (self.off_level + self.on_level) / HALFWAY
    }
}

fn skip(camera: &Camera, frames: u32, timeout: Duration) -> Result<()> {
    for _ in 0..frames {
        camera.next(timeout)?;
    }
    Ok(())
}

/// The mean of `frames` frames, pixel by pixel.
fn mean_frame(camera: &Camera, frames: u32, timeout: Duration) -> Result<Vec<f64>> {
    let mut sums = vec![0u64; FRAME_BYTES];
    for _ in 0..frames {
        let frame = camera.next(timeout)?;
        for (sum, &pixel) in sums.iter_mut().zip(frame.pixels.iter()) {
            *sum += u64::from(pixel);
        }
    }
    let count = f64::from(frames.max(AT_LEAST_ONE_FRAME));
    Ok(sums.iter().map(|&sum| sum as f64 / count).collect())
}
