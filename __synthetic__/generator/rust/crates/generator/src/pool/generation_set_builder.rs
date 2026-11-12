use crate::proto::{FixedPose, GenerationSet, Range};

/// Builds one set of the pool from the master: it starts as a copy of the
/// master, and each method replaces one key. `overlay` calls the method of
/// every key a set file gives, and only those.
///
/// Example: the master with seed 1001, and Set-02.json holding only
/// `{"name": "Set-02", "seed": 2002}`, gives the master's every value except
/// name "Set-02" and seed 2002:
/// `GenerationSetBuilder::new(master).name("Set-02".into()).seed(2002).build()`.
#[derive(Debug)]
pub(crate) struct GenerationSetBuilder {
    set: GenerationSet,
}

impl GenerationSetBuilder {
    pub(crate) const fn new(master: GenerationSet) -> Self {
        Self { set: master }
    }

    /// Replaces every key the set file gives. In proto3 a key left out of
    /// the JSON reads as its zero value, so a key counts as given when it is
    /// not zero: a non-zero number, a non-empty string or list, a range or
    /// max_z that is present. A set file therefore cannot change a key to zero or empty.
    #[must_use]
    pub(crate) fn overlay(mut self, set: GenerationSet) -> Self {
        // Every field named, so a field added to generation_set.proto is a
        // compile error here until it is merged too.
        let GenerationSet {
            name,
            seed,
            states,
            fixed_poses,
            fixed_pose_frames,
            camera_image_width,
            camera_image_height,
            camera_yaw,
            camera_pitch,
            camera_height,
            camera_lateral,
            max_z,
            desks,
            skyboxes,
            skybox_yaw,
            light_yaw,
            light_pitch,
            light_intensity,
            unlimited_hinge,
        } = set;
        if !name.is_empty() {
            self = self.name(name);
        }
        if seed != 0 {
            self = self.seed(seed);
        }
        if states != 0 {
            self = self.states(states);
        }
        if !fixed_poses.is_empty() {
            self = self.fixed_poses(fixed_poses);
        }
        if fixed_pose_frames != 0 {
            self = self.fixed_pose_frames(fixed_pose_frames);
        }
        if camera_image_width != 0 {
            self = self.camera_image_width(camera_image_width);
        }
        if camera_image_height != 0 {
            self = self.camera_image_height(camera_image_height);
        }
        if let Some(range) = camera_yaw {
            self = self.camera_yaw(range);
        }
        if let Some(range) = camera_pitch {
            self = self.camera_pitch(range);
        }
        if let Some(range) = camera_height {
            self = self.camera_height(range);
        }
        if let Some(range) = camera_lateral {
            self = self.camera_lateral(range);
        }
        if let Some(meters) = max_z {
            self = self.max_z(meters);
        }
        if !desks.is_empty() {
            self = self.desks(desks);
        }
        if !skyboxes.is_empty() {
            self = self.skyboxes(skyboxes);
        }
        if let Some(range) = skybox_yaw {
            self = self.skybox_yaw(range);
        }
        if let Some(range) = light_yaw {
            self = self.light_yaw(range);
        }
        if let Some(range) = light_pitch {
            self = self.light_pitch(range);
        }
        if let Some(range) = light_intensity {
            self = self.light_intensity(range);
        }
        if let Some(range) = unlimited_hinge {
            self = self.unlimited_hinge(range);
        }
        self
    }

    #[must_use]
    pub(crate) fn name(mut self, name: String) -> Self {
        self.set.name = name;
        self
    }

    #[must_use]
    pub(crate) const fn seed(mut self, seed: i32) -> Self {
        self.set.seed = seed;
        self
    }

    #[must_use]
    pub(crate) const fn states(mut self, states: u32) -> Self {
        self.set.states = states;
        self
    }

    #[must_use]
    pub(crate) fn fixed_poses(mut self, fixed_poses: Vec<FixedPose>) -> Self {
        self.set.fixed_poses = fixed_poses;
        self
    }

    #[must_use]
    pub(crate) const fn fixed_pose_frames(mut self, frames: u32) -> Self {
        self.set.fixed_pose_frames = frames;
        self
    }

    #[must_use]
    pub(crate) const fn camera_image_width(mut self, pixels: u32) -> Self {
        self.set.camera_image_width = pixels;
        self
    }

    #[must_use]
    pub(crate) const fn camera_image_height(mut self, pixels: u32) -> Self {
        self.set.camera_image_height = pixels;
        self
    }

    #[must_use]
    pub(crate) const fn camera_yaw(mut self, range: Range) -> Self {
        self.set.camera_yaw = Some(range);
        self
    }

    #[must_use]
    pub(crate) const fn camera_pitch(mut self, range: Range) -> Self {
        self.set.camera_pitch = Some(range);
        self
    }

    #[must_use]
    pub(crate) const fn camera_height(mut self, range: Range) -> Self {
        self.set.camera_height = Some(range);
        self
    }

    #[must_use]
    pub(crate) const fn camera_lateral(mut self, range: Range) -> Self {
        self.set.camera_lateral = Some(range);
        self
    }

    #[must_use]
    pub(crate) const fn max_z(mut self, meters: f32) -> Self {
        self.set.max_z = Some(meters);
        self
    }

    #[must_use]
    pub(crate) fn desks(mut self, desks: Vec<String>) -> Self {
        self.set.desks = desks;
        self
    }

    #[must_use]
    pub(crate) fn skyboxes(mut self, skyboxes: Vec<String>) -> Self {
        self.set.skyboxes = skyboxes;
        self
    }

    #[must_use]
    pub(crate) const fn skybox_yaw(mut self, range: Range) -> Self {
        self.set.skybox_yaw = Some(range);
        self
    }

    #[must_use]
    pub(crate) const fn light_yaw(mut self, range: Range) -> Self {
        self.set.light_yaw = Some(range);
        self
    }

    #[must_use]
    pub(crate) const fn light_pitch(mut self, range: Range) -> Self {
        self.set.light_pitch = Some(range);
        self
    }

    #[must_use]
    pub(crate) const fn light_intensity(mut self, range: Range) -> Self {
        self.set.light_intensity = Some(range);
        self
    }

    #[must_use]
    pub(crate) const fn unlimited_hinge(mut self, range: Range) -> Self {
        self.set.unlimited_hinge = Some(range);
        self
    }

    #[must_use]
    pub(crate) fn build(self) -> GenerationSet {
        self.set
    }
}
