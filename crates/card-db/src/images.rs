//! Owned by B2 (docs/ROADMAP.md): the card image cache. Each image is fetched once,
//! stored at `<dir>/<size>/<passcode>.jpg`, and evicted least recently used first above
//! the size cap.

use std::fs::{self, File, FileTimes};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use domain::Passcode;
use reqwest::{StatusCode, Url};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::limiter::RateLimiter;
use crate::{CardDb, CardDbError};

/// Where YGOPRODeck serves card images, as `<size folder>/<passcode>.jpg` below it.
const DEFAULT_BASE_URL: &str = "https://images.ygoprodeck.com/images/";

/// Identifies the app and nothing else: no user or machine details.
const USER_AGENT: &str = concat!("draw-ex/", env!("CARGO_PKG_VERSION"));

const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
/// The longest silence while waiting for the next part of a response.
const READ_TIMEOUT: Duration = Duration::from_secs(30);
/// The longest one image download may take. A full-size image is a few hundred KB.
const TOTAL_TIMEOUT: Duration = Duration::from_secs(60);

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

impl ImageSize {
    const ALL: [Self; 3] = [Self::Full, Self::Small, Self::Cropped];

    /// The folder this size is stored in under the cache directory, the same name as in
    /// the `cardimg://<size>/<passcode>` URL.
    fn dir_name(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Small => "small",
            Self::Cropped => "cropped",
        }
    }

    /// The folder YGOPRODeck serves this size from.
    fn url_segment(self) -> &'static str {
        match self {
            Self::Full => "cards",
            Self::Small => "cards_small",
            Self::Cropped => "cards_cropped",
        }
    }
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

/// What [`ImageCache::prefetch`] did with each passcode it was given.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct PrefetchReport {
    /// Images downloaded now.
    pub fetched: u32,
    /// Images that were already cached, so not requested.
    pub already_cached: u32,
    /// Images YGOPRODeck does not have, now or remembered from before.
    pub not_available: u32,
    /// The passcodes whose image could not be downloaded or stored, in the order given.
    /// They are requested again next time.
    pub failed: Vec<Passcode>,
}

/// Card images on disk, fetched from YGOPRODeck on first use.
///
/// Images are stored at `<dir>/<size>/<passcode>.jpg`, where `<size>` is `full`,
/// `small` or `cropped`. A passcode YGOPRODeck has no image for is remembered with a
/// `<passcode>.missing` marker next to where the image would be, and never requested
/// again.
///
/// Every download waits on the YGOPRODeck rate limiter. In the app, create the cache
/// with [`CardDb::image_cache`], so card data and images share one limiter.
#[derive(Debug)]
pub struct ImageCache {
    dir: PathBuf,
    cap_bytes: u64,
    http: reqwest::Client,
    base_url: Url,
    limiter: Arc<RateLimiter>,
    /// Held while one image is downloaded and stored, so the same image is never
    /// downloaded twice at once and two evictions never race. The limiter spaces
    /// requests anyway, so holding it costs little.
    fetch_lock: tokio::sync::Mutex<()>,
}

impl CardDb {
    /// Creates an image cache in `dir` that keeps at most `cap_bytes` bytes of images
    /// and downloads from `https://images.ygoprodeck.com/images/`.
    ///
    /// The cache shares this database's YGOPRODeck rate limiter, so card data and image
    /// downloads together stay under the limit. `dir` is the images folder itself, e.g.
    /// `<app data>/images`.
    #[must_use]
    pub fn image_cache(&self, dir: impl Into<PathBuf>, cap_bytes: u64) -> ImageCache {
        let base_url = Url::parse(DEFAULT_BASE_URL).expect("the default image URL is valid");
        ImageCache::build(
            dir.into(),
            cap_bytes,
            base_url,
            Arc::clone(self.client.limiter()),
        )
        .expect("the HTTP client already built for card data builds again")
    }

