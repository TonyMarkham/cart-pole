use std::{
    ffi::{CString, c_void},
    ptr::{self, NonNull},
};

use anyhow::{Context, Result, anyhow, bail};

use crate::{BOX_ROWS, DetectorConfig, edgetpu_ffi as ffi};

/// The int8 Edge TPU model through TFLite's C API with the Coral USB
/// delegate — the `edgetpu` build (the Pi). The ops the Edge TPU compiler
/// could not map run on the CPU on `threads` threads. Raw pointers make it
/// neither `Send` nor `Sync`: it lives on the thread that loaded it.
pub(crate) struct Engine {
    // Declaration order is drop order: the interpreter goes before the
    // delegate and the model it was built from.
    interpreter: Interpreter,
    _delegate: Delegate,
    _model: Model,
    input: ffi::TfLiteQuantizationParams,
    output: ffi::TfLiteQuantizationParams,
    size: usize,
    rows: usize,
    candidates: usize,
}

impl Engine {
    /// Loads `config.edgetpu_model` onto the first Coral USB device and
    /// checks the tensors: input `[1, input_size, input_size, 1]` int8,
    /// output `[1, 4 + classes, N]` int8; any other model fails here
    /// instead of being misread. Prints one line with what it found.
    pub(crate) fn load(config: &DetectorConfig) -> Result<Engine> {
        let size = config.input_size;
        let path = config.edgetpu_model.display().to_string();
        let c_path = CString::new(path.as_str()).context("model path contains a NUL")?;
        let threads = i32::try_from(config.threads)?;

        let devices = usb_devices();
        if devices == 0 {
            bail!("no Coral USB Edge TPU found (edgetpu_list_devices)");
        }
        // SAFETY: c_path is NUL-terminated and outlives the call.
        let model = Model(
            NonNull::new(unsafe { ffi::TfLiteModelCreateFromFile(c_path.as_ptr()) })
                .ok_or_else(|| anyhow!("loading {path}"))?,
        );
        // SAFETY: no name and no options select the first USB device.
        let delegate = Delegate(
            NonNull::new(unsafe { ffi::edgetpu_create_delegate(ffi::EDGETPU_APEX_USB, ptr::null(), ptr::null(), 0) })
                .ok_or_else(|| anyhow!("opening the Coral USB Edge TPU (edgetpu_create_delegate)"))?,
        );
        let interpreter = {
            // SAFETY: no arguments.
            let options = NonNull::new(unsafe { ffi::TfLiteInterpreterOptionsCreate() })
                .ok_or_else(|| anyhow!("TfLiteInterpreterOptionsCreate failed"))?;
            // SAFETY: options is live; the delegate outlives the interpreter
            // (Engine's field order), as TFLite requires; the options may be
            // deleted once the interpreter exists (c_api.h).
            let interpreter = unsafe {
                ffi::TfLiteInterpreterOptionsSetNumThreads(options.as_ptr(), threads);
                ffi::TfLiteInterpreterOptionsAddDelegate(options.as_ptr(), delegate.0.as_ptr());
                let interpreter = ffi::TfLiteInterpreterCreate(model.0.as_ptr(), options.as_ptr());
                ffi::TfLiteInterpreterOptionsDelete(options.as_ptr());
                interpreter
            };
            Interpreter(NonNull::new(interpreter).ok_or_else(|| anyhow!("{path}: TfLiteInterpreterCreate failed"))?)
        };
        // SAFETY: the interpreter is live.
        if unsafe { ffi::TfLiteInterpreterAllocateTensors(interpreter.0.as_ptr()) } != ffi::K_TFLITE_OK {
            bail!("{path}: TfLiteInterpreterAllocateTensors failed");
        }

        // SAFETY: the interpreter is live and its tensors are allocated.
        let (inputs, outputs) = unsafe {
            (
                ffi::TfLiteInterpreterGetInputTensorCount(interpreter.0.as_ptr()),
                ffi::TfLiteInterpreterGetOutputTensorCount(interpreter.0.as_ptr()),
            )
        };
        if (inputs, outputs) != (1, 1) {
            bail!("{path}: expected one input and one output, found {inputs} and {outputs}");
        }
        let input_tensor = interpreter.input();
        let output_tensor = interpreter.output();
        let input_shape = shape(input_tensor);
        let output_shape = shape(output_tensor);
        let rows = BOX_ROWS + config.classes.len();
        // SAFETY: both tensors belong to the live interpreter.
        let types = unsafe { (ffi::TfLiteTensorType(input_tensor), ffi::TfLiteTensorType(output_tensor)) };
        if types != (ffi::K_TFLITE_INT8, ffi::K_TFLITE_INT8) {
            bail!("{path}: tensor types {types:?}, expected int8 in and out ({})", ffi::K_TFLITE_INT8);
        }
        if input_shape != [1, size, size, 1] {
            bail!("{path}: input shape {input_shape:?}, expected [1, {size}, {size}, 1]");
        }
        let candidates = match output_shape.as_slice() {
            [1, r, n] if *r == rows && *n > 0 => *n,
            _ => bail!("{path}: output shape {output_shape:?}, expected [1, {rows}, N] (4 box rows + {} classes)", config.classes.len()),
        };
        // SAFETY: both tensors belong to the live interpreter.
        let (input, output) = unsafe {
            (ffi::TfLiteTensorQuantizationParams(input_tensor), ffi::TfLiteTensorQuantizationParams(output_tensor))
        };
        if !(input.scale > 0.0 && output.scale > 0.0) {
            bail!("{path}: tensors are not quantized (input scale {}, output scale {})", input.scale, output.scale);
        }
        println!(
            "detector: Edge TPU ({devices} USB device(s)) {path}  input {input_shape:?} int8 q({}, {})  output {output_shape:?} int8 q({}, {})  threads {threads}",
            input.scale, input.zero_point, output.scale, output.zero_point
        );
        Ok(Engine { interpreter, _delegate: delegate, _model: model, input, output, size, rows, candidates })
    }

