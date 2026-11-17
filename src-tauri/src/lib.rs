//! The draw-ex app shell: builds and runs the Tauri application.
//!
//! Keep this crate thin. Logic belongs in the `tracker` and `card-db` crates, and
//! recognition in draw-core. Every module below is registered here once; Phase 1
//! tickets change only the inside of their own module.

mod ipc;
mod models;
mod pipeline;
mod settings;

/// Starts the Tauri application and blocks until it exits.
///
/// # Errors
///
/// Returns an error when the Tauri runtime cannot start, for example when the system
/// webview is missing.
pub fn run() -> tauri::Result<()> {
    let app =
        ipc::register_commands(tauri::Builder::default()).build(tauri::generate_context!())?;
    ipc::mount_events(&app);
    app.run(|app, event| {
        if let tauri::RunEvent::Exit = event {
            tauri::Manager::state::<pipeline::Pipeline>(app).shutdown();
        }
    });
    Ok(())
}
