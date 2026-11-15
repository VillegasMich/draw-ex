//! Owned by B1 (docs/ROADMAP.md): the YGOPRODeck HTTP client. Every request goes through the shared rate limiter.

use std::sync::Arc;
use std::time::Duration;

use domain::Passcode;
use reqwest::Url;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::value::RawValue;

use crate::limiter::RateLimiter;
use crate::{Card, CardDbError, Language};

/// The YGOPRODeck API v7 root.
pub(crate) const DEFAULT_BASE_URL: &str = "https://db.ygoprodeck.com/api/v7/";

/// Identifies the app and nothing else: no user or machine details.
const USER_AGENT: &str = concat!("draw-ex/", env!("CARGO_PKG_VERSION"));

const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
/// The longest silence while waiting for the next part of a response.
const READ_TIMEOUT: Duration = Duration::from_secs(30);
/// The longest a whole request may take. A bulk `cardinfo.php` is tens of MB, so this
/// only catches a transfer that trickles forever.
const TOTAL_TIMEOUT: Duration = Duration::from_mins(10);

/// Talks to YGOPRODeck. Every request waits on the shared [`RateLimiter`] first.
#[derive(Debug)]
pub(crate) struct Client {
    http: reqwest::Client,
    base_url: Url,
    limiter: Arc<RateLimiter>,
}

/// One card from `cardinfo.php`: the fields [`Card`] needs, plus the raw JSON.
#[derive(Debug)]
pub(crate) struct ApiCard {
    pub(crate) card: Card,
    /// Every artwork's image ID. The first is normally the passcode itself; the others
    /// are alternate artworks of the same card.
    pub(crate) image_ids: Vec<Passcode>,
    /// The card's JSON object exactly as YGOPRODeck sent it.
    pub(crate) raw_json: Box<RawValue>,
}

impl Client {
    /// A client for the YGOPRODeck API rooted at `base_url`.
    pub(crate) fn new(base_url: &str) -> Result<Self, CardDbError> {
        let base_url = parse_base_url(base_url)?;
        let http = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .connect_timeout(CONNECT_TIMEOUT)
            .read_timeout(READ_TIMEOUT)
            .timeout(TOTAL_TIMEOUT)
            .build()?;
        Ok(Self {
            http,
            base_url,
            limiter: Arc::new(RateLimiter::ygoprodeck()),
        })
    }

    /// The rate limiter every YGOPRODeck request waits on. The image cache shares it, so
    /// card data and image downloads together stay under the limit.
    #[expect(dead_code, reason = "used by the image cache (B2)")]
    pub(crate) fn limiter(&self) -> &Arc<RateLimiter> {
        &self.limiter
    }

    /// YGOPRODeck's current `database_version`, from `checkDBVer.php`.
    pub(crate) async fn database_version(&self) -> Result<String, CardDbError> {
        #[derive(Deserialize)]
        struct DbVersion {
            database_version: String,
        }

        let [DbVersion { database_version }] = self.fetch("checkDBVer.php", &[]).await?;
        Ok(database_version)
    }

    /// Every card from `cardinfo.php` in `language`. English cards come without a
    /// `language` parameter; other languages hold only the cards translated into them.
    pub(crate) async fn card_info(&self, language: Language) -> Result<Vec<ApiCard>, CardDbError> {
        #[derive(Deserialize)]
        struct CardInfo {
            data: Vec<Box<RawValue>>,
        }

        let query: &[(&str, &str)] = match language {
            Language::En => &[],
            other => &[("language", other.code())],
        };
        let CardInfo { data } = self.fetch("cardinfo.php", query).await?;
        data.into_iter()
            .map(|raw_json| parse_card(raw_json).map_err(CardDbError::InvalidResponse))
            .collect()
    }

    async fn fetch<T: DeserializeOwned>(
        &self,
        endpoint: &str,
        query: &[(&str, &str)],
    ) -> Result<T, CardDbError> {
        let mut url = self
            .base_url
            .join(endpoint)
            .expect("endpoints are valid relative URLs");
        if !query.is_empty() {
            url.query_pairs_mut().extend_pairs(query);
        }
        self.limiter.until_ready().await;
        let body = self
            .http
            .get(url)
            .send()
            .await
            .and_then(reqwest::Response::error_for_status)?
            .bytes()
            .await?;
        serde_json::from_slice(&body).map_err(CardDbError::InvalidResponse)
    }
}

/// Parses `base_url`, making sure it ends in `/` so endpoints join below it rather than
/// replacing its last segment.
fn parse_base_url(base_url: &str) -> Result<Url, CardDbError> {
    let invalid = || CardDbError::InvalidBaseUrl {
        url: base_url.to_owned(),
    };
    let mut url = Url::parse(base_url).map_err(|_| invalid())?;
    if url.cannot_be_a_base() {
        return Err(invalid());
    }
    if !url.path().ends_with('/') {
        let path = format!("{}/", url.path());
        url.set_path(&path);
    }
    Ok(url)
}

/// Maps one `cardinfo.php` card object to an [`ApiCard`].
fn parse_card(raw_json: Box<RawValue>) -> Result<ApiCard, serde_json::Error> {
    #[derive(Deserialize)]
    struct Fields {
        id: Passcode,
        name: String,
        #[serde(rename = "type")]
        kind: String,
        #[serde(default)]
        desc: String,
        attribute: Option<String>,
        race: Option<String>,
        level: Option<i64>,
        linkval: Option<i64>,
        atk: Option<i64>,
        def: Option<i64>,
        #[serde(default)]
        card_images: Vec<Image>,
    }

    #[derive(Deserialize)]
    struct Image {
        id: Passcode,
    }

    let fields: Fields = serde_json::from_str(raw_json.get())?;
    let link_rating = fields.linkval.and_then(|value| u8::try_from(value).ok());
    Ok(ApiCard {
        card: Card {
            passcode: fields.id,
            name: fields.name,
            kind: fields.kind,
            description: fields.desc,
            attribute: fields.attribute,
            race: fields.race,
            // Link monsters come with `level: 0`; they have no level.
            level: fields
                .level
                .filter(|_| link_rating.is_none())
                .and_then(|value| u8::try_from(value).ok()),
            link_rating,
            // A printed `?` comes as a negative number or not at all.
            atk: fields.atk.and_then(|value| u32::try_from(value).ok()),
            def: fields.def.and_then(|value| u32::try_from(value).ok()),
        },
        image_ids: fields
            .card_images
            .into_iter()
            .map(|image| image.id)
            .collect(),
        raw_json,
    })
}
