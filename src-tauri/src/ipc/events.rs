//! The events Rust emits to the webview. The event name is in each type's docs.

use card_db::SyncProgress;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri_specta::Event;
use tracker::TrackEvent;

use super::types::ModelVariantDto;

/// Event `track-event`: a change to the confirmed cards on the table. The payload is
/// the tracker's [`TrackEvent`] as is.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type, Event)]
#[serde(transparent)]
#[tauri_specta(event_name = "track-event")]
pub struct TrackEventPayload(pub TrackEvent);

/// Event `model-download-progress`: how far the download of one model file has got.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct ModelDownloadProgress {
    /// The variant being downloaded.
    pub variant: ModelVariantDto,
    /// The file's path inside the model repo, as draw-core's manifest lists it, e.g.
    /// `onnx/ygo_yolo.onnx`.
    pub file: String,
    /// Bytes of this file downloaded so far.
    pub bytes_done: u64,
    /// Size of this file, in bytes.
    pub bytes_total: u64,
}

/// Event `card-db-sync-progress`: how far a card database sync has got. The payload is
/// card-db's [`SyncProgress`] as is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, Event)]
#[serde(transparent)]
pub struct CardDbSyncProgress(pub SyncProgress);

#[cfg(test)]
mod tests {
    use card_db::SyncStage;
    use domain::{Passcode, Player};
    use serde_json::json;
    use tracker::TrackId;

    use super::*;

    #[test]
    fn event_names_match_the_contract() {
        assert_eq!(TrackEventPayload::NAME, "track-event");
        assert_eq!(ModelDownloadProgress::NAME, "model-download-progress");
        assert_eq!(CardDbSyncProgress::NAME, "card-db-sync-progress");
    }

    #[test]
    fn event_payloads_use_camel_case_keys_and_wrappers_are_transparent() {
        assert_eq!(
            serde_json::to_value(TrackEventPayload(TrackEvent::Lost {
                track: TrackId(7),
                player: Player::B,
                passcode: Passcode(55_144_522),
                time_ms: 10.5,
            }))
            .unwrap(),
            json!({
                "kind": "lost",
                "track": 7,
                "player": "b",
                "passcode": 55_144_522,
                "timeMs": 10.5,
            })
        );
        assert_eq!(
            serde_json::to_value(ModelDownloadProgress {
                variant: ModelVariantDto::Fp16,
                file: "onnx/ygo_yolo.onnx".into(),
                bytes_done: 5,
                bytes_total: 10,
            })
            .unwrap(),
            json!({
                "variant": "fp16",
                "file": "onnx/ygo_yolo.onnx",
                "bytesDone": 5,
                "bytesTotal": 10,
            })
        );
        assert_eq!(
            serde_json::to_value(CardDbSyncProgress(SyncProgress {
                stage: SyncStage::Storing,
                done: 1,
                total: 2,
            }))
            .unwrap(),
            json!({ "stage": "storing", "done": 1, "total": 2 })
        );
    }
}
