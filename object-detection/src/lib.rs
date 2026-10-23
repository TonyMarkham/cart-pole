//! The virtual Pi's detector (detector-plan.md): runs the trained
//! `best.onnx` named in `ObjectDetection.toml` on a frame JPEG and
//! returns the 4-slot marker frame. One source for the loop and the Pi.

mod config;
mod grey;
mod letterbox;
mod marker_frame;

pub use crate::{
    config::DetectorConfig,
    grey::Grey,
    letterbox::Letterbox,
    marker_frame::{MarkerFrame, NOT_DETECTED},
};

use std::sync::Arc;

use anyhow::{Context, Result, bail};
use tract_onnx::prelude::{DatumExt, Framework, InferenceModelExt, IntoRunnable, Tensor, TypedRunnableModel, tvec};

/// Slots in the marker frame: 0 Left, 1 Right, 2 Pivot, 3 End
/// (loop-plan.md, the class-id contract).
pub const SLOTS: usize = 4;

// Output rows before the class scores: box center x, center y, width,
// height, in the input's pixels.
const BOX_ROWS: usize = 4;

/// The loaded model and its configuration. Holds no state between
/// frames.
pub struct Detector {
    config: DetectorConfig,
    model: Arc<TypedRunnableModel>,
    // Candidates per frame: the output's last dimension (8400 for 640).
    candidates: usize,
}

impl Detector {
    /// Loads `config.model`, fixes its input to `[1, 1, input_size,
    /// input_size]` f32, optimizes it, and checks the output is
    /// `[1, 4 + classes, N]`; any other model fails here instead of
    /// being misread. Prints one line with what it found.
    pub fn load(config: DetectorConfig) -> Result<Detector> {
        let size = config.input_size;
        let path = config.model.display().to_string();
        let model = tract_onnx::onnx()
            .model_for_path(&config.model)
            .with_context(|| format!("loading {path}"))?
            .with_input_fact(0, f32::fact([1, 1, size, size]).into())
            .with_context(|| format!("{path}: setting the input to [1, 1, {size}, {size}] f32"))?
            .into_optimized()
            .with_context(|| format!("{path}: optimizing"))?;
        let outputs = model.output_outlets()?.len();
        if outputs != 1 {
            bail!("{path}: expected one output, found {outputs}");
        }
        let fact = model.output_fact(0)?;
        let shape = fact
            .shape
            .as_concrete()
            .with_context(|| format!("{path}: output shape {:?} is not fixed", fact.shape))?
            .to_vec();
        let rows = BOX_ROWS + config.classes.len();
        let candidates = match shape.as_slice() {
            [1, r, n] if *r == rows && *n > 0 => *n,
            _ => bail!("{path}: output shape {shape:?}, expected [1, {rows}, N] (4 box rows + {} classes)", config.classes.len()),
        };
        let model = model.into_runnable().with_context(|| format!("{path}: preparing to run"))?;
        println!("detector: {path}  input [1, 1, {size}, {size}]  output {shape:?}");
        Ok(Detector { config, model, candidates })
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
    /// values, row-major — as `4 + classes` rows of `N` candidates.
    pub fn infer(&self, input: &[f32]) -> Result<Vec<f32>> {
        let size = self.config.input_size;
        let tensor = Tensor::from_shape(&[1, 1, size, size], input)
            .with_context(|| format!("input of {} values is not {size}x{size}", input.len()))?;
        let outputs = self.model.run(tvec!(tensor.into())).context("running the model")?;
        let output = outputs.first().context("the model returned no output")?;
        let plain = output.try_as_plain_ram().context("reading the output")?;
        let values = plain.as_slice::<f32>().context("reading the output as f32")?;
        let expected = (BOX_ROWS + self.config.classes.len()) * self.candidates;
        if values.len() != expected {
            bail!("the model returned {} values, expected {expected}", values.len());
        }
        Ok(values.to_vec())
    }

    // D3: per class, the candidate with the highest score; detected when
    // that score reaches `confidence`. No NMS — one best box per class is
    // the contract. Its center maps back through the letterbox and is
    // normalized by the frame size.
    fn marker_frame(&self, output: &[f32], letterbox: &Letterbox, width: usize, height: usize) -> MarkerFrame {
        let n = self.candidates;
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
