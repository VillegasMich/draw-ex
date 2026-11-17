//! Tracks, voting, confirmation and player assignment. No Tauri dependency.
//!
//! The [`Tracker`] turns per-frame [`Observation`]s into [`TrackEvent`]s about confirmed
//! cards, and assigns each card to a player through the [`CameraSetup`]. The
//! [`DuelLog`] writes a session's events to disk.
//!
//! This crate never depends on draw-core (ADR-0002): it takes its own [`Observation`]
//! input, which the app shell builds from draw-core detections.
//!
//! # Units and coordinate spaces
//!
//! - Coordinates are `f32` pixels of the observing camera's source frame, origin at the
//!   top-left, y pointing down (see [`domain::Point`]).
//! - Passcodes are integers ([`domain::Passcode`]).
//! - Scores are `f32` from 0 to 1.
//! - Times named `*_ms` are milliseconds as `f64` from the frontend's
//!   `performance.now()`, except [`SessionInfo::started_at_ms`], which is wall-clock time.
//!
//! Every type serializes with camelCase keys, and enums with camelCase variant names.
//! With the `specta` cargo feature, every type also derives `specta::Type`.

#![forbid(unsafe_code)]

mod log;
mod players;
mod track;
mod vote;

pub use log::{
    DuelLog, DuelLogContents, DuelLogHeader, LogError, LoggedEvent, SessionId, SessionInfo,
};
pub use players::{CameraSetup, SplitLine};
pub use track::{Observation, ObservedCandidate, TrackEvent, TrackId, Tracker, TrackerConfig};