    /// Like [`CardDb::image_cache`], but downloads images from below `base_url` instead,
    /// e.g. a local mock server in tests.
    ///
    /// # Errors
    ///
    /// - [`CardDbError::InvalidBaseUrl`] if `base_url` is not an absolute URL.
    /// - [`CardDbError::Request`] if the HTTP client cannot be built.
    pub fn image_cache_with_base_url(
        &self,
        dir: impl Into<PathBuf>,
        cap_bytes: u64,
        base_url: &str,
    ) -> Result<ImageCache, CardDbError> {
        let base_url = parse_base_url(base_url)?;
        let limiter = Arc::clone(self.client.limiter());
        Ok(ImageCache::build(dir.into(), cap_bytes, base_url, limiter)?)
    }
}

impl ImageCache {
    /// Creates a cache in `dir` that keeps at most `cap_bytes` bytes of images.
    ///
    /// The cache gets its own YGOPRODeck rate limiter. The app must use
    /// [`CardDb::image_cache`] instead, which shares the card database's limiter: two
    /// limiters together could exceed YGOPRODeck's limit.
    #[must_use]
    pub fn new(dir: impl Into<PathBuf>, cap_bytes: u64) -> Self {
        let base_url = Url::parse(DEFAULT_BASE_URL).expect("the default image URL is valid");
        let limiter = Arc::new(RateLimiter::ygoprodeck());
        // `reqwest::Client::new` panics the same way: the TLS backend is built in.
        Self::build(dir.into(), cap_bytes, base_url, limiter)
            .expect("the HTTP client builds with the built-in TLS backend")
    }

    fn build(
        dir: PathBuf,
        cap_bytes: u64,
        base_url: Url,
        limiter: Arc<RateLimiter>,
    ) -> Result<Self, reqwest::Error> {
        let http = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .connect_timeout(CONNECT_TIMEOUT)
            .read_timeout(READ_TIMEOUT)
            .timeout(TOTAL_TIMEOUT)
            .build()?;
        Ok(Self {
            dir,
            cap_bytes,
            http,
            base_url,
            limiter,
            fetch_lock: tokio::sync::Mutex::new(()),
        })
    }

    /// Where the image for `passcode` at `size` is stored, whether or not it is cached
    /// yet.
    #[must_use]
    pub fn path_for(&self, passcode: Passcode, size: ImageSize) -> PathBuf {
        self.dir
            .join(size.dir_name())
            .join(format!("{}.jpg", passcode.0))
    }

    /// Returns the path of the cached image, fetching it from YGOPRODeck first if it is
    /// not cached.
    ///
    /// A cached image is marked as just used, so eviction keeps it longest. After a
    /// download, the least recently used images are deleted until the cache fits its cap
    /// again; the image just downloaded is never one of them.
    ///
    /// # Errors
    ///
    /// - [`ImageCacheError::NotAvailable`] if YGOPRODeck has no such image (some cards
    ///   have no cropped artwork). This is remembered: later calls fail the same way
    ///   without a request.
    /// - [`ImageCacheError::Download`] if the download failed. This is not remembered.
    /// - [`ImageCacheError::Io`] if the file cannot be read or written.
    pub async fn get_or_fetch(
        &self,
        passcode: Passcode,
        size: ImageSize,
    ) -> Result<PathBuf, ImageCacheError> {
        self.fetch(passcode, size).await.map(|(path, _)| path)
    }

