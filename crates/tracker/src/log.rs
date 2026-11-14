//! Owned by C2 (docs/ROADMAP.md): the duel log, a JSONL file with a session header and
//! one track event per line.

use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::TrackEvent;

/// Identifies one session (one duel). Serializes as a plain JSON string.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(transparent)]
pub struct SessionId(pub String);

/// What identifies a session, written as the duel log's header.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    /// The session's identifier.
    pub id: SessionId,
    /// When the operator started the session: **wall-clock** milliseconds since the Unix
    /// epoch. This is the one time in the contract that is not from `performance.now()`,
    /// because the log must record when the duel happened.
    pub started_at_ms: f64,
}

/// The ordered record of one session's track events, appended to a file as they happen.
#[derive(Debug)]
pub struct DuelLog {
    // Replaced by the real writer in C2.
    _placeholder: (),
}

impl DuelLog {
    /// Creates the log file at `path` and writes the `session` header.
    ///
    /// # Errors
    ///
    /// [`LogError::Io`] if the file cannot be created or written.
    pub fn create(path: impl AsRef<Path>, session: &SessionInfo) -> Result<Self, LogError> {
        let _ = (path.as_ref(), session);
        todo!("C2")
    }

    /// Appends one event to the log.
    ///
    /// # Errors
    ///
    /// [`LogError::Io`] if the event cannot be written.
    pub fn append(&mut self, event: &TrackEvent) -> Result<(), LogError> {
        let _ = event;
        todo!("C2")
    }
}

/// Why the duel log could not be written.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum LogError {
    /// Reading or writing the log file failed.
    #[error("duel log I/O failed")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn session_info_round_trips_with_camel_case_keys() {
        let session = SessionInfo {
            id: SessionId("2026-11-14T15-52-00".into()),
            started_at_ms: 1_763_135_520_000.0,
        };
        let expected = json!({ "id": "2026-11-14T15-52-00", "startedAtMs": 1_763_135_520_000.0 });

        assert_eq!(serde_json::to_value(&session).unwrap(), expected);
        assert_eq!(
            serde_json::from_value::<SessionInfo>(expected).unwrap(),
            session
        );
    }
}
