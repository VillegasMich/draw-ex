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

impl CameraSetup {
    /// The player a card seen by `camera` at `point` (in that camera's source-frame
    /// pixels) belongs to, or `None` when `camera` is not part of this setup, in which
    /// case the card should be ignored.
    ///
    /// For a card, pass the center of its quad: the [`Tracker`](crate::Tracker) uses
    /// the center of the quad's axis-aligned bounds.
    ///
    /// - `OneSide`: the configured player, wherever the point is.
    /// - `SplitLine`: [`SplitLine::player_at`] of the point.
    /// - `TwoCameras`: the player whose camera it is, wherever the point is. If `a` and
    ///   `b` are the same camera, Player A wins.
    #[must_use]
    pub fn player_for(&self, camera: &CameraId, point: Point) -> Option<Player> {
        match self {
            Self::OneSide {
                camera: setup_camera,
                player,
            } => (camera == setup_camera).then_some(*player),
            Self::SplitLine {
                camera: setup_camera,
                line,
            } => (camera == setup_camera).then(|| line.player_at(point)),
            Self::TwoCameras { a, b } => {
                if camera == a {
                    Some(Player::A)
                } else if camera == b {
                    Some(Player::B)
                } else {
                    None
                }
            }
        }
    }
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
    // If it does not, `player_at` still answers deterministically: Player A.
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
    /// the right to Player B, whatever the line's angle.
    ///
    /// The ambiguous cases resolve to Player A, so the result is always deterministic:
    ///
    /// - points exactly on the line (including past either end) belong to Player A, for
    ///   both directions of the line;
    /// - a degenerate line (`from` equals `to`) has no sides, so every point belongs to
    ///   Player A. All of a camera's cards then land in one panel, which the operator
    ///   sees and fixes by redrawing the line.
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

    /// A line through (100, 100) heading at `degrees` on screen (0° points right, 90°
    /// points down, as y points down), with two points on its left and right.
    fn line_at(degrees: f32) -> (SplitLine, Point, Point) {
        let from = point(100.0, 100.0);
        let (dx, dy) = match degrees {
            0.0 => (1.0, 0.0),
            45.0 => (1.0, 1.0),
            90.0 => (0.0, 1.0),
            180.0 => (-1.0, 0.0),
            _ => unreachable!("no fixture for {degrees}°"),
        };
        let to = point(from.x + 50.0 * dx, from.y + 50.0 * dy);
        // Rotating the heading a quarter turn counter-clockwise on screen, (dy, -dx),
        // points to the left of the direction of travel.
        let left = point(from.x + 20.0 * dy, from.y - 20.0 * dx);
        let right = point(from.x - 20.0 * dy, from.y + 20.0 * dx);
        (SplitLine { from, to }, left, right)
    }

    #[test]
    fn split_lines_at_any_angle_give_player_a_their_left_side() {
        for degrees in [0.0, 45.0, 90.0, 180.0] {
            let (line, left, right) = line_at(degrees);

            assert_eq!(line.player_at(left), Player::A, "left of {degrees}°");
            assert_eq!(line.player_at(right), Player::B, "right of {degrees}°");
        }
    }

    #[test]
    fn reversing_a_split_line_swaps_the_players() {
        for degrees in [0.0, 45.0, 90.0, 180.0] {
            let (line, left, right) = line_at(degrees);
            let reversed = SplitLine {
                from: line.to,
                to: line.from,
            };

            assert_eq!(reversed.player_at(left), Player::B, "left of {degrees}°");
            assert_eq!(reversed.player_at(right), Player::A, "right of {degrees}°");
        }
    }

    #[test]
    fn points_on_a_split_line_at_any_angle_give_player_a() {
        for degrees in [0.0, 45.0, 90.0, 180.0] {
            let (line, ..) = line_at(degrees);
            let (dx, dy) = (line.to.x - line.from.x, line.to.y - line.from.y);
            let on_line = [
                line.from,
                line.to,
                point(line.from.x + 0.5 * dx, line.from.y + 0.5 * dy),
                // Past both ends: the line is infinite.
                point(line.from.x - 3.0 * dx, line.from.y - 3.0 * dy),
                point(line.from.x + 7.0 * dx, line.from.y + 7.0 * dy),
            ];

            for on in on_line {
                assert_eq!(line.player_at(on), Player::A, "{on:?} on {degrees}°");
                let reversed = SplitLine {
                    from: line.to,
                    to: line.from,
                };
                assert_eq!(
                    reversed.player_at(on),
                    Player::A,
                    "{on:?} on reversed {degrees}°"
                );
            }
        }
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

    fn camera(id: &str) -> CameraId {
        CameraId(id.into())
    }

    #[test]
    fn one_side_setup_gives_every_point_of_its_camera_to_its_player() {
        let setup = CameraSetup::OneSide {
            camera: camera("cam-1"),
            player: Player::B,
        };

        assert_eq!(
            setup.player_for(&camera("cam-1"), point(10.0, 10.0)),
            Some(Player::B)
        );
        assert_eq!(
            setup.player_for(&camera("cam-1"), point(900.0, 700.0)),
            Some(Player::B)
        );
    }

    #[test]
    fn split_line_setup_gives_each_half_of_its_camera_to_one_player() {
        let setup = CameraSetup::SplitLine {
            camera: camera("cam-1"),
            line: SplitLine::default_for(1280, 720),
        };

        assert_eq!(
            setup.player_for(&camera("cam-1"), point(640.0, 700.0)),
            Some(Player::A)
        );
        assert_eq!(
            setup.player_for(&camera("cam-1"), point(640.0, 20.0)),
            Some(Player::B)
        );
    }

    #[test]
    fn two_cameras_setup_gives_each_camera_to_its_player() {
        let setup = CameraSetup::TwoCameras {
            a: camera("left"),
            b: camera("right"),
        };

        assert_eq!(
            setup.player_for(&camera("left"), point(5.0, 5.0)),
            Some(Player::A)
        );
        assert_eq!(
            setup.player_for(&camera("right"), point(5.0, 5.0)),
            Some(Player::B)
        );
    }

    #[test]
    fn two_cameras_setup_with_the_same_camera_twice_gives_player_a() {
        let setup = CameraSetup::TwoCameras {
            a: camera("cam-1"),
            b: camera("cam-1"),
        };

        assert_eq!(
            setup.player_for(&camera("cam-1"), point(5.0, 5.0)),
            Some(Player::A)
        );
    }

    #[test]
    fn split_line_setup_with_a_degenerate_line_gives_every_point_to_player_a() {
        let setup = CameraSetup::SplitLine {
            camera: camera("cam-1"),
            line: SplitLine {
                from: point(300.0, 200.0),
                to: point(300.0, 200.0),
            },
        };

        for at in [
            point(0.0, 0.0),
            point(300.0, 200.0),
            point(1919.0, 1079.0),
            point(301.0, 0.0),
        ] {
            assert_eq!(
                setup.player_for(&camera("cam-1"), at),
                Some(Player::A),
                "{at:?}"
            );
        }
    }

    #[test]
    fn a_camera_not_in_the_setup_has_no_player() {
        let setups = [
            CameraSetup::OneSide {
                camera: camera("cam-1"),
                player: Player::A,
            },
            CameraSetup::SplitLine {
                camera: camera("cam-1"),
                line: SplitLine::default_for(1280, 720),
            },
            CameraSetup::TwoCameras {
                a: camera("left"),
                b: camera("right"),
            },
        ];

        for setup in setups {
            assert_eq!(
                setup.player_for(&camera("other"), point(5.0, 5.0)),
                None,
                "{setup:?}"
            );
        }
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
