/// The Orin's webserver, where `--server` points when it isn't given: the
/// Orin's address on the rig's network (`__live__/README.md`) and the port in
/// `[server] address` of the webserver's `settings.toml`.
pub(crate) const DEFAULT_SERVER: &str = "http://192.168.0.215:8080";

/// Where the webserver takes a PNG: must match its `IMAGE_ROUTE`.
pub(crate) const IMAGE_ROUTE: &str = "/image";

/// PNG's registered media type.
pub(crate) const PNG_CONTENT_TYPE: &str = "image/png";

/// The 8 bytes every PNG file starts with (the W3C PNG specification, 5.2,
/// *PNG signature*).
/// Must match the webserver's `PNG_SIGNATURE`.
pub(crate) const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";
