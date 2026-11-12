use crate::proto::GenerationSet;

/// One set from its JSON (protobuf's standard JSON form). A misspelled field
/// or a value of the wrong type is an error, not a silent zero. The error is
/// serde_json's own: the caller knows which file the text came from and
/// names it.
pub(crate) fn parse_set(text: &str) -> Result<GenerationSet, serde_json::Error> {
    serde_json::from_str(text)
}
