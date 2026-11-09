//! MuJoCo for the sim loop (mujoco-plan.md): loads the MJCF the caller
//! names, sets the cart actuator's `ctrl`, steps, reads the joints'
//! `qpos`, the sites' world positions, the pole's speed and centre of
//! mass, the pole's mass, inertia and centre-of-mass distance, and the
//! cart top speed (the actuator's `ctrlrange`). All `unsafe` in the
//! workspace lives in this crate.

mod config;
mod ffi;
mod names;
mod pole;
mod pole_body;
mod pose;
mod sites;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------------------------- //

pub use crate::{
    config::PhysicsConfig, names::Names, pole::PoleState, pole_body::PoleBody, pose::Pose,
    sites::Sites,
};

use std::{
    ffi::{CStr, CString, c_char, c_int},
    io::Write,
    ptr::{self, NonNull},
    slice,
    sync::atomic::{AtomicUsize, Ordering},
};

use anyhow::{Context, Result, anyhow, bail};

// qpos and ctrl lengths the model check demands: one slide and one hinge
// take one qpos slot each; one actuator takes one ctrl slot.
const QPOS_LEN: usize = 2;
const CTRL_LEN: usize = 1;
// Two 1-DOF joints take one qvel slot each too.
const QVEL_LEN: usize = 2;

// The named sites: Left, Right, Pivot, End, Target, Dynamic Target, Cart,
// Cart Target and the four strike faces (synthetic-loop-dia.md, Contracts,
// the sites).
const SITES: usize = 12;

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
    // The cart top speed, m/s: the actuator's ctrlrange upper bound, which
    // load proved symmetric.
    max_speed: f64,
    // The model's site count, and the twelve named sites' ids in Sites'
    // field order.
    nsite: usize,
    site_ids: [usize; SITES],
    // The model's body count, the pole joint's body (the root of the
    // pole's subtree), and what load read of the pole's body.
    nbody: usize,
    pole_body_id: usize,
    pole_body: PoleBody,
}

