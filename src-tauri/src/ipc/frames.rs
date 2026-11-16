//! `submit_frame`, the raw-binary command (ADR-0003).
//!
//! The body is the frame's raw RGBA8 bytes; the camera, size and capture time travel in
//! headers. tauri-specta cannot type a raw command, so the frontend calls it through the
//! hand-written wrapper in `src/lib/frames.ts`.

use domain::CameraId;
use draw_core::FrameRef;
use tauri::State;
use tauri::http::HeaderMap;
use tauri::ipc::{InvokeBody, Request};

use super::types::{DetectionDto, IpcError, IpcErrorKind};
use crate::pipeline::{Pipeline, PipelineError};

/// The command name, as the frontend invokes it.
pub const SUBMIT_FRAME: &str = "submit_frame";
/// Header: the camera the frame comes from, as a [`CameraId`] string.
pub const CAMERA_ID_HEADER: &str = "draw-ex-camera-id";
/// Header: the frame width, in pixels, as a decimal `u32`.
pub const FRAME_WIDTH_HEADER: &str = "draw-ex-frame-width";
/// Header: the frame height, in pixels, as a decimal `u32`.
pub const FRAME_HEIGHT_HEADER: &str = "draw-ex-frame-height";
/// Header: when the frame was captured, in milliseconds from the frontend's
/// `performance.now()`, as a finite decimal number.
pub const FRAME_TIME_MS_HEADER: &str = "draw-ex-frame-time-ms";

/// What the `submit_frame` headers say about a frame.
#[derive(Debug, Clone, PartialEq)]
pub struct FrameHeaders {
    /// The camera the frame comes from.
    pub camera: CameraId,
    /// Frame width, in pixels.
    pub width: u32,
    /// Frame height, in pixels.
    pub height: u32,
    /// Capture time, in milliseconds from `performance.now()`.
    pub time_ms: f64,
}

fn invalid(message: String) -> IpcError {
    IpcError::new(IpcErrorKind::InvalidArgument, message)
}

fn header<'h>(headers: &'h HeaderMap, name: &str) -> Result<&'h str, IpcError> {
    let value = headers
        .get(name)
        .ok_or_else(|| invalid(format!("missing header `{name}`")))?;
    value
        .to_str()
        .map_err(|_| invalid(format!("header `{name}` is not visible ASCII")))
}

fn pixels(headers: &HeaderMap, name: &str) -> Result<u32, IpcError> {
    let value = header(headers, name)?;
    value
        .parse()
        .map_err(|_| invalid(format!("header `{name}` is not a pixel count: {value:?}")))
}

impl FrameHeaders {
    /// Reads and validates the `draw-ex-*` frame headers.
    ///
    /// # Errors
    ///
    /// [`IpcErrorKind::InvalidArgument`] if a header is missing, not ASCII, or not a
    /// number of the right kind (a `u32` pixel count, or a finite time).
    pub fn parse(headers: &HeaderMap) -> Result<Self, IpcError> {
        let camera = CameraId(header(headers, CAMERA_ID_HEADER)?.to_owned());
        let width = pixels(headers, FRAME_WIDTH_HEADER)?;
        let height = pixels(headers, FRAME_HEIGHT_HEADER)?;
        let time = header(headers, FRAME_TIME_MS_HEADER)?;
        let time_ms = time
            .parse::<f64>()
            .ok()
            .filter(|t| t.is_finite())
            .ok_or_else(|| {
                invalid(format!(
                    "header `{FRAME_TIME_MS_HEADER}` is not a time in milliseconds: {time:?}"
                ))
            })?;
        Ok(Self {
            camera,
            width,
            height,
            time_ms,
        })
    }
}

/// Runs recognition on one frame and returns its detections, in that frame's pixels.
///
/// The frame is copied into a job for the [`Pipeline`]'s worker thread, and the
/// response waits for that job without blocking the async runtime. The pipeline runs a
/// stand-in recognizer with canned detections until real recognition lands (A3/I1).
///
/// # Errors
///
/// [`IpcErrorKind::InvalidArgument`] if a header is missing or malformed, the body is
/// not raw bytes, or its length is not `width * height * 4`.
#[tauri::command]
pub async fn submit_frame(
    request: Request<'_>,
    pipeline: State<'_, Pipeline>,
) -> Result<Vec<DetectionDto>, IpcError> {
    let headers = FrameHeaders::parse(request.headers())?;
    let InvokeBody::Raw(rgba) = request.body() else {
        return Err(invalid(
            "the body must be the frame's raw RGBA8 bytes, not JSON".into(),
        ));
    };
    let frame =
        FrameRef::new(headers.width, headers.height, rgba).map_err(|e| invalid(e.to_string()))?;

    // Recognition errors can name model files, so their text stays on the Rust side.
    let detections = pipeline.recognize(frame).await.map_err(|error| {
        let message = match error {
            PipelineError::Stopped => "recognition has stopped",
            PipelineError::Recognize(_) => "recognition failed on this frame",
        };
        IpcError::new(IpcErrorKind::Internal, message)
    })?;
    Ok(detections.into_iter().map(DetectionDto::from).collect())
}
