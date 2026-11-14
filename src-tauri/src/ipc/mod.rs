//! The IPC contract between the webview and Rust: every command, event and payload type.
//!
//! - Typed commands live in [`commands`] and events in [`events`]; tauri-specta exports
//!   both, with their types, to `src/lib/bindings.ts`.
//! - `submit_frame` (in [`frames`]) is raw binary (ADR-0003), so it is registered with
//!   plain Tauri and the frontend calls it through `src/lib/frames.ts`.
//! - draw-core types are mapped to IPC types only in [`convert`].
//!
//! Payloads use camelCase keys and enum variant names. Units: coordinates are `f32`
//! source-frame pixels, passcodes are integers, scores and confidences are 0 to 1, and
//! times named `*_ms` are milliseconds from the frontend's `performance.now()` (except
//! `SessionInfo::started_at_ms`, which is wall-clock). Every command rejects with an
//! [`IpcError`].

mod canned;
pub mod commands;
pub mod convert;
pub mod events;
pub mod frames;
pub mod types;

use tauri::Runtime;
use tauri::ipc::Invoke;
use tauri_specta::{collect_commands, collect_events};

use events::{CardDbSyncProgress, ModelDownloadProgress, TrackEventPayload};
use types::{DetectionDto, IpcError};

/// The tauri-specta builder holding every typed command and event.
///
/// 64-bit integers (byte counts, track IDs) are exported as TypeScript `number`. That is
/// what JSON carries anyway, and it is exact below 2^53, far above any value here.
fn specta_builder<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::<R>::new()
        .commands(collect_commands![
            commands::set_camera_setup,
            commands::start_session,
            commands::end_session,
            commands::get_card,
            commands::list_decks,
            commands::import_deck,
            commands::select_deck,
            commands::get_settings,
            commands::update_settings,
            commands::model_status,
            // The runtime parameter only matters for the exported types; the handler
            // infers the real one.
            commands::download_models::<tauri::Wry>,
            commands::cache_stats,
            commands::clear_cache,
            commands::sync_card_db::<tauri::Wry>,
        ])
        .events(collect_events![
            TrackEventPayload,
            ModelDownloadProgress,
            CardDbSyncProgress
        ])
        // Not reachable from a typed command, so registered explicitly: the
        // `submit_frame` result, the error every command rejects with, and the size in
        // card image URLs.
        .typ::<DetectionDto>()
        .typ::<IpcError>()
        .typ::<card_db::ImageSize>()
        .dangerously_cast_bigints_to_number()
        // One TypeScript type per Rust type: none of ours serializes differently from how
        // it deserializes.
        .disable_serde_phases()
}

/// Registers every IPC command on a Tauri app builder. Call [`mount_events`] on the
/// built app before it emits any event.
pub fn register_commands<R: Runtime>(app: tauri::Builder<R>) -> tauri::Builder<R> {
    let typed = specta_builder::<R>().invoke_handler();
    let raw = raw_handler::<R>();
    app.invoke_handler(move |invoke: Invoke<R>| {
        if invoke.message.command() == frames::SUBMIT_FRAME {
            raw(invoke)
        } else {
            typed(invoke)
        }
    })
}

/// Makes every typed event emittable on `app`. Done here rather than in a setup hook,
/// because Tauri keeps only one setup hook and runs it only once the event loop starts.
pub fn mount_events<R: Runtime>(app: &tauri::App<R>) {
    specta_builder::<R>().mount_events(app);
}

/// The handler for the raw commands, which tauri-specta cannot type.
fn raw_handler<R: Runtime>() -> impl Fn(Invoke<R>) -> bool + Send + Sync + 'static {
    tauri::generate_handler![frames::submit_frame]
}

#[cfg(test)]
mod tests;
