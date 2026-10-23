//! The MuJoCo 3.15.0 C API this crate uses, declared by hand. Every
//! signature is copied from the 3.15.0 headers (`include\mujoco\`; line
//! numbers in the comments) and is valid only for that version —
//! `MJ_VERSION` is checked at load. `mjModel`, `mjData`, and `mjVFS` are
//! opaque: state goes through `mj_getState` / `mj_setState`, never
//! through their fields, so their layouts are never mirrored.

// The names mirror the C header.
#![allow(non_camel_case_types, non_upper_case_globals)]

use std::ffi::{c_char, c_int};

/// `mjVERSION_HEADER` (mujoco.h:19); `mj_version()` must return it.
pub const MJ_VERSION: c_int = 3015000;

// mjtObj (mjtype.h), counted from mjOBJ_UNKNOWN = 0.
/// mjtype.h:301
pub const mjOBJ_JOINT: c_int = 3;
/// mjtype.h:317
pub const mjOBJ_ACTUATOR: c_int = 19;

// mjtState bits (mjtype.h).
/// mjtype.h:509
pub const mjSTATE_TIME: c_int = 1 << 0;
/// mjtype.h:510
pub const mjSTATE_QPOS: c_int = 1 << 1;
/// mjtype.h:515
pub const mjSTATE_CTRL: c_int = 1 << 6;

// mjtLogLevel (mjtype.h), counted from mjLOG_DEBUG = 0.
/// mjtype.h:610
pub const mjLOG_WARNING: c_int = 2;
/// mjtype.h:611
pub const mjLOG_ERROR: c_int = 3;

#[repr(C)]
pub struct mjModel {
    _opaque: [u8; 0],
}

#[repr(C)]
pub struct mjData {
    _opaque: [u8; 0],
}

#[repr(C)]
pub struct mjVFS {
    _opaque: [u8; 0],
}

/// mjtype.h:625. `timestamp` is `mjtBool` (C `_Bool`, one byte); it is
/// read as `u8` because a Rust `bool` holding anything but 0 or 1 is
/// undefined behavior.
#[repr(C)]
pub struct mjLogMessage {
    pub level: c_int,
    pub topic: c_int,
    pub subject: [c_char; 1024],
    pub body: *const c_char,
    pub func: *const c_char,
    pub file: *const c_char,
    pub line: c_int,
    pub timestamp: u8,
}

/// mjtype.h:644; `None` is C's NULL (restores MuJoCo's default handler).
pub type mjfLogHandler = Option<unsafe extern "C" fn(*const mjLogMessage)>;

unsafe extern "C" {
    /// mujoco.h:132
    pub fn mj_loadXML(filename: *const c_char, vfs: *const mjVFS, error: *mut c_char, error_sz: c_int)
    -> *mut mjModel;
    /// mujoco.h:189
    pub fn mj_step(m: *const mjModel, d: *mut mjData);
    /// mujoco.h:198
    pub fn mj_forward(m: *const mjModel, d: *mut mjData);
    /// mujoco.h:242
    pub fn mj_deleteModel(m: *mut mjModel);
    /// mujoco.h:249
    pub fn mj_makeData(m: *const mjModel) -> *mut mjData;
    /// mujoco.h:262
    pub fn mj_resetData(m: *const mjModel, d: *mut mjData);
    /// mujoco.h:292
    pub fn mj_deleteData(d: *mut mjData);
    /// mujoco.h:505
    pub fn mj_stateSize(m: *const mjModel, sig: c_int) -> c_int;
    /// mujoco.h:508
    pub fn mj_getState(m: *const mjModel, d: *const mjData, state: *mut f64, sig: c_int);
    /// mujoco.h:515
    pub fn mj_setState(m: *const mjModel, d: *mut mjData, state: *const f64, sig: c_int);
    /// mujoco.h:604; returns -1 when no object of that type has the name.
    pub fn mj_name2id(m: *const mjModel, type_: c_int, name: *const c_char) -> c_int;
    /// mujoco.h:684
    pub fn mj_version() -> c_int;
    /// mujoco.h:1007; returns the previous handler.
    pub fn mju_setLogHandler(handler: mjfLogHandler) -> mjfLogHandler;
}
