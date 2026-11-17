//! `ImageCache` fetching, missing images, eviction, prefetch, stats and clear, against a
//! `wiremock` server standing in for `images.ygoprodeck.com`. No test reaches the live
//! site.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use card_db::{CardDb, ImageCache, ImageCacheError, ImageCacheStats, ImageSize, PrefetchReport};
use domain::Passcode;
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const ASH_BLOSSOM: Passcode = Passcode(14_558_127);
const DARK_MAGICIAN: Passcode = Passcode(46_986_414);
const POT_OF_GREED: Passcode = Passcode(55_144_522);
const DECODE_TALKER: Passcode = Passcode(1_861_629);

/// YGOPRODeck's limiter allows one request per 100 ms.
const LIMITER_SPACING: Duration = Duration::from_millis(100);

/// A tiny stand-in JPEG of exactly `len` bytes: start and end markers around zeros.
fn jpeg(len: usize) -> Vec<u8> {
    assert!(len >= 4, "a JPEG needs its start and end markers");
    let mut bytes = vec![0; len];
    bytes[..2].copy_from_slice(&[0xFF, 0xD8]);
    bytes[len - 2..].copy_from_slice(&[0xFF, 0xD9]);
    bytes
}

/// Serves `body` as the image at `url_path` (e.g. `/cards/14558127.jpg`).
async fn serve_image(server: &MockServer, url_path: &str, body: Vec<u8>) {
    Mock::given(method("GET"))
        .and(path(url_path))
        .respond_with(ResponseTemplate::new(200).set_body_raw(body, "image/jpeg"))
        .mount(server)
        .await;
}

/// Every file under `dir`, as sorted `/`-separated paths relative to it.
fn files_in(dir: &Path) -> Vec<String> {
    fn walk(root: &Path, dir: &Path, files: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(root, &path, files);
            } else {
                let relative = path.strip_prefix(root).unwrap();
                let parts: Vec<_> = relative.iter().map(|part| part.to_string_lossy()).collect();
                files.push(parts.join("/"));
            }
        }
    }
    let mut files = Vec::new();
    walk(dir, dir, &mut files);
    files.sort();
    files
}

async fn request_count(server: &MockServer) -> usize {
    server.received_requests().await.unwrap().len()
}

/// A card database in `root`, whose API is never called by these tests.
fn open_db(root: &Path) -> CardDb {
    CardDb::open_with_base_url(root.join("cards.db"), "http://127.0.0.1:9/").unwrap()
}

/// An image cache in a fresh temp dir with a `cap_bytes` cap, downloading from
/// `server`. Keep the `TempDir` alive.
fn open_cache(server: &MockServer, cap_bytes: u64) -> (TempDir, ImageCache) {
    let root = TempDir::new().unwrap();
    let cache = open_db(root.path())
        .image_cache_with_base_url(root.path().join("images"), cap_bytes, &server.uri())
        .unwrap();
    (root, cache)
}

#[tokio::test]
async fn a_second_request_for_the_same_image_makes_no_network_call() {
    let server = MockServer::start().await;
    serve_image(&server, "/cards/14558127.jpg", jpeg(100)).await;
    let (root, cache) = open_cache(&server, 1 << 20);

    let first = cache
        .get_or_fetch(ASH_BLOSSOM, ImageSize::Full)
        .await
        .unwrap();
    let second = cache
        .get_or_fetch(ASH_BLOSSOM, ImageSize::Full)
        .await
        .unwrap();

    let expected: PathBuf = root.path().join("images/full/14558127.jpg");
    assert_eq!(first, expected);
    assert_eq!(second, expected);
    assert_eq!(std::fs::read(&expected).unwrap(), jpeg(100));
    assert_eq!(request_count(&server).await, 1);
}

#[tokio::test]
async fn a_missing_cropped_image_is_remembered_and_never_requested_again() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/cards_cropped/14558127.jpg"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    let (root, cache) = open_cache(&server, 1 << 20);

    for _ in 0..2 {
        let error = cache
            .get_or_fetch(ASH_BLOSSOM, ImageSize::Cropped)
            .await
            .unwrap_err();
        assert!(
            matches!(
                error,
                ImageCacheError::NotAvailable {
                    passcode: ASH_BLOSSOM,
                    size: ImageSize::Cropped
                }
            ),
            "{error:?}"
        );
    }

    assert_eq!(request_count(&server).await, 1);
    assert_eq!(
        files_in(&root.path().join("images")),
        ["cropped/14558127.missing"]
    );
}

