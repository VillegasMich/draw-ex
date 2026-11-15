//! Owned by B1 (docs/ROADMAP.md): bulk sync of `cardinfo.php` into the card database,
//! re-run only when `checkDBVer.php` reports a new `database_version`.

use std::path::Path;

use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use serde::{Deserialize, Serialize};

use crate::client::ApiCard;
use crate::db::connect;
use crate::{CardDb, CardDbError, Language};

/// Which step of a sync is running.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum SyncStage {
    /// Asking `checkDBVer.php` for the current `database_version`.
    CheckingVersion,
    /// Downloading `cardinfo.php`, once per language.
    Downloading,
    /// Writing the downloaded cards into the database.
    Storing,
}

/// How far a sync has got. `done` and `total` count the current stage's items: requests
/// for [`SyncStage::CheckingVersion`] and [`SyncStage::Downloading`], cards for
/// [`SyncStage::Storing`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SyncProgress {
    /// The running stage.
    pub stage: SyncStage,
    /// Items of this stage finished so far, from 0 to `total`.
    pub done: u32,
    /// Items in this stage.
    pub total: u32,
}

/// What a finished sync did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SyncOutcome {
    /// The local data already matched YGOPRODeck's `database_version`; nothing was
    /// downloaded.
    UpToDate {
        /// YGOPRODeck's `database_version`.
        version: String,
    },
    /// The card data was downloaded and stored.
    Updated {
        /// YGOPRODeck's new `database_version`.
        version: String,
        /// Number of cards stored.
        card_count: u32,
    },
}

impl CardDb {
    /// Adds `language` to the languages synced from now on, so the next
    /// [`CardDb::sync`] downloads it even when the `database_version` has not changed.
    /// English is always synced, so requesting it does nothing. The request is stored in
    /// the database and holds for every later sync.
    ///
    /// # Errors
    ///
    /// [`CardDbError`] if the database cannot be read or written.
    pub fn request_language(&self, language: Language) -> Result<(), CardDbError> {
        if language == Language::En {
            return Ok(());
        }
        let conn = self.lock();
        let mut requested = read_languages(&conn, REQUESTED_LANGUAGES)?;
        if !requested.contains(&language) {
            requested.push(language);
            write_languages(&conn, REQUESTED_LANGUAGES, &requested)?;
        }
        Ok(())
    }

    /// Brings the card data up to date with YGOPRODeck, calling `on_progress` as it goes.
    ///
    /// Always asks `checkDBVer.php` first. When the `database_version` is the one stored
    /// and every wanted language is present, that is the only request. When the version
    /// changed, English and every requested language are downloaded again and replace
    /// the stored data in one transaction. Only one sync runs at a time; a second call
    /// waits for the first.
    ///
    /// # Errors
    ///
    /// [`CardDbError`] if a request fails or the data cannot be stored. The previous data
    /// stays usable.
    pub async fn sync(
        &self,
        mut on_progress: impl FnMut(SyncProgress),
    ) -> Result<SyncOutcome, CardDbError> {
        let _one_sync_at_a_time = self.sync_lock.lock().await;

        on_progress(progress(SyncStage::CheckingVersion, 0, 1));
        let version = self.client.database_version().await?;
        on_progress(progress(SyncStage::CheckingVersion, 1, 1));

        let path = self.path().to_owned();
        let state = blocking(move || SyncState::read(&path)).await?;
        let languages = state.wanted_languages();

        let mut downloads = Vec::with_capacity(languages.len());
        on_progress(progress(SyncStage::Downloading, 0, languages.len()));
        for &language in &languages {
            downloads.push(Download {
                language,
                cards: self.client.card_info(language).await?,
            });
            on_progress(progress(SyncStage::Downloading, downloads.len(), languages.len()));
        }

        let card_count = downloads.iter().map(|download| download.cards.len()).sum();
        on_progress(progress(SyncStage::Storing, 0, card_count));
        let path = self.path().to_owned();
        let stored_version = version.clone();
        blocking(move || store(&path, &stored_version, &downloads)).await?;
        on_progress(progress(SyncStage::Storing, card_count, card_count));

        Ok(SyncOutcome::Updated {
            version,
            card_count: saturating_u32(card_count),
        })
    }
}

/// Metadata keys.
const DATABASE_VERSION: &str = "database_version";
/// The languages besides English the operator asked for, as a JSON array of codes.
const REQUESTED_LANGUAGES: &str = "requested_languages";
/// The languages stored for `database_version`, as a JSON array of codes.
const SYNCED_LANGUAGES: &str = "synced_languages";

/// What the database holds, as far as a sync needs to know.
struct SyncState {
    requested_languages: Vec<Language>,
}

