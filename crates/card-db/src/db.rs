//! Owned by B1 (docs/ROADMAP.md): the SQLite card database and lookups by passcode.

use std::path::Path;

use domain::Passcode;
use serde::{Deserialize, Serialize};
use thiserror::Error;

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
#[derive(Debug)]
pub struct CardDb {
    // Replaced by the real connection in B1.
    _placeholder: (),
}

impl CardDb {
    /// Opens the database file at `path`, creating it if it does not exist.
    ///
    /// # Errors
    ///
    /// [`CardDbError`] if the file cannot be opened or its schema created.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, CardDbError> {
        let _ = path.as_ref();
        todo!("B1")
    }

    /// Looks up a card's data in `language`. Returns `None` if the passcode is unknown.
    ///
    /// # Errors
    ///
    /// [`CardDbError`] if the database cannot be read.
    pub fn get(&self, passcode: Passcode, language: Language) -> Result<Option<Card>, CardDbError> {
        let _ = (passcode, language);
        todo!("B1")
    }
}

/// Why the card database could not be opened, read or synced.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum CardDbError {
    /// Reading or writing the database file failed.
    #[error("card database I/O failed")]
    Io(#[from] std::io::Error),
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
