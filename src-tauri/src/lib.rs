//! The draw-ex app shell: builds and runs the Tauri application.
//!
//! Keep this crate thin. Logic belongs in the `tracker` and `card-db` crates.

/// Starts the Tauri application and blocks until it exits.
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("the Tauri runtime failed to start the application");
}
