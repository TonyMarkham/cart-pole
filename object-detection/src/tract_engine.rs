use std::sync::Arc;

use anyhow::{Context, Result, bail};
use tract_linalg::multithread::Executor;
use tract_onnx::prelude::{DatumExt, Framework, InferenceModelExt, RunOptions, Tensor, TypedRunnableModel, tvec};

use crate::{BOX_ROWS, DetectorConfig};

/// `best.onnx` (float) through tract on the CPU — the default build.
pub(crate) struct Engine {
    model: Arc<TypedRunnableModel>,
    size: usize,
    rows: usize,
    // Candidates per frame: the output's last dimension (8400 for 640).
    candidates: usize,
}

impl Engine {
    /// Loads `config.model`, fixes its input to `[1, 1, input_size,
    /// input_size]` f32, optimizes it, and checks the output is
    /// `[1, 4 + classes, N]`; any other model fails here instead of
    /// being misread. With `threads` > 1 inference runs on its own pool
    /// of that many threads. Prints one line with what it found.
    pub(crate) fn load(config: &DetectorConfig) -> Result<Engine> {
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
        let threads = config.threads;
        let options = RunOptions { executor: (threads > 1).then(|| Executor::multithread(threads)), ..Default::default() };
        let model = model
            .into_runnable_with_options(&options)
            .with_context(|| format!("{path}: preparing to run"))?;
        println!("detector: tract {path}  input [1, 1, {size}, {size}]  output {shape:?}  threads {threads}");
        Ok(Engine { model, size, rows, candidates })
    }

    pub(crate) fn candidates(&self) -> usize {
        self.candidates
    }

    /// The model's raw output for one pre-processed input — `input_size²`
    /// values, row-major — as `4 + classes` rows of `N` candidates.
    pub(crate) fn infer(&self, input: &[f32]) -> Result<Vec<f32>> {
        let size = self.size;
        let tensor = Tensor::from_shape(&[1, 1, size, size], input)
            .with_context(|| format!("input of {} values is not {size}x{size}", input.len()))?;
        let outputs = self.model.run(tvec!(tensor.into())).context("running the model")?;
        let output = outputs.first().context("the model returned no output")?;
        let plain = output.try_as_plain_ram().context("reading the output")?;
        let values = plain.as_slice::<f32>().context("reading the output as f32")?;
        let expected = self.rows * self.candidates;
        if values.len() != expected {
            bail!("the model returned {} values, expected {expected}", values.len());
        }
        Ok(values.to_vec())
    }
}
