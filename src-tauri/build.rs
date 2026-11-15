use std::env;
use std::path::PathBuf;

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();

    let mut attributes = tauri_build::Attributes::new();
    if target_os == "windows" && target_env == "msvc" {
        // tauri-build embeds its app manifest (the Common Controls v6 dependency) as a
        // resource that only links into the app binary. Test binaries that link tauri
        // then fail to load with STATUS_ENTRYPOINT_NOT_FOUND (0xc0000139). Embed the
        // same manifest through the linker instead, so every target gets it, and turn
        // tauri-build's copy off to avoid a duplicate manifest resource. This is the
        // workaround tauri uses for its own tests (tauri's build.rs,
        // `embed_manifest_for_tests`; https://github.com/tauri-apps/tauri/pull/4383).
        embed_manifest_for_all_targets();
        attributes = attributes
            .windows_attributes(tauri_build::WindowsAttributes::new_without_app_manifest());
    }

    tauri_build::try_build(attributes).expect("failed to run tauri-build");
}

fn embed_manifest_for_all_targets() {
    // A copy of tauri-build's default `windows-app-manifest.xml`.
    let manifest =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("windows-app-manifest.xml");
    println!("cargo:rerun-if-changed={}", manifest.display());
    println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
    println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
    // Keep the embedded manifest identical to tauri-build's: no UAC trustInfo section.
    println!("cargo:rustc-link-arg=/MANIFESTUAC:NO");
}
