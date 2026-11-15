//! Owned by B3 (docs/ROADMAP.md): deck lists, parsed from `.ydk` files and saved.

use std::collections::HashSet;

use domain::{ParsePasscodeError, Passcode};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Identifies one imported deck list. Serializes as a plain JSON string.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(transparent)]
pub struct DeckId(pub String);

/// A deck list: the passcodes a player is known to be using, per section.
///
/// A passcode appears once per copy, so a card played at three copies appears three
/// times.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Deck {
    /// The deck's identifier.
    pub id: DeckId,
    /// The name shown to the operator, e.g. the `.ydk` file name without extension.
    pub name: String,
    /// The main deck (`#main` section).
    pub main: Vec<Passcode>,
    /// The extra deck (`#extra` section).
    pub extra: Vec<Passcode>,
    /// The side deck (`!side` section).
    pub side: Vec<Passcode>,
}

impl Deck {
    /// Parses the text of a `.ydk` file into a deck called `name`.
    ///
    /// Handles the `#main`, `#extra` and `!side` sections, comment lines, and passcodes
    /// padded with leading zeros.
    ///
    /// # Errors
    ///
    /// [`DeckError::InvalidPasscode`] if a line in a section is not a passcode.
    pub fn parse_ydk(name: &str, text: &str) -> Result<Self, DeckError> {
        let _ = (name, text);
        todo!("B3")
    }

    /// Every distinct passcode in the deck list, across all three sections. This is the
    /// set recognition filters candidates by.
    #[must_use]
    pub fn passcodes(&self) -> HashSet<Passcode> {
        todo!("B3")
    }
}

/// Why a deck list could not be read.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum DeckError {
    /// A line inside a section is not a passcode.
    #[error("line {line} is not a passcode")]
    InvalidPasscode {
        /// The offending line, counting from 1.
        line: usize,
        /// Why it could not be parsed.
        #[source]
        source: ParsePasscodeError,
    },
}
