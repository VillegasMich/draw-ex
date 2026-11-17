// Frame capture at the inference rate, with at most one frame in flight per camera
// (D2, ADR-0003). Awaiting each frame's response is the backpressure: a tick that finds
// the camera's previous frame still in flight is skipped, never queued.
import type { CameraId } from '../lib/bindings';
import { submitFrame, type FramePixels } from '../lib/frames';
import { liveView } from '../overlay/live-view.svelte';

/** Grabs the camera's current frame, or `null` while it has none to give. */
export type FrameSource = () => FramePixels | null;

/** The inference rate when the settings give none that makes sense. */
export const DEFAULT_INFERENCE_FPS = 6;

/** Counters for one camera, for debugging and tests. */
export interface CaptureStats {
  /** Frames sent to `submit_frame`. */
  sent: number;
  /** Ticks skipped because the camera's previous frame was still in flight. */
  skipped: number;
  /** Frames sent and not answered yet. */
  inFlight: number;
  /** The most frames that were ever in flight at once; it should never exceed 1. */
  maxInFlight: number;
  /** Frames whose `submit_frame` call failed. */
  failed: number;
  /** How long the last answered frame took, from send to response, in milliseconds. */
  lastRoundTripMs: number | null;
}

/** One `start` … stop span of a camera, shared by every source showing that camera. */
interface Run {
  sources: FrameSource[];
  /** The rate the timer ticks at, in frames per second. */
  fps: number;
  timer: ReturnType<typeof setInterval>;
}

/**
 * A camera's capture state. It outlives runs, so a camera that is stopped and started
 * again while a frame is still in flight waits for that frame instead of sending a
 * second one.
 */
interface CameraEntry {
  inFlight: number;
  run: Run | null;
}

const NO_STATS: CaptureStats = Object.freeze({
  sent: 0,
  skipped: 0,
  inFlight: 0,
  maxInFlight: 0,
  failed: 0,
  lastRoundTripMs: null,
});

/** `fps` as a tick interval in milliseconds, falling back to the default rate. */
function tickMs(fps: number): number {
  const rate = Number.isFinite(fps) && fps > 0 ? fps : DEFAULT_INFERENCE_FPS;
  return 1000 / rate;
}

/** Sends each camera's frames to `submit_frame` and feeds the detections to `liveView`. */
export class FrameCapture {
  // Bookkeeping that nothing renders; the counters below are the reactive part.
  // eslint-disable-next-line svelte/prefer-svelte-reactivity
  #cameras = new Map<CameraId, CameraEntry>();
  #stats = $state<Record<CameraId, CaptureStats>>({});

  /**
   * Captures `camera` from `source` at `fps` frames per second until the returned
   * function is called. Starting a camera that is already capturing (the same camera
   * shown twice) shares its rate and in-flight frame; the latest source is used. Once
   * nothing captures the camera, its detections are cleared.
   */
  start(camera: CameraId, source: FrameSource, fps: number): () => void {
    const entry = this.#entry(camera);
    if (!entry.run) {
      const run: Run = {
        sources: [],
        fps,
        timer: setInterval(() => this.#tick(camera, entry, run), tickMs(fps)),
      };
      entry.run = run;
    }
    const run = entry.run;
    run.sources.push(source);

    let stopped = false;
    return () => {
      if (stopped) return;
      stopped = true;
      run.sources.splice(run.sources.indexOf(source), 1);
      if (run.sources.length === 0) {
        clearInterval(run.timer);
        if (entry.run === run) entry.run = null;
        liveView.setDetections(camera, []);
      }
    };
  }

  /**
   * Changes the rate of `camera`'s running capture to `fps` frames per second, e.g. when
   * the operator saves a new inference rate. The run carries on: its sources, in-flight
   * frame and detections stay. Does nothing while the camera is not capturing.
   */
  setFps(camera: CameraId, fps: number): void {
    const entry = this.#cameras.get(camera);
    const run = entry?.run;
    if (!entry || !run || run.fps === fps) return;
    clearInterval(run.timer);
    run.fps = fps;
    run.timer = setInterval(() => this.#tick(camera, entry, run), tickMs(fps));
  }

  /** `camera`'s counters; all zero before it is first captured. */
  stats(camera: CameraId): CaptureStats {
    return this.#stats[camera] ?? NO_STATS;
  }

  #entry(camera: CameraId): CameraEntry {
    let entry = this.#cameras.get(camera);
    if (!entry) {
      entry = { inFlight: 0, run: null };
      this.#cameras.set(camera, entry);
      this.#stats[camera] = { ...NO_STATS };
    }
    return entry;
  }

  #tick(camera: CameraId, entry: CameraEntry, run: Run): void {
    const stats = this.#stats[camera];
    if (entry.inFlight > 0) {
      stats.skipped += 1;
      return;
    }
    const frame = run.sources.at(-1)?.();
    if (!frame) return;

    const sentAt = performance.now();
    entry.inFlight += 1;
    stats.sent += 1;
    stats.inFlight = entry.inFlight;
    stats.maxInFlight = Math.max(stats.maxInFlight, entry.inFlight);
    submitFrame(camera, frame, sentAt)
      .then(
        (detections) => {
          // A response that arrives after its run stopped is stale: ignore it.
          if (entry.run !== run) return;
          stats.lastRoundTripMs = performance.now() - sentAt;
          liveView.setDetections(camera, detections);
        },
        () => {
          if (entry.run === run) stats.failed += 1;
        },
      )
      .finally(() => {
        entry.inFlight -= 1;
        stats.inFlight = entry.inFlight;
      });
  }
}

/** The app's frame capture, sending through `submitFrame` into `liveView`. */
export const frameCapture = new FrameCapture();
