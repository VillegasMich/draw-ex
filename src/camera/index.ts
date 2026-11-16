// Owned by D1 (docs/ROADMAP.md): camera enumeration, the setup picker and preview.
export { default as CameraView } from './CameraView.svelte';
export { default as SetupModal } from './SetupModal.svelte';
export { listCameras, onCamerasChanged, requestCameraAccess, type CameraDevice } from './devices';
export { describeCameraFailure, type CameraError } from './errors';
export { browserStorage, loadSetup, saveSetup } from './persistence';
export { describeSelection, toCameraSetup, type SetupSelection, type VideoSize } from './setup';
