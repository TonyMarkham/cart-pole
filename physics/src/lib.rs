//! MuJoCo for the sim loop (physics-plan.md): loads the MJCF named in
//! `physics.toml`, sets the cart actuator's `ctrl`, steps, reads the
//! joints' `qpos`. All `unsafe` in the workspace lives in this crate.

mod config;
mod ffi;
mod pose;

pub use crate::{
    config::{Names, PhysicsConfig},
    pose::Pose,
};

use std::{
    ffi::{CStr, CString, c_char, c_int},
    io::Write,
    ptr::{self, NonNull},
    sync::atomic::{AtomicUsize, Ordering},
};

use anyhow::{Context, Result, anyhow, bail};

// qpos and ctrl lengths the model check demands: one slide and one hinge
// take one qpos slot each; one actuator takes one ctrl slot.
const QPOS_LEN: usize = 2;
const CTRL_LEN: usize = 1;

// mj_loadXML's error buffer.
const LOAD_ERROR_LEN: usize = 1000;

// MuJoCo warnings seen by the log handler since the process started. The
// handler is process-wide (mju_setLogHandler), so the count is too.
static WARNINGS: AtomicUsize = AtomicUsize::new(0);

/// One MuJoCo model and its simulation state. Raw pointers make it
/// neither `Send` nor `Sync`: it lives on the thread that loaded it.
pub struct Physics {
    model: NonNull<ffi::mjModel>,
    data: NonNull<ffi::mjData>,
    // qpos indices of the two joints (P2: a joint's qpos index is its id).
    cart: usize,
    pole: usize,
    timestep: f64,
}

impl Physics {
    /// Loads `config.model`, checks the library version (P1) and the
    /// model's shape (P2), and measures the timestep. Prints one line
    /// with what it found. The state is MuJoCo's reset state until
    /// [`Physics::reset`].
    pub fn load(config: &PhysicsConfig) -> Result<Physics> {
        // SAFETY: log_handler matches mjfLogHandler and never unwinds.
        unsafe { ffi::mju_setLogHandler(Some(log_handler)) };

        // SAFETY: no arguments, no preconditions.
        let version = unsafe { ffi::mj_version() };
        if version != ffi::MJ_VERSION {
            bail!(
                "mujoco.dll is version {version}, but the declarations in physics/src/ffi.rs are for {}",
                ffi::MJ_VERSION
            );
        }

        let path = config
            .model
            .to_str()
            .ok_or_else(|| anyhow!("model path is not UTF-8: {}", config.model.display()))?;
        let c_path = CString::new(path).context("model path contains a NUL")?;
        let mut error = [0 as c_char; LOAD_ERROR_LEN];
        // SAFETY: c_path is NUL-terminated; vfs may be NULL; error is
        // LOAD_ERROR_LEN long, as passed.
        let model = unsafe {
            ffi::mj_loadXML(
                c_path.as_ptr(),
                ptr::null(),
                error.as_mut_ptr(),
                c_int::try_from(LOAD_ERROR_LEN)?,
            )
        };
        let Some(model) = NonNull::new(model) else {
            bail!("loading {path}: {}", c_text(&error));
        };
        // SAFETY: model is a live model from mj_loadXML.
        let data = unsafe { ffi::mj_makeData(model.as_ptr()) };
        let Some(data) = NonNull::new(data) else {
            // SAFETY: model is live and owned by nobody else yet.
            unsafe { ffi::mj_deleteModel(model.as_ptr()) };
            bail!("mj_makeData failed for {path}");
        };

        // From here Drop owns both pointers.
        let mut physics = Physics { model, data, cart: 0, pole: 0, timestep: 0.0 };
        let names = &config.names;
        let cart = physics.id(ffi::mjOBJ_JOINT, "joint", &names.cart_joint)?;
        let pole = physics.id(ffi::mjOBJ_JOINT, "joint", &names.pole_joint)?;
        let actuator = physics.id(ffi::mjOBJ_ACTUATOR, "actuator", &names.actuator)?;
        let qpos_len = physics.state_len(ffi::mjSTATE_QPOS);
        let ctrl_len = physics.state_len(ffi::mjSTATE_CTRL);
        // P2: MuJoCo assigns qpos slots in joint order and a slide or hinge
        // takes one, so with exactly these two joints a joint's qpos index
        // is its id. Same for the actuator and ctrl.
        if qpos_len != QPOS_LEN || (cart.min(pole), cart.max(pole)) != (0, 1) {
            bail!(
                "{path}: expected exactly two 1-DOF joints (qpos 2, ids 0 and 1), found qpos {qpos_len}, {} id {cart}, {} id {pole}",
                names.cart_joint,
                names.pole_joint
            );
        }
        if ctrl_len != CTRL_LEN || actuator != 0 {
            bail!(
                "{path}: expected exactly one actuator (ctrl 1, id 0), found ctrl {ctrl_len}, {} id {actuator}",
                names.actuator
            );
        }
        physics.cart = cart;
        physics.pole = pole;

        // R4: the timestep is a struct field; one step from reset reads it
        // as the elapsed time instead.
        physics.reset_data();
        physics.step(1);
        physics.timestep = physics.time();
        physics.reset_data();
        if !physics.timestep.is_finite() || physics.timestep <= 0.0 {
            bail!("{path}: measured timestep {} s is not positive", physics.timestep);
        }

        println!(
            "MuJoCo {}.{}.{}  qpos {qpos_len} ({} {cart}, {} {pole})  ctrl {ctrl_len} ({} {actuator})  timestep {} s",
            version / 1_000_000,
            version / 1000 % 1000,
            version % 1000,
            names.cart_joint,
            names.pole_joint,
            names.actuator,
            physics.timestep
        );
        Ok(physics)
    }

