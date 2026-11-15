//! Owned by C1 (docs/ROADMAP.md): associating observations into tracks, confirmation,
//! and the events the tracker emits.

use domain::{CameraId, Passcode, Player, Point, Quad};
use serde::{Deserialize, Serialize};

use crate::CameraSetup;
use crate::vote::{Vote, VoteWindow};

/// One detection as the tracker receives it: where a card was seen, by which camera,
/// when, and what it might be.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Observation {
    /// The camera whose frame contained the card.
    pub camera: CameraId,
    /// The card's corners, in that camera's source-frame pixels.
    pub quad: Quad,
    /// The passcodes the classifier considers possible, sorted by score, highest first.
    pub candidates: Vec<ObservedCandidate>,
    /// The passcode recognition settled on for this frame, or `None` when the
    /// recognition was low confidence, not in the deck list, or unreadable.
    pub recognized: Option<Passcode>,
    /// When the frame was captured: milliseconds from the frontend's
    /// `performance.now()`.
    #[cfg_attr(feature = "specta", specta(type = domain::FiniteNumber))]
    pub time_ms: f64,
}

/// A passcode the classifier considers possible for an observation.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct ObservedCandidate {
    /// The candidate card.
    pub passcode: Passcode,
    /// How likely the classifier finds this candidate, from 0 to 1.
    #[cfg_attr(feature = "specta", specta(type = domain::FiniteNumber))]
    pub score: f32,
}

/// Identifies one track (one physical card followed across frames) for the lifetime of
/// a [`Tracker`]. Serializes as a plain JSON number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(transparent)]
pub struct TrackId(pub u64);

/// A change in what the tracker believes is on the table. Only confirmed cards produce
/// events.
///
/// Serializes with a `kind` tag, e.g.
/// `{ "kind": "confirmed", "track": 1, "player": "a", "passcode": 83764719, "timeMs": 1234.5 }`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum TrackEvent {
    /// A track gathered enough consistent evidence to become a confirmed card.
    Confirmed {
        /// The track that became confirmed.
        track: TrackId,
        /// The player the card belongs to.
        player: Player,
        /// The confirmed card.
        passcode: Passcode,
        /// The observation time that confirmed it: milliseconds from `performance.now()`.
        #[cfg_attr(feature = "specta", specta(type = domain::FiniteNumber))]
        time_ms: f64,
    },
    /// A confirmed track's identity changed to a different card.
    Changed {
        /// The track whose identity changed.
        track: TrackId,
        /// The player the card belongs to.
        player: Player,
        /// The card it was confirmed as before.
        from: Passcode,
        /// The card it is confirmed as now.
        to: Passcode,
        /// The observation time of the change: milliseconds from `performance.now()`.
        #[cfg_attr(feature = "specta", specta(type = domain::FiniteNumber))]
        time_ms: f64,
    },
    /// A confirmed track went unseen for longer than the grace period and was closed.
    Lost {
        /// The closed track.
        track: TrackId,
        /// The player the card belonged to.
        player: Player,
        /// The card the track was confirmed as.
        passcode: Passcode,
        /// When the track was closed: milliseconds from `performance.now()`.
        #[cfg_attr(feature = "specta", specta(type = domain::FiniteNumber))]
        time_ms: f64,
    },
}

/// Thresholds for associating, confirming and closing tracks.
///
/// [`TrackerConfig::default`] gives the decided defaults (docs/ROADMAP.md, C1).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct TrackerConfig {
    /// How many of a track's most recent observations take part in its vote. Default 5.
    pub window_size: u32,
    /// Minimum number of votes in the window the leading passcode needs to win.
    /// Default 3.
    pub min_votes: u32,
    /// Minimum ratio of the leading passcode's summed score to the runner-up's summed
    /// score (an absent runner-up counts as 0). Default 1.5.
    #[cfg_attr(feature = "specta", specta(type = domain::FiniteNumber))]
    pub min_margin: f32,
    /// How long a track may go unseen before it is closed, in milliseconds.
    /// Default 2000.
    #[cfg_attr(feature = "specta", specta(type = domain::FiniteNumber))]
    pub grace_ms: f64,
    /// Minimum overlap, as intersection over union of the axis-aligned bounds of the two
    /// quads (0 to 1), for an observation to continue a track. Default 0.3.
    #[cfg_attr(feature = "specta", specta(type = domain::FiniteNumber))]
    pub min_iou: f32,
    /// Maximum distance between the centers of an observation and a track for the
    /// observation to continue it, as a fraction of the card's size: the shorter side
    /// of the axis-aligned bounds of the track's latest quad. Default 0.5.
    #[cfg_attr(feature = "specta", specta(type = domain::FiniteNumber))]
    pub max_center_distance: f32,
}

