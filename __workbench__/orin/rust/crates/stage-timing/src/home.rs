use std::{env, path::PathBuf};

/// A path from the settings with a leading `~` made into the home folder, as
/// the brain's `settings/home.rs` does, so this file can name the brain's
/// files the way its `settings.toml` does. `HOME` on the Orin; `USERPROFILE`
/// for the tests on the PC. A `~` with no home known is left as it is.
pub(crate) fn expand_home(path: &str) -> PathBuf {
    let Some(rest) = path.strip_prefix('~') else {
        return PathBuf::from(path);
    };
    if !(rest.is_empty() || rest.starts_with('/') || rest.starts_with('\\')) {
        return PathBuf::from(path);
    }
    let home = env::var_os("HOME").or_else(|| env::var_os("USERPROFILE"));
    match home {
        Some(home) => {
            let mut expanded = PathBuf::from(home);
            let rest = rest.trim_start_matches(['/', '\\']);
            if !rest.is_empty() {
                expanded.push(rest);
            }
            expanded
        }
        None => PathBuf::from(path),
    }
}
