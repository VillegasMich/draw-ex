//! The typed commands. Most are stubs that return canned data and never call the
//! `tracker` or `card-db` crates; the Phase 1 tickets replace the bodies. The settings
//! and model commands (D4) are real.

use card_db::{Card, DeckId, Language, SyncProgress, SyncStage};
use domain::Passcode;
use tauri::{AppHandle, Runtime, State};
use tauri_specta::Event;
use tracker::{CameraSetup, SessionId, SessionInfo};

use super::canned;
use super::events::{CardDbSyncProgress, ModelDownloadProgress};
use super::types::{
    CacheKind, CacheStats, DeckSummary, IpcError, IpcErrorKind, ModelStatus, ModelVariantDto,
    SessionSummary, Settings,
};
use crate::models::ModelStore;
use crate::settings::SettingsStore;

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
/// Never fails: unusable stored settings were replaced by the defaults at startup.
#[tauri::command]
#[specta::specta]
pub fn get_settings(store: State<'_, SettingsStore>) -> Result<Settings, IpcError> {
    Ok(store.get())
}

/// Replaces the settings and returns them as stored.
///
/// # Errors
///
/// `invalidArgument` when a value is out of range, `io` when `settings.json` cannot be
/// written. Either way the previous settings stay in effect.
#[tauri::command]
#[specta::specta]
pub fn update_settings(
    store: State<'_, SettingsStore>,
    settings: Settings,
) -> Result<Settings, IpcError> {
    Ok(store.update(settings)?)
}

/// Reports which model variants are installed and which is in use.
///
/// # Errors
///
/// Never fails.
#[tauri::command]
#[specta::specta]
pub fn model_status(
    models: State<'_, ModelStore>,
    settings: State<'_, SettingsStore>,
) -> Result<ModelStatus, IpcError> {
    Ok(models.status(settings.get().model_variant.into()))
}

/// Downloads the files `variant` needs, emitting `model-download-progress` as it goes.
/// Files already present with the right SHA-256 are skipped, and a partial download is
/// resumed.
///
/// # Errors
///
/// `notReady` while another download runs, `network` when a request fails or a file
/// does not match its checksum after a retry, `io` when the files cannot be written.
#[tauri::command]
#[specta::specta]
pub async fn download_models<R: Runtime>(
    app: AppHandle<R>,
    models: State<'_, ModelStore>,
    variant: ModelVariantDto,
) -> Result<(), IpcError> {
    models
        .download(variant.into(), |progress| {
            let event = ModelDownloadProgress {
                variant,
                file: progress.file.into(),
                bytes_done: progress.bytes_done,
                bytes_total: progress.bytes_total,
            };
            // A missed progress event must not abort the download.
            if let Err(error) = event.emit(&app) {
                eprintln!("warning: {}", emit_failed(&error).message);
            }
        })
        .await?;
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
