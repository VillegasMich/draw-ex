//! YGOPRODeck client, SQLite store, image cache and deck lists. No Tauri dependency.
//!
//! - [`CardDb`] holds the card data, bulk-synced from YGOPRODeck into SQLite.
//! - [`ImageCache`] fetches each card image once and serves it from disk afterwards.
//! - [`Deck`] is a deck list imported from a `.ydk` file.
//!
//! Every YGOPRODeck request goes through one shared rate limiter: the API blocks an IP
//! above 20 requests per second (see `CLAUDE.md`).
//!
//! # Units
//!
//! - Passcodes are integers ([`domain::Passcode`]); leading zeros are not significant.
//! - Sizes named `*_bytes` are bytes.
//!
//! Every type serializes with camelCase keys, and enums with camelCase variant names.
//! With the `specta` cargo feature, every type also derives `specta::Type`.

#![forbid(unsafe_code)]

mod client;
mod db;
mod deck;
mod images;
mod limiter;
mod sync;

pub use db::{Card, CardDb, CardDbError, Language};
pub use deck::{Deck, DeckError, DeckId};
pub use images::{ImageCache, ImageCacheError, ImageCacheStats, ImageSize};
pub use sync::{SyncOutcome, SyncProgress, SyncStage};