impl Default for TrackerConfig {
    fn default() -> Self {
        Self {
            window_size: 5,
            min_votes: 3,
            min_margin: 1.5,
            grace_ms: 2000.0,
            min_iou: 0.3,
            max_center_distance: 0.5,
        }
    }
}

/// Follows cards across frames, confirms their identity and assigns them to players.
///
/// Feed it every [`Observation`] with [`Tracker::observe`], and call [`Tracker::tick`]
/// regularly so tracks that are no longer seen are closed.
#[derive(Debug)]
pub struct Tracker {
    config: TrackerConfig,
    setup: CameraSetup,
    /// Open tracks, in creation order (so by increasing ID).
    tracks: Vec<Track>,
    next_id: u64,
}

/// One physical card followed across frames of one camera.
#[derive(Debug)]
struct Track {
    id: TrackId,
    camera: CameraId,
    /// The quad of the latest observation.
    quad: Quad,
    last_seen_ms: f64,
    votes: VoteWindow,
    /// Set once the track is confirmed: the card's player and current identity.
    confirmed: Option<(Player, Passcode)>,
}

impl Tracker {
    /// Creates a tracker with no tracks.
    #[must_use]
    pub fn new(config: TrackerConfig, setup: CameraSetup) -> Self {
        Self {
            config,
            setup,
            tracks: Vec::new(),
            next_id: 1,
        }
    }

    /// Replaces the camera setup. Existing tracks keep their player; new confirmations
    /// use `setup`.
    pub fn set_setup(&mut self, setup: CameraSetup) {
        self.setup = setup;
    }

    /// Adds one observation and returns the events it caused, in order.
    pub fn observe(&mut self, observation: Observation) -> Vec<TrackEvent> {
        let index = match self.matching_track(&observation) {
            Some(index) => index,
            None => self.start_track(&observation),
        };

        let config = self.config;
        let player = player_for(&self.setup, &observation);
        let track = &mut self.tracks[index];
        track.quad = observation.quad;
        track.last_seen_ms = observation.time_ms;
        track.votes.push(vote_of(&observation));

        let Some(winner) = track.votes.winner(config.min_votes, config.min_margin) else {
            return Vec::new();
        };
        match track.confirmed {
            None => {
                track.confirmed = Some((player, winner));
                vec![TrackEvent::Confirmed {
                    track: track.id,
                    player,
                    passcode: winner,
                    time_ms: observation.time_ms,
                }]
            }
            Some(_) => Vec::new(),
        }
    }

    /// Closes tracks unseen for longer than the grace period as of `now_ms`
    /// (milliseconds from `performance.now()`), and returns their
    /// [`TrackEvent::Lost`] events.
    pub fn tick(&mut self, now_ms: f64) -> Vec<TrackEvent> {
        let _ = now_ms;
        Vec::new()
    }

    /// The index of the open track `observation` continues, if any.
    ///
    /// A candidate track is of the same camera, was seen within the grace period, has
    /// not taken an observation of this frame yet, and overlaps the observation or has a
    /// nearby center (see [`TrackerConfig`]). Of those, the one with the nearest center
    /// wins.
    fn matching_track(&self, observation: &Observation) -> Option<usize> {
        let bounds = Bounds::of(&observation.quad);
        self.tracks
            .iter()
            .enumerate()
            .filter(|(_, track)| {
                track.camera == observation.camera
                    && track.last_seen_ms != observation.time_ms
                    && observation.time_ms - track.last_seen_ms <= self.config.grace_ms
            })
            .filter_map(|(index, track)| {
                let track_bounds = Bounds::of(&track.quad);
                let distance = track_bounds.center_distance(&bounds) / track_bounds.short_side();
                let near = distance <= self.config.max_center_distance
                    || track_bounds.iou(&bounds) >= self.config.min_iou;
                near.then_some((index, distance))
            })
            .min_by(|(_, a), (_, b)| a.total_cmp(b))
            .map(|(index, _)| index)
    }

    /// Opens a new, unconfirmed track for `observation` and returns its index.
    fn start_track(&mut self, observation: &Observation) -> usize {
        let id = TrackId(self.next_id);
        self.next_id += 1;
        self.tracks.push(Track {
            id,
            camera: observation.camera.clone(),
            quad: observation.quad,
            last_seen_ms: observation.time_ms,
            votes: VoteWindow::new(self.config.window_size as usize),
            confirmed: None,
        });
        self.tracks.len() - 1
    }
}