    /// The physics step, in seconds of simulated time.
    pub fn timestep(&self) -> f64 {
        self.timestep
    }

    /// Back to time 0 at `start`, at rest (`mj_resetData` zeroes `qvel`
    /// and `ctrl`), with derived quantities recomputed for the new pose.
    pub fn reset(&mut self, start: Pose) -> Result<()> {
        if !start.cart.is_finite() || !start.pole.is_finite() {
            bail!("start pose is not finite: {start:?}");
        }
        self.reset_data();
        let mut qpos = [0.0; QPOS_LEN];
        qpos[self.cart] = start.cart;
        qpos[self.pole] = start.pole;
        self.set_state(ffi::mjSTATE_QPOS, &qpos);
        // SAFETY: model and data are live and belong together.
        unsafe { ffi::mj_forward(self.model.as_ptr(), self.data.as_ptr()) };
        Ok(())
    }

    /// The cart actuator's `ctrl`, in the units of the model's actuator
    /// element: for the cart-pole's `<velocity>` actuator, the target cart
    /// speed in m/s (sign = direction); a `<position>` actuator would take
    /// a target position in m. MuJoCo clamps it to the model's `ctrlrange`
    /// (`ctrllimited="true"`).
    pub fn set_ctrl(&mut self, ctrl: f64) -> Result<()> {
        if !ctrl.is_finite() {
            bail!("ctrl is not finite: {ctrl}");
        }
        self.set_state(ffi::mjSTATE_CTRL, &[ctrl]);
        Ok(())
    }

    /// Advances simulated time by `steps` physics steps.
    pub fn step(&mut self, steps: u32) {
        for _ in 0..steps {
            // SAFETY: model and data are live and belong together.
            unsafe { ffi::mj_step(self.model.as_ptr(), self.data.as_ptr()) };
        }
    }

    /// Simulated time in seconds (MuJoCo's `data.time`).
    pub fn time(&self) -> f64 {
        let [time] = self.get_state::<1>(ffi::mjSTATE_TIME);
        time
    }

    /// The two joints' `qpos`.
    pub fn pose(&self) -> Pose {
        let qpos = self.get_state::<QPOS_LEN>(ffi::mjSTATE_QPOS);
        Pose { cart: qpos[self.cart], pole: qpos[self.pole] }
    }

    /// MuJoCo warnings since the process started (P3: any fails a run).
    pub fn warnings(&self) -> usize {
        WARNINGS.load(Ordering::Relaxed)
    }

    fn id(&self, kind: c_int, label: &str, name: &str) -> Result<usize> {
        let c_name = CString::new(name).with_context(|| format!("{label} name {name:?} contains a NUL"))?;
        // SAFETY: model is live; c_name is NUL-terminated.
        let id = unsafe { ffi::mj_name2id(self.model.as_ptr(), kind, c_name.as_ptr()) };
        usize::try_from(id).map_err(|_| anyhow!("the model has no {label} named {name:?}"))
    }

    fn state_len(&self, sig: c_int) -> usize {
        // SAFETY: model is live.
        let len = unsafe { ffi::mj_stateSize(self.model.as_ptr(), sig) };
        usize::try_from(len).unwrap_or(0)
    }

    fn reset_data(&mut self) {
        // SAFETY: model and data are live and belong together.
        unsafe { ffi::mj_resetData(self.model.as_ptr(), self.data.as_ptr()) };
    }

    // N must equal mj_stateSize(sig): load checked it for QPOS and CTRL,
    // and TIME is one number by definition.
    fn get_state<const N: usize>(&self, sig: c_int) -> [f64; N] {
        let mut state = [0.0; N];
        // SAFETY: model and data are live; state holds the N numbers
        // mj_getState writes for sig.
        unsafe { ffi::mj_getState(self.model.as_ptr(), self.data.as_ptr(), state.as_mut_ptr(), sig) };
        state
    }

    fn set_state(&mut self, sig: c_int, state: &[f64]) {
        // SAFETY: model and data are live; state holds the
        // mj_stateSize(sig) numbers mj_setState reads (checked at load).
        unsafe { ffi::mj_setState(self.model.as_ptr(), self.data.as_ptr(), state.as_ptr(), sig) };
    }
}

impl Drop for Physics {
    fn drop(&mut self) {
        // SAFETY: both are live, owned only by self, and freed once; data
        // goes first because it was made from model.
        unsafe {
            ffi::mj_deleteData(self.data.as_ptr());
            ffi::mj_deleteModel(self.model.as_ptr());
        }
    }
}

// Called by MuJoCo for every log message. P5: it must not unwind into C,
// so it writes with writeln! and drops the result. An error message is
// followed by MuJoCo ending the process (mju_error does not return).
unsafe extern "C" fn log_handler(message: *const ffi::mjLogMessage) {
    // SAFETY: MuJoCo passes a valid message for the duration of the call.
    let Some(message) = (unsafe { message.as_ref() }) else {
        return;
    };
    let label = match message.level {
        ffi::mjLOG_ERROR => "error",
        ffi::mjLOG_WARNING => {
            WARNINGS.fetch_add(1, Ordering::Relaxed);
            "warning"
        }
        _ => "info",
    };
    let mut stderr = std::io::stderr();
    let _ = writeln!(stderr, "mujoco {label}: {}", c_text(&message.subject));
    if !message.body.is_null() {
        // SAFETY: a non-NULL body is a NUL-terminated string (mjtype.h).
        let body = unsafe { CStr::from_ptr(message.body) };
        let _ = writeln!(stderr, "{}", body.to_string_lossy());
    }
}

// Text from a fixed-size C buffer: up to the first NUL, or all of it.
fn c_text(buffer: &[c_char]) -> String {
    let bytes: Vec<u8> = buffer.iter().map(|&c| c as u8).collect();
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}
