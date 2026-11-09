use crate::{
    constants::{IDLE_WAIT_MARGIN_MS, SERVICE_BUSY_PATH, STOP_POLL_MS},
    error::app_error::AppError,
    geometry::homography::Homography,
    log::run_log::RunLog,
    run::{run_loop, run_state::RunState, shared::Shared, status_answer::StatusAnswer},
    settings::{Settings, home::expand_home},
    worker::Worker,
};

use serde_json::Value;
use std::{
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
    time::{Duration, Instant, SystemTime},
};

/// The brain as the requests see it: the settings, the worker, the shared
/// state and the loop's thread while a run is going. Cloned into every
/// request; the clones share everything.
#[derive(Clone)]
pub(crate) struct Brain {
    pub(crate) settings: Arc<Settings>,
    pub(crate) worker: Arc<Mutex<Worker>>,
    pub(crate) shared: Arc<Mutex<Shared>>,
    thread: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl Brain {
    pub(crate) fn new(settings: Settings, worker: Worker) -> Brain {
        Brain {
            settings: Arc::new(settings),
            worker: Arc::new(Mutex::new(worker)),
            shared: Arc::new(Mutex::new(Shared::default())),
            thread: Arc::new(Mutex::new(None)),
        }
    }

    /// Run: refused, with the reason, while a run is going, while
    /// `calibrate_service.py` has the camera, when the matrix may not be used,
    /// or when the run's folder cannot be made (`real-loop-dia.md`, *The run
    /// and idle states*). Otherwise the loop starts on a thread of its own and
    /// this returns at once. Blocking: run it off the server's threads.
    pub(crate) fn start(&self) -> Result<(), String> {
        {
            let shared = self
                .shared
                .lock()
                .map_err(|_| "the brain's state is poisoned".to_string())?;
            if shared.state == RunState::Run {
                return Err("a run is going".to_string());
            }
        }
        // The last run's thread may still be closing its camera: it must have
        // let go of it before this run opens it.
        self.join_last_run();
        if self.service_busy() {
            return Err(
                "the calibration service has the camera: finish or cancel its job first"
                    .to_string(),
            );
        }
        let pressed = SystemTime::now();
        let homography = Homography::load(&expand_home(&self.settings.brain.homography))
            .map_err(|error| error.message().to_string())?;
        let log = RunLog::open(&expand_home(&self.settings.brain.runs_dir), pressed)
            .map_err(|error| error.message().to_string())?;
        let run_dir = log.dir.display().to_string();

        let mut shared = self
            .shared
            .lock()
            .map_err(|_| "the brain's state is poisoned".to_string())?;
        shared.state = RunState::Run;
        shared.message.clear();
        shared.run_dir = Some(run_dir);
        shared.frames = 0;
        shared.stop = false;
        drop(shared);

        let brain = self.clone();
        let handle = thread::Builder::new()
            .name("run".to_string())
            .spawn(move || {
                let ended = run_loop::run(&brain, homography, log);
                let reason = match ended {
                    Ok(reason) => reason,
                    Err(error) => {
                        eprintln!("{error}");
                        error.message().to_string()
                    }
                };
                println!("run ended: {reason}");
                // The loop marks idle itself before closing the camera; this
                // covers a run that failed before its loop began.
                brain.mark_idle(reason);
            })
            .map_err(|error| format!("the run's thread did not start: {error}"))?;
        if let Ok(mut thread) = self.thread.lock() {
            *thread = Some(handle);
        }
        Ok(())
    }

    /// Idle: asks the loop to stop and waits until it has, so that when this
    /// returns no command will follow (`real-loop-dia.md`, *The run and idle
    /// states*, STOP). It does not wait for the camera to close: that follows
    /// on the run's thread, and `start` waits for it. Idle while idle changes
    /// nothing. Blocking.
    pub(crate) fn stop(&self) {
        if let Ok(mut shared) = self.shared.lock() {
            shared.stop = true;
        }
        let wait =
            Duration::from_millis(self.settings.brain.frame_timeout_ms + IDLE_WAIT_MARGIN_MS);
        let deadline = Instant::now() + wait;
        while Instant::now() < deadline {
            if self
                .shared
                .lock()
                .map(|shared| shared.state == RunState::Idle)
                .unwrap_or(true)
            {
                return;
            }
            thread::sleep(Duration::from_millis(STOP_POLL_MS));
        }
        eprintln!(
            "idle: the loop did not stop within {} ms; the run's thread is left to end on its own",
            wait.as_millis()
        );
    }

    /// The brain is idle, and this is why the run ended or Run was refused.
    pub(crate) fn mark_idle(&self, message: String) {
        if let Ok(mut shared) = self.shared.lock() {
            shared.state = RunState::Idle;
            shared.message = message;
        }
    }

    pub(crate) fn status(&self) -> StatusAnswer {
        match self.shared.lock() {
            Ok(shared) => StatusAnswer::of(&shared),
            Err(_) => StatusAnswer {
                state: RunState::Idle,
                message: "the brain's state is poisoned".to_string(),
                run_dir: None,
                frames: 0,
            },
        }
    }

    /// Waits for the last run's thread, if any, to end: its camera is closed
    /// once it has.
    fn join_last_run(&self) {
        let handle = self.thread.lock().ok().and_then(|mut thread| thread.take());
        if let Some(handle) = handle {
            let _joined = handle.join();
        }
    }

    /// Whether `calibrate_service.py` has the camera. A service that does not
    /// answer is not running, and so not busy, as the webserver takes it for
    /// the moves.
    fn service_busy(&self) -> bool {
        let brain = &self.settings.brain;
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(Duration::from_millis(brain.service_timeout_ms)))
            .build()
            .into();
        let url = format!("{}{SERVICE_BUSY_PATH}", brain.service);
        let Ok(mut response) = agent.get(&url).call() else {
            return false;
        };
        response
            .body_mut()
            .read_to_vec()
            .ok()
            .and_then(|body| serde_json::from_slice::<Value>(&body).ok())
            .is_some_and(|answer| answer["busy"].as_bool() == Some(true))
    }

    /// The worker, for the loop. Poisoned only if a run's thread panicked
    /// holding it, which the denied `panic` lint is there to prevent.
    pub(crate) fn worker(&self) -> Result<std::sync::MutexGuard<'_, Worker>, AppError> {
        self.worker
            .lock()
            .map_err(|_| AppError::worker("the worker's lock is poisoned"))
    }
}
