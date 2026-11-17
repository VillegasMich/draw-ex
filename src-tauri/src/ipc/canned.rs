//! Canned data the stub handlers return until the Phase 1 tickets wire in the real
//! components. It uses three real cards: Monster Reborn, Ash Blossom & Joyous Spring and
//! Pot of Greed.

use card_db::{Card, DeckId};
use domain::Passcode;
use draw_core::{Candidate, Detection, ModelManifest, ModelVariant, Point, Quad, Recognition};
use tracker::SessionId;

use super::types::{CacheStats, DeckSummary, SessionSummary};

pub const MONSTER_REBORN: Passcode = Passcode(83_764_719);
pub const ASH_BLOSSOM: Passcode = Passcode(14_558_127);
pub const POT_OF_GREED: Passcode = Passcode(55_144_522);

pub const SESSION_ID: &str = "canned-session";
/// 2026-11-14T15:52:00Z, in wall-clock milliseconds since the Unix epoch.
pub const SESSION_STARTED_AT_MS: f64 = 1_763_135_520_000.0;
pub const DECK_ID: &str = "canned-deck";

/// The variant whose files the canned cache stats count as downloaded.
pub const CURRENT_VARIANT: ModelVariant = ModelVariant::Int8;

/// A draw-core candidate, as the fake recognizer returns it.
fn candidate(passcode: Passcode, score: f32) -> Candidate {
    Candidate {
        passcode: draw_core::Passcode(passcode.0),
        score,
    }
}

/// An axis-aligned quad covering the given fractions (0 to 1) of a `width` × `height`
/// frame, so the canned cards land inside any frame size.
fn quad(width: u32, height: u32, left: f32, top: f32, right: f32, bottom: f32) -> Quad {
    // Frame sizes are far below 2^24 pixels, so the f32 conversion is exact.
    let (w, h) = (width as f32, height as f32);
    let corner = |x: f32, y: f32| Point { x: x * w, y: y * h };
    Quad([
        corner(left, top),
        corner(right, top),
        corner(right, bottom),
        corner(left, bottom),
    ])
}

/// One detection per verdict, placed in a `width` × `height` pixel frame: two
/// recognized cards (Monster Reborn in the bottom half, Ash Blossom in the top half), a
/// low-confidence Pot of Greed, a card not in the deck list and an unreadable card.
pub fn detections(width: u32, height: u32) -> Vec<Detection> {
    let monster_reborn = candidate(MONSTER_REBORN, 0.93);
    let ash_blossom = candidate(ASH_BLOSSOM, 0.88);
    vec![
        Detection {
            quad: quad(width, height, 0.10, 0.55, 0.25, 0.85),
            detector_confidence: 0.91,
            candidates: vec![monster_reborn, candidate(POT_OF_GREED, 0.04)],
            recognition: Recognition::Recognized(monster_reborn),
        },
        Detection {
            quad: quad(width, height, 0.40, 0.15, 0.55, 0.45),
            detector_confidence: 0.87,
            candidates: vec![ash_blossom, candidate(MONSTER_REBORN, 0.06)],
            recognition: Recognition::Recognized(ash_blossom),
        },
        Detection {
            quad: quad(width, height, 0.70, 0.55, 0.85, 0.85),
            detector_confidence: 0.64,
            candidates: vec![
                candidate(POT_OF_GREED, 0.12),
                candidate(MONSTER_REBORN, 0.05),
            ],
            recognition: Recognition::LowConfidence,
        },
        Detection {
            quad: quad(width, height, 0.70, 0.15, 0.85, 0.45),
            detector_confidence: 0.58,
            candidates: vec![candidate(ASH_BLOSSOM, 0.41)],
            recognition: Recognition::NotInDeck,
        },
        Detection {
            quad: quad(width, height, 0.40, 0.55, 0.55, 0.85),
            detector_confidence: 0.33,
            candidates: Vec::new(),
            recognition: Recognition::Unreadable,
        },
    ]
}

/// The English card data of the three canned cards, or `None` for any other passcode.
pub fn card(passcode: Passcode) -> Option<Card> {
    let card = match passcode {
        MONSTER_REBORN => Card {
            passcode,
            name: "Monster Reborn".into(),
            kind: "Spell Card".into(),
            description: "Target 1 monster in either GY; Special Summon it.".into(),
            attribute: None,
            race: Some("Normal".into()),
            level: None,
            link_rating: None,
            atk: None,
            def: None,
        },
        ASH_BLOSSOM => Card {
            passcode,
            name: "Ash Blossom & Joyous Spring".into(),
            kind: "Tuner Monster".into(),
            description: "When a card or effect is activated that includes any of these \
                effects (Quick Effect): You can discard this card; negate that activation.\n\
                ● Add a card from the Deck to the hand.\n\
                ● Special Summon from the Deck.\n\
                ● Send a card from the Deck to the GY.\n\
                You can only use this effect of \"Ash Blossom & Joyous Spring\" once per turn."
                .into(),
            attribute: Some("FIRE".into()),
            race: Some("Zombie".into()),
            level: Some(3),
            link_rating: None,
            atk: Some(0),
            def: Some(1800),
        },
        POT_OF_GREED => Card {
            passcode,
            name: "Pot of Greed".into(),
            kind: "Spell Card".into(),
            description: "Draw 2 cards.".into(),
            attribute: None,
            race: Some("Normal".into()),
            level: None,
            link_rating: None,
            atk: None,
            def: None,
        },
        _ => return None,
    };
    Some(card)
}

pub fn deck_summary() -> DeckSummary {
    DeckSummary {
        id: DeckId(DECK_ID.into()),
        name: "Canned Deck".into(),
        card_count: 40,
    }
}

pub fn session_summary() -> SessionSummary {
    SessionSummary {
        id: SessionId(SESSION_ID.into()),
        confirmed_count: 3,
        log_path: format!("duels/{SESSION_ID}.jsonl"),
    }
}

fn variant_bytes(variant: ModelVariant) -> u64 {
    ModelManifest::PINNED
        .files(variant)
        .iter()
        .map(|file| file.size)
        .sum()
}

pub fn cache_stats() -> CacheStats {
    CacheStats {
        images_bytes: 3 * 150_000,
        image_count: 3,
        card_db_bytes: 48_000_000,
        card_db_version: Some("canned".into()),
        models_bytes: variant_bytes(CURRENT_VARIANT),
    }
}
