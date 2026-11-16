//! Deck lists: `.ydk` parsing, against hand-written fixtures in the YGOPRODeck and
//! EDOPro export styles (see `fixtures/decks/README.md`).

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use card_db::{Deck, DeckError, DeckId, DeckStore, DeckStoreError, DeckSummary};
use domain::{ParsePasscodeError, Passcode};
use tempfile::TempDir;

const YGOPRODECK: &str = include_str!("fixtures/decks/ygoprodeck.ydk");
const EDOPRO: &str = include_str!("fixtures/decks/edopro.ydk");

fn set(passcodes: &[u32]) -> HashSet<Passcode> {
    passcodes.iter().copied().map(Passcode).collect()
}

#[test]
fn ygoprodeck_export_parses_into_its_passcode_set() {
    let deck = Deck::parse_ydk("Dark Magician", YGOPRODECK).unwrap();

    assert_eq!(deck.name, "Dark Magician");
    assert_eq!(
        (deck.main.len(), deck.extra.len(), deck.side.len()),
        (20, 5, 5)
    );
    assert_eq!(
        deck.passcodes(),
        set(&[
            46_986_414, 38_033_121, 14_558_127, 23_434_538, 97_268_402, 10_045_474, 24_224_830,
            83_764_719, 12_580_477, 54_693_926, 8_267_140, 24_299_458, 86_066_372, 38_342_335,
            65_741_786, 29_301_450, 4_280_258, 94_145_021, 73_642_296, 14_532_163, 15_693_423,
        ])
    );
}

#[test]
fn edopro_export_with_crlf_parses_into_its_passcode_set() {
    assert!(
        EDOPRO.contains("\r\n"),
        "the fixture must keep its CRLF line endings"
    );

    let deck = Deck::parse_ydk("Blue-Eyes", EDOPRO).unwrap();

    assert_eq!(
        (deck.main.len(), deck.extra.len(), deck.side.len()),
        (9, 2, 3)
    );
    assert_eq!(
        deck.passcodes(),
        set(&[
            89_631_139, 14_558_127, 23_434_538, 18_144_506, 83_764_719, 8_267_140, 4_280_258,
            86_066_372, 27_204_311, 15_693_423,
        ])
    );
}

#[test]
fn copies_are_kept_in_file_order_per_section() {
    let text = "#main\n46986414\n14558127\n46986414\n#extra\n4280258\n!side\n27204311\n";

    let deck = Deck::parse_ydk("deck", text).unwrap();

    assert_eq!(
        deck.main,
        [
            Passcode(46_986_414),
            Passcode(14_558_127),
            Passcode(46_986_414)
        ]
    );
    assert_eq!(deck.extra, [Passcode(4_280_258)]);
    assert_eq!(deck.side, [Passcode(27_204_311)]);
}

#[test]
fn leading_zeros_bom_comments_blank_lines_and_spaces_are_ignored() {
    let text = "\u{feff}#created by Someone\r\n#main\r\n  04280258  \r\n\r\n# a note\r\n\
                00014558127\r\n\t\r\n#extra\r\n!side\r\n";

    let deck = Deck::parse_ydk("deck", text).unwrap();

    assert_eq!(deck.main, [Passcode(4_280_258), Passcode(14_558_127)]);
    assert_eq!(deck.extra, []);
    assert_eq!(deck.side, []);
}

#[test]
fn lines_before_any_section_belong_to_the_main_deck() {
    let deck = Deck::parse_ydk("deck", "#created by Someone\n14558127\n#extra\n4280258\n").unwrap();

    assert_eq!(deck.main, [Passcode(14_558_127)]);
    assert_eq!(deck.extra, [Passcode(4_280_258)]);
}

#[test]
fn an_invalid_line_is_an_error_naming_its_line_number() {
    let text = "#created by Someone\r\n#main\r\n14558127\r\n\r\nPot of Greed\r\n55144522\r\n";

    let error = Deck::parse_ydk("deck", text).unwrap_err();

    assert_eq!(
        error,
        DeckError::InvalidPasscode {
            line: 5,
            source: ParsePasscodeError::InvalidCharacter { found: 'P' },
        }
    );
    assert_eq!(error.to_string(), "line 5 is not a passcode");
}