/// The axis-aligned bounds of a quad, in source-frame pixels.
#[derive(Debug, Clone, Copy)]
struct Bounds {
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
}

impl Bounds {
    fn of(quad: &Quad) -> Self {
        let [first, rest @ ..] = quad.0;
        rest.iter().fold(
            Self {
                left: first.x,
                top: first.y,
                right: first.x,
                bottom: first.y,
            },
            |bounds, point| Self {
                left: bounds.left.min(point.x),
                top: bounds.top.min(point.y),
                right: bounds.right.max(point.x),
                bottom: bounds.bottom.max(point.y),
            },
        )
    }

    fn width(&self) -> f32 {
        self.right - self.left
    }

    fn height(&self) -> f32 {
        self.bottom - self.top
    }

    fn center(&self) -> Point {
        Point {
            x: (self.left + self.right) / 2.0,
            y: (self.top + self.bottom) / 2.0,
        }
    }

    fn short_side(&self) -> f32 {
        self.width().min(self.height())
    }

    fn center_distance(&self, other: &Self) -> f32 {
        let (a, b) = (self.center(), other.center());
        (a.x - b.x).hypot(a.y - b.y)
    }

    /// Intersection over union, from 0 to 1 (0 when both are empty).
    fn iou(&self, other: &Self) -> f32 {
        let overlap_width = (self.right.min(other.right) - self.left.max(other.left)).max(0.0);
        let overlap_height = (self.bottom.min(other.bottom) - self.top.max(other.top)).max(0.0);
        let intersection = overlap_width * overlap_height;
        let union = self.width() * self.height() + other.width() * other.height() - intersection;
        if union > 0.0 {
            intersection / union
        } else {
            0.0
        }
    }
}

/// The observation's vote: the recognized passcode with its candidate score.
fn vote_of(observation: &Observation) -> Option<Vote> {
    let passcode = observation.recognized?;
    let score = observation
        .candidates
        .iter()
        .find(|candidate| candidate.passcode == passcode)
        .map_or(0.0, |candidate| candidate.score);
    Some(Vote { passcode, score })
}

