//! Owned by D4 (docs/ROADMAP.md): loading and persisting the operator's settings.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use thiserror::Error;

use crate::ipc::types::{IpcError, IpcErrorKind, Settings};

/// The range [`Settings::inference_fps`] must fall in, in frames per second.
pub const INFERENCE_FPS_RANGE: std::ops::RangeInclusive<u8> = 1..=30;
/// The smallest allowed [`Settings::image_cache_mb`], in megabytes.
pub const MIN_IMAGE_CACHE_MB: u32 = 64;

/// Why settings could not be saved.
#[derive(Debug, Error)]
pub enum SettingsError {
    /// A value is out of range. Nothing was saved.
    #[error("{0}")]
    Invalid(String),
    /// Writing `settings.json` failed. The previous file is left as it was.
    #[error("the settings could not be saved: {0}")]
    Io(#[from] std::io::Error),
}

impl From<SettingsError> for IpcError {
    fn from(error: SettingsError) -> Self {
        let kind = match error {
            SettingsError::Invalid(_) => IpcErrorKind::InvalidArgument,
            SettingsError::Io(_) => IpcErrorKind::Io,
        };
        Self::new(kind, error.to_string())
    }
}

/// The settings file, `settings.json` in the app data directory, and the settings it
/// holds.
#[derive(Debug)]
pub struct SettingsStore {
    path: PathBuf,
    current: Mutex<Settings>,
}

impl SettingsStore {
    /// Opens the settings stored in `dir`.
    ///
    /// A missing file gives the defaults. So does a file that cannot be read, parsed or
    /// validated; it is moved aside to `settings.json.bak` for the operator, with a
    /// warning on stderr.
    pub fn open(dir: &Path) -> Self {
        let path = dir.join("settings.json");
        let current = match std::fs::read(&path) {
            Ok(bytes) => parse(&bytes).unwrap_or_else(|reason| {
                let backup = path.with_extension("json.bak");
                let kept = std::fs::rename(&path, &backup)
                    .map_or_else(|e| format!("could not be kept ({e})"), |()| "kept".into());
                eprintln!(
                    "warning: settings.json is unusable ({reason}); using the defaults. \
                     The old file was {kept} as settings.json.bak."
                );
                Settings::default()
            }),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Settings::default(),
            Err(e) => {
                eprintln!("warning: settings.json could not be read ({e}); using the defaults.");
                Settings::default()
            }
        };
        Self {
            path,
            current: Mutex::new(current),
        }
    }

    /// The current settings.
    #[must_use]
    pub fn get(&self) -> Settings {
        self.current.lock().expect("not poisoned").clone()
    }

    /// Saves `settings` and returns them as stored.
    ///
    /// # Errors
    ///
    /// [`SettingsError::Invalid`] when a value is out of range, and
    /// [`SettingsError::Io`] when the file cannot be written. Either way the stored
    /// settings are unchanged.
    pub fn update(&self, settings: Settings) -> Result<Settings, SettingsError> {
        validate(&settings).map_err(SettingsError::Invalid)?;
        let mut current = self.current.lock().expect("not poisoned");
        write_atomically(&self.path, &settings)?;
        *current = settings.clone();
        Ok(settings)
    }
}

/// Parses and validates the contents of `settings.json`.
fn parse(bytes: &[u8]) -> Result<Settings, String> {
    let settings = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    validate(&settings)?;
    Ok(settings)
}

/// Checks the values the type alone does not constrain. The selected deck is not checked
/// against the imported decks yet (I1).
fn validate(settings: &Settings) -> Result<(), String> {
    if !INFERENCE_FPS_RANGE.contains(&settings.inference_fps) {
        return Err(format!(
            "The inference rate must be {} to {} frames per second, not {}.",
            INFERENCE_FPS_RANGE.start(),
            INFERENCE_FPS_RANGE.end(),
            settings.inference_fps
        ));
    }
    if settings.image_cache_mb < MIN_IMAGE_CACHE_MB {
        return Err(format!(
            "The image cache must be at least {MIN_IMAGE_CACHE_MB} MB, not {} MB.",
            settings.image_cache_mb
        ));
    }
    Ok(())
}

/// Writes `settings` to a temporary file next to `path`, then renames it over `path`.
/// The rename replaces the file in one step, so a crash or a failed write never leaves a
/// partly written `settings.json`.
fn write_atomically(path: &Path, settings: &Settings) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let json = serde_json::to_vec_pretty(settings).map_err(std::io::Error::other)?;
    let temp = path.with_extension("json.tmp");
    let mut file = std::fs::File::create(&temp)?;
    file.write_all(&json)?;
    file.sync_all()?;
    drop(file);
    std::fs::rename(&temp, path)
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn a_missing_file_gives_the_defaults() {
        let dir = TempDir::new().unwrap();

        assert_eq!(SettingsStore::open(dir.path()).get(), Settings::default());
    }

