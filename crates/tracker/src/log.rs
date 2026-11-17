//! Owned by C2 (docs/ROADMAP.md): the duel log, a JSONL file with a session header and
//! one track event per line.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{CameraSetup, TrackEvent};

/// Identifies one session (one duel). Serializes as a plain JSON string.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(transparent)]
pub struct SessionId(pub String);

/// What identifies a session, written as the duel log's header.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    /// The session's identifier.
    pub id: SessionId,
    /// When the operator started the session: **wall-clock** milliseconds since the Unix
    /// epoch. This is the one time in the contract that is not from `performance.now()`,
    /// because the log must record when the duel happened.
    #[cfg_attr(feature = "specta", specta(type = domain::FiniteNumber))]
    pub started_at_ms: f64,
}

/// The ordered record of one session's track events, appended to a file as they happen.
///
/// The file is [JSON Lines](https://jsonlines.org/): UTF-8, one JSON object per line,
/// each line ending in `\n`.
///
/// - **Line 1** is the [`DuelLogHeader`]:
///   `{ "id", "startedAtMs", "setup", "originPerfMs" }`.
/// - **Every later line** is a [`LoggedEvent`]: the [`TrackEvent`]'s own fields (with its
///   `kind` tag and `performance.now()` `timeMs`) plus the derived wall-clock
///   `wallClockMs`, e.g.
///   `{ "kind": "confirmed", "track": 1, "player": "a", "passcode": 83764719,
///   "timeMs": 1234.5, "wallClockMs": 1763135521234.5 }`.
///
/// Each line is written whole and flushed to the operating system before
/// [`DuelLog::start`] or [`DuelLog::append`] returns, so if the app crashes the file
/// holds every line appended so far, and at most a partial last line, which
/// [`DuelLog::read`] tolerates. Lines are not synced to the disk itself, so a power
/// loss may lose the latest lines.
#[derive(Debug)]
pub struct DuelLog {
    file: File,
    header: DuelLogHeader,
}

/// The duel log's first line: the session, its camera setup and its
/// `performance.now()` origin.
///
/// Serializes flat, with the [`SessionInfo`] fields at the top level:
/// `{ "id", "startedAtMs", "setup", "originPerfMs" }`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DuelLogHeader {
    /// The session the log records.
    #[serde(flatten)]
    pub session: SessionInfo,
    /// The camera setup when the session started.
    pub setup: CameraSetup,
    /// The frontend's `performance.now()` reading, in milliseconds, taken at the same
    /// moment as [`SessionInfo::started_at_ms`]. It maps event times to wall-clock time.
    pub origin_perf_ms: f64,
}

impl DuelLogHeader {
    /// The wall-clock time, in milliseconds since the Unix epoch, of a
    /// `performance.now()` reading `perf_ms` taken in this session:
    /// `startedAtMs + (perf_ms − originPerfMs)`.
    #[must_use]
    pub fn wall_clock_ms(&self, perf_ms: f64) -> f64 {
        self.session.started_at_ms + (perf_ms - self.origin_perf_ms)
    }
}

/// One event line of the duel log: a [`TrackEvent`] and when it happened on the wall
/// clock.
///
/// Serializes as the event's own fields plus `wallClockMs`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoggedEvent {
    /// The event, with its `performance.now()` time.
    #[serde(flatten)]
    pub event: TrackEvent,
    /// When the event happened: wall-clock milliseconds since the Unix epoch, derived
    /// with [`DuelLogHeader::wall_clock_ms`].
    pub wall_clock_ms: f64,
}

/// Everything [`DuelLog::read`] found in a duel log file.
#[derive(Debug, Clone, PartialEq)]
pub struct DuelLogContents {
    /// The log's first line.
    pub header: DuelLogHeader,
    /// The events, in the order they were appended.
    pub events: Vec<LoggedEvent>,
    /// Whether a partially written last line (one the app was writing when it stopped)
    /// was skipped.
    pub truncated: bool,
}

