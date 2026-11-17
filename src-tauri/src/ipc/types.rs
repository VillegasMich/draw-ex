//! The data types the IPC commands take and return, beyond those of the crates.
//!
//! All of them serialize with camelCase keys and camelCase enum variant names. Units:
//! coordinates are `f32` source-frame pixels, passcodes are integers, scores and
//! confidences are 0 to 1, sizes named `*_bytes` are bytes.

use card_db::{DeckId, Language};
use domain::{Passcode, Quad};
use serde::{Deserialize, Serialize};
use specta::Type;
use thiserror::Error;
use tracker::SessionId;

/// One card found in one frame, as `submit_frame` returns it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DetectionDto {
    /// Where the card is, in the submitted frame's pixels.
    pub quad: Quad,
    /// How sure the detector is that this is a card, from 0 to 1.
    #[specta(type = domain::FiniteNumber)]
    pub detector_confidence: f32,
    /// The top-k candidates, sorted by score, highest first.
    pub candidates: Vec<CandidateDto>,
    /// The verdict on this detection.
    pub recognition: RecognitionDto,
}

/// A passcode the classifier considers possible for a detection.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CandidateDto {
    /// The candidate card.
    pub passcode: Passcode,
    /// How likely the classifier finds this candidate, from 0 to 1.
    #[specta(type = domain::FiniteNumber)]
    pub score: f32,
}

/// The verdict on a single detection, tagged by `kind`:
/// `{ kind: "recognized", passcode, score }`, `{ kind: "lowConfidence" }`,
/// `{ kind: "notInDeck" }` or `{ kind: "unreadable" }`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Type)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum RecognitionDto {
    /// The detection is this candidate.
    Recognized {
        /// The recognized card.
        passcode: Passcode,
        /// The candidate's score, from 0 to 1.
        #[specta(type = domain::FiniteNumber)]
        score: f32,
    },
    /// The best candidate scored below the minimum score.
    LowConfidence,
    /// None of the top-k candidates is in the deck list.
    NotInDeck,
    /// The crop or the card's orientation could not be determined.
    Unreadable,
}

/// One of the interchangeable classifier builds (draw-core's `ModelVariant`).
///
/// Serializes as `"fp32"`, `"fp16"`, `"int8"` or `"yugiscanInt8"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ModelVariantDto {
    /// Full-precision ViT classifier (largest, most accurate).
    Fp32,
    /// Half-precision ViT classifier. Not recommended: the pinned file has 13,820 outputs
    /// but its labels file lists 13,968 cards, so it reports wrong cards.
    Fp16,
    /// 8-bit integer ViT classifier. The default.
    Int8,
    /// 8-bit integer Yugiscan ViT classifier.
    YugiscanInt8,
}

/// The operator's settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// The language card data is shown in. Default English.
    pub language: Language,
    /// The classifier build recognition uses. Default [`ModelVariantDto::Int8`].
    pub model_variant: ModelVariantDto,
    /// How many frames per second, per camera, are sent for recognition. Default 6.
    pub inference_fps: u8,
    /// The deck list recognition prefers, if any. Default none.
    pub selected_deck: Option<DeckId>,
    /// The card image cache's size cap, in megabytes (2^20 bytes). Default 1024.
    pub image_cache_mb: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: Language::En,
            model_variant: ModelVariantDto::Int8,
            inference_fps: 6,
            selected_deck: None,
            image_cache_mb: 1024,
        }
    }
}

/// Which model variants are installed, and which one is in use.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ModelStatus {
    /// The variant recognition uses (from the settings).
    pub current: ModelVariantDto,
    /// One entry per variant, in draw-core's `ModelVariant::ALL` order.
    pub variants: Vec<VariantStatus>,
}

/// The install state of one model variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct VariantStatus {
    /// The variant.
    pub variant: ModelVariantDto,
    /// Whether every file the variant needs is on disk and verified.
    pub installed: bool,
    /// Bytes of this variant's files on disk, including partial downloads.
    pub bytes_on_disk: u64,
    /// Bytes of all the files this variant needs, from draw-core's manifest.
    pub bytes_total: u64,
}

/// What the local caches hold.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CacheStats {
    /// Total size of the cached card images, in bytes.
    pub images_bytes: u64,
    /// Number of cached card image files.
    pub image_count: u32,
    /// Size of the card database file, in bytes.
    pub card_db_bytes: u64,
    /// YGOPRODeck's `database_version` of the local card data, or `None` before the first
    /// sync.
    pub card_db_version: Option<String>,
    /// Total size of the downloaded model files, in bytes.
    pub models_bytes: u64,
}

/// A local cache the operator can clear.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum CacheKind {
    /// The card image cache.
    Images,
    /// The card database. It is downloaded again on the next sync.
    CardData,
    /// The downloaded model files. They are downloaded again before recognition.
    Models,
}

/// An imported deck list, as listed for the operator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DeckSummary {
    /// The deck's identifier.
    pub id: DeckId,
    /// The deck's name.
    pub name: String,
    /// Number of cards across the main, extra and side decks, counting copies.
    pub card_count: u32,
}

/// What a finished session left behind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    /// The session's identifier.
    pub id: SessionId,
    /// Number of `confirmed` track events in the session.
    pub confirmed_count: u32,
    /// Where the session's duel log was saved: an absolute path under the app's data
    /// directory, for the operator to open. The stub returns a relative placeholder.
    pub log_path: String,
}

