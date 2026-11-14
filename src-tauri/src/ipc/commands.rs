//! The typed commands. Each is a stub that returns canned data and never calls the
//! `tracker` or `card-db` crates; the Phase 1 tickets replace the bodies.

use card_db::{Card, DeckId, Language, SyncProgress, SyncStage};
use domain::Passcode;
use draw_core::{ModelManifest, ModelVariant};
use tauri::{AppHandle, Runtime};
use tauri_specta::Event;
use tracker::{CameraSetup, SessionId, SessionInfo};

use super::canned;
use super::events::{CardDbSyncProgress, ModelDownloadProgress};
use super::types::{
    CacheKind, CacheStats, DeckSummary, IpcError, IpcErrorKind, ModelStatus, ModelVariantDto,
    SessionSummary, Settings,
};

fn emit_failed(error: &tauri::Error) -> IpcError {
    IpcError::new(
        IpcErrorKind::Internal,
        format!("could not emit an event: {error}"),
    )
}

/// Sets how cameras map to players. Takes effect for the next frames.
///
/// # Errors
///
/// Never fails in the stub.
#[tauri::command]
#[specta::specta]
pub fn set_camera_setup(setup: CameraSetup) -> Result<(), IpcError> {
    let _ = setup;
    Ok(())
}

/// Starts a session (one duel) and its duel log.
///
/// # Errors
///
/// Never fails in the stub.
#[tauri::command]
#[specta::specta]
pub fn start_session() -> Result<SessionInfo, IpcError> {
    Ok(SessionInfo {
        id: SessionId(canned::SESSION_ID.into()),
        started_at_ms: canned::SESSION_STARTED_AT_MS,
    })
}

/// Ends the current session and closes its duel log.
///
/// # Errors
///
/// Never fails in the stub.
#[tauri::command]
#[specta::specta]
pub fn end_session() -> Result<SessionSummary, IpcError> {
    Ok(canned::session_summary())
}

/// Looks up a card's data in `language`. Returns `None` for an unknown passcode.
///
/// # Errors
///
/// Never fails in the stub, which knows three cards, in English only.
#[tauri::command]
#[specta::specta]
pub fn get_card(passcode: Passcode, language: Language) -> Result<Option<Card>, IpcError> {
    let _ = language;
    Ok(canned::card(passcode))
}

/// Lists the imported deck lists.
///
/// # Errors
///
/// Never fails in the stub.
#[tauri::command]
#[specta::specta]
pub fn list_decks() -> Result<Vec<DeckSummary>, IpcError> {
    Ok(vec![canned::deck_summary()])
}

/// Imports the `.ydk` file at `path` (chosen by the operator) as a deck list.
///
/// # Errors
///
/// Never fails in the stub.
#[tauri::command]
#[specta::specta]
pub fn import_deck(path: String) -> Result<DeckSummary, IpcError> {
    let _ = path;
    Ok(canned::deck_summary())
}

/// Selects the deck list recognition prefers, or none.
///
/// # Errors
///
/// Never fails in the stub.
#[tauri::command]
#[specta::specta]
pub fn select_deck(id: Option<DeckId>) -> Result<(), IpcError> {
    let _ = id;
    Ok(())
}

/// Returns the current settings.
///
/// # Errors
///
/// Never fails in the stub, which returns the defaults.
#[tauri::command]
#[specta::specta]
pub fn get_settings() -> Result<Settings, IpcError> {
    Ok(Settings::default())
}

/// Replaces the settings and returns them as stored.
///
/// # Errors
///
/// Never fails in the stub, which stores nothing and echoes `settings`.
#[tauri::command]
#[specta::specta]
pub fn update_settings(settings: Settings) -> Result<Settings, IpcError> {
    Ok(settings)
}

/// Reports which model variants are installed and which is in use.
///
/// # Errors
///
/// Never fails in the stub.
#[tauri::command]
#[specta::specta]
pub fn model_status() -> Result<ModelStatus, IpcError> {
    Ok(canned::model_status())
}

/// Downloads the files `variant` needs, emitting `model-download-progress` as it goes.
///
/// # Errors
///
/// In the stub, only if an event cannot be emitted.
#[tauri::command]
#[specta::specta]
pub fn download_models<R: Runtime>(
    app: AppHandle<R>,
    variant: ModelVariantDto,
) -> Result<(), IpcError> {
    // The stub reports each of the variant's manifest files as fully downloaded.
    for file in ModelManifest::PINNED.files(ModelVariant::from(variant)) {
        ModelDownloadProgress {
            variant,
            file: file.path.into(),
            bytes_done: file.size,
            bytes_total: file.size,
        }
        .emit(&app)
        .map_err(|e| emit_failed(&e))?;
    }
    Ok(())
}

/// Reports what the local caches hold.
///
/// # Errors
///
/// Never fails in the stub.
#[tauri::command]
#[specta::specta]
pub fn cache_stats() -> Result<CacheStats, IpcError> {
    Ok(canned::cache_stats())
}

/// Deletes one local cache.
///
/// # Errors
///
/// Never fails in the stub, which deletes nothing.
#[tauri::command]
#[specta::specta]
pub fn clear_cache(kind: CacheKind) -> Result<(), IpcError> {
    let _ = kind;
    Ok(())
}

/// Brings the card data up to date with YGOPRODeck, emitting `card-db-sync-progress` as
/// it goes.
///
/// # Errors
///
/// In the stub, only if an event cannot be emitted.
#[tauri::command]
#[specta::specta]
pub fn sync_card_db<R: Runtime>(app: AppHandle<R>) -> Result<(), IpcError> {
    let steps = [
        (SyncStage::CheckingVersion, 1),
        (SyncStage::Downloading, 5),
        (SyncStage::Storing, 3),
    ];
    for (stage, total) in steps {
        CardDbSyncProgress(SyncProgress {
            stage,
            done: total,
            total,
        })
        .emit(&app)
        .map_err(|e| emit_failed(&e))?;
    }
    Ok(())
}