#[tokio::test]
async fn a_failed_download_leaves_no_file_behind_and_is_retried() {
    let server = MockServer::start().await;
    // The connection closes after 100 of the promised 1000 bytes.
    Mock::given(method("GET"))
        .and(path("/cards/14558127.jpg"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(jpeg(100), "image/jpeg")
                .insert_header("content-length", "1000"),
        )
        .mount(&server)
        .await;
    let (root, cache) = open_cache(&server, 1 << 20);

    for _ in 0..2 {
        let error = cache
            .get_or_fetch(ASH_BLOSSOM, ImageSize::Full)
            .await
            .unwrap_err();
        assert!(matches!(error, ImageCacheError::Download(_)), "{error:?}");
        assert_eq!(files_in(&root.path().join("images")), [] as [&str; 0]);
    }
    assert_eq!(request_count(&server).await, 2);
}

#[tokio::test]
async fn the_cap_evicts_the_least_recently_used_images_first() {
    let server = MockServer::start().await;
    serve_image(&server, "/cards/14558127.jpg", jpeg(100)).await;
    serve_image(&server, "/cards/46986414.jpg", jpeg(100)).await;
    serve_image(&server, "/cards/55144522.jpg", jpeg(100)).await;
    // Room for two images, not three.
    let (root, cache) = open_cache(&server, 250);

    cache
        .get_or_fetch(ASH_BLOSSOM, ImageSize::Full)
        .await
        .unwrap();
    cache
        .get_or_fetch(DARK_MAGICIAN, ImageSize::Full)
        .await
        .unwrap();
    // Reading Ash Blossom again makes Dark Magician the least recently used.
    cache
        .get_or_fetch(ASH_BLOSSOM, ImageSize::Full)
        .await
        .unwrap();
    cache
        .get_or_fetch(POT_OF_GREED, ImageSize::Full)
        .await
        .unwrap();

    assert_eq!(
        files_in(&root.path().join("images")),
        ["full/14558127.jpg", "full/55144522.jpg"]
    );
}

#[tokio::test]
async fn an_image_larger_than_the_cap_is_still_kept_once_downloaded() {
    let server = MockServer::start().await;
    serve_image(&server, "/cards/14558127.jpg", jpeg(100)).await;
    serve_image(&server, "/cards/46986414.jpg", jpeg(100)).await;
    let (root, cache) = open_cache(&server, 50);

    cache
        .get_or_fetch(ASH_BLOSSOM, ImageSize::Full)
        .await
        .unwrap();
    let path = cache
        .get_or_fetch(DARK_MAGICIAN, ImageSize::Full)
        .await
        .unwrap();

    assert_eq!(std::fs::read(path).unwrap(), jpeg(100));
    assert_eq!(files_in(&root.path().join("images")), ["full/46986414.jpg"]);
}

#[tokio::test]
async fn stats_count_the_cached_images_but_not_the_missing_markers() {
    let server = MockServer::start().await;
    serve_image(&server, "/cards/14558127.jpg", jpeg(100)).await;
    serve_image(&server, "/cards_small/46986414.jpg", jpeg(40)).await;
    Mock::given(method("GET"))
        .and(path("/cards_cropped/14558127.jpg"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    let (_root, cache) = open_cache(&server, 1000);
    assert_eq!(
        cache.stats(),
        ImageCacheStats {
            bytes: 0,
            count: 0,
            cap_bytes: 1000
        }
    );

    cache
        .get_or_fetch(ASH_BLOSSOM, ImageSize::Full)
        .await
        .unwrap();
    cache
        .get_or_fetch(DARK_MAGICIAN, ImageSize::Small)
        .await
        .unwrap();
    cache
        .get_or_fetch(ASH_BLOSSOM, ImageSize::Cropped)
        .await
        .unwrap_err();

    assert_eq!(
        cache.stats(),
        ImageCacheStats {
            bytes: 140,
            count: 2,
            cap_bytes: 1000
        }
    );
}

#[tokio::test]
async fn clear_deletes_every_image_and_marker_so_they_are_fetched_again() {
    let server = MockServer::start().await;
    serve_image(&server, "/cards/14558127.jpg", jpeg(100)).await;
    Mock::given(method("GET"))
        .and(path("/cards_cropped/14558127.jpg"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    let (root, cache) = open_cache(&server, 1000);
    cache
        .get_or_fetch(ASH_BLOSSOM, ImageSize::Full)
        .await
        .unwrap();
    cache
        .get_or_fetch(ASH_BLOSSOM, ImageSize::Cropped)
        .await
        .unwrap_err();

    cache.clear().unwrap();

    assert_eq!(files_in(&root.path().join("images")), [] as [&str; 0]);
    assert_eq!(cache.stats().count, 0);
    cache
        .get_or_fetch(ASH_BLOSSOM, ImageSize::Full)
        .await
        .unwrap();
    cache
        .get_or_fetch(ASH_BLOSSOM, ImageSize::Cropped)
        .await
        .unwrap_err();
    assert_eq!(request_count(&server).await, 4);
}

#[test]
fn clearing_a_cache_that_was_never_used_succeeds() {
    let root = TempDir::new().unwrap();
    let cache = open_db(root.path()).image_cache(root.path().join("images"), 1000);

    cache.clear().unwrap();

    assert_eq!(cache.stats().count, 0);
}

#[tokio::test]
async fn prefetch_downloads_only_the_missing_images_through_the_limiter() {
    let server = MockServer::start().await;
    serve_image(&server, "/cards_small/14558127.jpg", jpeg(10)).await;
    serve_image(&server, "/cards_small/46986414.jpg", jpeg(10)).await;
    serve_image(&server, "/cards_small/55144522.jpg", jpeg(10)).await;
    Mock::given(method("GET"))
        .and(path("/cards_small/1861629.jpg"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    let (root, cache) = open_cache(&server, 1000);
    cache
        .get_or_fetch(ASH_BLOSSOM, ImageSize::Small)
        .await
        .unwrap();

    let start = Instant::now();
    let report = cache
        .prefetch(
            &[ASH_BLOSSOM, DARK_MAGICIAN, POT_OF_GREED, DECODE_TALKER],
            ImageSize::Small,
        )
        .await;

    assert_eq!(
        report,
        PrefetchReport {
            fetched: 2,
            already_cached: 1,
            not_available: 1,
            failed: vec![],
        }
    );
    // One request before, three during: none for the image already cached.
    assert_eq!(request_count(&server).await, 4);
    // The three requests were spaced by the limiter: at least two gaps between them.
    assert!(
        start.elapsed() >= 2 * LIMITER_SPACING,
        "prefetch took only {:?}",
        start.elapsed()
    );
    assert_eq!(
        files_in(&root.path().join("images")),
        [
            "small/14558127.jpg",
            "small/1861629.missing",
            "small/46986414.jpg",
            "small/55144522.jpg",
        ]
    );
}

#[tokio::test]
async fn prefetch_counts_a_failed_download_and_carries_on() {
    let server = MockServer::start().await;
    serve_image(&server, "/cards/14558127.jpg", jpeg(10)).await;
    Mock::given(method("GET"))
        .and(path("/cards/46986414.jpg"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;
    serve_image(&server, "/cards/55144522.jpg", jpeg(10)).await;
    let (root, cache) = open_cache(&server, 1000);

    let report = cache
        .prefetch(&[ASH_BLOSSOM, DARK_MAGICIAN, POT_OF_GREED], ImageSize::Full)
        .await;

    assert_eq!(
        report,
        PrefetchReport {
            fetched: 2,
            already_cached: 0,
            not_available: 0,
            failed: vec![DARK_MAGICIAN],
        }
    );
    assert_eq!(
        files_in(&root.path().join("images")),
        ["full/14558127.jpg", "full/55144522.jpg"]
    );
}

#[tokio::test]
async fn image_caches_share_the_card_database_limiter() {
    let server = MockServer::start().await;
    serve_image(&server, "/cards/14558127.jpg", jpeg(10)).await;
    let root = TempDir::new().unwrap();
    let db = open_db(root.path());
    let first = db
        .image_cache_with_base_url(root.path().join("first"), 1000, &server.uri())
        .unwrap();
    let second = db
        .image_cache_with_base_url(root.path().join("second"), 1000, &server.uri())
        .unwrap();

    let start = Instant::now();
    first
        .get_or_fetch(ASH_BLOSSOM, ImageSize::Full)
        .await
        .unwrap();
    second
        .get_or_fetch(ASH_BLOSSOM, ImageSize::Full)
        .await
        .unwrap();

    // A cache with its own limiter would send its first request at once.
    assert!(
        start.elapsed() >= LIMITER_SPACING,
        "two downloads took only {:?}",
        start.elapsed()
    );
    assert_eq!(request_count(&server).await, 2);
}
