//! The virtual Pi's detector (detector-plan.md): runs the trained model
//! named in `ObjectDetection.toml` on a frame JPEG and returns the 4-slot
//! marker frame. One source for the loop and the Pi.
//!
//! Two inference engines, chosen at build time: by default tract runs the
//! float `best.onnx` on the CPU (the loop on the PC); with the `edgetpu`
//! feature TFLite runs the int8 Edge TPU model on a Coral USB accelerator
//! (the Pi). Pre- and post-processing are the same for both. The `edgetpu`
//! build's `unsafe` lives in `edgetpu_engine.rs`.

mod config;
mod grey;
mod letterbox;
mod marker_frame;

#[cfg(feature = "edgetpu")]
mod edgetpu_engine;
#[cfg(feature = "edgetpu")]
mod edgetpu_ffi;
#[cfg(not(feature = "edgetpu"))]
mod tract_engine;

pub use crate::{
    config::DetectorConfig,
    grey::Grey,
    letterbox::Letterbox,
    marker_frame::{MarkerFrame, NOT_DETECTED},
};

#[cfg(feature = "edgetpu")]
use crate::edgetpu_engine::Engine;
#[cfg(not(feature = "edgetpu"))]
use crate::tract_engine::Engine;

use anyhow::{Result, bail};

/// Slots in the marker frame: 0 Left, 1 Right, 2 Pivot, 3 End
/// (synthetic-loop-dia.md, Contracts: the class-id contract).
pub const SLOTS: usize = 4;

// Output rows before the class scores: box center x, center y, width,
// height, in the input's pixels.
const BOX_ROWS: usize = 4;

/// The loaded model and its configuration. Holds no state between
/// frames.
pub struct Detector {
    config: DetectorConfig,
    engine: Engine,
}

impl Detector {
    /// Loads the build's engine and model (`model` for tract,
    /// `edgetpu_model` for the `edgetpu` build) and checks its tensor
    /// shapes; any other model fails here instead of being misread.
    /// Prints one line with what it found.
    pub fn load(config: DetectorConfig) -> Result<Detector> {
        let engine = Engine::load(&config)?;
        Ok(Detector { config, engine })
    }

    pub fn config(&self) -> &DetectorConfig {
        &self.config
    }

    /// The marker frame for one frame JPEG. `width × height` is the frame
    /// size the slots are normalized to; a frame of another size, or one
    /// that is not a JPEG, is an error, never empty slots (D4).
    pub fn detect(&self, jpeg: &[u8], width: usize, height: usize) -> Result<MarkerFrame> {
        let frame = Grey::decode(jpeg)?;
        if (frame.width, frame.height) != (width, height) {
            bail!("the frame is {}x{}, expected {width}x{height}", frame.width, frame.height);
        }
        let letterbox = Letterbox::new(width, height, self.config.input_size)?;
        let output = self.infer(&letterbox.apply(&frame, self.config.pad_value))?;
        Ok(self.marker_frame(&output, &letterbox, width, height))
    }

    /// The model's raw output for one pre-processed input — `input_size²`
    /// values in 0..1, row-major — as `4 + classes` rows of `N`
    /// candidates, box rows in the input's pixels.
    pub fn infer(&self, input: &[f32]) -> Result<Vec<f32>> {
        self.engine.infer(input)
    }

    // D3: per class, the candidate with the highest score; detected when
    // that score reaches `confidence`. No NMS — one best box per class is
    // the contract. Its center maps back through the letterbox and is
    // normalized by the frame size.
    fn marker_frame(&self, output: &[f32], letterbox: &Letterbox, width: usize, height: usize) -> MarkerFrame {
        let n = self.engine.candidates();
        let mut frame = MarkerFrame { slots: [NOT_DETECTED; SLOTS], confidences: [0.0; SLOTS] };
        for class in 0..SLOTS {
            let scores = &output[(BOX_ROWS + class) * n..(BOX_ROWS + class + 1) * n];
            let Some((best, &score)) = scores.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1)) else {
                continue;
            };
            frame.confidences[class] = score;
            if score >= self.config.confidence {
                let (x, y) = letterbox.to_frame(output[best], output[n + best]);
                frame.slots[class] = ((x / width as f64) as f32, (y / height as f64) as f32);
            }
        }
        frame
    }
}
