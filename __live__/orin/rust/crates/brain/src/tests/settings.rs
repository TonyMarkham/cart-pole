use crate::settings::{Settings, home::expand_home};

use std::path::PathBuf;

/// The file shipped beside the crates, so a key added to one but not the
/// other fails here rather than at the Orin's next start.
const SHIPPED: &str = include_str!("../../../../settings.toml");

#[test]
fn given_the_shipped_settings_when_loaded_then_the_brains_tables_are_read() {
    // Given
    let text = SHIPPED;

    // When
    let settings = toml::from_str::<Settings>(text).unwrap();

    // Then
    assert_eq!(settings.brain.address, "127.0.0.1:8091");
    assert_eq!(
        settings.brain.worker,
        vec!["uv", "run", "python", "src/brain_worker.py"]
    );
    assert_eq!(settings.brain.delay_ms, 0);
    assert_eq!(settings.brain.pre_warm_gap_ms, 32.0);
    assert_eq!(settings.arduino.address, "192.168.0.220");
    assert_eq!(settings.arduino.port, 4210);
    assert_eq!(settings.camera.settle_frames, 30);
    assert_eq!(settings.camera.gain, 180);
}

#[test]
fn given_a_misspelt_brain_key_when_loaded_then_the_start_is_refused() {
    // Given
    let text = SHIPPED.replace("delay_ms = 0", "delay_milliseconds = 0");

    // When
    let settings = toml::from_str::<Settings>(&text);

    // Then
    assert!(settings.is_err());
}

#[test]
fn given_a_path_under_the_home_folder_when_expanded_then_the_home_folder_replaces_the_tilde() {
    // Given
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap();

    // When
    let expanded = expand_home("~/live-orin-python/homography.json");

    // Then
    assert_eq!(expanded, home.join("live-orin-python/homography.json"));
}

#[test]
fn given_a_path_without_a_tilde_when_expanded_then_it_is_unchanged() {
    assert_eq!(expand_home("runs"), PathBuf::from("runs"));
    assert_eq!(expand_home("~tony/runs"), PathBuf::from("~tony/runs"));
}
