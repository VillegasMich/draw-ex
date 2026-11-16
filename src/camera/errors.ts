/** Why a camera is not showing live video. */
export type CameraErrorKind =
  'permissionDenied' | 'inUse' | 'notFound' | 'overConstrained' | 'disconnected' | 'unknown';

/** A camera's error state: what went wrong, and the message the operator sees. */
export interface CameraError {
  kind: CameraErrorKind;
  message: string;
}

/**
 * Something that stopped a camera: `getUserMedia` rejected with `error`, or the live
 * video track fired `ended` (camera unplugged, or access revoked while running).
 */
export type CameraFailure = { kind: 'openFailed'; error: unknown } | { kind: 'trackEnded' };

const MESSAGES: Record<Exclude<CameraErrorKind, 'unknown'>, string> = {
  permissionDenied:
    'Camera access was denied. Allow camera access for draw-ex in your system privacy settings, then retry.',
  inUse: 'The camera is in use by another app. Close that app, then retry.',
  notFound:
    'Camera not found. Check that it is plugged in, then retry or pick another camera in Setup.',
  overConstrained: 'The camera does not support the requested video settings. Try another camera.',
  disconnected: 'Camera disconnected.',
};

/** Maps a camera failure to the error state shown in place of the video. */
export function describeCameraFailure(failure: CameraFailure): CameraError {
  const kind = failure.kind === 'openFailed' ? openErrorKind(failure.error) : 'disconnected';
  if (kind === 'unknown') {
    const detail = failure.kind === 'openFailed' ? errorDetail(failure.error) : '';
    return {
      kind,
      message: detail
        ? `The camera could not be opened (${detail}).`
        : 'The camera could not be opened.',
    };
  }
  return { kind, message: MESSAGES[kind] };
}

function openErrorKind(error: unknown): CameraErrorKind {
  switch (errorName(error)) {
    case 'NotAllowedError':
    case 'SecurityError':
      return 'permissionDenied';
    case 'NotReadableError':
      return 'inUse';
    case 'NotFoundError':
      return 'notFound';
    case 'OverconstrainedError':
      // An `exact` deviceId that matches no camera fails as over-constrained on deviceId.
      return constraintOf(error) === 'deviceId' ? 'notFound' : 'overConstrained';
    default:
      return 'unknown';
  }
}

function errorName(error: unknown): string | undefined {
  return error instanceof Error || error instanceof DOMException ? error.name : undefined;
}

function errorDetail(error: unknown): string {
  if (error instanceof Error || error instanceof DOMException) {
    return error.message || error.name;
  }
  return typeof error === 'string' ? error : '';
}

function constraintOf(error: unknown): unknown {
  return typeof error === 'object' && error !== null && 'constraint' in error
    ? error.constraint
    : undefined;
}
