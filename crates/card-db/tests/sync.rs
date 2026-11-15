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
    Mock::given(method("GET"))
        .and(path("/checkDBVer.php"))
        .respond_with(json(&check_db_ver(version)))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/cardinfo.php"))
        .and(query_param_is_missing("language"))
        .respond_with(json(CARDINFO_EN))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/cardinfo.php"))
        .and(query_param("language", "fr"))
        .respond_with(json(CARDINFO_FR))
        .mount(&server)
        .await;
    server
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
