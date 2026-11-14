// Hand-typed wrapper for `submit_frame`, the raw-binary command (ADR-0003) that
// tauri-specta cannot type. The Rust side is `src-tauri/src/ipc/frames.rs`; keep the
// header names in sync with it.
import { invoke } from '@tauri-apps/api/core';
import type { CameraId, DetectionDto } from './bindings';

export const SUBMIT_FRAME_COMMAND = 'submit_frame';

/** Request headers `submit_frame` reads. */
export const FRAME_HEADERS = {
  /** The camera the frame comes from (its `MediaDeviceInfo.deviceId`). */
  cameraId: 'draw-ex-camera-id',
  /** Frame width, in pixels. */
  width: 'draw-ex-frame-width',
  /** Frame height, in pixels. */
  height: 'draw-ex-frame-height',
  /** Capture time, in milliseconds from `performance.now()`. */
  timeMs: 'draw-ex-frame-time-ms',
} as const;

/**
 * One frame's pixels: tightly packed RGBA8, row-major, `width * height * 4` bytes.
 * `ImageData` (from `getImageData`) has exactly this shape.
 */
export interface FramePixels {
  width: number;
  height: number;
  data: Uint8Array | Uint8ClampedArray;
}

/**
 * Sends one frame for recognition and resolves to its detections, whose quads are in
 * this frame's pixels. Await it before sending the same camera's next frame: that is
 * the backpressure that keeps frames from queueing.
 *
 * Rejects with an `IpcError` (`kind: "invalidArgument"`) if the size does not match
 * the data length.
 */
export function submitFrame(
  cameraId: CameraId,
  frame: FramePixels,
  timeMs: number,
): Promise<DetectionDto[]> {
  const { width, height, data } = frame;
  // A view over the same memory: the pixels are not copied before the IPC call.
  const body = new Uint8Array(data.buffer, data.byteOffset, data.byteLength);
  return invoke<DetectionDto[]>(SUBMIT_FRAME_COMMAND, body, {
    headers: {
      [FRAME_HEADERS.cameraId]: cameraId,
      [FRAME_HEADERS.width]: String(width),
      [FRAME_HEADERS.height]: String(height),
      [FRAME_HEADERS.timeMs]: String(timeMs),
    },
  });
}
