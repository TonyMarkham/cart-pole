//! Hand-written declarations for the parts of TFLite's C API
//! (`tensorflow/lite/c/c_api.h`, `core/c/c_api_types.h`; Ubuntu's
//! libtensorflow-lite 2.14.1) and of libedgetpu's `edgetpu_c.h` that the
//! Edge TPU engine calls. Values were read from those headers.

use std::ffi::{c_char, c_int, c_void};

// Opaque handles.
#[repr(C)]
pub struct TfLiteModel {
    _private: [u8; 0],
}
#[repr(C)]
pub struct TfLiteInterpreterOptions {
    _private: [u8; 0],
}
#[repr(C)]
pub struct TfLiteInterpreter {
    _private: [u8; 0],
}
#[repr(C)]
pub struct TfLiteTensor {
    _private: [u8; 0],
}
#[repr(C)]
pub struct TfLiteDelegate {
    _private: [u8; 0],
}

// TfLiteStatus (c_api_types.h): only success is compared against.
pub const K_TFLITE_OK: c_int = 0;

// TfLiteType (c_api_types.h).
pub const K_TFLITE_INT8: c_int = 9;

/// Affine quantization of a tensor: real = scale × (q − zero_point).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TfLiteQuantizationParams {
    pub scale: f32,
    pub zero_point: i32,
}

// edgetpu_c.h: enum edgetpu_device_type.
pub const EDGETPU_APEX_USB: c_int = 1;

/// edgetpu_c.h: struct edgetpu_device.
#[repr(C)]
pub struct EdgetpuDevice {
    pub kind: c_int,
    pub path: *const c_char,
}

/// edgetpu_c.h: struct edgetpu_option.
#[repr(C)]
pub struct EdgetpuOption {
    pub name: *const c_char,
    pub value: *const c_char,
}

#[link(name = "tensorflow-lite")]
unsafe extern "C" {
    pub fn TfLiteModelCreateFromFile(model_path: *const c_char) -> *mut TfLiteModel;
    pub fn TfLiteModelDelete(model: *mut TfLiteModel);

    pub fn TfLiteInterpreterOptionsCreate() -> *mut TfLiteInterpreterOptions;
    pub fn TfLiteInterpreterOptionsDelete(options: *mut TfLiteInterpreterOptions);
    pub fn TfLiteInterpreterOptionsSetNumThreads(options: *mut TfLiteInterpreterOptions, num_threads: i32);
    pub fn TfLiteInterpreterOptionsAddDelegate(options: *mut TfLiteInterpreterOptions, delegate: *mut TfLiteDelegate);

    pub fn TfLiteInterpreterCreate(
        model: *const TfLiteModel,
        optional_options: *const TfLiteInterpreterOptions,
    ) -> *mut TfLiteInterpreter;
    pub fn TfLiteInterpreterDelete(interpreter: *mut TfLiteInterpreter);
    pub fn TfLiteInterpreterAllocateTensors(interpreter: *mut TfLiteInterpreter) -> c_int;
    pub fn TfLiteInterpreterInvoke(interpreter: *mut TfLiteInterpreter) -> c_int;
    pub fn TfLiteInterpreterGetInputTensorCount(interpreter: *const TfLiteInterpreter) -> i32;
    pub fn TfLiteInterpreterGetInputTensor(interpreter: *const TfLiteInterpreter, input_index: i32) -> *mut TfLiteTensor;
    pub fn TfLiteInterpreterGetOutputTensorCount(interpreter: *const TfLiteInterpreter) -> i32;
    pub fn TfLiteInterpreterGetOutputTensor(
        interpreter: *const TfLiteInterpreter,
        output_index: i32,
    ) -> *const TfLiteTensor;

    pub fn TfLiteTensorType(tensor: *const TfLiteTensor) -> c_int;
    pub fn TfLiteTensorNumDims(tensor: *const TfLiteTensor) -> i32;
    pub fn TfLiteTensorDim(tensor: *const TfLiteTensor, dim_index: i32) -> i32;
    pub fn TfLiteTensorByteSize(tensor: *const TfLiteTensor) -> usize;
    pub fn TfLiteTensorQuantizationParams(tensor: *const TfLiteTensor) -> TfLiteQuantizationParams;
    pub fn TfLiteTensorCopyFromBuffer(tensor: *mut TfLiteTensor, input_data: *const c_void, input_data_size: usize)
    -> c_int;
    pub fn TfLiteTensorCopyToBuffer(
        output_tensor: *const TfLiteTensor,
        output_data: *mut c_void,
        output_data_size: usize,
    ) -> c_int;
}

// The runtime package ships only the versioned soname (no
// libedgetpu-dev), so it is linked by its exact file name.
#[link(name = "libedgetpu.so.1", kind = "dylib", modifiers = "+verbatim")]
unsafe extern "C" {
    pub fn edgetpu_list_devices(num_devices: *mut usize) -> *mut EdgetpuDevice;
    pub fn edgetpu_free_devices(dev: *mut EdgetpuDevice);
    pub fn edgetpu_create_delegate(
        kind: c_int,
        name: *const c_char,
        options: *const EdgetpuOption,
        num_options: usize,
    ) -> *mut TfLiteDelegate;
    pub fn edgetpu_free_delegate(delegate: *mut TfLiteDelegate);
}
