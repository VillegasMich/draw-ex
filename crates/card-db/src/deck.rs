//! Owned by B3 (docs/ROADMAP.md): deck lists, parsed from `.ydk` files and saved.

use std::collections::HashSet;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

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

/// The imported deck lists, one folder per deck: `<dir>/<id>/<name>.ydk`.
///
/// Each folder holds a byte-for-byte copy of the imported `.ydk` file. The folder's name
/// is the deck's id and the file's stem is its name, so there is no index to keep in
/// sync.
#[derive(Debug, Clone)]
pub struct DeckStore {
    dir: PathBuf,
}

impl DeckStore {
    /// Opens the deck store in `dir` (the app's `decks/` folder), creating it if it does
    /// not exist.
    ///
    /// # Errors
    ///
    /// [`DeckStoreError::Io`] if `dir` cannot be created.
    pub fn open(dir: impl Into<PathBuf>) -> Result<Self, DeckStoreError> {
        let dir = dir.into();
        fs::create_dir_all(&dir).map_err(|source| DeckStoreError::io(&dir, source))?;
        Ok(Self { dir })
    }

    /// Imports the `.ydk` file at `path` as a new deck list and returns it.
    ///
    /// The file is parsed first and copied into the store only if it parses. The deck's
    /// name is the file stem, even when the file has a `#created by` line, and its id is
    /// that name as a slug (see [`Deck::parse_ydk`]). If the id is taken, it gets the
    /// first free suffix from `-2` on, so importing the same file twice gives two deck
    /// lists.
    ///
    /// # Errors
    ///
    /// - [`DeckStoreError::Io`] if the file cannot be read or the copy cannot be
    ///   written.
    /// - [`DeckStoreError::InvalidDeck`] if the file is not a deck list. Nothing is
    ///   copied.
    pub fn import(&self, path: impl AsRef<Path>) -> Result<Deck, DeckStoreError> {
        let path = path.as_ref();
        let (bytes, mut deck) = read_deck(path)?;
        let (id, folder) = self.create_folder(&deck.id.0)?;
        let mut file_name = OsString::from(path.file_stem().unwrap_or_default());
        file_name.push(".ydk");
        let copy = folder.join(file_name);
        if let Err(source) = fs::write(&copy, &bytes) {
            // Best effort: an empty folder would only be skipped by `list`.
            let _ = fs::remove_dir_all(&folder);
            return Err(DeckStoreError::io(&copy, source));
        }
        deck.id = id;
        Ok(deck)
    }

