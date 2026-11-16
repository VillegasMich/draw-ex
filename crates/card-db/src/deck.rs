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
    /// Handles the `#main`, `#extra` and `!side` sections, comment lines (`#created by
    /// …` and any other line starting with `#`), blank lines, CRLF line endings, a UTF-8
    /// byte order mark, surrounding whitespace, and passcodes padded with leading zeros.
    /// Lines before the first section belong to the main deck. Copies are kept in file
    /// order, once per line.
    ///
    /// The deck's id is `name` as a slug: its ASCII letters and digits in lowercase, with
    /// every run of other characters turned into one `-` (`"Blue-Eyes (2024)"` →
    /// `blue-eyes-2024`), or `deck` if that leaves nothing. `name` is kept as given, even
    /// when the file has a `#created by` line.
    ///
    /// # Errors
    ///
    /// [`DeckError::InvalidPasscode`] if a line in a section is not a passcode.
    pub fn parse_ydk(name: &str, text: &str) -> Result<Self, DeckError> {
        let mut deck = Self {
            id: DeckId(slug(name)),
            name: name.to_owned(),
            main: Vec::new(),
            extra: Vec::new(),
            side: Vec::new(),
        };
        let mut section = Section::Main;
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        for (index, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.eq_ignore_ascii_case("#main") {
                section = Section::Main;
            } else if line.eq_ignore_ascii_case("#extra") {
                section = Section::Extra;
            } else if line.eq_ignore_ascii_case("!side") {
                section = Section::Side;
            } else if !line.is_empty() && !line.starts_with('#') {
                let passcode = line.parse().map_err(|source| DeckError::InvalidPasscode {
                    line: index + 1,
                    source,
                })?;
                match section {
                    Section::Main => deck.main.push(passcode),
                    Section::Extra => deck.extra.push(passcode),
                    Section::Side => deck.side.push(passcode),
                }
            }
        }
        Ok(deck)
    }

    /// Every distinct passcode in the deck list, across all three sections. This is the
    /// set recognition filters candidates by.
    #[must_use]
    pub fn passcodes(&self) -> HashSet<Passcode> {
        self.main
            .iter()
            .chain(&self.extra)
            .chain(&self.side)
            .copied()
            .collect()
    }
}

/// `name` as a deck id: see [`Deck::parse_ydk`].
fn slug(name: &str) -> String {
    let mut slug = String::with_capacity(name.len());
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
        } else if !slug.is_empty() && !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let trimmed = slug.trim_end_matches('-').len();
    slug.truncate(trimmed);
    if slug.is_empty() {
        slug.push_str("deck");
    }
    slug
}

/// The section of a `.ydk` file a passcode line belongs to.
#[derive(Debug, Clone, Copy)]
enum Section {
    Main,
    Extra,
    Side,
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