    #[test]
    fn updated_settings_are_returned_and_survive_a_reopen() {
        let dir = TempDir::new().unwrap();
        let changed = Settings {
            language: card_db::Language::De,
            model_variant: crate::ipc::types::ModelVariantDto::Int8,
            inference_fps: 10,
            selected_deck: Some(card_db::DeckId("my-deck".into())),
            image_cache_mb: 256,
        };

        let saved = SettingsStore::open(dir.path()).update(changed.clone());

        assert_eq!(saved.expect("the settings are saved"), changed);
        assert_eq!(SettingsStore::open(dir.path()).get(), changed);
    }

    fn fps(inference_fps: u8) -> Settings {
        Settings {
            inference_fps,
            ..Settings::default()
        }
    }

    #[test]
    fn a_failed_write_keeps_the_previous_file_and_settings() {
        let dir = TempDir::new().unwrap();
        let store = SettingsStore::open(dir.path());
        store.update(fps(10)).expect("the first save succeeds");
        let before = std::fs::read(dir.path().join("settings.json")).unwrap();
        // A directory where the temporary file goes makes the write fail before the
        // rename, as a full disk would.
        std::fs::create_dir(dir.path().join("settings.json.tmp")).unwrap();

        let result = store.update(fps(20));

        assert!(matches!(result, Err(SettingsError::Io(_))), "{result:?}");
        assert_eq!(
            std::fs::read(dir.path().join("settings.json")).unwrap(),
            before
        );
        assert_eq!(store.get(), fps(10));
        assert_eq!(SettingsStore::open(dir.path()).get(), fps(10));
    }

    #[test]
    fn a_corrupt_file_gives_the_defaults_and_is_kept_as_bak() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("settings.json"), b"{ not json").unwrap();

        let store = SettingsStore::open(dir.path());

        assert_eq!(store.get(), Settings::default());
        assert_eq!(
            std::fs::read(dir.path().join("settings.json.bak")).unwrap(),
            b"{ not json"
        );
        assert!(!dir.path().join("settings.json").exists());
    }

    #[test]
    fn a_file_with_out_of_range_values_counts_as_corrupt() {
        let dir = TempDir::new().unwrap();
        let mut json = serde_json::to_value(Settings::default()).unwrap();
        json["inferenceFps"] = 99.into();
        std::fs::write(dir.path().join("settings.json"), json.to_string()).unwrap();

        assert_eq!(SettingsStore::open(dir.path()).get(), Settings::default());
        assert!(dir.path().join("settings.json.bak").exists());
    }

    #[test]
    fn update_rejects_an_inference_fps_outside_1_to_30() {
        let dir = TempDir::new().unwrap();
        let store = SettingsStore::open(dir.path());

        for bad in [0, 31, 255] {
            let result = store.update(fps(bad));
            assert!(
                matches!(result, Err(SettingsError::Invalid(ref m)) if m.contains("inference")),
                "{bad}: {result:?}"
            );
        }
        for good in [1, 30] {
            assert_eq!(store.update(fps(good)).expect("in range"), fps(good));
        }
        assert_eq!(store.get(), fps(30));
    }

    #[test]
    fn update_rejects_an_image_cache_below_64_mb_and_saves_nothing() {
        let dir = TempDir::new().unwrap();
        let store = SettingsStore::open(dir.path());
        let small = Settings {
            image_cache_mb: 63,
            ..Settings::default()
        };

        let result = store.update(small);

        assert!(
            matches!(result, Err(SettingsError::Invalid(ref m)) if m.contains("image cache")),
            "{result:?}"
        );
        assert!(!dir.path().join("settings.json").exists());
        let at_min = Settings {
            image_cache_mb: 64,
            ..Settings::default()
        };
        assert_eq!(store.update(at_min.clone()).expect("64 is allowed"), at_min);
    }
}
