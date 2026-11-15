//! `CardDb` sync and lookups, against a `wiremock` server serving recorded YGOPRODeck
//! fixtures (see `fixtures/README.md`). No test reaches the live API.

use card_db::{Card, CardDb, Language, SyncOutcome};
use domain::Passcode;
use tempfile::TempDir;
use wiremock::matchers::{method, path, query_param, query_param_is_missing};
use wiremock::{Mock, MockServer, ResponseTemplate};

const CHECK_DB_VER: &str = include_str!("fixtures/checkDBVer.json");
const CARDINFO_EN: &str = include_str!("fixtures/cardinfo_en.json");
const CARDINFO_FR: &str = include_str!("fixtures/cardinfo_fr.json");

/// The `database_version` in `fixtures/checkDBVer.json`.
const VERSION: &str = "147.88";

const ASH_BLOSSOM: Passcode = Passcode(14_558_127);

fn json(body: &str) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_raw(body.to_owned(), "application/json")
}

fn check_db_ver(version: &str) -> String {
    CHECK_DB_VER.replace(VERSION, version)
}

/// A mock YGOPRODeck serving the fixtures, with `version` as the `database_version`.
async fn ygoprodeck(version: &str) -> MockServer {
    let server = MockServer::start().await;
    serve_fixtures(&server, version).await;
    server
}

/// Mounts the fixtures on `server`, with `version` as the `database_version`.
async fn serve_fixtures(server: &MockServer, version: &str) {
    Mock::given(method("GET"))
        .and(path("/checkDBVer.php"))
        .respond_with(json(&check_db_ver(version)))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/cardinfo.php"))
        .and(query_param_is_missing("language"))
        .respond_with(json(CARDINFO_EN))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/cardinfo.php"))
        .and(query_param("language", "fr"))
        .respond_with(json(CARDINFO_FR))
        .mount(server)
        .await;
}

/// A card database in a fresh temp dir, talking to `server`. Keep the `TempDir` alive.
fn open_db(server: &MockServer) -> (TempDir, CardDb) {
    let dir = TempDir::new().unwrap();
    let db = open_in(&dir, server);
    (dir, db)
}

fn open_in(dir: &TempDir, server: &MockServer) -> CardDb {
    CardDb::open_with_base_url(dir.path().join("cards.db"), &server.uri()).unwrap()
}

async fn requests_to(server: &MockServer) -> Vec<String> {
    server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .map(|request| {
            let url = &request.url;
            match url.query() {
                Some(query) => format!("{}?{query}", url.path()),
                None => url.path().to_owned(),
            }
        })
        .collect()
}

fn ash_blossom_en() -> Card {
    Card {
        passcode: ASH_BLOSSOM,
        name: "Ash Blossom & Joyous Spring".into(),
        kind: "Tuner Monster".into(),
        description: "When a card or effect is activated that includes any of these effects \
                      (Quick Effect): You can discard this card; negate that effect.\r\n\
                      ● Add a card from the Deck to the hand.\r\n\
                      ● Special Summon from the Deck.\r\n\
                      ● Send a card from the Deck to the GY.\r\n\
                      You can only use this effect of \"Ash Blossom & Joyous Spring\" once \
                      per turn."
            .into(),
        attribute: Some("FIRE".into()),
        race: Some("Zombie".into()),
        level: Some(3),
        link_rating: None,
        atk: Some(0),
        def: Some(1800),
    }
}

#[tokio::test]
async fn sync_stores_english_card_data_that_get_returns() {
    let server = ygoprodeck(VERSION).await;
    let (_dir, db) = open_db(&server);

    let outcome = db.sync(|_| {}).await.unwrap();

    assert_eq!(
        outcome,
        SyncOutcome::Updated {
            version: VERSION.into(),
            card_count: 5,
        }
    );
    assert_eq!(
        db.get(ASH_BLOSSOM, Language::En).unwrap(),
        Some(ash_blossom_en())
    );
    assert_eq!(
        requests_to(&server).await,
        ["/checkDBVer.php", "/cardinfo.php"]
    );
}

const POT_OF_GREED: Passcode = Passcode(55_144_522);

fn ash_blossom_fr() -> Card {
    Card {
        name: "Floraison de Cendres et Joyeux Printemps".into(),
        description: "Lorsqu'une carte ou un effet qui inclut n'importe lequel de ces effets \
                      est activé (Effet Rapide) : vous pouvez défausser cette carte ; annulez \
                      l'effet.\n\
                      ●Ajouter une carte depuis le Deck à la main.\n\
                      ●Invoquer Spécialement depuis le Deck.\n\
                      ●Envoyer une carte depuis le Deck au Cimetière.\n\
                      Vous ne pouvez utiliser cet effet de \"Floraison de Cendres et Joyeux \
                      Printemps\" qu'une fois par tour."
            .into(),
        ..ash_blossom_en()
    }
}

fn pot_of_greed_en() -> Card {
    Card {
        passcode: POT_OF_GREED,
        name: "Pot of Greed".into(),
        kind: "Spell Card".into(),
        description: "Draw 2 cards.".into(),
        attribute: None,
        race: Some("Normal".into()),
        level: None,
        link_rating: None,
        atk: None,
        def: None,
    }
}

#[tokio::test]
async fn a_requested_language_is_synced_and_get_returns_its_fields() {
    let server = ygoprodeck(VERSION).await;
    let (_dir, db) = open_db(&server);

    db.request_language(Language::Fr).unwrap();
    db.sync(|_| {}).await.unwrap();

    assert_eq!(
        db.get(ASH_BLOSSOM, Language::Fr).unwrap(),
        Some(ash_blossom_fr())
    );
    assert_eq!(
        requests_to(&server).await,
        [
            "/checkDBVer.php",
            "/cardinfo.php",
            "/cardinfo.php?language=fr"
        ]
    );
}

