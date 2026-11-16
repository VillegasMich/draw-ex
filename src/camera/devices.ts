// Camera access through the webview's `getUserMedia` (ADR-0003: the webview owns the
// cameras). A camera's `CameraId` is its `MediaDeviceInfo.deviceId`.
import type { CameraId } from '../lib/bindings';

/** A video input the operator can pick. */
export interface CameraDevice {
  id: CameraId;
  /** The device's name, or a numbered fallback before permission is granted. */
  label: string;
}

/** Each camera is requested at 1920×1080 ideal, falling back to what it supports. */
const IDEAL_WIDTH = 1920;
const IDEAL_HEIGHT = 1080;

function mediaDevices(): MediaDevices {
  // `navigator.mediaDevices` is missing outside a secure context.
  if (!navigator.mediaDevices) {
    throw new DOMException('Camera access is not available in this window', 'NotSupportedError');
  }
  return navigator.mediaDevices;
}

/**
 * Asks for camera permission once, so that `listCameras` sees device labels and IDs.
 *
 * Rejects with the `getUserMedia` error when access is refused or no camera works.
 */
export async function requestCameraAccess(): Promise<void> {
  const stream = await mediaDevices().getUserMedia({ video: true });
  stopStream(stream);
}

/** The video inputs present now. */
export async function listCameras(): Promise<CameraDevice[]> {
  const devices = await mediaDevices().enumerateDevices();
  return devices
    .filter((device) => device.kind === 'videoinput' && device.deviceId !== '')
    .map((device, index) => ({
      id: device.deviceId,
      label: device.label || `Camera ${index + 1}`,
    }));
}

/** Calls `listener` whenever a camera is plugged in or removed. Returns the unsubscriber. */
export function onCamerasChanged(listener: () => void): () => void {
  const devices = navigator.mediaDevices;
  if (!devices) {
    return () => {};
  }
  devices.addEventListener('devicechange', listener);
  return () => devices.removeEventListener('devicechange', listener);
}

/**
 * Opens camera `id` at 1920×1080 ideal.
 *
 * Rejects with the `getUserMedia` error; `describeCameraFailure` explains it.
 */
export function openCamera(id: CameraId): Promise<MediaStream> {
  return mediaDevices().getUserMedia({
    audio: false,
    video: {
      deviceId: { exact: id },
      width: { ideal: IDEAL_WIDTH },
      height: { ideal: IDEAL_HEIGHT },
    },
  });
}

/** Releases the camera behind `stream`. */
export function stopStream(stream: MediaStream): void {
  for (const track of stream.getTracks()) {
    track.stop();
  }
}
