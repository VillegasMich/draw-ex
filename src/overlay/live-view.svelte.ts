// What the overlays draw, per camera: each camera's frame size and latest detections,
// and the camera setup they share. The camera views report frame sizes, App reports the
// setup it sends, and frame capture (D2) feeds detections once wired (I1).
import type { CameraId, CameraSetup, DetectionDto } from '../lib/bindings';
import type { Size } from './geometry';

const NO_DETECTIONS: DetectionDto[] = [];

class LiveView {
  /** How cameras map to players, as last sent with `set_camera_setup`; `null` before. */
  setup = $state<CameraSetup | null>(null);
  /** Each camera's frame size (its video's `videoWidth` × `videoHeight`), in pixels. */
  frames = $state<Record<CameraId, Size>>({});
  /** Each camera's latest detections, in that camera's frame pixels. */
  #detections = $state<Record<CameraId, DetectionDto[]>>({});

  /** Records `camera`'s frame size. Ignores the 0 × 0 a video reports before metadata. */
  setFrame(camera: CameraId, size: Size): void {
    const known = this.frames[camera];
    if (size.width <= 0 || size.height <= 0) return;
    if (known?.width === size.width && known.height === size.height) return;
    this.frames[camera] = size;
  }

  /** `camera`'s latest detections; empty until frame capture reports some. */
  detections(camera: CameraId): DetectionDto[] {
    return this.#detections[camera] ?? NO_DETECTIONS;
  }

  /** Replaces `camera`'s detections with the latest frame's. */
  setDetections(camera: CameraId, detections: DetectionDto[]): void {
    this.#detections[camera] = detections;
  }

  /** Forgets `camera`'s frame size and detections. */
  forget(camera: CameraId): void {
    delete this.frames[camera];
    delete this.#detections[camera];
  }
}

export const liveView = new LiveView();
