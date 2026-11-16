//! Deck lists: `.ydk` parsing, against hand-written fixtures in the YGOPRODeck and
//! EDOPro export styles (see `fixtures/decks/README.md`).

use std::collections::HashSet;

use card_db::{Deck, DeckError};
use domain::{ParsePasscodeError, Passcode};

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