impl DuelLog {
    /// The conventional file name of `session`'s log, to be placed in the app-data
    /// `duels/` directory: its start time in UTC, ISO 8601-like with `-` in place of
    /// `:` (which Windows forbids in file names), to the millisecond, e.g.
    /// `2025-11-14T15-52-00.123Z.jsonl`.
    ///
    /// Names sort in start order. Fractions of a millisecond are dropped.
    #[must_use]
    pub fn file_name(session: &SessionInfo) -> String {
        // Saturating cast: a non-finite start (never produced by the wall clock) still
        // gives a valid name.
        #[expect(
            clippy::cast_possible_truncation,
            reason = "wall-clock epoch milliseconds are far below i64::MAX"
        )]
        let ms = session.started_at_ms.floor() as i64;
        let (days, ms_of_day) = (ms.div_euclid(MS_PER_DAY), ms.rem_euclid(MS_PER_DAY));
        let (year, month, day) = civil_from_days(days);
        let (seconds, millis) = (ms_of_day / 1000, ms_of_day % 1000);
        format!(
            "{year:04}-{month:02}-{day:02}T{:02}-{:02}-{:02}.{millis:03}Z.jsonl",
            seconds / 3600,
            seconds / 60 % 60,
            seconds % 60,
        )
    }

    /// Creates the log file at `path` and writes its [`DuelLogHeader`].
    ///
    /// `origin_perf_ms` is the frontend's `performance.now()`, in milliseconds, at the
    /// moment [`SessionInfo::started_at_ms`] was read from the wall clock. The parent
    /// directory must exist; [`DuelLog::file_name`] gives the file's conventional name.
    ///
    /// # Errors
    ///
    /// [`LogError::Io`] if the file cannot be created or written. A file that already
    /// exists at `path` is left untouched, and the error's kind is
    /// [`std::io::ErrorKind::AlreadyExists`].
    pub fn start(
        path: impl AsRef<Path>,
        session: &SessionInfo,
        setup: &CameraSetup,
        origin_perf_ms: f64,
    ) -> Result<Self, LogError> {
        let file = OpenOptions::new().write(true).create_new(true).open(path)?;
        let mut log = Self {
            file,
            header: DuelLogHeader {
                session: session.clone(),
                setup: setup.clone(),
                origin_perf_ms,
            },
        };
        write_line(&mut log.file, &log.header)?;
        Ok(log)
    }

    /// Appends one event to the log, with its derived wall-clock time.
    ///
    /// Times are expected to be finite, as the contract requires: JSON has no
    /// infinity or NaN, so such a time would be written as `null` and
    /// [`DuelLog::read`] would reject the line.
    ///
    /// # Errors
    ///
    /// [`LogError::Io`] if the event cannot be written.
    pub fn append(&mut self, event: &TrackEvent) -> Result<(), LogError> {
        let line = LoggedEvent {
            event: event.clone(),
            wall_clock_ms: self.header.wall_clock_ms(event_time_ms(event)),
        };
        write_line(&mut self.file, &line)
    }

    /// Reads a duel log file written by [`DuelLog`].
    ///
    /// Every line [`DuelLog`] writes ends in `\n`, so a last line without one was being
    /// written when the app stopped. If it does not parse, it is skipped and
    /// [`DuelLogContents::truncated`] is set; the events before it are returned. If it
    /// parses (the write completed but its `\n` did not), it is kept.
    ///
    /// # Errors
    ///
    /// - [`LogError::Io`] if the file cannot be read.
    /// - [`LogError::Format`] if a complete line is not a valid event, or if line 1 is
    ///   not a complete, valid header (an empty file included).
    pub fn read(path: impl AsRef<Path>) -> Result<DuelLogContents, LogError> {
        let bytes = fs::read(path)?;
        // Splitting on every `\n` leaves, after the last one, what was written of an
        // unfinished line: nothing when the file ends cleanly.
        let mut lines: Vec<&[u8]> = bytes.split(|&byte| byte == b'\n').collect();
        let unterminated = lines.pop().unwrap_or_default();

        let Some((header_line, event_lines)) = lines.split_first() else {
            // No complete line: the header itself was being written.
            return Ok(DuelLogContents {
                header: parse_line(unterminated, 1)?,
                events: Vec::new(),
                truncated: false,
            });
        };
        let header = parse_line(header_line, 1)?;
        let mut events = event_lines
            .iter()
            .zip(2..)
            .map(|(line, number)| parse_line(line, number))
            .collect::<Result<Vec<LoggedEvent>, _>>()?;

        let mut truncated = false;
        if !unterminated.is_empty() {
            match serde_json::from_slice(unterminated) {
                Ok(event) => events.push(event),
                Err(_) => truncated = true,
            }
        }
        Ok(DuelLogContents {
            header,
            events,
            truncated,
        })
    }
}

/// Writes `value` to `file` as one JSON line and flushes it.
fn write_line(file: &mut File, value: &impl Serialize) -> Result<(), LogError> {
    // Serializing the log's own types cannot fail (string keys, no custom
    // serializers), so a failure here is reported as the write failing.
    let mut line = serde_json::to_vec(value).map_err(std::io::Error::other)?;
    line.push(b'\n');
    // The line is handed over whole, never interleaved with another, so a crash can
    // at worst cut off its end, which `read` tolerates. `File` is unbuffered: once
    // `write_all` returns the line is with the operating system, and `flush` only
    // guards against a buffered writer replacing `File` later.
    file.write_all(&line)?;
    file.flush()?;
    Ok(())
}

/// The `performance.now()` time of `event`, in milliseconds.
fn event_time_ms(event: &TrackEvent) -> f64 {
    match event {
        TrackEvent::Confirmed { time_ms, .. }
        | TrackEvent::Changed { time_ms, .. }
        | TrackEvent::Lost { time_ms, .. } => *time_ms,
    }
}

