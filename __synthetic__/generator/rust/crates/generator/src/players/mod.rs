//! The Unity players that render the sets. Each is launched in generator mode
//! (`launch`) on its own port and window (`placement`, in the trainer's
//! window grid, `PlayerGrid`), reached over its
//! WebSocket (`connection`), and fed one `GenerationJob` at a time from the
//! shared pool until it is empty (`serve_player`); `run_players` runs them all
//! at once and gathers every finished run (`RunResult`), each checked against
//! the set it was given (`check_done`).

pub(crate) mod check_done;
pub(crate) mod connection;
pub(crate) mod launch;
pub(crate) mod placement;
pub(crate) mod player_grid;
pub(crate) mod run_players;
pub(crate) mod run_result;
pub(crate) mod serve_player;
pub(crate) mod window_size;

pub(crate) use crate::players::run_players::run_players;
