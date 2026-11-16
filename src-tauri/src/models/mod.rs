//! Owned by D4 (docs/ROADMAP.md): model download, verification and status, driven by draw-core's manifest.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use draw_core::{ModelManifest, ModelVariant};
use reqwest::StatusCode;
use sha2::{Digest, Sha256};
use thiserror::Error;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

/// Sent with every request: the app and its version, nothing about the operator.
const USER_AGENT: &str = concat!("draw-ex/", env!("CARGO_PKG_VERSION"));
/// How long to wait for a connection to the model host.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
/// How long a download may go without receiving any data before it fails. There is no
/// overall timeout: a large file on a slow line can take many minutes.
const READ_TIMEOUT: Duration = Duration::from_secs(30);
/// The least time between two progress reports for one file.
const PROGRESS_INTERVAL: Duration = Duration::from_millis(250);

use crate::ipc::types::{IpcError, IpcErrorKind, ModelStatus, VariantStatus};

/// One model file to download: where it goes, where it comes from and how to verify it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelFile {
    /// Path inside the model repo, as the manifest lists it, e.g. `onnx/ygo_yolo.onnx`.
    /// The file is stored at the same path under the models directory.
    pub path: String,
    /// SHA-256 of the contents, as 64 lowercase hex characters.
    pub sha256: String,
    /// Size, in bytes.
    pub size: u64,
    /// Where to download it from.
    pub url: String,
}

/// The files each model variant needs. [`Catalog::pinned`] is draw-core's manifest;
/// tests build their own to point at a local server.
#[derive(Debug, Clone)]
pub struct Catalog {
    variants: HashMap<ModelVariant, Vec<ModelFile>>,
}

impl Catalog {
    /// The files of draw-core's [`ModelManifest::PINNED`], at its pinned URLs.
    #[must_use]
    pub fn pinned() -> Self {
        let manifest = ModelManifest::PINNED;
        Self::new(ModelVariant::ALL.map(|variant| {
            let files = manifest
                .files(variant)
                .iter()
                .map(|file| ModelFile {
                    path: file.path.into(),
                    sha256: file.sha256.into(),
                    size: file.size,
                    url: manifest.url(file),
                })
                .collect();
            (variant, files)
        }))
    }

    /// A catalog of the given files per variant.
    pub fn new(variants: impl IntoIterator<Item = (ModelVariant, Vec<ModelFile>)>) -> Self {
        Self {
            variants: variants.into_iter().collect(),
        }
    }

    /// The files `variant` needs, or none if the catalog does not list it.
    #[must_use]
    pub fn files(&self, variant: ModelVariant) -> &[ModelFile] {
        self.variants.get(&variant).map_or(&[], Vec::as_slice)
    }
}

/// How far the download of one file has got.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DownloadProgress<'a> {
    /// The file's path inside the model repo.
    pub file: &'a str,
    /// Bytes of the file on disk so far.
    pub bytes_done: u64,
    /// The file's size, in bytes.
    pub bytes_total: u64,
}