/// The player a card seen in `observation` belongs to under `setup`.
fn player_for(setup: &CameraSetup, observation: &Observation) -> Player {
    let _ = observation;
    match setup {
        CameraSetup::OneSide { player, .. } => *player,
        CameraSetup::SplitLine { .. } | CameraSetup::TwoCameras { .. } => Player::A,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn track_events_round_trip_with_a_kind_tag_and_camel_case_keys() {
        let cases = [
            (
                TrackEvent::Confirmed {
                    track: TrackId(1),
                    player: Player::A,
                    passcode: Passcode(83_764_719),
                    time_ms: 1234.5,
                },
                json!({
                    "kind": "confirmed",
                    "track": 1,
                    "player": "a",
                    "passcode": 83_764_719,
                    "timeMs": 1234.5,
                }),
            ),
            (
                TrackEvent::Changed {
                    track: TrackId(2),
                    player: Player::B,
                    from: Passcode(55_144_522),
                    to: Passcode(14_558_127),
                    time_ms: 2000.0,
                },
                json!({
                    "kind": "changed",
                    "track": 2,
                    "player": "b",
                    "from": 55_144_522,
                    "to": 14_558_127,
                    "timeMs": 2000.0,
                }),
            ),
            (
                TrackEvent::Lost {
                    track: TrackId(1),
                    player: Player::A,
                    passcode: Passcode(83_764_719),
                    time_ms: 9000.25,
                },
                json!({
                    "kind": "lost",
                    "track": 1,
                    "player": "a",
                    "passcode": 83_764_719,
                    "timeMs": 9000.25,
                }),
            ),
        ];

        for (event, expected) in cases {
            assert_eq!(serde_json::to_value(&event).unwrap(), expected);
            assert_eq!(
                serde_json::from_value::<TrackEvent>(expected).unwrap(),
                event
            );
        }
    }

    const MONSTER_REBORN: Passcode = Passcode(83_764_719);

    fn camera(id: &str) -> CameraId {
        CameraId(id.into())
    }

    fn one_side_tracker() -> Tracker {
        Tracker::new(
            TrackerConfig::default(),
            CameraSetup::OneSide {
                camera: camera("cam"),
                player: Player::A,
            },
        )
    }

    /// An upright 100 × 140 px card whose top-left corner is at (`x`, `y`).
    fn card_at(x: f32, y: f32) -> Quad {
        let point = |dx: f32, dy: f32| domain::Point {
            x: x + dx,
            y: y + dy,
        };
        Quad([
            point(0.0, 0.0),
            point(100.0, 0.0),
            point(100.0, 140.0),
            point(0.0, 140.0),
        ])
    }

    /// An observation from `cam` recognized as `passcode` with score 0.9, or an
    /// unrecognized one when `passcode` is `None`.
    fn seen(quad: Quad, passcode: Option<Passcode>, time_ms: f64) -> Observation {
        seen_by("cam", quad, passcode, time_ms)
    }

    fn seen_by(cam: &str, quad: Quad, passcode: Option<Passcode>, time_ms: f64) -> Observation {
        Observation {
            camera: camera(cam),
            quad,
            candidates: passcode
                .map(|passcode| ObservedCandidate {
                    passcode,
                    score: 0.9,
                })
                .into_iter()
                .collect(),
            recognized: passcode,
            time_ms,
        }
    }

    #[test]
    fn steady_card_is_confirmed_on_its_third_matching_observation() {
        let mut tracker = one_side_tracker();
        let quad = card_at(200.0, 300.0);

        assert_eq!(tracker.observe(seen(quad, Some(MONSTER_REBORN), 0.0)), []);
        assert_eq!(tracker.observe(seen(quad, Some(MONSTER_REBORN), 100.0)), []);
        assert_eq!(
            tracker.observe(seen(quad, Some(MONSTER_REBORN), 200.0)),
            [TrackEvent::Confirmed {
                track: TrackId(1),
                player: Player::A,
                passcode: MONSTER_REBORN,
                time_ms: 200.0,
            }]
        );
        assert_eq!(
            tracker.observe(seen(quad, Some(MONSTER_REBORN), 300.0)),
            [],
            "a track is confirmed once"
        );
    }

    #[test]
    fn duplicate_copies_at_different_positions_are_two_confirmed_tracks() {
        let mut tracker = one_side_tracker();
        let (left, right) = (card_at(100.0, 300.0), card_at(260.0, 300.0));

        let mut events = Vec::new();
        for frame in 0..3 {
            let time_ms = f64::from(frame) * 100.0;
            events.extend(tracker.observe(seen(left, Some(MONSTER_REBORN), time_ms)));
            events.extend(tracker.observe(seen(right, Some(MONSTER_REBORN), time_ms)));
        }

        assert_eq!(
            events,
            [
                TrackEvent::Confirmed {
                    track: TrackId(1),
                    player: Player::A,
                    passcode: MONSTER_REBORN,
                    time_ms: 200.0,
                },
                TrackEvent::Confirmed {
                    track: TrackId(2),
                    player: Player::A,
                    passcode: MONSTER_REBORN,
                    time_ms: 200.0,
                },
            ]
        );
    }

    #[test]
    fn flicker_under_the_grace_period_does_not_create_a_duplicate_track() {
        let mut tracker = one_side_tracker();
        let quad = card_at(200.0, 300.0);

        let mut events = tracker.observe(seen(quad, Some(MONSTER_REBORN), 0.0));
        // Not detected for 1.5 s, then seen again in the same place.
        events.extend(tracker.observe(seen(quad, Some(MONSTER_REBORN), 1500.0)));
        events.extend(tracker.observe(seen(quad, Some(MONSTER_REBORN), 1600.0)));
        events.extend(tracker.tick(1700.0));

        assert_eq!(
            events,
            [TrackEvent::Confirmed {
                track: TrackId(1),
                player: Player::A,
                passcode: MONSTER_REBORN,
                time_ms: 1600.0,
            }]
        );
    }

    #[test]
    fn slightly_moving_card_stays_one_track() {
        let mut tracker = one_side_tracker();

        let mut events = Vec::new();
        for frame in 0..10_u8 {
            // Drifts 15 px right and 10 px down per frame: 150 px over the sequence,
            // more than the card's width.
            let step = f32::from(frame);
            let quad = card_at(200.0 + 15.0 * step, 300.0 + 10.0 * step);
            events.extend(tracker.observe(seen(
                quad,
                Some(MONSTER_REBORN),
                f64::from(frame) * 100.0,
            )));
        }

        assert_eq!(
            events,
            [TrackEvent::Confirmed {
                track: TrackId(1),
                player: Player::A,
                passcode: MONSTER_REBORN,
                time_ms: 200.0,
            }]
        );
    }
}