    /// Like [`ImageCache::get_or_fetch`], also saying whether the image was downloaded.
    async fn fetch(
        &self,
        passcode: Passcode,
        size: ImageSize,
    ) -> Result<(PathBuf, Origin), ImageCacheError> {
        let path = self.path_for(passcode, size);
        let cached = || {
            let path = path.clone();
            blocking(move || lookup(path, passcode, size))
        };
        if let Some(path) = cached().await? {
            return Ok((path, Origin::Cached));
        }

        let _fetching = self.fetch_lock.lock().await;
        // Another call may have stored it while this one waited.
        if let Some(path) = cached().await? {
            return Ok((path, Origin::Cached));
        }

        let url = self
            .base_url
            .join(&format!("{}/{}.jpg", size.url_segment(), passcode.0))
            .expect("an image path is a valid relative URL");
        self.limiter.until_ready().await;
        let response = self.http.get(url).send().await?;
        if response.status() == StatusCode::NOT_FOUND {
            blocking(move || mark_missing(&path)).await?;
            return Err(ImageCacheError::NotAvailable { passcode, size });
        }
        let body = response.error_for_status()?.bytes().await?;

        let (dir, cap_bytes) = (self.dir.clone(), self.cap_bytes);
        blocking(move || {
            store(&path, &body)?;
            evict(&dir, cap_bytes, &path)?;
            Ok((path, Origin::Downloaded))
        })
        .await
    }

    /// Fetches every image of `size` for `passcodes` that is not cached yet, one at a
    /// time through the rate limiter, e.g. for a deck list just loaded.
    ///
    /// Best effort: a failed image is counted in [`PrefetchReport::failed`] and the rest
    /// are still fetched. Each download is bounded by the client's timeouts.
    pub async fn prefetch(&self, passcodes: &[Passcode], size: ImageSize) -> PrefetchReport {
        let mut report = PrefetchReport::default();
        for &passcode in passcodes {
            match self.fetch(passcode, size).await {
                Ok((_, Origin::Downloaded)) => report.fetched += 1,
                Ok((_, Origin::Cached)) => report.already_cached += 1,
                Err(ImageCacheError::NotAvailable { .. }) => report.not_available += 1,
                Err(_) => report.failed.push(passcode),
            }
        }
        report
    }

    /// How much the cache holds now. Missing-image markers are not images and do not
    /// count.
    ///
    /// This lists the cache folders on the calling thread. If a folder cannot be read,
    /// the cache counts as empty.
    #[must_use]
    pub fn stats(&self) -> ImageCacheStats {
        let images = cached_images(&self.dir).unwrap_or_default();
        ImageCacheStats {
            bytes: images.iter().map(|image| image.bytes).sum(),
            count: u32::try_from(images.len()).unwrap_or(u32::MAX),
            cap_bytes: self.cap_bytes,
        }
    }

    /// Deletes every cached image and every missing-image marker, so each image is
    /// fetched again on its next request.
    ///
    /// # Errors
    ///
    /// [`ImageCacheError::Io`] if a file cannot be deleted.
    pub fn clear(&self) -> Result<(), ImageCacheError> {
        for size in ImageSize::ALL {
            match fs::remove_dir_all(self.dir.join(size.dir_name())) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
        Ok(())
    }
}

/// Where a cached image came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Origin {
    /// It was already on disk.
    Cached,
    /// It was just downloaded.
    Downloaded,
}

/// Runs blocking file work on tokio's blocking pool.
async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, ImageCacheError> + Send + 'static,
) -> Result<T, ImageCacheError> {
    match tokio::task::spawn_blocking(work).await {
        Ok(result) => result,
        Err(error) if error.is_panic() => std::panic::resume_unwind(error.into_panic()),
        // Only a runtime shutting down cancels blocking work.
        Err(error) => Err(io::Error::other(error).into()),
    }
}