    pub(crate) fn candidates(&self) -> usize {
        self.candidates
    }

    /// The model's output for one pre-processed input — `input_size²`
    /// values in 0..1, row-major — as `4 + classes` rows of `N`
    /// candidates, dequantized, with the box rows in the input's pixels
    /// like tract's. Ultralytics' TFLite export divides box coordinates
    /// by the input size (`utils/export/tensorflow.py`
    /// `_tf_decode_boxes`: stride / (8 · 80) = stride / 640), so they are
    /// multiplied back by it.
    pub(crate) fn infer(&self, input: &[f32]) -> Result<Vec<f32>> {
        let size = self.size;
        if input.len() != size * size {
            bail!("input of {} values is not {size}x{size}", input.len());
        }
        // q = round(x / scale) + zero_point, saturated to int8: TFLite's
        // affine quantization (round half away from zero, as std::round).
        let quantized: Vec<i8> = input
            .iter()
            .map(|&x| ((x / self.input.scale).round() + self.input.zero_point as f32).clamp(-128.0, 127.0) as i8)
            .collect();
        let mut raw = vec![0i8; self.rows * self.candidates];
        let input_tensor = self.interpreter.input();
        let output_tensor = self.interpreter.output();
        // SAFETY: the tensors belong to the live interpreter; the buffers
        // are exactly the tensors' byte sizes (checked by TFLite, which
        // returns an error status on a mismatch).
        unsafe {
            if ffi::TfLiteTensorCopyFromBuffer(input_tensor, quantized.as_ptr().cast::<c_void>(), quantized.len())
                != ffi::K_TFLITE_OK
            {
                bail!("copying the input ({} bytes, tensor {})", quantized.len(), ffi::TfLiteTensorByteSize(input_tensor));
            }
            if ffi::TfLiteInterpreterInvoke(self.interpreter.0.as_ptr()) != ffi::K_TFLITE_OK {
                bail!("TfLiteInterpreterInvoke failed");
            }
            if ffi::TfLiteTensorCopyToBuffer(output_tensor, raw.as_mut_ptr().cast::<c_void>(), raw.len()) != ffi::K_TFLITE_OK {
                bail!("copying the output ({} bytes, tensor {})", raw.len(), ffi::TfLiteTensorByteSize(output_tensor));
            }
        }
        let box_values = BOX_ROWS * self.candidates;
        let (scale, zero) = (self.output.scale, self.output.zero_point);
        Ok(raw
            .iter()
            .enumerate()
            .map(|(i, &q)| {
                let value = scale * (i32::from(q) - zero) as f32;
                if i < box_values { value * size as f32 } else { value }
            })
            .collect())
    }
}

// Coral USB devices libedgetpu can see.
fn usb_devices() -> usize {
    let mut count = 0usize;
    // SAFETY: count is a valid out-pointer; the list is freed once, below.
    let devices = unsafe { ffi::edgetpu_list_devices(&mut count) };
    if devices.is_null() {
        return 0;
    }
    // SAFETY: devices holds count entries until freed.
    let usb = (0..count)
        .filter(|&i| unsafe { (*devices.add(i)).kind } == ffi::EDGETPU_APEX_USB)
        .count();
    // SAFETY: devices came from edgetpu_list_devices and is freed once.
    unsafe { ffi::edgetpu_free_devices(devices) };
    usb
}

// A tensor's dimensions.
fn shape(tensor: *const ffi::TfLiteTensor) -> Vec<usize> {
    // SAFETY: callers pass a tensor of a live interpreter.
    let dims = unsafe { ffi::TfLiteTensorNumDims(tensor) };
    (0..dims)
        .map(|i| usize::try_from(unsafe { ffi::TfLiteTensorDim(tensor, i) }).unwrap_or(0))
        .collect()
}

struct Model(NonNull<ffi::TfLiteModel>);

impl Drop for Model {
    fn drop(&mut self) {
        // SAFETY: owned, freed once, after the interpreter (Engine order).
        unsafe { ffi::TfLiteModelDelete(self.0.as_ptr()) }
    }
}

struct Delegate(NonNull<ffi::TfLiteDelegate>);

impl Drop for Delegate {
    fn drop(&mut self) {
        // SAFETY: owned, freed once, after the interpreter (Engine order).
        unsafe { ffi::edgetpu_free_delegate(self.0.as_ptr()) }
    }
}

struct Interpreter(NonNull<ffi::TfLiteInterpreter>);

impl Interpreter {
    // Tensor pointers are stable once tensors are allocated (load did).
    fn input(&self) -> *mut ffi::TfLiteTensor {
        // SAFETY: the interpreter is live; index 0 exists (checked at load).
        unsafe { ffi::TfLiteInterpreterGetInputTensor(self.0.as_ptr(), 0) }
    }

    fn output(&self) -> *const ffi::TfLiteTensor {
        // SAFETY: the interpreter is live; index 0 exists (checked at load).
        unsafe { ffi::TfLiteInterpreterGetOutputTensor(self.0.as_ptr(), 0) }
    }
}

impl Drop for Interpreter {
    fn drop(&mut self) {
        // SAFETY: owned and freed once.
        unsafe { ffi::TfLiteInterpreterDelete(self.0.as_ptr()) }
    }
}