const MS_PER_DAY: i64 = 86_400_000;

/// The proleptic Gregorian `(year, month, day)` of `days` since 1970-01-01.
///
/// Howard Hinnant's `civil_from_days`
/// (<https://howardhinnant.github.io/date_algorithms.html#civil_from_days>).
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    // Months counted from March, so the leap day is the last day of the year.
    let march_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * march_month + 2) / 5 + 1;
    let month = if march_month < 10 {
        march_month + 3
    } else {
        march_month - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// Parses one line of the file; `number` counts from 1.
fn parse_line<T: DeserializeOwned>(line: &[u8], number: usize) -> Result<T, LogError> {
    serde_json::from_slice(line).map_err(|source| LogError::Format {
        line: number,
        source,
    })
}

/// Why the duel log could not be written or read.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum LogError {
    /// Reading or writing the log file failed.
    #[error("duel log I/O failed")]
    Io(#[from] std::io::Error),
    /// A line of the log file is not valid JSON of the expected shape.
    #[error("duel log line {line} is malformed")]
    Format {
        /// The malformed line's number, counting from 1 (the header).
        line: usize,
        /// Why the line could not be parsed.
        #[source]
        source: serde_json::Error,
    },
}

#[cfg(test)]
mod tests {
    use domain::{CameraId, Passcode, Player};
    use serde_json::json;

    use super::*;
    use crate::{SplitLine, TrackId};

    fn session() -> SessionInfo {
        SessionInfo {
            id: SessionId("2025-11-14T15-52-00".into()),
            started_at_ms: 1_763_135_520_000.0,
        }
    }

    fn setup() -> CameraSetup {
        CameraSetup::SplitLine {
            camera: CameraId("cam-1".into()),
            line: SplitLine::default_for(1920, 1080),
        }
    }

    fn confirmed(track: u64, time_ms: f64) -> TrackEvent {
        TrackEvent::Confirmed {
            track: TrackId(track),
            player: Player::A,
            passcode: Passcode(83_764_719),
            time_ms,
        }
    }

    #[test]
    fn header_and_events_round_trip_through_read() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("duel.jsonl");
        let events = [
            confirmed(1, 5_100.0),
            TrackEvent::Changed {
                track: TrackId(1),
                player: Player::A,
                from: Passcode(83_764_719),
                to: Passcode(46_986_414),
                time_ms: 5_200.25,
            },
            TrackEvent::Lost {
                track: TrackId(1),
                player: Player::B,
                passcode: Passcode(46_986_414),
                time_ms: 9_000.0,
            },
        ];

        let mut log = DuelLog::start(&path, &session(), &setup(), 5_000.0).unwrap();
        for event in &events {
            log.append(event).unwrap();
        }
        drop(log);
        let contents = DuelLog::read(&path).unwrap();

        assert_eq!(
            contents.header,
            DuelLogHeader {
                session: session(),
                setup: setup(),
                origin_perf_ms: 5_000.0,
            }
        );
        let read_events: Vec<TrackEvent> = contents
            .events
            .into_iter()
            .map(|logged| logged.event)
            .collect();
        assert_eq!(read_events, events);
        assert!(!contents.truncated);
    }

    #[test]
    fn events_carry_the_wall_clock_time_derived_from_the_session_origin() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("duel.jsonl");

        // The session started at wall-clock 1_763_135_520_000 when performance.now()
        // read 5_000, so performance.now() 6_234.5 is 1_234.5 ms later on the wall clock.
        let mut log = DuelLog::start(&path, &session(), &setup(), 5_000.0).unwrap();
        log.append(&confirmed(7, 6_234.5)).unwrap();
        // An event stamped before the origin is still mapped linearly.
        log.append(&confirmed(8, 4_000.0)).unwrap();
        drop(log);

        let contents = DuelLog::read(&path).unwrap();
        let wall_clock: Vec<f64> = contents
            .events
            .iter()
            .map(|logged| logged.wall_clock_ms)
            .collect();
        assert_eq!(wall_clock, [1_763_135_521_234.5, 1_763_135_519_000.0]);

        let text = fs::read_to_string(&path).unwrap();
        let lines: Vec<serde_json::Value> = text
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(
            lines[0],
            json!({
                "id": "2025-11-14T15-52-00",
                "startedAtMs": 1_763_135_520_000.0,
                "setup": {
                    "kind": "splitLine",
                    "camera": "cam-1",
                    "line": {
                        "from": { "x": 1920.0, "y": 540.0 },
                        "to": { "x": 0.0, "y": 540.0 },
                    },
                },
                "originPerfMs": 5_000.0,
            })
        );
        assert_eq!(
            lines[1],
            json!({
                "kind": "confirmed",
                "track": 7,
                "player": "a",
                "passcode": 83_764_719,
                "timeMs": 6_234.5,
                "wallClockMs": 1_763_135_521_234.5,
            })
        );
    }

    #[test]
    fn each_line_reaches_the_file_as_soon_as_it_is_appended() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("duel.jsonl");
        let line_count = || fs::read_to_string(&path).unwrap().lines().count();

        let mut log = DuelLog::start(&path, &session(), &setup(), 0.0).unwrap();
        assert_eq!(line_count(), 1, "the header is on disk once start returns");

        log.append(&confirmed(1, 10.0)).unwrap();
        assert_eq!(line_count(), 2);
        log.append(&confirmed(2, 20.0)).unwrap();
        assert_eq!(line_count(), 3);

        // Read while the writer is still alive.
        let contents = DuelLog::read(&path).unwrap();
        assert_eq!(contents.events.len(), 2);
        drop(log);
    }

    #[test]
    fn starting_a_log_never_overwrites_an_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("duel.jsonl");
        fs::write(&path, "an earlier duel\n").unwrap();

        let error = DuelLog::start(&path, &session(), &setup(), 0.0).unwrap_err();

        assert!(
            matches!(&error, LogError::Io(io) if io.kind() == std::io::ErrorKind::AlreadyExists),
            "{error:?}"
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), "an earlier duel\n");
    }

    /// Writes a log with a header and `count` events, then appends `raw` bytes as if
    /// the app had been writing them when it stopped.
    fn log_followed_by(dir: &tempfile::TempDir, count: u64, raw: &[u8]) -> std::path::PathBuf {
        let path = dir.path().join("duel.jsonl");
        let mut log = DuelLog::start(&path, &session(), &setup(), 0.0).unwrap();
        for track in 1..=count {
            log.append(&confirmed(track, 10.0)).unwrap();
        }
        drop(log);
        let mut file = OpenOptions::new().append(true).open(&path).unwrap();
        file.write_all(raw).unwrap();
        path
    }

    #[test]
    fn read_skips_and_reports_a_partially_written_last_line() {
        let dir = tempfile::tempdir().unwrap();
        let path = log_followed_by(&dir, 2, br#"{"kind":"confirmed","track":3,"pla"#);

        let contents = DuelLog::read(&path).unwrap();

        assert_eq!(contents.events.len(), 2);
        assert!(contents.truncated);
    }

    #[test]
    fn read_reports_a_malformed_complete_line_with_its_number() {
        let dir = tempfile::tempdir().unwrap();
        let path = log_followed_by(&dir, 2, b"not json\n");

        let error = DuelLog::read(&path).unwrap_err();

        assert!(
            matches!(error, LogError::Format { line: 4, .. }),
            "{error:?}"
        );
    }

    #[test]
    fn read_rejects_a_file_without_a_complete_header() {
        let dir = tempfile::tempdir().unwrap();
        for raw in [&b""[..], br#"{"id":"2025-11-14T15-52-00","startedAt"#] {
            let path = dir.path().join("cut.jsonl");
            fs::write(&path, raw).unwrap();

            let error = DuelLog::read(&path).unwrap_err();

            assert!(
                matches!(error, LogError::Format { line: 1, .. }),
                "{error:?}"
            );
        }
    }

    #[test]
    fn file_name_is_the_utc_session_start_without_colons() {
        let cases = [
            (0.0, "1970-01-01T00-00-00.000Z.jsonl"),
            (1_763_135_520_123.0, "2025-11-14T15-52-00.123Z.jsonl"),
            // Fractions of a millisecond are dropped, not rounded up.
            (1_763_135_520_123.9, "2025-11-14T15-52-00.123Z.jsonl"),
            (1_709_251_199_999.0, "2024-02-29T23-59-59.999Z.jsonl"),
            (951_782_400_000.0, "2000-02-29T00-00-00.000Z.jsonl"),
            (4_102_444_800_000.0, "2100-01-01T00-00-00.000Z.jsonl"),
        ];

        for (started_at_ms, expected) in cases {
            let session = SessionInfo {
                id: SessionId("any".into()),
                started_at_ms,
            };
            assert_eq!(DuelLog::file_name(&session), expected, "{started_at_ms}");
        }
    }

    #[test]
    fn session_info_round_trips_with_camel_case_keys() {
        let session = SessionInfo {
            id: SessionId("2025-11-14T15-52-00".into()),
            started_at_ms: 1_763_135_520_000.0,
        };
        let expected = json!({ "id": "2025-11-14T15-52-00", "startedAtMs": 1_763_135_520_000.0 });

        assert_eq!(serde_json::to_value(&session).unwrap(), expected);
        assert_eq!(
            serde_json::from_value::<SessionInfo>(expected).unwrap(),
            session
        );
    }
}
