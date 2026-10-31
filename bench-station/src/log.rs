//! The run's record: every line that crosses the serial line, and the
//! Pi's own notes, written to a file as it happens and kept in memory for
//! the page. A new file starts each time the serial line opens (the
//! service starting, the board coming back after a flash or a replug), so
//! nothing a reset or a crash interrupts is lost.

use std::{
    collections::VecDeque,
    fs::File,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

/// Who wrote a line: the Arduino, the Pi to the Arduino, or the Pi alone.
#[derive(Clone, Copy, Debug)]
pub enum Source {
    Board,
    Sent,
    Note,
}

impl Source {
    /// The mark between the time and the text: `<` from the board, `>` to
    /// it, `#` the Pi's own.
    const fn mark(self) -> char {
        match self {
            Source::Board => '<',
            Source::Sent => '>',
            Source::Note => '#',
        }
    }
}

pub struct LogBook {
    dir: PathBuf,
    file: Option<BufWriter<File>>,
    path: Option<PathBuf>,
    recent: VecDeque<String>,
    keep: usize,
}

impl LogBook {
    pub fn new(dir: &Path, keep: usize) -> Self {
        Self { dir: dir.to_owned(), file: None, path: None, recent: VecDeque::new(), keep }
    }

    /// Starts a new file, `log-<date>_<time>.txt`, closing the last.
    pub fn start_file(&mut self) {
        self.close();
        let path = self.dir.join(format!("log-{}.txt", stamp()));
        match std::fs::create_dir_all(&self.dir).and_then(|()| File::create(&path)) {
            Ok(file) => {
                self.file = Some(BufWriter::new(file));
                self.path = Some(path.clone());
                self.write(Source::Note, &format!("log file {}", path.display()));
            }
            Err(error) => self.write(Source::Note, &format!("cannot create {}: {error}; logging to the page only", path.display())),
        }
    }

    /// Flushes and closes the file, so it is whole on disk.
    pub fn close(&mut self) {
        if let Some(mut file) = self.file.take() {
            let _ = file.flush();
        }
    }

    /// One line, timestamped; flushed at once, so a crash keeps it.
    pub fn write(&mut self, source: Source, text: &str) {
        let line = format!("{} {} {text}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"), source.mark());
        if let Some(file) = self.file.as_mut()
            && (writeln!(file, "{line}").is_err() || file.flush().is_err())
        {
            self.file = None;
            self.recent.push_back("(the log file could not be written; logging to the page only)".to_owned());
        }
        self.recent.push_back(line);
        while self.recent.len() > self.keep {
            self.recent.pop_front();
        }
    }

    pub fn recent(&self) -> impl Iterator<Item = &String> {
        self.recent.iter()
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }
}

/// The local time as `YYYYMMDD_HHMMSS`, for file names.
pub fn stamp() -> String {
    chrono::Local::now().format("%Y%m%d_%H%M%S").to_string()
}