#[test]
fn an_empty_file_is_an_empty_deck() {
    let deck = Deck::parse_ydk("deck", "").unwrap();

    assert_eq!(deck.passcodes(), HashSet::new());
}

/// A deck store in `<temp>/decks`, and a folder outside it to import `.ydk` files from.
/// Keep the `TempDir` alive.
fn store() -> (TempDir, DeckStore) {
    let temp = TempDir::new().unwrap();
    fs::create_dir(temp.path().join("downloads")).unwrap();
    let store = DeckStore::open(temp.path().join("decks")).unwrap();
    (temp, store)
}

/// Writes `text` to `<temp>/downloads/<file_name>`, ready to import.
fn download(temp: &TempDir, file_name: &str, text: &str) -> PathBuf {
    let path = temp.path().join("downloads").join(file_name);
    fs::write(&path, text).unwrap();
    path
}

/// Every file under `dir`, relative to it, sorted.
fn files_under(dir: &Path) -> Vec<String> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().into_string().unwrap();
        if entry.file_type().unwrap().is_dir() {
            files.extend(
                files_under(&entry.path())
                    .into_iter()
                    .map(|file| format!("{name}/{file}")),
            );
        } else {
            files.push(name);
        }
    }
    files.sort();
    files
}

#[test]
fn import_copies_the_file_into_decks_and_list_and_load_return_it() {
    let (temp, store) = store();
    let source = download(&temp, "Dark Magician.ydk", YGOPRODECK);

    let imported = store.import(&source).unwrap();

    let expected = Deck {
        id: DeckId("dark-magician".to_owned()),
        ..Deck::parse_ydk("Dark Magician", YGOPRODECK).unwrap()
    };
    assert_eq!(imported, expected);
    assert_eq!(
        files_under(&temp.path().join("decks")),
        ["dark-magician/Dark Magician.ydk"]
    );
    assert_eq!(
        fs::read_to_string(temp.path().join("decks/dark-magician/Dark Magician.ydk")).unwrap(),
        YGOPRODECK
    );
    assert_eq!(
        store.list().unwrap(),
        [DeckSummary {
            id: DeckId("dark-magician".to_owned()),
            name: "Dark Magician".to_owned(),
            card_count: 30,
        }]
    );
    assert_eq!(store.load(&expected.id).unwrap(), expected);
}

#[test]
fn a_file_that_does_not_parse_is_rejected_and_not_copied() {
    let (temp, store) = store();
    let source = download(&temp, "notes.ydk", "#main\n14558127\nAsh Blossom x3\n");

    let error = store.import(&source).unwrap_err();

    assert!(
        matches!(
            error,
            DeckStoreError::InvalidDeck {
                source: DeckError::InvalidPasscode { line: 3, .. },
                ..
            }
        ),
        "{error:?}"
    );
    assert_eq!(
        files_under(&temp.path().join("decks")),
        Vec::<String>::new()
    );
    assert_eq!(store.list().unwrap(), []);
}

#[test]
fn an_id_collision_gets_a_numeric_suffix() {
    let (temp, store) = store();
    let first = store
        .import(download(&temp, "Blue-Eyes.ydk", EDOPRO))
        .unwrap();
    fs::create_dir(temp.path().join("downloads/other")).unwrap();
    let second = store
        .import(download(&temp, "other/blue eyes.ydk", YGOPRODECK))
        .unwrap();

    assert_eq!(first.id, DeckId("blue-eyes".to_owned()));
    assert_eq!(second.id, DeckId("blue-eyes-2".to_owned()));
    assert_eq!(
        files_under(&temp.path().join("decks")),
        ["blue-eyes-2/blue eyes.ydk", "blue-eyes/Blue-Eyes.ydk"]
    );
    assert_eq!(
        store.list().unwrap(),
        [
            DeckSummary {
                id: DeckId("blue-eyes".to_owned()),
                name: "Blue-Eyes".to_owned(),
                card_count: 14,
            },
            DeckSummary {
                id: DeckId("blue-eyes-2".to_owned()),
                name: "blue eyes".to_owned(),
                card_count: 30,
            },
        ]
    );
    assert_eq!(store.load(&second.id).unwrap(), second);
}

