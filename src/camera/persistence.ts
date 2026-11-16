// The last camera setup lives in the webview's `localStorage` (ROADMAP D1): it is a
// webview concern, so the IPC contract does not carry it. Storage can be missing or throw
// (private mode, blocked site data, quota), so every access falls back to "no setup".
import type { CameraId, Player } from '../lib/bindings';
import { setupCameras, type SetupSelection } from './setup';

const STORAGE_KEY = 'draw-ex.camera-setup';

/** The webview's `localStorage`, or `null` where reading it throws. */
export function browserStorage(): Storage | null {
  try {
    return globalThis.localStorage ?? null;
  } catch {
    return null;
  }
}

/** Remembers `selection` as the last camera setup. Does nothing if storage fails. */
export function saveSetup(storage: Storage | null, selection: SetupSelection): void {
  try {
    storage?.setItem(STORAGE_KEY, JSON.stringify(selection));
  } catch {
    // Remembering the setup is a convenience; the operator can pick it again.
  }
}

/**
 * The last camera setup, or `null` when there is none, storage fails, the saved value is
 * malformed, or one of its cameras is not in `cameras` (the device IDs present now).
 */
export function loadSetup(
  storage: Storage | null,
  cameras: readonly CameraId[],
): SetupSelection | null {
  let saved: string | null;
  try {
    saved = storage?.getItem(STORAGE_KEY) ?? null;
  } catch {
    return null;
  }
  const selection = saved === null ? null : parseSelection(saved);
  if (!selection) {
    return null;
  }
  return setupCameras(selection).every((camera) => cameras.includes(camera)) ? selection : null;
}

function parseSelection(saved: string): SetupSelection | null {
  let value: unknown;
  try {
    value = JSON.parse(saved);
  } catch {
    return null;
  }
  if (typeof value !== 'object' || value === null) {
    return null;
  }
  const record = value as Record<string, unknown>;
  switch (record.kind) {
    case 'oneSide':
      return isCameraId(record.camera) && isPlayer(record.player)
        ? { kind: 'oneSide', camera: record.camera, player: record.player }
        : null;
    case 'splitLine':
      return isCameraId(record.camera) ? { kind: 'splitLine', camera: record.camera } : null;
    case 'twoCameras':
      return isCameraId(record.a) && isCameraId(record.b)
        ? { kind: 'twoCameras', a: record.a, b: record.b }
        : null;
    default:
      return null;
  }
}

function isCameraId(value: unknown): value is CameraId {
  return typeof value === 'string' && value !== '';
}

function isPlayer(value: unknown): value is Player {
  return value === 'a' || value === 'b';
}
