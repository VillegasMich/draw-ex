//! Identifiers and geometry shared by the draw-ex crates: passcode, camera, player,
//! point and quad.
//!
//! This crate depends on no other workspace crate, and never on draw-core (ADR-0002).
//! draw-core defines its own `Passcode` and `Quad` with the same meaning; the app shell
//! maps between them in one place.
//!
//! # Units and coordinate spaces
//!
//! - Coordinates ([`Point`], [`Quad`]) are `f32` pixels of the source frame, origin at
//!   the top-left, x pointing right and y pointing down.
//! - Passcodes are integers; leading zeros are not significant.
//!
//! Every type serializes with camelCase keys. With the `specta` cargo feature, every type
//! also derives `specta::Type` for the TypeScript bindings, and [`FiniteNumber`] is
//! available to export float fields as a plain TypeScript `number`.

#![forbid(unsafe_code)]

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// A card's identity: the 8-digit number printed on the card, as an integer.
///
/// Leading zeros are not significant, so `"00012345"` and `"12345"` parse to the same
/// passcode. It serializes as a plain JSON number. The value is not range-checked, like
/// draw-core's passcode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(transparent)]
pub struct Passcode(pub u32);

impl FromStr for Passcode {
    type Err = ParsePasscodeError;

    /// Parses a passcode written in decimal digits, ignoring leading zeros and
    /// surrounding whitespace, as found in `.ydk` files (`"083764719"` → `83764719`).
    ///
    /// # Errors
    ///
    /// - [`ParsePasscodeError::Empty`] if there are no digits.
    /// - [`ParsePasscodeError::InvalidCharacter`] if anything other than ASCII digits
    ///   remains after trimming, including a sign.
    /// - [`ParsePasscodeError::TooLarge`] if the value does not fit in a `u32`.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let digits = text.trim();
        if digits.is_empty() {
            return Err(ParsePasscodeError::Empty);
        }
        if let Some(found) = digits.chars().find(|c| !c.is_ascii_digit()) {
            return Err(ParsePasscodeError::InvalidCharacter { found });
        }
        let significant = digits.trim_start_matches('0');
        if significant.is_empty() {
            return Ok(Self(0));
        }
        significant
            .parse()
            .map(Self)
            .map_err(|_| ParsePasscodeError::TooLarge)
    }
}

impl fmt::Display for Passcode {
    /// Writes the passcode as a plain integer, without leading zeros.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Why text could not be parsed as a [`Passcode`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ParsePasscodeError {
    /// The text is empty or only whitespace.
    #[error("passcode is empty")]
    Empty,
    /// The text contains something other than ASCII digits.
    #[error("passcode contains {found:?}, expected only digits")]
    InvalidCharacter {
        /// The first character that is not an ASCII digit.
        found: char,
    },
    /// The number does not fit in 32 bits.
    #[error("passcode is too large")]
    TooLarge,
}

/// Identifies one camera: the webview's `MediaDeviceInfo.deviceId` for it.
///
/// The value is opaque; only equality matters. It serializes as a plain JSON string.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(transparent)]
pub struct CameraId(pub String);

/// One of the two duelists. Serializes as `"a"` or `"b"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum Player {
    /// Player A, shown in the left panel.
    A,
    /// Player B, shown in the right panel.
    B,
}

/// A position in a camera's source frame, in pixels.
///
/// The origin is the frame's top-left corner, x points right and y points down. Values
/// are fractional because corners come from sub-pixel model output.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Point {
    /// Horizontal position, in source-frame pixels from the left edge.
    #[cfg_attr(feature = "specta", specta(type = FiniteNumber))]
    pub x: f32,
    /// Vertical position, in source-frame pixels from the top edge.
    #[cfg_attr(feature = "specta", specta(type = FiniteNumber))]
    pub y: f32,
}

/// The four corners of a card as seen in a frame, in source-frame pixels.
///
/// Corners are ordered top-left, top-right, bottom-right, bottom-left **of the card read
/// upright**, not of the frame: for a card lying upside down, the first corner is the
/// lowest-right one on screen. Serializes as an array of four [`Point`]s.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(transparent)]
pub struct Quad(pub [Point; 4]);

/// The TypeScript type of a float field that is always finite: exports as `number`.
///
/// specta exports every `f32`/`f64` as `number | null`, because serde_json writes `NaN`
/// and infinities as `null`. The contract's coordinates, scores and times are always
/// finite, so their fields use `#[specta(type = domain::FiniteNumber)]` instead.
#[cfg(feature = "specta")]
#[derive(Debug)]
pub struct FiniteNumber;

