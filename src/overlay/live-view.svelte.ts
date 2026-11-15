// What the overlay draws for the camera on screen: its frame size, the camera setup and
// its latest detections. Frame capture (D2) and the camera setup (D1) feed it once wired
// (I1); until then only the dev demo does.
import type { CameraId, CameraSetup, DetectionDto } from '../lib/bindings';
import type { Size } from './geometry';

/** The frame size assumed before a camera reports its own. */
export const ASSUMED_FRAME: Size = { width: 1920, height: 1080 };

class LiveView {
  /** The camera shown, or `null` when none is. */
  camera = $state<CameraId | null>(null);
  /** The camera's frame size, in pixels. */
  frame = $state<Size>(ASSUMED_FRAME);
  /** How cameras map to players, or `null` before the operator sets it up. */
  setup = $state<CameraSetup | null>(null);
  /** The camera's latest detections, in frame pixels. */
  detections = $state<DetectionDto[]>([]);

  /** Back to no camera, no setup and no detections. */
  clear(): void {
    this.camera = null;
    this.frame = ASSUMED_FRAME;
    this.setup = null;
    this.detections = [];
  }
}

export const liveView = new LiveView();
