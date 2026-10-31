//! The jump-limit experiment's trial records, sent by the Arduino: one
//! `POST /trial` per trial, so the whole record lives on the Pi and the
//! board keeps none of it. The body is one CSV line in the brief's
//! section 6 fields, with the trial's number at its j
//! (`kind,j,trial,c_a,c_b,dt_ms,switch_count,lost,pass`); the
//! Pi adds the time it arrived and appends it to `trials.csv` in the log
//! folder, written and flushed at once so a crash keeps every line.

use std::{
    collections::VecDeque,
    fs::{File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

/// The fields a trial line carries, in order: the file's header after
/// the Pi's own `time`.
pub const FIELDS: [&str; 9] = ["kind", "j", "trial", "c_a", "c_b", "dt_ms", "switch_count", "lost", "pass"];

/// One record's place in the run, for the page's progress bars.
#[derive(Clone, Debug)]
pub struct Place {
    pub kind: String,
    pub j: u32,
    pub trial: u32,
    /// When it arrived.
    pub at: std::time::Instant,
}

/// How many of the newest trials the page shows.
const RECENT: usize = 20;

pub struct TrialBook {
    path: PathBuf,
    file: Option<File>,
    /// Trials received since the service started.
    pub count: u64,
    recent: VecDeque<String>,
    /// The newest record's place in the run.
    pub last: Option<Place>,
}

impl TrialBook {
    pub fn new(dir: &Path) -> Self {
        Self { path: dir.join("trials.csv"), file: None, count: 0, recent: VecDeque::new(), last: None }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Checks and records one trial line. Errors say what was wrong with
    /// it, for the board's log.
    pub fn record(&mut self, body: &str) -> Result<(), String> {
        let line = body.trim();
        if line.contains('\n') {
            return Err("one trial per request".to_owned());
        }
        let fields: Vec<&str> = line.split(',').map(str::trim).collect();
        if fields.len() != FIELDS.len() {
            return Err(format!("expected {} fields ({}), got {}", FIELDS.len(), FIELDS.join(","), fields.len()));
        }
        for (name, value) in FIELDS.iter().zip(&fields).skip(1).take(7) {
            if value.parse::<i64>().is_err() {
                return Err(format!("{name} is not a whole number: {value}"));
            }
        }
        let stamped = format!("{},{}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"), fields.join(","));
        self.write(&stamped)?;
        self.count += 1;
        self.last = Some(Place {
            kind: fields[0].to_owned(),
            j: fields[1].parse().unwrap_or(0),
            trial: fields[2].parse().unwrap_or(0),
            at: std::time::Instant::now(),
        });
        self.recent.push_back(stamped);
        while self.recent.len() > RECENT {
            self.recent.pop_front();
        }
        Ok(())
    }

    /// The newest trials, oldest first.
    pub fn recent(&self) -> impl Iterator<Item = &String> {
        self.recent.iter()
    }

    /// Appends one line, opening the file (and writing the header to a new
    /// one) the first time.
    fn write(&mut self, line: &str) -> Result<(), String> {
        if self.file.is_none() {
            let new = !self.path.exists();
            if let Some(dir) = self.path.parent() {
                std::fs::create_dir_all(dir).map_err(|error| format!("cannot create {}: {error}", dir.display()))?;
            }
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)
                .map_err(|error| format!("cannot open {}: {error}", self.path.display()))?;
            if new {
                writeln!(file, "time,{}", FIELDS.join(",")).map_err(|error| error.to_string())?;
            }
            self.file = Some(file);
        }
        let Some(file) = self.file.as_mut() else {
            return Err("no trials file".to_owned());
        };
        let written = writeln!(file, "{line}").and_then(|()| file.flush());
        if let Err(error) = written {
            self.file = None;
            return Err(format!("cannot write {}: {error}", self.path.display()));
        }
        Ok(())
    }
}
