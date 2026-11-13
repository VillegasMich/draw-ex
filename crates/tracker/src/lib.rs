//! Tracks, voting, confirmation and player assignment.
//!
//! Associates per-frame detections into tracks across frames, confirms a card once its
//! vote is clear, and assigns it to a player. Has no Tauri dependency, so it can be
//! tested and run headless.
