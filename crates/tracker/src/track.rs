//! Owned by C1 (docs/ROADMAP.md): associating observations into tracks, confirmation,
//! and the events the tracker emits.

use domain::{CameraId, Passcode, Player, Quad};
use serde::{Deserialize, Serialize};

use crate::CameraSetup;

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
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct TrackerConfig {
    /// Minimum number of votes the leading passcode needs before the track is confirmed.
    pub min_votes: u32,
    /// Minimum lead of the leading passcode's vote weight over the runner-up's, as a
    /// fraction of the total vote weight, from 0 to 1.
    #[cfg_attr(feature = "specta", specta(type = domain::FiniteNumber))]
    pub min_margin: f32,
    /// How long a track may go unseen before it is closed, in milliseconds.
    #[cfg_attr(feature = "specta", specta(type = domain::FiniteNumber))]
    pub grace_ms: f64,
}

/// Follows cards across frames, confirms their identity and assigns them to players.
///
/// Feed it every [`Observation`] with [`Tracker::observe`], and call [`Tracker::tick`]
/// regularly so tracks that are no longer seen are closed.
#[derive(Debug)]
pub struct Tracker {
    // Replaced by the real state in C1.
    _placeholder: (),
}

impl Tracker {
    /// Creates a tracker with no tracks.
    #[must_use]
    pub fn new(config: TrackerConfig, setup: CameraSetup) -> Self {
        let _ = (config, setup);
        todo!("C1")
    }

    /// Replaces the camera setup. Existing tracks keep their player; new confirmations
    /// use `setup`.
    pub fn set_setup(&mut self, setup: CameraSetup) {
        let _ = setup;
        todo!("C1")
    }

    /// Adds one observation and returns the events it caused, in order.
    pub fn observe(&mut self, observation: Observation) -> Vec<TrackEvent> {
        let _ = observation;
        todo!("C1")
    }

    /// Closes tracks unseen for longer than the grace period as of `now_ms`
    /// (milliseconds from `performance.now()`), and returns their
    /// [`TrackEvent::Lost`] events.
    pub fn tick(&mut self, now_ms: f64) -> Vec<TrackEvent> {
        let _ = now_ms;
        todo!("C1")
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
}