    /// Summaries of every imported deck list, sorted by id. Folders that do not hold a
    /// readable deck list (e.g. edited by hand) are skipped.
    ///
    /// # Errors
    ///
    /// [`DeckStoreError::Io`] if the store's folder cannot be read.
    pub fn list(&self) -> Result<Vec<DeckSummary>, DeckStoreError> {
        let entries =
            fs::read_dir(&self.dir).map_err(|source| DeckStoreError::io(&self.dir, source))?;
        let mut summaries: Vec<DeckSummary> = entries
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let id = DeckId(entry.file_name().into_string().ok()?);
                self.load(&id).ok().map(|deck| DeckSummary::of(&deck))
            })
            .collect();
        summaries.sort_by(|a, b| a.id.0.cmp(&b.id.0));
        Ok(summaries)
    }

    /// Loads the deck list `id`.
    ///
    /// # Errors
    ///
    /// - [`DeckStoreError::NotFound`] if there is no deck list `id`.
    /// - [`DeckStoreError::Io`] if its file cannot be read.
    /// - [`DeckStoreError::InvalidDeck`] if its file no longer parses.
    pub fn load(&self, id: &DeckId) -> Result<Deck, DeckStoreError> {
        let (_, mut deck) = read_deck(&self.deck_file(id)?)?;
        deck.id = id.clone();
        Ok(deck)
    }

    /// Deletes the deck list `id`.
    ///
    /// # Errors
    ///
    /// - [`DeckStoreError::NotFound`] if there is no deck list `id`.
    /// - [`DeckStoreError::Io`] if its folder cannot be deleted.
    pub fn remove(&self, id: &DeckId) -> Result<(), DeckStoreError> {
        self.deck_file(id)?;
        let folder = self.dir.join(&id.0);
        fs::remove_dir_all(&folder).map_err(|source| DeckStoreError::io(&folder, source))
    }

    /// Creates the folder for a new deck list whose id is `slug`, or `slug` with the
    /// first free suffix, and returns the id and the folder.
    fn create_folder(&self, slug: &str) -> Result<(DeckId, PathBuf), DeckStoreError> {
        let mut id = slug.to_owned();
        let mut suffix = 1_u64;
        loop {
            let folder = self.dir.join(&id);
            // `create_dir` fails if the folder exists, so two imports never share an id.
            match fs::create_dir(&folder) {
                Ok(()) => return Ok((DeckId(id), folder)),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                    suffix += 1;
                    id = format!("{slug}-{suffix}");
                }
                Err(source) => return Err(DeckStoreError::io(&folder, source)),
            }
        }
    }

    /// The `.ydk` file of the deck list `id`. A folder edited by hand to hold several
    /// takes the first by file name, so the choice is stable.
    fn deck_file(&self, id: &DeckId) -> Result<PathBuf, DeckStoreError> {
        let not_found = || DeckStoreError::NotFound { id: id.clone() };
        // Ids come from the frontend: only a slug may name a folder, so no id reaches
        // outside the store.
        if !is_slug(&id.0) {
            return Err(not_found());
        }
        let folder = self.dir.join(&id.0);
        let entries = match fs::read_dir(&folder) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Err(not_found()),
            Err(source) => return Err(DeckStoreError::io(&folder, source)),
        };
        entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension().is_some_and(|extension| extension == "ydk") && path.is_file()
            })
            .min()
            .ok_or_else(not_found)
    }
}

/// Reads the `.ydk` file at `path` and parses it into a deck named after the file stem.
/// Returns the file's bytes too, to copy them unchanged.
fn read_deck(path: &Path) -> Result<(Vec<u8>, Deck), DeckStoreError> {
    let bytes = fs::read(path).map_err(|source| DeckStoreError::io(path, source))?;
    let name = path.file_stem().unwrap_or_default().to_string_lossy();
    // Lossy, so a `#created by` line saved in another encoding does not reject the file:
    // a replaced character anywhere else still fails the parse, with its line number.
    let text = String::from_utf8_lossy(&bytes);
    let deck = Deck::parse_ydk(&name, &text).map_err(|source| DeckStoreError::InvalidDeck {
        path: path.to_owned(),
        source,
    })?;
    Ok((bytes, deck))
}

/// An imported deck list, as listed for the operator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct DeckSummary {
    /// The deck's identifier.
    pub id: DeckId,
    /// The deck's name: the imported file's stem.
    pub name: String,
    /// Number of cards across the main, extra and side decks, counting copies.
    pub card_count: u32,
}

impl DeckSummary {
    fn of(deck: &Deck) -> Self {
        let cards = deck.main.len() + deck.extra.len() + deck.side.len();
        Self {
            id: deck.id.clone(),
            name: deck.name.clone(),
            card_count: u32::try_from(cards).unwrap_or(u32::MAX),
        }
    }
}

/// Why the deck store failed.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum DeckStoreError {
    /// There is no deck list with this id.
    #[error("no deck list {id:?}")]
    NotFound {
        /// The id asked for.
        id: DeckId,
    },
    /// The file is not a deck list.
    #[error("{path} is not a deck list")]
    InvalidDeck {
        /// The file.
        path: PathBuf,
        /// Why it could not be parsed.
        #[source]
        source: DeckError,
    },
    /// A file or folder could not be read or written.
    #[error("cannot access {path}")]
    Io {
        /// The file or folder.
        path: PathBuf,
        /// The underlying error.
        #[source]
        source: io::Error,
    },
}

impl DeckStoreError {
    fn io(path: &Path, source: io::Error) -> Self {
        Self::Io {
            path: path.to_owned(),
            source,
        }
    }
}

/// Whether `id` could have come from [`slug`].
fn is_slug(id: &str) -> bool {
    !id.is_empty()
        && !id.starts_with('-')
        && !id.ends_with('-')
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}