#[cfg(feature = "specta")]
impl specta::Type for FiniteNumber {
    fn definition(_: &mut specta::Types) -> specta::datatype::DataType {
        use specta::datatype::{DataType, Enum, Field, Primitive, Variant};

        // The same shape specta gives `serde_json::Number`, minus its integer variants:
        // an untagged number marked finite, which specta-typescript renders as `number`.
        let mut number = Enum::default();
        number.variants.push((
            "f64".into(),
            Variant::unnamed()
                .field(Field::new(DataType::Primitive(Primitive::f64)))
                .build(),
        ));
        number
            .attributes
            .insert(specta::markers::SERDE_CONTAINER_UNTAGGED, true);
        number
            .attributes
            .insert(specta::markers::FINITE_NUMBER_MARKER, true);
        DataType::Enum(number)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn passcode_parses_text_padded_with_leading_zeros() {
        assert_eq!("083764719".parse(), Ok(Passcode(83_764_719)));
        assert_eq!("00014558127".parse(), Ok(Passcode(14_558_127)));
    }

    #[test]
    fn passcode_parses_unpadded_text_and_ignores_surrounding_whitespace() {
        assert_eq!("55144522".parse(), Ok(Passcode(55_144_522)));
        assert_eq!(" 83764719\r\n".parse(), Ok(Passcode(83_764_719)));
    }

    #[test]
    fn passcode_parses_all_zeros_as_zero() {
        assert_eq!("0000".parse(), Ok(Passcode(0)));
    }

    #[test]
    fn passcode_rejects_empty_text() {
        assert_eq!("".parse::<Passcode>(), Err(ParsePasscodeError::Empty));
        assert_eq!("  \t".parse::<Passcode>(), Err(ParsePasscodeError::Empty));
    }

    #[test]
    fn passcode_rejects_signs_letters_and_inner_spaces() {
        assert_eq!(
            "+83764719".parse::<Passcode>(),
            Err(ParsePasscodeError::InvalidCharacter { found: '+' })
        );
        assert_eq!(
            "#main".parse::<Passcode>(),
            Err(ParsePasscodeError::InvalidCharacter { found: '#' })
        );
        assert_eq!(
            "8376 4719".parse::<Passcode>(),
            Err(ParsePasscodeError::InvalidCharacter { found: ' ' })
        );
    }

    #[test]
    fn passcode_rejects_values_beyond_32_bits_even_after_zero_padding() {
        assert_eq!(
            "4294967296".parse::<Passcode>(),
            Err(ParsePasscodeError::TooLarge)
        );
        assert_eq!(
            "0004294967295".parse(),
            Ok(Passcode(u32::MAX)),
            "padding must not count towards the size limit"
        );
    }

    #[test]
    fn passcode_displays_without_padding() {
        assert_eq!(Passcode(5_318_639).to_string(), "5318639");
    }

    #[test]
    fn passcode_and_camera_id_serialize_as_bare_values() {
        assert_eq!(
            serde_json::to_value(Passcode(83_764_719)).unwrap(),
            json!(83_764_719)
        );
        assert_eq!(
            serde_json::to_value(CameraId("cam-1".into())).unwrap(),
            json!("cam-1")
        );
        assert_eq!(
            serde_json::from_value::<Passcode>(json!(14_558_127)).unwrap(),
            Passcode(14_558_127)
        );
    }

    #[test]
    fn player_serializes_as_camel_case_names() {
        assert_eq!(serde_json::to_value(Player::A).unwrap(), json!("a"));
        assert_eq!(serde_json::to_value(Player::B).unwrap(), json!("b"));
        assert_eq!(
            serde_json::from_value::<Player>(json!("b")).unwrap(),
            Player::B
        );
    }

    #[test]
    fn quad_round_trips_as_four_points_with_x_and_y_keys() {
        let quad = Quad([
            Point { x: 10.0, y: 20.0 },
            Point { x: 110.0, y: 20.0 },
            Point { x: 110.0, y: 165.5 },
            Point { x: 10.0, y: 165.5 },
        ]);
        let expected = json!([
            { "x": 10.0, "y": 20.0 },
            { "x": 110.0, "y": 20.0 },
            { "x": 110.0, "y": 165.5 },
            { "x": 10.0, "y": 165.5 },
        ]);

        assert_eq!(serde_json::to_value(quad).unwrap(), expected);
        assert_eq!(serde_json::from_value::<Quad>(expected).unwrap(), quad);
    }
}
