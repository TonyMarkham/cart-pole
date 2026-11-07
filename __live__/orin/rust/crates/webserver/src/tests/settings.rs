use crate::settings::Settings;

/// The file shipped beside the crates, so a key added to one but not the
/// other fails here rather than at the Orin's next start.
const SHIPPED: &str = include_str!("../../../../settings.toml");

#[test]
fn given_the_shipped_settings_when_loaded_then_the_server_section_is_read() {
    // Given
    let text = SHIPPED;

    // When
    let settings = toml::from_str::<Settings>(text).unwrap();

    // Then
    assert_eq!(settings.server.address, "0.0.0.0:8080");
    assert_eq!(settings.server.max_image_bytes, 16 * 1024 * 1024);
}
