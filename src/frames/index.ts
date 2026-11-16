// Owned by D2 (docs/ROADMAP.md): frame capture at the inference rate, one frame in flight per camera.
export { default as CameraCapture } from './CameraCapture.svelte';
export {
  DEFAULT_INFERENCE_FPS,
  FrameCapture,
  frameCapture,
  type CaptureStats,
  type FrameSource,
} from './capture.svelte';
export { videoFrameSource } from './video-source';
