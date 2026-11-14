//! The one place where draw-core types meet draw-ex types (ADR-0002).
//!
//! draw-core's types have no serde or specta, so the IPC mirrors them. Both define a
//! `Passcode` and a `Quad` with the same meaning and units, and the mapping is
//! field-for-field.

use draw_core::ModelVariant;

use super::types::{CandidateDto, DetectionDto, ModelVariantDto, RecognitionDto};

/// Maps a draw-core passcode to the domain one. Same integer, same meaning.
pub fn passcode(passcode: draw_core::Passcode) -> domain::Passcode {
    domain::Passcode(passcode.0)
}

/// Maps a draw-core quad to the domain one. Same corners, order and pixel space.
pub fn quad(quad: draw_core::Quad) -> domain::Quad {
    domain::Quad(quad.0.map(|p| domain::Point { x: p.x, y: p.y }))
}

impl From<draw_core::Candidate> for CandidateDto {
    fn from(candidate: draw_core::Candidate) -> Self {
        Self {
            passcode: passcode(candidate.passcode),
            score: candidate.score,
        }
    }
}

impl From<draw_core::Recognition> for RecognitionDto {
    fn from(recognition: draw_core::Recognition) -> Self {
        match recognition {
            draw_core::Recognition::Recognized(candidate) => Self::Recognized {
                passcode: passcode(candidate.passcode),
                score: candidate.score,
            },
            draw_core::Recognition::LowConfidence => Self::LowConfidence,
            draw_core::Recognition::NotInDeck => Self::NotInDeck,
            draw_core::Recognition::Unreadable => Self::Unreadable,
        }
    }
}

impl From<draw_core::Detection> for DetectionDto {
    fn from(detection: draw_core::Detection) -> Self {
        Self {
            quad: quad(detection.quad),
            detector_confidence: detection.detector_confidence,
            candidates: detection.candidates.into_iter().map(Into::into).collect(),
            recognition: detection.recognition.into(),
        }
    }
}

impl From<ModelVariant> for ModelVariantDto {
    fn from(variant: ModelVariant) -> Self {
        match variant {
            ModelVariant::Fp32 => Self::Fp32,
            ModelVariant::Fp16 => Self::Fp16,
            ModelVariant::Int8 => Self::Int8,
            ModelVariant::YugiscanInt8 => Self::YugiscanInt8,
        }
    }
}

impl From<ModelVariantDto> for ModelVariant {
    fn from(variant: ModelVariantDto) -> Self {
        match variant {
            ModelVariantDto::Fp32 => Self::Fp32,
            ModelVariantDto::Fp16 => Self::Fp16,
            ModelVariantDto::Int8 => Self::Int8,
            ModelVariantDto::YugiscanInt8 => Self::YugiscanInt8,
        }
    }
}
