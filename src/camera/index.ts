// Owned by D1 (docs/ROADMAP.md): camera enumeration, the setup picker and preview.
export { default as CameraView } from './CameraView.svelte';
export { default as SetupModal } from './SetupModal.svelte';
export {
  listCameras,
  onCamerasChanged,
  openCamera,
  requestCameraAccess,
  stopStream,
  type CameraDevice,
} from './devices';
export {
  describeCameraFailure,
  type CameraError,
  type CameraErrorKind,
  type CameraFailure,
} from './errors';
export { browserStorage, loadSetup, saveSetup } from './persistence';
export { setupCameras, toCameraSetup, type SetupSelection, type VideoSize } from './setup';
