use std::path::PathBuf;

/// One set's finished run, as its player reported it (`GenerationDone`) and
/// `check_done` accepted it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RunResult {
    /// The set's name.
    pub(crate) set_name: String,
    /// Whether the set is in generator.toml's `val` list.
    pub(crate) is_val: bool,
    /// The run folder the player wrote, `<output_dir>/run_<set>_<timestamp>`.
    pub(crate) run_directory: PathBuf,
    /// The frames in it: the set's `states`.
    pub(crate) frames: u32,
}
