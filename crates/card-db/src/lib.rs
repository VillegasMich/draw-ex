//! YGOPRODeck client, SQLite store and image cache.
//!
//! Syncs card data into SQLite, caches card images on disk, and sends every YGOPRODeck
//! request through one shared rate limiter. Has no Tauri dependency, so it can be tested
//! and run headless.
