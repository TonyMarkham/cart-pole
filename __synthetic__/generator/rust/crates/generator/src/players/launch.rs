use crate::{
    config::Config,
    error::{app_error::AppError, app_result::AppResult},
    players::{placement::window_position, player_grid::PlayerGrid},
};

use std::process::Stdio;
use tokio::process::{Child, Command};
use tracing::info;

/// Picks the player's mode: PlayerController's `k_PlayerModeArgument`
/// (unity/packages/simulation.cart-pole.sim-loop/Runtime/PlayerController.cs).
const PLAYER_MODE_ARGUMENT: &str = "-player-mode";

/// The generator mode's name: `PlayerMode.SyntheticGenerator`
/// (unity/packages/simulation.cart-pole.sim-loop/Runtime/PlayerMode.cs).
const GENERATOR_MODE: &str = "SyntheticGenerator";

/// The StateServer's port: `StateServer.AddressArgument` (greyhound's
/// com.greyhound.mujoco, Runtime/StateServer.cs), followed by the port.
const STATE_SERVER_ARGUMENT: &str = "-state-server";

/// The window's top-left corner as `x,y`: PlayerController's
/// `k_WindowPositionArgument`, followed by the two numbers joined by
/// `POSITION_SEPARATOR`.
const WINDOW_POSITION_ARGUMENT: &str = "-window-position";

/// Between x and y in the window position: PlayerController's
/// `k_PositionSeparator`, which it splits the value on.
const POSITION_SEPARATOR: char = ',';

// Unity's standalone player arguments, from the Unity Manual's "Unity
// Standalone Player command line arguments".

/// Picks full screen or a window; followed by `WINDOWED`.
const FULLSCREEN_ARGUMENT: &str = "-screen-fullscreen";

/// `-screen-fullscreen`'s value for a window: 0 is off (1 is full screen, the
/// build's default, which would put every player over the whole screen).
const WINDOWED: &str = "0";

/// A window with no border or title bar, so the grid's cells touch.
const BORDERLESS_ARGUMENT: &str = "-popupwindow";

/// The window's width in pixels; followed by the grid's window width.
const SCREEN_WIDTH_ARGUMENT: &str = "-screen-width";

/// The window's height in pixels; followed by the grid's window height.
const SCREEN_HEIGHT_ARGUMENT: &str = "-screen-height";

/// Where the player writes its log; followed by the file's path.
const LOG_FILE_ARGUMENT: &str = "-logFile";

/// Player `k`'s log file name in generator.toml's `log_dir` starts with
/// this, then `k`: `player_<k>.log`.
const LOG_FILE_PREFIX: &str = "player_";

/// The log file's extension, after the dot: `player_<k>.log`.
const LOG_FILE_EXTENSION: &str = "log";

/// Starts player `k` in generator mode, listening on `port`, its window at its
/// place in `grid` (`window_position`) and its log in `log_dir`. Example,
/// player 5 of a 3 x 4 grid on 3840 x 2112: SyntheticCamera.exe -player-mode
/// SyntheticGenerator -state-server 9106 -window-position 960,704
/// -screen-fullscreen 0 -popupwindow -screen-width 960 -screen-height 704
/// -logFile <log_dir>/player_5.log. The process is killed if the generator
/// drops it, so a generator that stops on an error leaves no player behind.
pub(crate) fn launch(config: &Config, grid: PlayerGrid, k: u16, port: u16) -> AppResult<Child> {
    let (x, y) = window_position(k, grid.columns, grid.window);
    let position = format!("{x}{POSITION_SEPARATOR}{y}");
    let log_file = config
        .log_dir
        .join(format!("{LOG_FILE_PREFIX}{k}.{LOG_FILE_EXTENSION}"));
    let mut command = Command::new(&config.player);
    command
        .arg(PLAYER_MODE_ARGUMENT)
        .arg(GENERATOR_MODE)
        .arg(STATE_SERVER_ARGUMENT)
        .arg(port.to_string())
        .arg(WINDOW_POSITION_ARGUMENT)
        .arg(&position)
        .arg(FULLSCREEN_ARGUMENT)
        .arg(WINDOWED)
        .arg(BORDERLESS_ARGUMENT)
        .arg(SCREEN_WIDTH_ARGUMENT)
        .arg(grid.window.width.to_string())
        .arg(SCREEN_HEIGHT_ARGUMENT)
        .arg(grid.window.height.to_string())
        .arg(LOG_FILE_ARGUMENT)
        .arg(&log_file)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    info!(
        player = k,
        port,
        window = %position,
        log = %log_file.display(),
        "launching"
    );
    command.spawn().map_err(|error| {
        AppError::player(format!(
            "launching player {k} ({}): {error}",
            config.player.display()
        ))
    })
}
