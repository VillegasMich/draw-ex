//! Owned by B1 (docs/ROADMAP.md): the SQLite card database and lookups by passcode.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use domain::Passcode;
use rusqlite::{Connection, OptionalExtension, Row, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::client::{Client, DEFAULT_BASE_URL};

/// A language the card data is available in. Card images exist only in English.
///
/// Serializes as `"en"`, `"fr"`, `"de"`, `"it"` or `"pt"`, the codes YGOPRODeck uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum Language {
    /// English, the default and the fallback for untranslated cards.
    #[default]
    En,
    /// French.
    Fr,
    /// German.
    De,
    /// Italian.
    It,
    /// Portuguese.
    Pt,
}

impl Language {
    /// The YGOPRODeck language code, also stored in the database: `"en"`, `"fr"`, ...
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Fr => "fr",
            Self::De => "de",
            Self::It => "it",
            Self::Pt => "pt",
        }
    }
}

/// The card data for one passcode in one language.
///
/// Text fields are free text as YGOPRODeck returns it. `name` and `description` are in
/// the requested [`Language`]; the other text fields are YGOPRODeck's English terms.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Card {
    /// The card's identity.
    pub passcode: Passcode,
    /// The card's name, localized.
    pub name: String,
    /// The card type, e.g. `"Effect Monster"` or `"Spell Card"`.
    pub kind: String,
    /// The effect or flavor text, localized.
    pub description: String,
    /// The monster's attribute, e.g. `"FIRE"`. `None` for Spells and Traps.
    pub attribute: Option<String>,
    /// The monster's type (e.g. `"Zombie"`), or a Spell/Trap's property (e.g.
    /// `"Normal"`, `"Quick-Play"`).
    pub race: Option<String>,
    /// The monster's level or rank, in stars. `None` for Link monsters, Spells and Traps.
    pub level: Option<u8>,
    /// The Link monster's link rating, in arrows. `None` for everything else.
    pub link_rating: Option<u8>,
    /// The monster's attack points. `None` for Spells, Traps and a printed `?`.
    pub atk: Option<u32>,
    /// The monster's defense points. `None` for Link monsters, Spells, Traps and a
    /// printed `?`.
    pub def: Option<u32>,
}

/// The local card database: card data for every passcode in every [`Language`].
///
/// English is always synced; other languages are synced once requested with
/// [`CardDb::request_language`]. Lookups work offline, from whatever the last sync
/// stored.
///
/// The synchronous methods ([`CardDb::get`], [`CardDb::needs_check`],
/// [`CardDb::request_language`]) run one small indexed SQLite query each and may wait
/// briefly for each other; [`CardDb::sync`] does its database work on tokio's blocking
/// pool.
#[derive(Debug)]
pub struct CardDb {
    path: PathBuf,
    /// Serves lookups. A sync writes through its own connection, so with SQLite's WAL
    /// journal a lookup never waits for a sync to finish storing.
    conn: Mutex<Connection>,
    pub(crate) client: Client,
    /// Held for the whole of a sync, so two syncs never download the same data twice.
    pub(crate) sync_lock: tokio::sync::Mutex<()>,
}

impl CardDb {
    /// Opens the database file at `path`, creating it if it does not exist. Syncs talk to
    /// YGOPRODeck at `https://db.ygoprodeck.com/api/v7/`.
    ///
    /// # Errors
    ///
    /// [`CardDbError`] if the file cannot be opened or its schema created.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, CardDbError> {
        Self::open_with_base_url(path, DEFAULT_BASE_URL)
    }

    /// Like [`CardDb::open`], but syncs talk to the YGOPRODeck API at `base_url` instead,
    /// e.g. a local mock server in tests.
    ///
    /// # Errors
    ///
    /// - [`CardDbError::InvalidBaseUrl`] if `base_url` is not an absolute URL.
    /// - [`CardDbError`] if the file cannot be opened or its schema created.
    pub fn open_with_base_url(path: impl AsRef<Path>, base_url: &str) -> Result<Self, CardDbError> {
        let path = path.as_ref().to_owned();
        let client = Client::new(base_url)?;
        let conn = connect(&path)?;
        Ok(Self {
            path,
            conn: Mutex::new(conn),
            client,
            sync_lock: tokio::sync::Mutex::new(()),
        })
    }

    /// Looks up a card's data in `language`. Returns `None` if the passcode is unknown.
    ///
    /// Falls back to English when `language` has not been synced or the card has no
    /// translation. An alternate artwork's image ID finds the card it belongs to, whose
    /// [`Card::passcode`] is the card's own passcode.
    ///
    /// # Errors
    ///
    /// [`CardDbError`] if the database cannot be read.
    pub fn get(&self, passcode: Passcode, language: Language) -> Result<Option<Card>, CardDbError> {
        let conn = self.lock();
        let mut lookup = conn.prepare_cached(
            "SELECT passcode, name, kind, description, attribute, race, level, link_rating,
                    atk, def
             FROM cards
             WHERE passcode = COALESCE(
                       (SELECT passcode FROM card_aliases WHERE image_id = ?1), ?1)
               AND language IN (?2, 'en')
             ORDER BY language = 'en'
             LIMIT 1",
        )?;
        let card = lookup
            .query_row(params![passcode.0, language.code()], card_from_row)
            .optional()?;
        Ok(card)
    }

    /// The database file, for a sync to open its own connection to.
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn lock(&self) -> MutexGuard<'_, Connection> {
        // A panic while holding the lock cannot leave the connection half-changed:
        // SQLite rolls back an unfinished transaction.
        self.conn.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