/// Looks up the image at `path` on disk: `Some(path)` if it is cached, marking it as
/// just used; `None` if it must be downloaded.
fn lookup(
    path: PathBuf,
    passcode: Passcode,
    size: ImageSize,
) -> Result<Option<PathBuf>, ImageCacheError> {
    if marker_path(&path).exists() {
        return Err(ImageCacheError::NotAvailable { passcode, size });
    }
    match touch(&path) {
        Ok(()) => Ok(Some(path)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

/// Marks the file at `path` as used now, for least-recently-used eviction.
fn touch(path: &Path) -> io::Result<()> {
    let now = SystemTime::now();
    File::options()
        .write(true)
        .open(path)?
        .set_times(FileTimes::new().set_accessed(now).set_modified(now))
}

/// The marker remembering that YGOPRODeck has no image for `image`'s passcode and size.
fn marker_path(image: &Path) -> PathBuf {
    image.with_extension("missing")
}

/// Remembers that YGOPRODeck has no image for `image`.
fn mark_missing(image: &Path) -> Result<(), ImageCacheError> {
    let marker = marker_path(image);
    if let Some(dir) = marker.parent() {
        fs::create_dir_all(dir)?;
    }
    File::create(marker)?;
    Ok(())
}

/// Writes `body` to `path` through a `.part` file and a rename, so a crash or a failed
/// write never leaves a truncated image at `path`.
fn store(path: &Path, body: &[u8]) -> Result<(), ImageCacheError> {
    let part = path.with_extension("jpg.part");
    let write = || -> io::Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let mut file = File::create(&part)?;
        file.write_all(body)?;
        file.sync_all()?;
        fs::rename(&part, path)
    };
    write().map_err(|error| {
        // Best effort: the write error is the one worth reporting.
        let _ = fs::remove_file(&part);
        error.into()
    })
}

/// Deletes the least recently used images under `dir` until they total at most
/// `cap_bytes`, never deleting `keep`.
fn evict(dir: &Path, cap_bytes: u64, keep: &Path) -> Result<(), ImageCacheError> {
    let mut images = cached_images(dir)?;
    let mut total: u64 = images.iter().map(|image| image.bytes).sum();
    if total <= cap_bytes {
        return Ok(());
    }
    images.sort_by_key(|image| image.last_used);
    for image in images.iter().filter(|image| image.path != keep) {
        if total <= cap_bytes {
            break;
        }
        match fs::remove_file(&image.path) {
            Ok(()) => {}
            // Already gone, e.g. cleared meanwhile: it no longer counts either way.
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        total -= image.bytes;
    }
    Ok(())
}

/// One image file in the cache.
#[derive(Debug)]
struct CachedImage {
    path: PathBuf,
    bytes: u64,
    /// When the image was last read or stored.
    last_used: SystemTime,
}

/// Every image under `dir`, in every size. Markers and unfinished `.part` files are not
/// images.
fn cached_images(dir: &Path) -> io::Result<Vec<CachedImage>> {
    let mut images = Vec::new();
    for size in ImageSize::ALL {
        let entries = match fs::read_dir(dir.join(size.dir_name())) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error),
        };
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_none_or(|extension| extension != "jpg") {
                continue;
            }
            let metadata = match entry.metadata() {
                Ok(metadata) if metadata.is_file() => metadata,
                Ok(_) => continue,
                // Deleted since the directory was listed.
                Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error),
            };
            // Every image is touched on read and on store, so the modification time is
            // the last use too where the file system does not keep access times.
            let last_used = metadata.accessed().or_else(|_| metadata.modified())?;
            images.push(CachedImage {
                path,
                bytes: metadata.len(),
                last_used,
            });
        }
    }
    Ok(images)
}

/// Parses `base_url`, making sure it ends in `/` so image paths join below it rather
/// than replacing its last segment.
fn parse_base_url(base_url: &str) -> Result<Url, CardDbError> {
    let invalid = || CardDbError::InvalidBaseUrl {
        url: base_url.to_owned(),
    };
    let mut url = Url::parse(base_url).map_err(|_| invalid())?;
    if url.cannot_be_a_base() {
        return Err(invalid());
    }
    if !url.path().ends_with('/') {
        let path = format!("{}/", url.path());
        url.set_path(&path);
    }
    Ok(url)
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
    /// Downloading the image failed: no connection, a timeout or an error status other
    /// than 404 Not Found.
    #[error("image download failed")]
    Download(#[from] reqwest::Error),
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