impl SyncState {
    fn read(path: &Path) -> Result<Self, CardDbError> {
        let conn = connect(path)?;
        Ok(Self {
            requested_languages: read_languages(&conn, REQUESTED_LANGUAGES)?,
        })
    }

    /// English, then every requested language in the order they were requested.
    fn wanted_languages(&self) -> Vec<Language> {
        let mut wanted = vec![Language::En];
        wanted.extend(&self.requested_languages);
        wanted
    }
}

/// The cards downloaded for one language.
struct Download {
    language: Language,
    cards: Vec<ApiCard>,
}

/// Replaces the stored card data with `downloads`, in one transaction.
fn store(path: &Path, version: &str, downloads: &[Download]) -> Result<(), CardDbError> {
    let mut conn = connect(path)?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    tx.execute("DELETE FROM cards", [])?;
    tx.execute("DELETE FROM card_aliases", [])?;
    insert_cards(&tx, downloads)?;
    write_metadata(&tx, DATABASE_VERSION, version)?;
    let synced: Vec<Language> = downloads.iter().map(|download| download.language).collect();
    write_languages(&tx, SYNCED_LANGUAGES, &synced)?;
    tx.commit()?;
    Ok(())
}

fn insert_cards(tx: &Transaction<'_>, downloads: &[Download]) -> Result<(), CardDbError> {
    let mut insert = tx.prepare(
        "INSERT OR REPLACE INTO cards (passcode, language, name, kind, description,
             attribute, race, level, link_rating, atk, def, raw_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
    )?;
    // Every language lists the same artworks, so writing aliases twice is harmless.
    let mut insert_alias = tx.prepare(
        "INSERT OR REPLACE INTO card_aliases (image_id, passcode) VALUES (?1, ?2)",
    )?;
    for Download { language, cards } in downloads {
        for ApiCard {
            card,
            image_ids,
            raw_json,
        } in cards
        {
            for image_id in image_ids.iter().filter(|&&id| id != card.passcode) {
                insert_alias.execute(params![image_id.0, card.passcode.0])?;
            }
            insert.execute(params![
                card.passcode.0,
                language.code(),
                card.name,
                card.kind,
                card.description,
                card.attribute,
                card.race,
                card.level,
                card.link_rating,
                card.atk,
                card.def,
                raw_json.get(),
            ])?;
        }
    }
    Ok(())
}

fn read_metadata(conn: &Connection, key: &str) -> Result<Option<String>, CardDbError> {
    let value = conn
        .query_row(
            "SELECT value FROM metadata WHERE key = ?1",
            [key],
            |row| row.get(0),
        )
        .optional()?;
    Ok(value)
}

fn write_metadata(conn: &Connection, key: &str, value: &str) -> Result<(), CardDbError> {
    conn.execute(
        "INSERT OR REPLACE INTO metadata (key, value) VALUES (?1, ?2)",
        params![key, value],
    )?;
    Ok(())
}

fn read_languages(conn: &Connection, key: &str) -> Result<Vec<Language>, CardDbError> {
    // Languages are only ever written by `write_languages`, so a value that does not
    // parse is treated as no languages rather than as a broken database.
    Ok(read_metadata(conn, key)?
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default())
}

fn write_languages(conn: &Connection, key: &str, languages: &[Language]) -> Result<(), CardDbError> {
    let json = serde_json::to_string(languages).expect("languages serialize as strings");
    write_metadata(conn, key, &json)
}

/// Runs blocking database work on tokio's blocking pool, off the async path.
async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, CardDbError> + Send + 'static,
) -> Result<T, CardDbError> {
    match tokio::task::spawn_blocking(work).await {
        Ok(result) => result,
        Err(error) if error.is_panic() => std::panic::resume_unwind(error.into_panic()),
        Err(error) => Err(CardDbError::Cancelled(error)),
    }
}

fn progress(stage: SyncStage, done: usize, total: usize) -> SyncProgress {
    SyncProgress {
        stage,
        done: saturating_u32(done),
        total: saturating_u32(total),
    }
}

fn saturating_u32(count: usize) -> u32 {
    u32::try_from(count).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn sync_progress_round_trips_with_camel_case_stage_names() {
        let progress = SyncProgress {
            stage: SyncStage::CheckingVersion,
            done: 0,
            total: 1,
        };
        let expected = json!({ "stage": "checkingVersion", "done": 0, "total": 1 });

        assert_eq!(serde_json::to_value(progress).unwrap(), expected);
        assert_eq!(
            serde_json::from_value::<SyncProgress>(expected).unwrap(),
            progress
        );
    }
}