/// Why a command failed. Every command rejects with this value.
///
/// `message` is for the operator. It never contains a file system path other than one
/// the operator chose.
#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
#[error("{message}")]
pub struct IpcError {
    /// What kind of failure this is, for the frontend to react to.
    pub kind: IpcErrorKind,
    /// A human-readable explanation.
    pub message: String,
}

impl IpcError {
    /// Creates an error of `kind` with `message`.
    #[must_use]
    pub fn new(kind: IpcErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

/// The kinds of command failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum IpcErrorKind {
    /// An argument, header or body was missing or malformed.
    InvalidArgument,
    /// The requested item (card, deck, file) does not exist.
    NotFound,
    /// A prerequisite is missing, e.g. models not downloaded or no session started.
    NotReady,
    /// A request to YGOPRODeck or Hugging Face failed.
    Network,
    /// Reading or writing local files failed.
    Io,
    /// Anything else; a bug.
    Internal,
}

#[cfg(test)]
mod tests {
    use domain::Point;
    use serde_json::json;

    use super::*;

    #[test]
    fn detection_dto_round_trips_with_camel_case_keys_and_a_kind_tagged_recognition() {
        let detection = DetectionDto {
            quad: Quad([
                Point { x: 1.0, y: 2.0 },
                Point { x: 3.0, y: 2.0 },
                Point { x: 3.0, y: 6.0 },
                Point { x: 1.0, y: 6.0 },
            ]),
            detector_confidence: 0.75,
            candidates: vec![CandidateDto {
                passcode: Passcode(83_764_719),
                score: 0.5,
            }],
            recognition: RecognitionDto::Recognized {
                passcode: Passcode(83_764_719),
                score: 0.5,
            },
        };
        let expected = json!({
            "quad": [
                { "x": 1.0, "y": 2.0 },
                { "x": 3.0, "y": 2.0 },
                { "x": 3.0, "y": 6.0 },
                { "x": 1.0, "y": 6.0 },
            ],
            "detectorConfidence": 0.75,
            "candidates": [{ "passcode": 83_764_719, "score": 0.5 }],
            "recognition": { "kind": "recognized", "passcode": 83_764_719, "score": 0.5 },
        });

        assert_eq!(serde_json::to_value(&detection).unwrap(), expected);
        assert_eq!(
            serde_json::from_value::<DetectionDto>(expected).unwrap(),
            detection
        );
    }

    #[test]
    fn verdicts_without_a_card_serialize_as_a_bare_kind() {
        let cases = [
            (RecognitionDto::LowConfidence, "lowConfidence"),
            (RecognitionDto::NotInDeck, "notInDeck"),
            (RecognitionDto::Unreadable, "unreadable"),
        ];
        for (recognition, kind) in cases {
            assert_eq!(
                serde_json::to_value(recognition).unwrap(),
                json!({ "kind": kind })
            );
        }
    }

    #[test]
    fn settings_default_and_round_trip_with_camel_case_keys() {
        let expected = json!({
            "language": "en",
            "modelVariant": "int8",
            "inferenceFps": 6,
            "selectedDeck": null,
            "imageCacheMb": 1024,
        });

        assert_eq!(serde_json::to_value(Settings::default()).unwrap(), expected);
        assert_eq!(
            serde_json::from_value::<Settings>(expected).unwrap(),
            Settings::default()
        );
    }

    #[test]
    fn model_variants_and_cache_kinds_serialize_as_camel_case_names() {
        assert_eq!(
            serde_json::to_value(ModelVariantDto::YugiscanInt8).unwrap(),
            json!("yugiscanInt8")
        );
        assert_eq!(
            serde_json::to_value(CacheKind::CardData).unwrap(),
            json!("cardData")
        );
    }

    #[test]
    fn summaries_and_stats_use_camel_case_keys() {
        assert_eq!(
            serde_json::to_value(DeckSummary {
                id: DeckId("d1".into()),
                name: "Deck".into(),
                card_count: 40,
            })
            .unwrap(),
            json!({ "id": "d1", "name": "Deck", "cardCount": 40 })
        );
        assert_eq!(
            serde_json::to_value(SessionSummary {
                id: SessionId("s1".into()),
                confirmed_count: 3,
                log_path: "duels/s1.jsonl".into(),
            })
            .unwrap(),
            json!({ "id": "s1", "confirmedCount": 3, "logPath": "duels/s1.jsonl" })
        );
        assert_eq!(
            serde_json::to_value(CacheStats {
                images_bytes: 1,
                image_count: 2,
                card_db_bytes: 3,
                card_db_version: Some("1.0".into()),
                models_bytes: 4,
            })
            .unwrap(),
            json!({
                "imagesBytes": 1,
                "imageCount": 2,
                "cardDbBytes": 3,
                "cardDbVersion": "1.0",
                "modelsBytes": 4,
            })
        );
        assert_eq!(
            serde_json::to_value(VariantStatus {
                variant: ModelVariantDto::Int8,
                installed: false,
                bytes_on_disk: 0,
                bytes_total: 10,
            })
            .unwrap(),
            json!({ "variant": "int8", "installed": false, "bytesOnDisk": 0, "bytesTotal": 10 })
        );
    }

    #[test]
    fn ipc_error_serializes_its_kind_and_message() {
        assert_eq!(
            serde_json::to_value(IpcError::new(IpcErrorKind::InvalidArgument, "bad")).unwrap(),
            json!({ "kind": "invalidArgument", "message": "bad" })
        );
    }
}
