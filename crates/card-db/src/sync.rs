//! Owned by B1 (docs/ROADMAP.md): bulk sync of `cardinfo.php` into the card database,
//! re-run only when `checkDBVer.php` reports a new `database_version`.

use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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

/// How long a startup check of `checkDBVer.php` counts for: see [`CardDb::needs_check`].
const CHECK_INTERVAL: Duration = Duration::from_hours(24);

impl CardDb {
    /// Whether a startup check is due at `now`: true if no sync has ever finished, or
    /// the last finished sync is 24 hours or more before `now`, or after `now` (the clock
    /// was set back).
    ///
    /// Call it on startup and run [`CardDb::sync`] only when it returns true. A manual
    /// "Sync now" calls [`CardDb::sync`] directly, which always checks. A sync that
    /// fails, for example offline, does not count, so the next startup checks again.
    ///
    /// # Errors
    ///
    /// [`CardDbError`] if the database cannot be read.
    pub fn needs_check(&self, now: SystemTime) -> Result<bool, CardDbError> {
        let Some(last_check) = read_last_check(&self.lock())? else {
            return Ok(true);
        };
        Ok(match now.duration_since(last_check) {
            Ok(elapsed) => elapsed >= CHECK_INTERVAL,
            Err(_) => true,
        })
    }

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
    /// is unchanged but a language was requested since, only that language is
    /// downloaded. When the version changed, English and every requested language are
    /// downloaded again and replace the stored data. Downloads are stored in one
    /// transaction. Only one sync runs at a time; a second call waits for the first.
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
        let checked_at = SystemTime::now();
        on_progress(progress(SyncStage::CheckingVersion, 1, 1));

        let path = self.path().to_owned();
        let state = blocking(move || SyncState::read(&path)).await?;
        let (languages, replace) = match state.plan(&version) {
            Plan::UpToDate => {
                let path = self.path().to_owned();
                blocking(move || write_last_check(&connect(&path)?, checked_at)).await?;
                return Ok(SyncOutcome::UpToDate { version });
            }
            Plan::Add(languages) => (languages, Replace::No),
            Plan::Replace(languages) => (languages, Replace::Yes),
        };

        let mut downloads = Vec::with_capacity(languages.len());
        on_progress(progress(SyncStage::Downloading, 0, languages.len()));
        for &language in &languages {
            downloads.push(Download {
                language,
                cards: self.client.card_info(language).await?,
            });
            on_progress(progress(
                SyncStage::Downloading,
                downloads.len(),
                languages.len(),
            ));
        }

        let card_count = downloads.iter().map(|download| download.cards.len()).sum();
        on_progress(progress(SyncStage::Storing, 0, card_count));
        let path = self.path().to_owned();
        let stored_version = version.clone();
        blocking(move || store(&path, &stored_version, &downloads, replace, checked_at)).await?;
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
/// When the last successful sync checked `checkDBVer.php`, in milliseconds since the
/// Unix epoch.
const LAST_CHECK: &str = "last_check_ms";

/// What the database holds, as far as a sync needs to know.
struct SyncState {
    /// The `database_version` the stored card data came from; `None` before the first
    /// sync.
    version: Option<String>,
    synced_languages: Vec<Language>,
    requested_languages: Vec<Language>,
}

impl SyncState {
    fn read(path: &Path) -> Result<Self, CardDbError> {
        let conn = connect(path)?;
        Ok(Self {
            version: read_metadata(&conn, DATABASE_VERSION)?,
            synced_languages: read_languages(&conn, SYNCED_LANGUAGES)?,
            requested_languages: read_languages(&conn, REQUESTED_LANGUAGES)?,
        })
    }

    /// What to download to bring the stored data to YGOPRODeck's `version`.
    fn plan(&self, version: &str) -> Plan {
        // English, then every requested language in the order they were requested.
        let wanted = std::iter::once(Language::En).chain(self.requested_languages.iter().copied());
        if self.version.as_deref() != Some(version) {
            return Plan::Replace(wanted.collect());
        }
        let missing: Vec<Language> = wanted
            .filter(|language| !self.synced_languages.contains(language))
            .collect();
        if missing.is_empty() {
            Plan::UpToDate
        } else {
            Plan::Add(missing)
        }
    }
}

/// What a sync downloads, once it knows YGOPRODeck's `database_version`.
#[derive(Debug, PartialEq, Eq)]
enum Plan {
    /// The stored data is that version in every wanted language: download nothing.
    UpToDate,
    /// The version is unchanged, but these requested languages are not stored yet.
    Add(Vec<Language>),
    /// The version changed (or nothing is stored): download every wanted language.
    Replace(Vec<Language>),
}

/// Whether a sync's downloads replace all stored card data or add to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Replace {
    Yes,
    No,
}

/// The cards downloaded for one language.
struct Download {
    language: Language,
    cards: Vec<ApiCard>,
}

/// Stores `downloads` as YGOPRODeck's `version`, in one transaction, either replacing
/// all stored card data or adding languages to it.
fn store(
    path: &Path,
    version: &str,
    downloads: &[Download],
    replace: Replace,
    checked_at: SystemTime,
) -> Result<(), CardDbError> {
    let mut conn = connect(path)?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let mut synced = match replace {
        Replace::Yes => {
            tx.execute("DELETE FROM cards", [])?;
            tx.execute("DELETE FROM card_aliases", [])?;
            Vec::new()
        }
        Replace::No => read_languages(&tx, SYNCED_LANGUAGES)?,
    };
    insert_cards(&tx, downloads)?;
    synced.extend(downloads.iter().map(|download| download.language));
    write_metadata(&tx, DATABASE_VERSION, version)?;
    write_languages(&tx, SYNCED_LANGUAGES, &synced)?;
    write_last_check(&tx, checked_at)?;
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
    let mut insert_alias =
        tx.prepare("INSERT OR REPLACE INTO card_aliases (image_id, passcode) VALUES (?1, ?2)")?;
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
        .query_row("SELECT value FROM metadata WHERE key = ?1", [key], |row| {
            row.get(0)
        })
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

fn read_last_check(conn: &Connection) -> Result<Option<SystemTime>, CardDbError> {
    // Written only by `write_last_check`; anything unparsable counts as never checked.
    Ok(read_metadata(conn, LAST_CHECK)?
        .and_then(|millis| millis.parse().ok())
        .map(|millis| UNIX_EPOCH + Duration::from_millis(millis)))
}

fn write_last_check(conn: &Connection, checked_at: SystemTime) -> Result<(), CardDbError> {
    // A clock before 1970 is stored as the epoch, which makes the next check due.
    let millis = checked_at
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since_epoch| since_epoch.as_millis());
    write_metadata(conn, LAST_CHECK, &millis.to_string())
}

fn read_languages(conn: &Connection, key: &str) -> Result<Vec<Language>, CardDbError> {
    // Languages are only ever written by `write_languages`, so a value that does not
    // parse is treated as no languages rather than as a broken database.
    Ok(read_metadata(conn, key)?
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default())
}

fn write_languages(
    conn: &Connection,
    key: &str,
    languages: &[Language],
) -> Result<(), CardDbError> {
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