fn card_from_row(row: &Row<'_>) -> rusqlite::Result<Card> {
    Ok(Card {
        passcode: Passcode(row.get(0)?),
        name: row.get(1)?,
        kind: row.get(2)?,
        description: row.get(3)?,
        attribute: row.get(4)?,
        race: row.get(5)?,
        level: row.get(6)?,
        link_rating: row.get(7)?,
        atk: row.get(8)?,
        def: row.get(9)?,
    })
}

/// The schema version this code reads and writes, kept in `PRAGMA user_version`.
const SCHEMA_VERSION: i32 = 1;

/// One row per card per language. Alternate artworks are not cards: `card_aliases` maps
/// their image IDs to the card's passcode. `metadata` holds the sync state as key/value
/// text.
const SCHEMA: &str = "
CREATE TABLE cards (
    passcode    INTEGER NOT NULL,
    language    TEXT    NOT NULL,
    name        TEXT    NOT NULL,
    kind        TEXT    NOT NULL,
    description TEXT    NOT NULL,
    attribute   TEXT,
    race        TEXT,
    level       INTEGER,
    link_rating INTEGER,
    atk         INTEGER,
    def         INTEGER,
    raw_json    TEXT    NOT NULL,
    PRIMARY KEY (passcode, language)
) WITHOUT ROWID;

CREATE TABLE card_aliases (
    image_id INTEGER PRIMARY KEY,
    passcode INTEGER NOT NULL
);

CREATE TABLE metadata (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
) WITHOUT ROWID;
";

/// Opens a connection to the database at `path`, creating the schema if the file is new.
pub(crate) fn connect(path: &Path) -> Result<Connection, CardDbError> {
    let mut conn = Connection::open(path)?;
    conn.busy_timeout(Duration::from_secs(5))?;
    conn.pragma_update_and_check(None, "journal_mode", "WAL", |_| Ok(()))?;

    // IMMEDIATE, so two connections opening a new file do not both create the schema.
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let version: i32 = tx.pragma_query_value(None, "user_version", |row| row.get(0))?;
    match version {
        0 => {
            tx.execute_batch(SCHEMA)?;
            tx.pragma_update(None, "user_version", SCHEMA_VERSION)?;
        }
        SCHEMA_VERSION => {}
        other => return Err(CardDbError::UnsupportedSchema { version: other }),
    }
    tx.commit()?;
    Ok(conn)
}

/// Why the card database could not be opened, read or synced.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum CardDbError {
    /// Reading or writing the database file failed.
    #[error("card database I/O failed")]
    Io(#[from] std::io::Error),
    /// A database query failed.
    #[error("card database query failed")]
    Database(#[from] rusqlite::Error),
    /// The database file was written by a newer version of draw-ex.
    #[error("card database schema version {version} is newer than this app supports")]
    UnsupportedSchema {
        /// The file's `PRAGMA user_version`.
        version: i32,
    },
    /// The YGOPRODeck base URL is not an absolute URL.
    #[error("invalid YGOPRODeck base URL {url:?}")]
    InvalidBaseUrl {
        /// The rejected URL.
        url: String,
    },
    /// A YGOPRODeck request failed: no connection, a timeout or an error status.
    #[error("YGOPRODeck request failed")]
    Request(#[from] reqwest::Error),
    /// YGOPRODeck answered with JSON this client does not understand.
    #[error("YGOPRODeck sent an unexpected response")]
    InvalidResponse(#[source] serde_json::Error),
    /// The background task storing a sync was cancelled before it finished.
    #[error("card database task was cancelled")]
    Cancelled(#[source] tokio::task::JoinError),
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn card_round_trips_with_camel_case_keys_and_null_for_missing_stats() {
        let card = Card {
            passcode: Passcode(14_558_127),
            name: "Ash Blossom & Joyous Spring".into(),
            kind: "Tuner Monster".into(),
            description: "Negate that activation.".into(),
            attribute: Some("FIRE".into()),
            race: Some("Zombie".into()),
            level: Some(3),
            link_rating: None,
            atk: Some(0),
            def: Some(1800),
        };
        let expected = json!({
            "passcode": 14_558_127,
            "name": "Ash Blossom & Joyous Spring",
            "kind": "Tuner Monster",
            "description": "Negate that activation.",
            "attribute": "FIRE",
            "race": "Zombie",
            "level": 3,
            "linkRating": null,
            "atk": 0,
            "def": 1800,
        });

        assert_eq!(serde_json::to_value(&card).unwrap(), expected);
        assert_eq!(serde_json::from_value::<Card>(expected).unwrap(), card);
    }

    #[test]
    fn languages_serialize_as_ygoprodeck_codes() {
        let codes = [
            (Language::En, "en"),
            (Language::Fr, "fr"),
            (Language::De, "de"),
            (Language::It, "it"),
            (Language::Pt, "pt"),
        ];
        for (language, code) in codes {
            assert_eq!(serde_json::to_value(language).unwrap(), json!(code));
            assert_eq!(
                serde_json::from_value::<Language>(json!(code)).unwrap(),
                language
            );
        }
    }
}
