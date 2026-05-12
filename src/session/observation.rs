use crate::error::{Result, ShoreError};
use crate::model::TrackId;

#[allow(dead_code)] // Used by the observation write workflow in a later plan task.
pub(crate) fn validated_track_id(value: &str) -> Result<TrackId> {
    let value = value.trim();
    if value.is_empty() {
        return Err(invalid_track_id("track id cannot be empty"));
    }
    if value.len() > 128 {
        return Err(invalid_track_id("track id must be 128 bytes or fewer"));
    }
    if matches!(value, "all" | "none" | "null" | "default" | "*") {
        return Err(invalid_track_id("track id is reserved"));
    }
    if value.starts_with("system:") || value.starts_with("import:") {
        return Err(invalid_track_id("track namespace is reserved"));
    }
    if !value.bytes().all(|byte| {
        byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b':')
    }) {
        return Err(invalid_track_id(
            "track id may only contain lowercase ASCII letters, digits, '-' and ':'",
        ));
    }

    Ok(TrackId::new(value.to_owned()))
}

fn invalid_track_id(message: &str) -> ShoreError {
    ShoreError::Message(message.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn track_policy_accepts_lowercase_local_and_namespaced_ids() {
        assert_eq!(validated_track_id("codex").unwrap().as_str(), "codex");
        assert_eq!(
            validated_track_id("agent:codex").unwrap().as_str(),
            "agent:codex"
        );
        assert_eq!(
            validated_track_id("human:kevin").unwrap().as_str(),
            "human:kevin"
        );
    }

    #[test]
    fn track_policy_rejects_reserved_or_unsafe_ids() {
        for bad in [
            "",
            "All",
            "all",
            "*",
            "none",
            "null",
            "default",
            "agent/codex",
            "agent codex",
            "system:shore",
            "import:hunk",
        ] {
            assert!(validated_track_id(bad).is_err(), "{bad} should be rejected");
        }
    }

    #[test]
    fn track_policy_rejects_overlong_ids() {
        let too_long = "a".repeat(129);

        assert!(validated_track_id(&too_long).is_err());
    }
}