#[test]
fn reimporting_the_same_file_adds_a_second_deck_list() {
    let (temp, store) = store();
    let source = download(&temp, "Dark Magician.ydk", YGOPRODECK);

    let first = store.import(&source).unwrap();
    let again = store.import(&source).unwrap();

    assert_eq!(first.id, DeckId("dark-magician".to_owned()));
    assert_eq!(again.id, DeckId("dark-magician-2".to_owned()));
    assert_eq!(again.name, "Dark Magician");
    assert_eq!(store.list().unwrap().len(), 2);
}

#[test]
fn a_name_with_no_ascii_letters_or_digits_gets_the_id_deck() {
    let (temp, store) = store();

    let deck = store.import(download(&temp, "青眼.ydk", EDOPRO)).unwrap();

    assert_eq!(deck.id, DeckId("deck".to_owned()));
    assert_eq!(deck.name, "青眼");
}

#[test]
fn remove_deletes_the_deck_list() {
    let (temp, store) = store();
    let kept = store.import(download(&temp, "Kept.ydk", EDOPRO)).unwrap();
    let removed = store
        .import(download(&temp, "Removed.ydk", EDOPRO))
        .unwrap();

    store.remove(&removed.id).unwrap();

    assert_eq!(files_under(&temp.path().join("decks")), ["kept/Kept.ydk"]);
    assert_eq!(
        store
            .list()
            .unwrap()
            .into_iter()
            .map(|deck| deck.id)
            .collect::<Vec<_>>(),
        [kept.id]
    );
    assert!(matches!(
        store.load(&removed.id),
        Err(DeckStoreError::NotFound { .. })
    ));
}

#[test]
fn an_id_that_is_not_in_the_store_or_reaches_outside_it_is_not_found() {
    let (temp, store) = store();
    download(&temp, "outside.ydk", YGOPRODECK);

    for id in ["missing", "../downloads", "", "Dark Magician"] {
        let id = DeckId(id.to_owned());
        assert!(
            matches!(store.load(&id), Err(DeckStoreError::NotFound { .. })),
            "load {id:?}"
        );
        assert!(
            matches!(store.remove(&id), Err(DeckStoreError::NotFound { .. })),
            "remove {id:?}"
        );
    }
}

#[test]
fn the_id_is_the_name_as_a_lowercase_ascii_slug() {
    for (name, id) in [
        ("Blue-Eyes (2024)", "blue-eyes-2024"),
        ("  --Dark  Magician!! ", "dark-magician"),
        ("Snake-Eye_Fire.King", "snake-eye-fire-king"),
        ("青眼", "deck"),
    ] {
        let deck = Deck::parse_ydk(name, "").unwrap();
        assert_eq!(deck.id, DeckId(id.to_owned()), "{name:?}");
        assert_eq!(deck.name, name);
    }
}

#[test]
fn a_comment_line_in_another_encoding_does_not_reject_the_file() {
    let (temp, store) = store();
    // "José" in Latin-1, as an old editor might save it: not valid UTF-8.
    let bytes = b"#created by Jos\xe9\r\n#main\r\n14558127\r\n".as_slice();
    let source = temp.path().join("downloads/Ash.ydk");
    fs::write(&source, bytes).unwrap();

    let deck = store.import(&source).unwrap();

    assert_eq!(deck.main, [Passcode(14_558_127)]);
    assert_eq!(
        fs::read(temp.path().join("decks/ash/Ash.ydk")).unwrap(),
        bytes
    );
    assert_eq!(store.load(&deck.id).unwrap(), deck);
}

#[test]
fn list_skips_folders_that_do_not_hold_a_deck_list() {
    let (temp, store) = store();
    let deck = store.import(download(&temp, "Kept.ydk", EDOPRO)).unwrap();
    let decks = temp.path().join("decks");
    fs::create_dir(decks.join("broken")).unwrap();
    fs::write(decks.join("broken/Broken.ydk"), "#main\nnot a passcode\n").unwrap();
    fs::create_dir(decks.join("empty")).unwrap();
    fs::write(decks.join("stray.ydk"), EDOPRO).unwrap();

    let ids: Vec<DeckId> = store
        .list()
        .unwrap()
        .into_iter()
        .map(|deck| deck.id)
        .collect();

    assert_eq!(ids, [deck.id]);
}
