//! Owned by B2 (docs/ROADMAP.md): the card image cache. Each image is fetched once,
//! stored at `<dir>/<size>/<passcode>.jpg`, and evicted least recently used first above
//! the size cap.

use std::path::PathBuf;

use domain::Passcode;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// One of the three card image sizes YGOPRODeck serves.
///
/// Serializes as `"full"`, `"small"` or `"cropped"`, the same names as the
/// `cardimg://<size>/<passcode>` URL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum ImageSize {
    /// The whole card, full resolution (YGOPRODeck `cards/`).
    Full,
    /// The whole card, thumbnail (YGOPRODeck `cards_small/`).
    Small,
    /// The artwork only (YGOPRODeck `cards_cropped/`). Missing for some cards.
    Cropped,
}

/// How much the image cache holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct ImageCacheStats {
    /// Total size of the cached images, in bytes.
    pub bytes: u64,
    /// Number of cached image files.
    pub count: u32,
    /// The size cap, in bytes.
    pub cap_bytes: u64,
}

/// Card images on disk, fetched from YGOPRODeck on first use.
#[derive(Debug)]
pub struct ImageCache {
    // Replaced by the real state in B2.
    _placeholder: (),
}

impl ImageCache {
    /// Creates a cache in `dir` that keeps at most `cap_bytes` bytes of images.
    #[must_use]
    pub fn new(dir: impl Into<PathBuf>, cap_bytes: u64) -> Self {
        let _ = (dir.into(), cap_bytes);
        todo!("B2")
    }

    /// Where the image for `passcode` at `size` is stored, whether or not it is cached
    /// yet.
    #[must_use]
    pub fn path_for(&self, passcode: Passcode, size: ImageSize) -> PathBuf {
        let _ = (passcode, size);
        todo!("B2")
    }

    /// Returns the path of the cached image, fetching it from YGOPRODeck first if it is
    /// not cached.
    ///
    /// # Errors
    ///
    /// - [`ImageCacheError::NotAvailable`] if YGOPRODeck has no such image (some cards
    ///   have no cropped artwork).
    /// - [`ImageCacheError::Io`] if the file cannot be read or written.
    pub async fn get_or_fetch(
        &self,
        passcode: Passcode,
        size: ImageSize,
    ) -> Result<PathBuf, ImageCacheError> {
        let _ = (passcode, size);
        todo!("B2")
    }

    /// How much the cache holds now.
    #[must_use]
    pub fn stats(&self) -> ImageCacheStats {
        todo!("B2")
    }

    /// Deletes every cached image.
    ///
    /// # Errors
    ///
    /// [`ImageCacheError::Io`] if a file cannot be deleted.
    pub fn clear(&self) -> Result<(), ImageCacheError> {
        todo!("B2")
    }
}

/// Why an image could not be served from the cache.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ImageCacheError {
    /// YGOPRODeck has no image of this size for this passcode.
    #[error("no {size:?} image exists for passcode {passcode}")]
    NotAvailable {
        /// The requested card.
        passcode: Passcode,
        /// The requested size.
        size: ImageSize,
    },
    /// Reading or writing an image file failed.
    #[error("image cache I/O failed")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn image_sizes_serialize_as_the_card_image_url_names() {
        let names = [
            (ImageSize::Full, "full"),
            (ImageSize::Small, "small"),
            (ImageSize::Cropped, "cropped"),
        ];
        for (size, name) in names {
            assert_eq!(serde_json::to_value(size).unwrap(), json!(name));
            assert_eq!(
                serde_json::from_value::<ImageSize>(json!(name)).unwrap(),
                size
            );
        }
    }
}
