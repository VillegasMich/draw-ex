//! Owned by B1 (docs/ROADMAP.md): bulk sync of `cardinfo.php` into the card database,
//! re-run only when `checkDBVer.php` reports a new `database_version`.

use serde::{Deserialize, Serialize};

use crate::{CardDb, CardDbError};

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
    /// Brings the card data up to date with YGOPRODeck, calling `on_progress` as it goes.
    ///
    /// # Errors
    ///
    /// [`CardDbError`] if a request fails or the data cannot be stored. The previous data
    /// stays usable.
    pub async fn sync(
        &self,
        on_progress: impl FnMut(SyncProgress),
    ) -> Result<SyncOutcome, CardDbError> {
        let _ = on_progress;
        todo!("B1")
    }
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