/// Why a model download failed. A partial download is kept for the next attempt to
/// resume, except after a checksum mismatch.
#[derive(Debug, Error)]
pub enum ModelError {
    /// Another download is already running.
    #[error("a model download is already running")]
    Busy,
    /// The request failed or the connection broke.
    #[error("{file} could not be downloaded: {source}")]
    Network {
        /// The file's path inside the model repo.
        file: String,
        /// What went wrong.
        #[source]
        source: reqwest::Error,
    },
    /// The server answered with an unexpected HTTP status.
    #[error("{file} could not be downloaded: the server answered {status}")]
    Status {
        /// The file's path inside the model repo.
        file: String,
        /// The HTTP status code.
        status: u16,
    },
    /// The file was downloaded twice and did not match its SHA-256 either time.
    #[error("{file} did not match its checksum, even after downloading it again")]
    Checksum {
        /// The file's path inside the model repo.
        file: String,
    },
    /// The HTTP client could not be set up.
    #[error("the download could not start: {0}")]
    Client(#[source] reqwest::Error),
    /// Reading or writing the model files failed.
    #[error("the model files could not be written: {0}")]
    Io(#[from] std::io::Error),
}

/// The model files in the app data directory's `models/` folder.
#[derive(Debug)]
pub struct ModelStore {
    dir: PathBuf,
    catalog: Catalog,
    /// Held while a download runs, so two never write the same files.
    downloading: Mutex<()>,
}

impl ModelStore {
    /// A store of `catalog`'s files under `dir`.
    #[must_use]
    pub fn new(dir: &Path, catalog: Catalog) -> Self {
        Self {
            dir: dir.to_owned(),
            catalog,
            downloading: Mutex::new(()),
        }
    }

    /// The store of draw-core's pinned model files, in `<app_data_dir>/models/`.
    #[must_use]
    pub fn pinned(app_data_dir: &Path) -> Self {
        Self::new(&app_data_dir.join("models"), Catalog::pinned())
    }

    /// Which variants are installed, with `current` as the one in use.
    ///
    /// A variant is installed when every file it needs is on disk with its manifest size.
    /// Hashing hundreds of megabytes on every call would be too slow, so the SHA-256 is
    /// checked by [`download`](Self::download) instead.
    #[must_use]
    pub fn status(&self, current: ModelVariant) -> ModelStatus {
        let variants = ModelVariant::ALL
            .into_iter()
            .map(|variant| {
                let files = self.catalog.files(variant);
                let mut installed = !files.is_empty();
                let mut bytes_on_disk = 0;
                for file in files {
                    let target = self.target(file);
                    match file_len(&target) {
                        Some(len) => {
                            installed &= len == file.size;
                            bytes_on_disk += len;
                        }
                        None => {
                            installed = false;
                            bytes_on_disk += file_len(&part_path(&target)).unwrap_or(0);
                        }
                    }
                }
                VariantStatus {
                    variant: variant.into(),
                    installed,
                    bytes_on_disk,
                    bytes_total: files.iter().map(|file| file.size).sum(),
                }
            })
            .collect();
        ModelStatus {
            current: current.into(),
            variants,
        }
    }

    /// Downloads the files `variant` needs, calling `on_progress` as it goes.
    ///
    /// # Errors
    ///
    /// See [`ModelError`].
    pub async fn download(
        &self,
        variant: ModelVariant,
        mut on_progress: impl FnMut(DownloadProgress<'_>),
    ) -> Result<(), ModelError> {
        let _running = self.downloading.try_lock().map_err(|_| ModelError::Busy)?;
        let client = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .connect_timeout(CONNECT_TIMEOUT)
            .read_timeout(READ_TIMEOUT)
            .build()
            .map_err(ModelError::Client)?;
        for file in self.catalog.files(variant) {
            self.fetch(&client, file, &mut on_progress).await?;
        }
        Ok(())
    }

    /// Makes sure `file` is in place with the right SHA-256, downloading it if it is not.
    /// A checksum mismatch discards the download and tries once more from zero.
    async fn fetch(
        &self,
        client: &reqwest::Client,
        file: &ModelFile,
        on_progress: &mut impl FnMut(DownloadProgress<'_>),
    ) -> Result<(), ModelError> {
        let target = self.target(file);
        let part = part_path(&target);
        let done = |on_progress: &mut dyn FnMut(DownloadProgress<'_>)| {
            on_progress(DownloadProgress {
                file: &file.path,
                bytes_done: file.size,
                bytes_total: file.size,
            });
        };
        if file_len(&target).is_some() {
            if hash_file(&target).await? == file.sha256 {
                done(on_progress);
                return Ok(());
            }
            // Corrupted or from another revision: it must not count as installed.
            tokio::fs::remove_file(&target).await?;
        }
        if let Some(dir) = target.parent() {
            tokio::fs::create_dir_all(dir).await?;
        }
        for _attempt in 0..2 {
            if download_part(client, file, &part, on_progress).await? == file.sha256 {
                tokio::fs::rename(&part, &target).await?;
                done(on_progress);
                return Ok(());
            }
            tokio::fs::remove_file(&part).await?;
        }
        Err(ModelError::Checksum {
            file: file.path.clone(),
        })
    }

    /// Where `file` is stored once downloaded.
    fn target(&self, file: &ModelFile) -> PathBuf {
        self.dir.join(&file.path)
    }
}

/// Downloads `file` into `part`, resuming from the bytes already there, and returns the
/// SHA-256 of the result in lowercase hex. Stops early, with a hash that cannot match,
/// if the server sends more than the manifest size.
async fn download_part(
    client: &reqwest::Client,
    file: &ModelFile,
    part: &Path,
    on_progress: &mut impl FnMut(DownloadProgress<'_>),
) -> Result<String, ModelError> {
    let mut done = file_len(part).unwrap_or(0);
    if done > file.size {
        tokio::fs::remove_file(part).await?;
        done = 0;
    }
    let mut hasher = if done > 0 {
        hash_prefix(part).await?
    } else {
        Sha256::new()
    };
    if done == file.size {
        return Ok(hex(hasher));
    }

    let network = |source| ModelError::Network {
        file: file.path.clone(),
        source,
    };
    let mut request = client.get(&file.url);
    if done > 0 {
        request = request.header(reqwest::header::RANGE, format!("bytes={done}-"));
    }
    let mut response = request.send().await.map_err(network)?;
    let mut out = match response.status() {
        StatusCode::PARTIAL_CONTENT if done > 0 => {
            tokio::fs::OpenOptions::new()
                .append(true)
                .open(part)
                .await?
        }
        StatusCode::OK => {
            // A server that ignores the range sends the whole file.
            done = 0;
            hasher = Sha256::new();
            tokio::fs::File::create(part).await?
        }
        status => {
            return Err(ModelError::Status {
                file: file.path.clone(),
                status: status.as_u16(),
            });
        }
    };

    let mut progress = Throttle::new(file, on_progress);
    progress.report(done);
    while let Some(chunk) = response.chunk().await.map_err(network)? {
        done += chunk.len() as u64;
        if done > file.size {
            out.flush().await?;
            return Ok(String::new());
        }
        out.write_all(&chunk).await?;
        hasher.update(&chunk);
        progress.report(done);
    }
    out.sync_all().await?;
    Ok(hex(hasher))
}

/// Reports progress at most every [`PROGRESS_INTERVAL`], so a fast download does not
/// flood the webview with events. The final report is made by the caller.
struct Throttle<'a, F> {
    file: &'a ModelFile,
    on_progress: &'a mut F,
    last: Option<Instant>,
}

impl<'a, F: FnMut(DownloadProgress<'_>)> Throttle<'a, F> {
    fn new(file: &'a ModelFile, on_progress: &'a mut F) -> Self {
        Self {
            file,
            on_progress,
            last: None,
        }
    }

    fn report(&mut self, bytes_done: u64) {
        let now = Instant::now();
        if self
            .last
            .is_some_and(|last| now.duration_since(last) < PROGRESS_INTERVAL)
        {
            return;
        }
        self.last = Some(now);
        (self.on_progress)(DownloadProgress {
            file: &self.file.path,
            bytes_done,
            bytes_total: self.file.size,
        });
    }
}

/// The SHA-256 of the file at `path`, in lowercase hex.
async fn hash_file(path: &Path) -> std::io::Result<String> {
    hash_prefix(path).await.map(hex)
}

/// A hasher fed with the whole file at `path`, so a resumed download can continue it.
/// Runs on a blocking thread: hashing hundreds of megabytes takes seconds.
async fn hash_prefix(path: &Path) -> std::io::Result<Sha256> {
    let path = path.to_owned();
    tokio::task::spawn_blocking(move || {
        let mut hasher = Sha256::new();
        std::io::copy(&mut std::fs::File::open(path)?, &mut hasher)?;
        Ok(hasher)
    })
    .await
    .map_err(std::io::Error::other)?
}

fn hex(hasher: Sha256) -> String {
    hasher
        .finalize()
        .iter()
        .fold(String::with_capacity(64), |mut out, byte| {
            let _ = write!(out, "{byte:02x}");
            out
        })
}

impl From<ModelError> for IpcError {
    fn from(error: ModelError) -> Self {
        let kind = match error {
            ModelError::Busy => IpcErrorKind::NotReady,
            ModelError::Network { .. }
            | ModelError::Status { .. }
            | ModelError::Checksum { .. }
            | ModelError::Client(_) => IpcErrorKind::Network,
            ModelError::Io(_) => IpcErrorKind::Io,
        };
        Self::new(kind, error.to_string())
    }
}

/// The length of the file at `path`, or `None` if there is no such file.
fn file_len(path: &Path) -> Option<u64> {
    std::fs::metadata(path)
        .ok()
        .filter(std::fs::Metadata::is_file)
        .map(|meta| meta.len())
}

/// Where a download of `target` is written until it is verified: `<name>.part`.
fn part_path(target: &Path) -> PathBuf {
    let mut name = target.as_os_str().to_owned();
    name.push(".part");
    PathBuf::from(name)
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use crate::ipc::types::ModelVariantDto;

    /// A two-file catalog for Int8 sharing its detector with Fp16.
    fn catalog() -> Catalog {
        let file = |path: &str, size| ModelFile {
            path: path.into(),
            sha256: "0".repeat(64),
            size,
            url: format!("http://localhost/{path}"),
        };
        Catalog::new([
            (
                ModelVariant::Int8,
                vec![file("onnx/det.onnx", 4), file("onnx/int8.onnx", 6)],
            ),
            (
                ModelVariant::Fp16,
                vec![file("onnx/det.onnx", 4), file("onnx/fp16.onnx", 8)],
            ),
        ])
    }

    fn put(dir: &Path, path: &str, len: usize) {
        let path = dir.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, vec![7; len]).unwrap();
    }

    fn variant(status: &ModelStatus, variant: ModelVariantDto) -> VariantStatus {
        *status
            .variants
            .iter()
            .find(|v| v.variant == variant)
            .expect("every variant is listed")
    }

    #[test]
    fn the_pinned_catalog_matches_draw_cores_manifest() {
        let catalog = Catalog::pinned();
        let int8 = catalog.files(ModelVariant::Int8);

        assert_eq!(
            int8.iter().map(|f| f.path.as_str()).collect::<Vec<_>>(),
            [
                "onnx/ygo_yolo.onnx",
                "onnx/vit_int8.onnx",
                "onnx/cardnames_onnx.json"
            ]
        );
        assert_eq!(int8[1].size, 97_771_550);
        assert_eq!(
            int8[1].url,
            format!(
                "https://huggingface.co/HichTala/draw2/resolve/{}/onnx/vit_int8.onnx",
                ModelManifest::PINNED.commit()
            )
        );
    }

    #[test]
    fn status_lists_every_variant_in_manifest_order_with_the_current_one() {
        let dir = TempDir::new().unwrap();

        let status = ModelStore::new(dir.path(), catalog()).status(ModelVariant::Int8);

        assert_eq!(status.current, ModelVariantDto::Int8);
        assert_eq!(
            status
                .variants
                .iter()
                .map(|v| v.variant)
                .collect::<Vec<_>>(),
            [
                ModelVariantDto::Fp32,
                ModelVariantDto::Fp16,
                ModelVariantDto::Int8,
                ModelVariantDto::YugiscanInt8
            ]
        );
    }

    #[test]
    fn a_variant_is_installed_when_every_file_has_its_manifest_size() {
        let dir = TempDir::new().unwrap();
        put(dir.path(), "onnx/det.onnx", 4);
        put(dir.path(), "onnx/int8.onnx", 6);

        let status = ModelStore::new(dir.path(), catalog()).status(ModelVariant::Int8);

        assert_eq!(
            variant(&status, ModelVariantDto::Int8),
            VariantStatus {
                variant: ModelVariantDto::Int8,
                installed: true,
                bytes_on_disk: 10,
                bytes_total: 10,
            }
        );
        // Fp16 shares the detector but lacks its classifier.
        assert_eq!(
            variant(&status, ModelVariantDto::Fp16),
            VariantStatus {
                variant: ModelVariantDto::Fp16,
                installed: false,
                bytes_on_disk: 4,
                bytes_total: 12,
            }
        );
    }

    #[test]
    fn a_missing_or_wrong_size_file_is_not_installed() {
        let dir = TempDir::new().unwrap();
        let store = ModelStore::new(dir.path(), catalog());

        assert!(!variant(&store.status(ModelVariant::Int8), ModelVariantDto::Int8).installed);

        put(dir.path(), "onnx/det.onnx", 4);
        put(dir.path(), "onnx/int8.onnx", 5);
        let int8 = variant(&store.status(ModelVariant::Int8), ModelVariantDto::Int8);

        assert!(!int8.installed);
        assert_eq!(int8.bytes_on_disk, 9);
    }

    #[test]
    fn bytes_on_disk_include_a_partial_download() {
        let dir = TempDir::new().unwrap();
        put(dir.path(), "onnx/int8.onnx.part", 3);

        let int8 = variant(
            &ModelStore::new(dir.path(), catalog()).status(ModelVariant::Int8),
            ModelVariantDto::Int8,
        );

        assert!(!int8.installed);
        assert_eq!(int8.bytes_on_disk, 3);
    }

    #[test]
    fn a_variant_missing_from_the_catalog_is_not_installed() {
        let dir = TempDir::new().unwrap();

        let fp32 = variant(
            &ModelStore::new(dir.path(), catalog()).status(ModelVariant::Int8),
            ModelVariantDto::Fp32,
        );

        assert!(!fp32.installed);
        assert_eq!(fp32.bytes_total, 0);
    }

    mod download {
        use sha2::{Digest, Sha256};
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        use super::*;

        const DETECTOR: &[u8] = b"fake detector weights";
        const CLASSIFIER: &[u8] = b"fake int8 classifier weights, a little longer";

        fn sha256(bytes: &[u8]) -> String {
            Sha256::digest(bytes)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect()
        }

        fn spec(server: &MockServer, name: &str, contents: &[u8]) -> ModelFile {
            ModelFile {
                path: format!("onnx/{name}"),
                sha256: sha256(contents),
                size: contents.len() as u64,
                url: format!("{}/{name}", server.uri()),
            }
        }

        /// Int8 needs the detector and the classifier served by `server`.
        fn store(dir: &Path, server: &MockServer) -> ModelStore {
            let catalog = Catalog::new([(
                ModelVariant::Int8,
                vec![
                    spec(server, "det.onnx", DETECTOR),
                    spec(server, "int8.onnx", CLASSIFIER),
                ],
            )]);
            ModelStore::new(dir, catalog)
        }

        async fn serve(server: &MockServer, name: &str, body: &[u8], times: u64) {
            Mock::given(method("GET"))
                .and(path(format!("/{name}")))
                .respond_with(ResponseTemplate::new(200).set_body_bytes(body))
                .expect(times)
                .mount(server)
                .await;
        }

        fn read(dir: &Path, name: &str) -> Vec<u8> {
            std::fs::read(dir.join("onnx").join(name)).unwrap()
        }

        fn parts(dir: &Path) -> Vec<PathBuf> {
            std::fs::read_dir(dir.join("onnx"))
                .map(|entries| {
                    entries
                        .map(|e| e.unwrap().path())
                        .filter(|p| p.extension().is_some_and(|ext| ext == "part"))
                        .collect()
                })
                .unwrap_or_default()
        }

        #[tokio::test]
        async fn a_fresh_download_is_verified_and_renamed_into_place() {
            let server = MockServer::start().await;
            serve(&server, "det.onnx", DETECTOR, 1).await;
            serve(&server, "int8.onnx", CLASSIFIER, 1).await;
            let dir = TempDir::new().unwrap();
            let store = store(dir.path(), &server);

            store
                .download(ModelVariant::Int8, |_| {})
                .await
                .expect("the download succeeds");

            assert_eq!(read(dir.path(), "det.onnx"), DETECTOR);
            assert_eq!(read(dir.path(), "int8.onnx"), CLASSIFIER);
            assert_eq!(parts(dir.path()), Vec::<PathBuf>::new());
            assert!(variant(&store.status(ModelVariant::Int8), ModelVariantDto::Int8).installed);
        }

        #[tokio::test]
        async fn an_interrupted_download_resumes_with_a_range_request() {
            let server = MockServer::start().await;
            serve(&server, "det.onnx", DETECTOR, 1).await;
            // Only a request for the missing bytes is answered; anything else is a 404.
            Mock::given(method("GET"))
                .and(path("/int8.onnx"))
                .and(header("range", "bytes=10-"))
                .respond_with(ResponseTemplate::new(206).set_body_bytes(&CLASSIFIER[10..]))
                .expect(1)
                .mount(&server)
                .await;
            let dir = TempDir::new().unwrap();
            std::fs::create_dir_all(dir.path().join("onnx")).unwrap();
            std::fs::write(dir.path().join("onnx/int8.onnx.part"), &CLASSIFIER[..10]).unwrap();

            store(dir.path(), &server)
                .download(ModelVariant::Int8, |_| {})
                .await
                .expect("the download resumes");

            assert_eq!(read(dir.path(), "int8.onnx"), CLASSIFIER);
            assert_eq!(parts(dir.path()), Vec::<PathBuf>::new());
        }

        #[tokio::test]
        async fn a_server_that_ignores_the_range_restarts_the_file_from_zero() {
            let server = MockServer::start().await;
            serve(&server, "det.onnx", DETECTOR, 1).await;
            serve(&server, "int8.onnx", CLASSIFIER, 1).await;
            let dir = TempDir::new().unwrap();
            put(dir.path(), "onnx/int8.onnx.part", 10);

            store(dir.path(), &server)
                .download(ModelVariant::Int8, |_| {})
                .await
                .expect("the download succeeds");

            assert_eq!(read(dir.path(), "int8.onnx"), CLASSIFIER);
        }

        #[tokio::test]
        async fn a_corrupted_file_is_detected_and_fetched_again() {
            let server = MockServer::start().await;
            serve(&server, "det.onnx", DETECTOR, 0).await;
            serve(&server, "int8.onnx", CLASSIFIER, 1).await;
            let dir = TempDir::new().unwrap();
            std::fs::create_dir_all(dir.path().join("onnx")).unwrap();
            std::fs::write(dir.path().join("onnx/det.onnx"), DETECTOR).unwrap();
            // Same size as the real file, so only the checksum can tell.
            put(dir.path(), "onnx/int8.onnx", CLASSIFIER.len());

            store(dir.path(), &server)
                .download(ModelVariant::Int8, |_| {})
                .await
                .expect("the download succeeds");

            assert_eq!(read(dir.path(), "int8.onnx"), CLASSIFIER);
        }

        #[tokio::test]
        async fn a_corrupted_file_stops_counting_as_installed_even_if_the_refetch_fails() {
            let server = MockServer::start().await;
            serve(&server, "det.onnx", DETECTOR, 1).await;
            let dir = TempDir::new().unwrap();
            put(dir.path(), "onnx/int8.onnx", CLASSIFIER.len());
            let store = store(dir.path(), &server);

            let result = store.download(ModelVariant::Int8, |_| {}).await;

            assert!(
                matches!(result, Err(ModelError::Status { status: 404, .. })),
                "{result:?}"
            );
            assert!(!variant(&store.status(ModelVariant::Int8), ModelVariantDto::Int8).installed);
        }

        #[tokio::test]
        async fn a_checksum_mismatch_is_retried_once_then_fails() {
            let server = MockServer::start().await;
            serve(&server, "det.onnx", DETECTOR, 1).await;
            let mut wrong = CLASSIFIER.to_vec();
            wrong[0] ^= 1;
            serve(&server, "int8.onnx", &wrong, 2).await;
            let dir = TempDir::new().unwrap();

            let result = store(dir.path(), &server)
                .download(ModelVariant::Int8, |_| {})
                .await;

            assert!(
                matches!(result, Err(ModelError::Checksum { ref file }) if file == "onnx/int8.onnx"),
                "{result:?}"
            );
            assert!(!dir.path().join("onnx/int8.onnx").exists());
            assert_eq!(parts(dir.path()), Vec::<PathBuf>::new());
        }

        #[tokio::test]
        async fn a_server_error_keeps_the_partial_file_for_later() {
            let server = MockServer::start().await;
            serve(&server, "det.onnx", DETECTOR, 1).await;
            Mock::given(method("GET"))
                .and(path("/int8.onnx"))
                .respond_with(ResponseTemplate::new(503))
                .mount(&server)
                .await;
            let dir = TempDir::new().unwrap();
            std::fs::create_dir_all(dir.path().join("onnx")).unwrap();
            std::fs::write(dir.path().join("onnx/int8.onnx.part"), &CLASSIFIER[..10]).unwrap();

            let result = store(dir.path(), &server)
                .download(ModelVariant::Int8, |_| {})
                .await;

            assert!(
                matches!(result, Err(ModelError::Status { status: 503, .. })),
                "{result:?}"
            );
            assert_eq!(
                std::fs::read(dir.path().join("onnx/int8.onnx.part")).unwrap(),
                &CLASSIFIER[..10]
            );
        }

        #[tokio::test]
        async fn progress_is_reported_up_to_the_full_size_of_every_file() {
            let server = MockServer::start().await;
            serve(&server, "det.onnx", DETECTOR, 0).await;
            serve(&server, "int8.onnx", CLASSIFIER, 1).await;
            let dir = TempDir::new().unwrap();
            // An already verified detector still reports as done.
            std::fs::create_dir_all(dir.path().join("onnx")).unwrap();
            std::fs::write(dir.path().join("onnx/det.onnx"), DETECTOR).unwrap();
            let mut seen = Vec::new();

            store(dir.path(), &server)
                .download(ModelVariant::Int8, |p| {
                    seen.push((p.file.to_owned(), p.bytes_done, p.bytes_total));
                })
                .await
                .expect("the download succeeds");

            let det = DETECTOR.len() as u64;
            let int8 = CLASSIFIER.len() as u64;
            assert_eq!(seen.first(), Some(&("onnx/det.onnx".into(), det, det)));
            assert_eq!(seen.last(), Some(&("onnx/int8.onnx".into(), int8, int8)));
            let classifier: Vec<u64> = seen
                .iter()
                .filter(|(file, ..)| file == "onnx/int8.onnx")
                .map(|&(_, done, _)| done)
                .collect();
            assert!(classifier.is_sorted(), "{classifier:?}");
        }

        #[tokio::test]
        async fn a_second_download_while_one_runs_is_refused() {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_bytes(DETECTOR)
                        .set_delay(std::time::Duration::from_millis(200)),
                )
                .mount(&server)
                .await;
            let dir = TempDir::new().unwrap();
            let store = store(dir.path(), &server);

            let (first, second) = tokio::join!(
                store.download(ModelVariant::Int8, |_| {}),
                store.download(ModelVariant::Int8, |_| {}),
            );

            assert!(matches!(second, Err(ModelError::Busy)), "{second:?}");
            // The first one gets the detector bytes for both files, so its classifier
            // fails the checksum; what matters is that it ran.
            assert!(!matches!(first, Err(ModelError::Busy)), "{first:?}");
        }
    }
}