#[tokio::test]
async fn get_falls_back_to_english_for_a_missing_translation_or_an_unsynced_language() {
    let server = ygoprodeck(VERSION).await;
    let (_dir, db) = open_db(&server);

    db.request_language(Language::Fr).unwrap();
    db.sync(|_| {}).await.unwrap();

    assert_eq!(
        db.get(POT_OF_GREED, Language::Fr).unwrap(),
        Some(pot_of_greed_en())
    );
    assert_eq!(
        db.get(ASH_BLOSSOM, Language::De).unwrap(),
        Some(ash_blossom_en())
    );
}

const DARK_MAGICIAN: Passcode = Passcode(46_986_414);
/// An alternate artwork of Dark Magician (see `fixtures/README.md`).
const DARK_MAGICIAN_ALT_ART: Passcode = Passcode(36_996_508);
const DECODE_TALKER: Passcode = Passcode(1_861_629);

#[tokio::test]
async fn get_returns_link_rating_and_no_level_or_def_for_a_link_monster() {
    let server = ygoprodeck(VERSION).await;
    let (_dir, db) = open_db(&server);

    db.sync(|_| {}).await.unwrap();

    assert_eq!(
        db.get(DECODE_TALKER, Language::En).unwrap(),
        Some(Card {
            passcode: DECODE_TALKER,
            name: "Decode Talker".into(),
            kind: "Link Monster".into(),
            description: "2+ Effect Monsters\r\nGains 500 ATK for each monster it points to. \
                          When your opponent activates a card or effect that targets a \
                          card(s) you control (Quick Effect): You can Tribute 1 monster this \
                          card points to; negate the activation, and if you do, destroy that \
                          card."
                .into(),
            attribute: Some("DARK".into()),
            race: Some("Cyberse".into()),
            level: None,
            link_rating: Some(3),
            atk: Some(2300),
            def: None,
        })
    );
}

#[tokio::test]
async fn an_alternate_artwork_image_id_looks_up_the_card_it_belongs_to() {
    let server = ygoprodeck(VERSION).await;
    let (_dir, db) = open_db(&server);

    db.sync(|_| {}).await.unwrap();

    let dark_magician = Card {
        passcode: DARK_MAGICIAN,
        name: "Dark Magician".into(),
        kind: "Normal Monster".into(),
        description: "''The ultimate wizard in terms of attack and defense.''".into(),
        attribute: Some("DARK".into()),
        race: Some("Spellcaster".into()),
        level: Some(7),
        link_rating: None,
        atk: Some(2500),
        def: Some(2100),
    };
    assert_eq!(
        db.get(DARK_MAGICIAN_ALT_ART, Language::En).unwrap(),
        Some(dark_magician.clone())
    );
    assert_eq!(
        db.get(DARK_MAGICIAN, Language::En).unwrap(),
        Some(dark_magician)
    );
}

#[tokio::test]
async fn a_second_sync_with_an_unchanged_database_version_makes_exactly_one_request() {
    let server = ygoprodeck(VERSION).await;
    let (_dir, db) = open_db(&server);
    db.request_language(Language::Fr).unwrap();
    db.sync(|_| {}).await.unwrap();
    let before = requests_to(&server).await.len();

    let outcome = db.sync(|_| {}).await.unwrap();

    assert_eq!(
        outcome,
        SyncOutcome::UpToDate {
            version: VERSION.into()
        }
    );
    assert_eq!(requests_to(&server).await[before..], ["/checkDBVer.php"]);
    assert_eq!(
        db.get(ASH_BLOSSOM, Language::Fr).unwrap(),
        Some(ash_blossom_fr())
    );
}

#[tokio::test]
async fn a_changed_database_version_downloads_english_and_every_requested_language_again() {
    let server = ygoprodeck(VERSION).await;
    let (_dir, db) = open_db(&server);
    db.request_language(Language::Fr).unwrap();
    db.sync(|_| {}).await.unwrap();
    server.reset().await;
    serve_fixtures(&server, "147.89").await;

    let outcome = db.sync(|_| {}).await.unwrap();

    assert_eq!(
        outcome,
        SyncOutcome::Updated {
            version: "147.89".into(),
            card_count: 7,
        }
    );
    assert_eq!(
        requests_to(&server).await,
        [
            "/checkDBVer.php",
            "/cardinfo.php",
            "/cardinfo.php?language=fr"
        ]
    );
    assert_eq!(
        db.get(ASH_BLOSSOM, Language::Fr).unwrap(),
        Some(ash_blossom_fr())
    );
}

#[tokio::test]
async fn a_language_requested_after_a_sync_is_downloaded_alone_when_the_version_is_unchanged() {
    let server = ygoprodeck(VERSION).await;
    let (_dir, db) = open_db(&server);
    db.sync(|_| {}).await.unwrap();
    let before = requests_to(&server).await.len();

    db.request_language(Language::Fr).unwrap();
    let outcome = db.sync(|_| {}).await.unwrap();

    assert_eq!(
        outcome,
        SyncOutcome::Updated {
            version: VERSION.into(),
            card_count: 2,
        }
    );
    assert_eq!(
        requests_to(&server).await[before..],
        ["/checkDBVer.php", "/cardinfo.php?language=fr"]
    );
    assert_eq!(
        db.get(ASH_BLOSSOM, Language::Fr).unwrap(),
        Some(ash_blossom_fr())
    );
    assert_eq!(
        db.get(POT_OF_GREED, Language::Fr).unwrap(),
        Some(pot_of_greed_en())
    );
}
