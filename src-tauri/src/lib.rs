//! The draw-ex app shell: builds and runs the Tauri application.
//!
//! Keep this crate thin. Logic belongs in the `tracker` and `card-db` crates.

/// Starts the Tauri application and blocks until it exits.
///
/// # Errors
///
/// Returns an error when the Tauri runtime cannot start, for example when the system
/// webview is missing.
pub fn run() -> tauri::Result<()> {
    tauri::Builder::default().run(tauri::generate_context!())
}