impl Physics {
    /// Loads `config.model`, checks the library version (P1) and the
    /// model's shape (P2), measures the timestep, reads the cart top
    /// speed and reads the pole's body (check 7, __synthetic__/orchestrator/rust/crates/physics/README.md).
    /// Prints one line with what it found. The state is MuJoCo's reset state until
    /// [`Physics::reset`].
    pub fn load(config: &PhysicsConfig) -> Result<Physics> {
        // SAFETY: log_handler matches mjfLogHandler and never unwinds.
        unsafe { ffi::mju_setLogHandler(Some(log_handler)) };

        // SAFETY: no arguments, no preconditions.
        let version = unsafe { ffi::mj_version() };
        if version != ffi::MJ_VERSION {
            bail!(
                "mujoco.dll is version {version}, but the declarations in __synthetic__/orchestrator/rust/crates/physics/src/ffi.rs are for {}",
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
        let mut physics = Physics {
            model,
            data,
            cart: 0,
            pole: 0,
            timestep: 0.0,
            max_speed: 0.0,
            nsite: 0,
            site_ids: [0; SITES],
            nbody: 0,
            pole_body_id: 0,
            pole_body: PoleBody {
                mass: 0.0,
                com_distance: 0.0,
                inertia: 0.0,
                gravity: 0.0,
            },
        };
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

        let nsite = physics.check_layout(path)?;
        physics.nsite = nsite;
        // In Sites' field order.
        let site_names = [
            &names.left_site,
            &names.right_site,
            &names.pivot_site,
            &names.end_site,
            &names.target_site,
            &names.dynamic_target_site,
            &names.cart_site,
            &names.cart_target_site,
            &names.left_rail_site,
            &names.left_cart_site,
            &names.right_rail_site,
            &names.right_cart_site,
        ];
        let mut site_ids = [0; SITES];
        for (slot, name) in site_ids.iter_mut().zip(site_names) {
            *slot = physics.id(ffi::mjOBJ_SITE, "site", name)?;
        }
        physics.site_ids = site_ids;

        // R4: the timestep is a struct field; one step from reset reads it
        // as the elapsed time instead.
        physics.reset_data();
        physics.step(1);
        physics.timestep = physics.time();
        physics.reset_data();
        if !physics.timestep.is_finite() || physics.timestep <= 0.0 {
            bail!(
                "{path}: measured timestep {} s is not positive",
                physics.timestep
            );
        }
        physics.max_speed = physics.check_model(path, &names.actuator)?;
        physics.check_pole(path, &names.pole_joint)?;

        let body = physics.pole_body;
        eprintln!(
            "MuJoCo {}.{}.{}  qpos {qpos_len} ({} {cart}, {} {pole})  ctrl {ctrl_len} ({} {actuator})  timestep {} s  cart top speed {} m/s  sites {nsite}  pole {} kg, com {} m from the hinge, inertia {} kg m², gravity {} m/s²",
            version / 1_000_000,
            version / 1000 % 1000,
            version % 1000,
            names.cart_joint,
            names.pole_joint,
            names.actuator,
            physics.timestep,
            physics.max_speed,
            body.mass,
            body.com_distance,
            body.inertia,
            body.gravity
        );
        Ok(physics)
    }

    /// The pole's body: its mass, its centre of mass's distance from the
    /// pole joint's axis, its inertia about that axis and gravity's size.
    /// Read and proved at load; none of them changes with the pose.
    pub fn pole_body(&self) -> PoleBody {
        self.pole_body
    }

    /// The physics step, in seconds of simulated time.
    pub fn timestep(&self) -> f64 {
        self.timestep
    }

    /// The cart top speed, m/s: the cart actuator's `ctrlrange` upper
    /// bound. Load refused an actuator that is not `ctrllimited` or whose
    /// range is not symmetric, so MuJoCo clamps every `ctrl` to ± this.
    pub fn max_speed(&self) -> f64 {
        self.max_speed
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
        Pose {
            cart: qpos[self.cart],
            pole: qpos[self.pole],
        }
    }

    /// The twelve sites' world positions for the current `qpos`, by role.
    /// They are computed here (`mj_kinematics`) because after `mj_step`
    /// MuJoCo's own copy still describes the state before that step.
    /// Computing them changes nothing in the simulation: the next
    /// `mj_step` recomputes every derived quantity from `qpos` before it
    /// uses one.
    pub fn sites(&mut self) -> Sites {
        // SAFETY: model and data are live and belong together.
        unsafe { ffi::mj_kinematics(self.model.as_ptr(), self.data.as_ptr()) };
        // SAFETY: data is live; load proved site_xpos holds nsite x 3
        // numbers inside MuJoCo's buffer, and nsite = self.nsite.
        let xpos = unsafe { slice::from_raw_parts(self.data.as_ref().site_xpos, 3 * self.nsite) };
        let (positions, _) = xpos.as_chunks::<3>();
        // Each id came from mj_name2id at load, so it is below nsite.
        let [
            left,
            right,
            pivot,
            end,
            target,
            dynamic_target,
            cart,
            cart_target,
            left_rail,
            left_cart,
            right_rail,
            right_cart,
        ] = self.site_ids.map(|id| positions[id]);
        Sites {
            left,
            right,
            pivot,
            end,
            target,
            dynamic_target,
            cart,
            cart_target,
            left_rail,
            left_cart,
            right_rail,
            right_cart,
        }
    }

    /// The pole's state for the current `qpos` and `qvel`: the pole
    /// joint's speed and angle, the pole's centre of mass and the hinge
    /// point. The two points are computed here (`mj_kinematics`,
    /// `mj_comPos`) for the same reason as the sites, and that changes
    /// nothing in the simulation either.
    pub fn pole(&mut self) -> PoleState {
        // SAFETY: model and data are live and belong together.
        unsafe {
            ffi::mj_kinematics(self.model.as_ptr(), self.data.as_ptr());
            ffi::mj_comPos(self.model.as_ptr(), self.data.as_ptr());
        }
        debug_assert!(self.pole_body_id < self.nbody);
        // P2: with exactly two 1-DOF joints, a joint's qpos and qvel index
        // is its id.
        let speed = self.get_state::<QVEL_LEN>(ffi::mjSTATE_QVEL)[self.pole];
        let angle = self.get_state::<QPOS_LEN>(ffi::mjSTATE_QPOS)[self.pole];
        // SAFETY: data is live; load proved subtree_com holds nbody x 3
        // numbers and xanchor njnt x 3 inside MuJoCo's buffer, with
        // pole_body_id < nbody and pole < njnt.
        let (com, hinge) = unsafe {
            let data = self.data.as_ref();
            let point =
                |p: *const f64, i: usize| [*p.add(3 * i), *p.add(3 * i + 1), *p.add(3 * i + 2)];
            (
                point(data.subtree_com, self.pole_body_id),
                point(data.xanchor, self.pole),
            )
        };
        PoleState {
            speed,
            angle,
            com,
            hinge,
        }
    }

    /// MuJoCo warnings since the process started (P3: any fails a run).
    pub fn warnings(&self) -> usize {
        WARNINGS.load(Ordering::Relaxed)
    }

    fn id(&self, kind: c_int, label: &str, name: &str) -> Result<usize> {
        let c_name =
            CString::new(name).with_context(|| format!("{label} name {name:?} contains a NUL"))?;
        // SAFETY: model is live; c_name is NUL-terminated.
        let id = unsafe { ffi::mj_name2id(self.model.as_ptr(), kind, c_name.as_ptr()) };
        usize::try_from(id).map_err(|_| anyhow!("the model has no {label} named {name:?}"))
    }

    // Proves the mirrored struct prefixes in ffi.rs against the DLL before
    // anything reads through them, and returns nsite. Each check compares
    // a field read through the mirror with the same quantity from the
    // API, or with MuJoCo's buffer bounds: a wrong offset reads another
    // number, or an address outside the buffer. Pointers are bounds-checked
    // before any is followed. Leaves a probe state behind; load resets.
    fn check_layout(&mut self, path: &str) -> Result<usize> {
        let mismatch = || {
            anyhow!(
                "{path}: __synthetic__/orchestrator/rust/crates/physics/src/ffi.rs's mjModel/mjData layout does not match this mujoco.dll"
            )
        };

        // SAFETY: model is live and the fields are inside the mirrored prefix.
        let (nq, nu, nsite) = unsafe {
            let model = self.model.as_ref();
            (model.nq, model.nu, model.nsite)
        };
        if usize::try_from(nq).ok() != Some(self.state_len(ffi::mjSTATE_QPOS))
            || usize::try_from(nu).ok() != Some(self.state_len(ffi::mjSTATE_CTRL))
        {
            return Err(mismatch().context(format!("mjModel nq {nq}, nu {nu} vs mj_stateSize")));
        }
        let nsite = usize::try_from(nsite)
            .map_err(|_| mismatch().context(format!("mjModel nsite {nsite}")))?;

        // Distinct values, so a misread number cannot match by chance.
        let probe_time = 1.5;
        let probe_qpos = [0.25, -0.5];
        self.reset_data();
        self.set_state(ffi::mjSTATE_TIME, &[probe_time]);
        self.set_state(ffi::mjSTATE_QPOS, &probe_qpos);

        // SAFETY: data is live and the fields are inside the mirrored
        // prefix; nothing is dereferenced here.
        let (time, nbuffer, buffer, qpos, site_xpos) = unsafe {
            let data = self.data.as_ref();
            (
                data.time,
                data.nbuffer,
                data.buffer as usize,
                data.qpos,
                data.site_xpos,
            )
        };
        let nbuffer = usize::try_from(nbuffer)
            .map_err(|_| mismatch().context(format!("mjData nbuffer {nbuffer}")))?;
        let inside = |start: usize, len: usize| {
            buffer.checked_add(nbuffer).is_some_and(|end| {
                start >= buffer && start.checked_add(len).is_some_and(|stop| stop <= end)
            })
        };
        let number = size_of::<f64>();
        if time != probe_time {
            return Err(mismatch().context(format!(
                "mjData time reads {time}, mj_setState wrote {probe_time}"
            )));
        }
        if !inside(qpos as usize, QPOS_LEN * number) {
            return Err(mismatch().context("mjData qpos points outside MuJoCo's buffer"));
        }
        if nsite > 0 && !inside(site_xpos as usize, 3 * nsite * number) {
            return Err(mismatch().context("mjData site_xpos points outside MuJoCo's buffer"));
        }
        // SAFETY: qpos lies inside MuJoCo's buffer and holds QPOS_LEN
        // numbers (nq = QPOS_LEN, checked by load and above).
        let read = unsafe { [*qpos, *qpos.add(1)] };
        if read != probe_qpos {
            return Err(mismatch().context(format!(
                "mjData qpos reads {read:?}, mj_setState wrote {probe_qpos:?}"
            )));
        }

        // site_xpos can only be checked against MuJoCo's own use of it:
        // the point read through the mirror must be inside its site, and
        // a point 1 m away must not (a site is a marker, millimeters
        // across). Another array of the buffer read by mistake fails this.
        // SAFETY: model and data are live and belong together.
        unsafe { ffi::mj_kinematics(self.model.as_ptr(), self.data.as_ptr()) };
        for id in 0..nsite {
            // SAFETY: site_xpos lies inside the buffer with nsite x 3
            // numbers (checked above); id < nsite.
            let point = unsafe {
                [
                    *site_xpos.add(3 * id),
                    *site_xpos.add(3 * id + 1),
                    *site_xpos.add(3 * id + 2),
                ]
            };
            let away = [point[0] + 1.0, point[1], point[2]];
            let c_id = c_int::try_from(id)?;
            // SAFETY: model and data are live; id < nsite; each point is
            // three numbers.
            let (at, off) = unsafe {
                (
                    ffi::mj_insideSite(
                        self.model.as_ptr(),
                        self.data.as_ptr(),
                        c_id,
                        point.as_ptr(),
                    ),
                    ffi::mj_insideSite(
                        self.model.as_ptr(),
                        self.data.as_ptr(),
                        c_id,
                        away.as_ptr(),
                    ),
                )
            };
            if at != 1 || off != 0 {
                return Err(mismatch().context(format!("site {id}: site_xpos reads {point:?}, which mj_insideSite does not place in that site")));
            }
        }
        Ok(nsite)
    }

    // Proves the mirrored mjModel past nsite (ffi.rs) against the DLL and
    // returns the cart top speed. Needs the measured timestep; leaves
    // MuJoCo's reset state. Each read is compared with the same quantity
    // reached another way: opt.timestep with the measured timestep; every
    // pointer from buffer to actuator_ctrlrange with MuJoCo's buffer, which
    // lays its arrays out in header order (so each lies inside it, in
    // order); qpos0 with the qpos mj_resetData copies from it; and the
    // ctrlrange read with MuJoCo's own clamp of ctrl.
    fn check_model(&mut self, path: &str, actuator: &str) -> Result<f64> {
        let mismatch = || {
            anyhow!(
                "{path}: __synthetic__/orchestrator/rust/crates/physics/src/ffi.rs's mjModel layout does not match this mujoco.dll"
            )
        };

        // SAFETY: model is live; every field read is inside the mirrored
        // prefix, and the fields from buffer to actuator_ctrlrange are
        // consecutive pointers in a repr(C) struct, so they are read as
        // that many addresses. Nothing is dereferenced here.
        let (timestep, nbuffer, addresses, qpos0, limited, range) = unsafe {
            let model = self.model.as_ref();
            let first = ptr::addr_of!(model.buffer).cast::<usize>();
            let last = ptr::addr_of!(model.actuator_ctrlrange).cast::<usize>();
            let count = usize::try_from(last.offset_from(first)).unwrap_or(0) + 1;
            (
                model.opt.timestep,
                model.nbuffer,
                slice::from_raw_parts(first, count).to_vec(),
                model.qpos0,
                model.actuator_ctrllimited,
                model.actuator_ctrlrange,
            )
        };
        if timestep != self.timestep {
            return Err(mismatch().context(format!(
                "mjModel opt.timestep reads {timestep}, measured {}",
                self.timestep
            )));
        }
        let nbuffer = usize::try_from(nbuffer)
            .map_err(|_| mismatch().context(format!("mjModel nbuffer {nbuffer}")))?;
        let buffer = addresses[0];
        let end = buffer
            .checked_add(nbuffer)
            .ok_or_else(|| mismatch().context("mjModel buffer passes the address space"))?;
        if let Some(i) =
            (1..addresses.len()).find(|&i| addresses[i] < addresses[i - 1] || addresses[i] > end)
        {
            return Err(mismatch().context(format!(
                "mjModel pointer {i} after buffer is outside MuJoCo's buffer or out of order"
            )));
        }
        let number = size_of::<f64>();
        let inside = |start: usize, len: usize| {
            start >= buffer && start.checked_add(len).is_some_and(|stop| stop <= end)
        };
        if !inside(qpos0 as usize, QPOS_LEN * number)
            || !inside(limited as usize, CTRL_LEN)
            || !inside(range as usize, 2 * CTRL_LEN * number)
        {
            return Err(mismatch().context("mjModel qpos0, actuator_ctrllimited or actuator_ctrlrange runs past MuJoCo's buffer"));
        }

        self.reset_data();
        // SAFETY: qpos0 lies inside MuJoCo's buffer and holds QPOS_LEN
        // numbers (checked above).
        let read = unsafe { [*qpos0, *qpos0.add(1)] };
        let reset = self.get_state::<QPOS_LEN>(ffi::mjSTATE_QPOS);
        if read != reset {
            return Err(mismatch().context(format!(
                "mjModel qpos0 reads {read:?}, mj_resetData set qpos {reset:?}"
            )));
        }

        // A mirror shifted by one pointer still passes the buffer checks
        // (MuJoCo pads every array to 64 bytes), and reads a neighbouring
        // array here, so a refusal names that cause too.
        const SHIFTED: &str = " (if the MJCF says otherwise, __synthetic__/orchestrator/rust/crates/physics/src/ffi.rs's mjModel mirror is wrong)";
        // The actuator's id is 0 (P2). SAFETY: both lie inside MuJoCo's
        // buffer with room for the actuator's entries (checked above).
        let (limited, [lo, hi]) = unsafe { (*limited, [*range, *range.add(1)]) };
        match limited {
            1 => {}
            0 => bail!(
                "{path}: actuator {actuator:?} reads as not ctrllimited: the cart top speed is its ctrlrange{SHIFTED}"
            ),
            _ => {
                return Err(mismatch().context(format!(
                    "mjModel actuator_ctrllimited reads {limited}, not a bool"
                )));
            }
        }
        if !hi.is_finite() || hi <= 0.0 || lo != -hi {
            bail!(
                "{path}: actuator {actuator:?} ctrlrange reads [{lo}, {hi}]; the cart top speed needs it symmetric, lo = -hi > 0{SHIFTED}"
            );
        }

        // MuJoCo's own clamp: one step from rest at ctrl hi and at ctrl
        // hi + 1 gives the same speeds when MuJoCo clamps at hi or below,
        // and one at hi / 2 gives other speeds when it clamps above hi / 2
        // (a velocity servo from rest pushes kv x ctrl, so this assumes the
        // force limit does not bind at kv x hi / 2).
        if self.state_len(ffi::mjSTATE_QVEL) != QVEL_LEN {
            return Err(mismatch().context("qvel is not two numbers"));
        }
        let speeds = |physics: &mut Physics, ctrl: f64| {
            physics.reset_data();
            physics.set_state(ffi::mjSTATE_CTRL, &[ctrl]);
            physics.step(1);
            physics.get_state::<QVEL_LEN>(ffi::mjSTATE_QVEL)
        };
        let at_hi = speeds(self, hi);
        let above = speeds(self, hi + 1.0);
        let half = speeds(self, hi / 2.0);
        self.reset_data();
        if above != at_hi || half == at_hi {
            return Err(mismatch().context(format!(
                "ctrlrange reads [{lo}, {hi}], but MuJoCo's clamp disagrees (qvel at hi {at_hi:?}, at hi + 1 {above:?}, at hi / 2 {half:?}), or the force limit binds at hi / 2"
            )));
        }
        Ok(hi)
    }

    // Proves the mirrored body arrays and subtree_com (ffi.rs) against
    // MuJoCo's own definitions, then reads the pole's body. Each read is
    // compared with the same quantity reached another way: body_mass's sum
    // with mj_getTotalmass; and, at a probe pose, every body's subtree_com
    // and body_subtreemass with the mass-weighted centre and the mass of
    // its subtree, summed here from xipos and body_mass along
    // body_parentid. A misread array fails one of them. The inertia comes
    // from MuJoCo's own product with its inertia matrix (mj_mulM), checked
    // against the parallel-axis floor, mass x distance². Leaves MuJoCo's
    // reset state.
    fn check_pole(&mut self, path: &str, joint: &str) -> Result<()> {
        let mismatch = || {
            anyhow!(
                "{path}: __synthetic__/orchestrator/rust/crates/physics/src/ffi.rs's mjModel/mjData layout does not match this mujoco.dll"
            )
        };
        let number = size_of::<f64>();
        let int = size_of::<c_int>();
        // Rounding between two summation orders of the same numbers.
        let close = |a: f64, b: f64, scale: f64| (a - b).abs() <= 1e-9 * scale.max(1.0);

        // SAFETY: model is live and the fields are inside the mirrored
        // prefix; nothing is dereferenced here.
        let (nbody, njnt, buffer, nbuffer, parentid, body_mass, subtreemass, jnt_bodyid, gravity) = unsafe {
            let model = self.model.as_ref();
            (
                model.nbody,
                model.njnt,
                model.buffer as usize,
                model.nbuffer,
                model.body_parentid,
                model.body_mass,
                model.body_subtreemass,
                model.jnt_bodyid,
                model.opt.gravity,
            )
        };
        let count = |n: i64, what: &str| {
            usize::try_from(n).map_err(|_| mismatch().context(format!("mjModel {what} {n}")))
        };
        let (nbody, njnt, nbuffer) = (
            count(nbody, "nbody")?,
            count(njnt, "njnt")?,
            count(nbuffer, "nbuffer")?,
        );
        let inside = |start: usize, len: usize| {
            buffer.checked_add(nbuffer).is_some_and(|end| {
                start >= buffer && start.checked_add(len).is_some_and(|stop| stop <= end)
            })
        };
        if !inside(parentid as usize, nbody * int)
            || !inside(body_mass as usize, nbody * number)
            || !inside(subtreemass as usize, nbody * number)
            || !inside(jnt_bodyid as usize, njnt * int)
            || self.pole >= njnt
        {
            return Err(mismatch().context("mjModel body_parentid, body_mass, body_subtreemass or jnt_bodyid runs past MuJoCo's buffer"));
        }
        // SAFETY: each lies inside MuJoCo's buffer with nbody entries
        // (jnt_bodyid njnt, and pole < njnt), checked above.
        let (parent, mass, subtreemass, body) = unsafe {
            (
                slice::from_raw_parts(parentid, nbody).to_vec(),
                slice::from_raw_parts(body_mass, nbody).to_vec(),
                slice::from_raw_parts(subtreemass, nbody).to_vec(),
                *jnt_bodyid.add(self.pole),
            )
        };
        // MuJoCo numbers every body after its parent; the world, body 0, is
        // its own parent.
        let parent: Vec<usize> = parent
            .iter()
            .map(|&p| usize::try_from(p).unwrap_or(usize::MAX))
            .collect();
        if nbody == 0 || parent[0] != 0 || (1..nbody).any(|b| parent[b] >= b) {
            return Err(mismatch()
                .context("mjModel body_parentid does not list each body after its parent"));
        }
        // SAFETY: model is live.
        let total = unsafe { ffi::mj_getTotalmass(self.model.as_ptr()) };
        let sum: f64 = mass.iter().sum();
        if !close(sum, total, total) {
            return Err(mismatch().context(format!(
                "mjModel body_mass adds up to {sum} kg, mj_getTotalmass gives {total}"
            )));
        }
        let body = usize::try_from(body)
            .ok()
            .filter(|&b| b > 0 && b < nbody)
            .ok_or_else(|| {
                mismatch().context(format!("mjModel jnt_bodyid of {joint} reads {body}"))
            })?;

        // A probe pose off the reset one, so a centre that ignored the
        // pose could not match. mj_forward computes poses, subtree_com and
        // the inertia matrix.
        self.reset_data();
        self.set_state(ffi::mjSTATE_QPOS, &[0.1, 1.0]);
        // SAFETY: model and data are live and belong together.
        unsafe { ffi::mj_forward(self.model.as_ptr(), self.data.as_ptr()) };
        // SAFETY: data is live and the fields are inside the mirrored
        // prefix; nothing is dereferenced here.
        let (dbuffer, dnbuffer, xipos, xanchor, xaxis, subtree_com) = unsafe {
            let data = self.data.as_ref();
            (
                data.buffer as usize,
                data.nbuffer,
                data.xipos,
                data.xanchor,
                data.xaxis,
                data.subtree_com,
            )
        };
        let dnbuffer = usize::try_from(dnbuffer)
            .map_err(|_| mismatch().context(format!("mjData nbuffer {dnbuffer}")))?;
        let in_data = |start: usize, len: usize| {
            dbuffer.checked_add(dnbuffer).is_some_and(|end| {
                start >= dbuffer && start.checked_add(len).is_some_and(|stop| stop <= end)
            })
        };
        if !in_data(xipos as usize, 3 * nbody * number)
            || !in_data(subtree_com as usize, 3 * nbody * number)
            || !in_data(xanchor as usize, 3 * njnt * number)
            || !in_data(xaxis as usize, 3 * njnt * number)
        {
            return Err(mismatch()
                .context("mjData xipos, subtree_com, xanchor or xaxis runs past MuJoCo's buffer"));
        }
        // SAFETY: each lies inside MuJoCo's buffer with 3 x nbody (3 x
        // njnt) numbers, checked above; pole < njnt.
        let (xipos, subtree_com, anchor, axis) = unsafe {
            let point =
                |p: *const f64, i: usize| [*p.add(3 * i), *p.add(3 * i + 1), *p.add(3 * i + 2)];
            (
                (0..nbody).map(|b| point(xipos, b)).collect::<Vec<_>>(),
                (0..nbody)
                    .map(|b| point(subtree_com, b))
                    .collect::<Vec<_>>(),
                point(xanchor, self.pole),
                point(xaxis, self.pole),
            )
        };

        // Children before parents: each body's sums pass on to its parent.
        let mut sub_mass = mass.clone();
        let mut moment: Vec<[f64; 3]> = (0..nbody).map(|b| xipos[b].map(|v| v * mass[b])).collect();
        for b in (1..nbody).rev() {
            let p = parent[b];
            sub_mass[p] += sub_mass[b];
            let child = moment[b];
            for (sum, v) in moment[p].iter_mut().zip(child) {
                *sum += v;
            }
        }
        for b in 0..nbody {
            if !close(sub_mass[b], subtreemass[b], total) {
                return Err(mismatch().context(format!(
                    "body {b}: body_subtreemass reads {} kg, its subtree's masses add up to {}",
                    subtreemass[b], sub_mass[b]
                )));
            }
            if sub_mass[b] > 0.0 {
                let centre = moment[b].map(|v| v / sub_mass[b]);
                if (0..3).any(|i| !close(centre[i], subtree_com[b][i], 1.0)) {
                    return Err(mismatch().context(format!(
                        "body {b}: subtree_com reads {:?}, its subtree's mass-weighted centre is {centre:?}",
                        subtree_com[b]
                    )));
                }
            }
        }

        let pole_mass = subtreemass[body];
        if !pole_mass.is_finite() || pole_mass <= 0.0 {
            bail!(
                "{path}: everything that swings on {joint} weighs {pole_mass} kg; the pole's energy needs a mass"
            );
        }
        // The centre's distance from the axis: the part of the hinge-to-centre
        // arrow at right angles to the axis.
        let length = axis.iter().map(|v| v * v).sum::<f64>().sqrt();
        if !close(length, 1.0, 1.0) {
            return Err(mismatch().context(format!(
                "mjData xaxis of {joint} reads {axis:?}, not a unit vector"
            )));
        }
        let arm: [f64; 3] = std::array::from_fn(|i| subtree_com[body][i] - anchor[i]);
        let along: f64 = (0..3).map(|i| arm[i] * axis[i]).sum();
        let com_distance = (0..3)
            .map(|i| (arm[i] - along * axis[i]).powi(2))
            .sum::<f64>()
            .sqrt();
        if !com_distance.is_finite() || com_distance <= 0.0 {
            bail!(
                "{path}: the centre of mass of what swings on {joint} lies on its axis; the pole's height energy needs it off the axis"
            );
        }

        // The joint's diagonal entry of the inertia matrix: M times the
        // unit vector of its dof. P2: its dof index is its joint id, and
        // check_model proved nv = QVEL_LEN.
        let mut unit = [0.0; QVEL_LEN];
        unit[self.pole] = 1.0;
        let mut product = [0.0; QVEL_LEN];
        // SAFETY: model and data are live and belong together; both
        // vectors hold nv numbers; mj_forward made the inertia matrix.
        unsafe {
            ffi::mj_mulM(
                self.model.as_ptr(),
                self.data.as_ptr(),
                product.as_mut_ptr(),
                unit.as_ptr(),
            )
        };
        let inertia = product[self.pole];
        let floor = pole_mass * com_distance * com_distance;
        if !inertia.is_finite() || inertia < floor * (1.0 - 1e-9) {
            return Err(mismatch().context(format!(
                "the inertia about {joint} reads {inertia} kg m², below its parallel-axis floor, mass x distance² = {floor}"
            )));
        }

        let [gx, gy, gz] = gravity;
        if gx != 0.0 || gy != 0.0 || !gz.is_finite() || gz >= 0.0 {
            bail!(
                "{path}: gravity reads {gravity:?} m/s²; the pole's height energy needs it straight down, -z"
            );
        }

        self.reset_data();
        self.nbody = nbody;
        self.pole_body_id = body;
        self.pole_body = PoleBody {
            mass: pole_mass,
            com_distance,
            inertia,
            gravity: -gz,
        };
        Ok(())
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

    // N must equal mj_stateSize(sig): load checked it for QPOS, QVEL and
    // CTRL, and TIME is one number by definition.
    fn get_state<const N: usize>(&self, sig: c_int) -> [f64; N] {
        let mut state = [0.0; N];
        // SAFETY: model and data are live; state holds the N numbers
        // mj_getState writes for sig.
        unsafe {
            ffi::mj_getState(
                self.model.as_ptr(),
                self.data.as_ptr(),
                state.as_mut_ptr(),
                sig,
            )
        };
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
