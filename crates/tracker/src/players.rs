//! Owned by C2 (docs/ROADMAP.md): how cards are assigned to players.

use domain::{CameraId, Player, Point};
use serde::{Deserialize, Serialize};

/// How cameras map to players.
///
/// Serializes with a `kind` tag: `{ "kind": "oneSide", "camera", "player" }`,
/// `{ "kind": "splitLine", "camera", "line" }` or `{ "kind": "twoCameras", "a", "b" }`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum CameraSetup {
    /// One camera on one player's side: every card it sees belongs to `player`.
    OneSide {
        /// The only camera.
        camera: CameraId,
        /// The player whose side the camera shows.
        player: Player,
    },
    /// One camera on both sides, divided by a split line.
    SplitLine {
        /// The only camera.
        camera: CameraId,
        /// The line dividing the camera's view, in that camera's frame pixels.
        line: SplitLine,
    },
    /// Two cameras, one per player.
    TwoCameras {
        /// The camera on Player A's side.
        a: CameraId,
        /// The camera on Player B's side.
        b: CameraId,
    },
}

/// A line across one camera's view that separates Player A's half of the table from
/// Player B's, in that camera's source-frame pixels.
///
/// **Player A is on the left of the direction `from → to` as seen on screen** (y
/// pointing down). The line extends infinitely past both points, so it can be drawn at
/// any angle.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SplitLine {
    /// Where the line starts, in source-frame pixels.
    pub from: Point,
    /// A second point the line passes through, in source-frame pixels. Must differ from
    /// `from`.
    pub to: Point,
}

impl SplitLine {
    /// The default split line for a `width` × `height` pixel frame: the horizontal line
    /// through the middle, drawn right to left, so Player A has the bottom half.
    #[must_use]
    pub fn default_for(width: u32, height: u32) -> Self {
        // Frame sizes are far below 2^24 pixels, so the f32 conversion is exact.
        let (width, middle) = (width as f32, height as f32 / 2.0);
        Self {
            from: Point {
                x: width,
                y: middle,
            },
            to: Point { x: 0.0, y: middle },
        }
    }

    /// Which player's half of the table `point` (in source-frame pixels) is on.
    ///
    /// Points to the left of `from → to` as seen on screen belong to Player A, points to
    /// the right to Player B. Points exactly on the line, and every point when `from`
    /// equals `to`, belong to Player A.
    #[must_use]
    pub fn player_at(&self, point: Point) -> Player {
        let (dx, dy) = (
            f64::from(self.to.x - self.from.x),
            f64::from(self.to.y - self.from.y),
        );
        let (px, py) = (
            f64::from(point.x - self.from.x),
            f64::from(point.y - self.from.y),
        );
        // With y pointing down, a negative cross product means `point` is on the
        // on-screen left of the direction `from → to`.
        let cross = dx * py - dy * px;
        if cross > 0.0 { Player::B } else { Player::A }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn point(x: f32, y: f32) -> Point {
        Point { x, y }
    }

    #[test]
    fn default_line_is_horizontal_through_the_middle_drawn_right_to_left() {
        assert_eq!(
            SplitLine::default_for(1920, 1080),
            SplitLine {
                from: point(1920.0, 540.0),
                to: point(0.0, 540.0),
            }
        );
    }

    #[test]
    fn default_line_gives_player_a_the_bottom_half_and_player_b_the_top_half() {
        let line = SplitLine::default_for(1280, 720);

        assert_eq!(line.player_at(point(640.0, 700.0)), Player::A);
        assert_eq!(line.player_at(point(10.0, 361.0)), Player::A);
        assert_eq!(line.player_at(point(640.0, 20.0)), Player::B);
        assert_eq!(line.player_at(point(1270.0, 359.0)), Player::B);
    }

    #[test]
    fn line_drawn_left_to_right_gives_player_a_the_top_half() {
        let line = SplitLine {
            from: point(0.0, 360.0),
            to: point(1280.0, 360.0),
        };

        assert_eq!(line.player_at(point(640.0, 100.0)), Player::A);
        assert_eq!(line.player_at(point(640.0, 600.0)), Player::B);
    }

    #[test]
    fn vertical_line_drawn_downwards_gives_player_a_the_right_half() {
        // Walking down the screen, your left hand points to the screen's right.
        let line = SplitLine {
            from: point(640.0, 0.0),
            to: point(640.0, 720.0),
        };

        assert_eq!(line.player_at(point(1000.0, 360.0)), Player::A);
        assert_eq!(line.player_at(point(100.0, 360.0)), Player::B);
    }

    #[test]
    fn diagonal_line_splits_beyond_its_end_points() {
        // From the bottom-left corner to the top-right corner of a 100x100 frame.
        let line = SplitLine {
            from: point(0.0, 100.0),
            to: point(100.0, 0.0),
        };

        assert_eq!(line.player_at(point(10.0, 10.0)), Player::A);
        assert_eq!(line.player_at(point(90.0, 90.0)), Player::B);
        assert_eq!(line.player_at(point(-50.0, 100.0)), Player::A);
        assert_eq!(line.player_at(point(300.0, 0.0)), Player::B);
    }

    #[test]
    fn points_on_the_line_and_degenerate_lines_give_player_a() {
        let line = SplitLine::default_for(100, 100);
        assert_eq!(line.player_at(point(30.0, 50.0)), Player::A);

        let degenerate = SplitLine {
            from: point(5.0, 5.0),
            to: point(5.0, 5.0),
        };
        assert_eq!(degenerate.player_at(point(80.0, 1.0)), Player::A);
    }

    #[test]
    fn camera_setup_round_trips_with_a_kind_tag_and_camel_case_keys() {
        let cases = [
            (
                CameraSetup::OneSide {
                    camera: CameraId("cam-1".into()),
                    player: Player::B,
                },
                json!({ "kind": "oneSide", "camera": "cam-1", "player": "b" }),
            ),
            (
                CameraSetup::SplitLine {
                    camera: CameraId("cam-1".into()),
                    line: SplitLine::default_for(200, 100),
                },
                json!({
                    "kind": "splitLine",
                    "camera": "cam-1",
                    "line": {
                        "from": { "x": 200.0, "y": 50.0 },
                        "to": { "x": 0.0, "y": 50.0 },
                    },
                }),
            ),
            (
                CameraSetup::TwoCameras {
                    a: CameraId("left".into()),
                    b: CameraId("right".into()),
                },
                json!({ "kind": "twoCameras", "a": "left", "b": "right" }),
            ),
        ];

        for (setup, expected) in cases {
            assert_eq!(serde_json::to_value(&setup).unwrap(), expected);
            assert_eq!(
                serde_json::from_value::<CameraSetup>(expected).unwrap(),
                setup
            );
        }
    }
}
