import { describe, expect, it } from 'vitest';
import { describeCameraFailure } from './errors';

function domError(name: string, message = ''): DOMException {
  return new DOMException(message, name);
}

describe('describeCameraFailure', () => {
  it('maps a denied permission to permissionDenied', () => {
    const error = describeCameraFailure({ kind: 'openFailed', error: domError('NotAllowedError') });

    expect(error.kind).toBe('permissionDenied');
    expect(error.message).toBe(
      'Camera access was denied. Allow camera access for draw-ex in your system privacy settings, then retry.',
    );
  });

  it('maps a camera held by another app to inUse', () => {
    const error = describeCameraFailure({
      kind: 'openFailed',
      error: domError('NotReadableError'),
    });

    expect(error).toEqual({
      kind: 'inUse',
      message: 'The camera is in use by another app. Close that app, then retry.',
    });
  });

  it('maps a missing device to notFound', () => {
    const error = describeCameraFailure({ kind: 'openFailed', error: domError('NotFoundError') });

    expect(error).toEqual({
      kind: 'notFound',
      message:
        'Camera not found. Check that it is plugged in, then retry or pick another camera in Setup.',
    });
  });

  it('maps an exact device ID that no longer matches any camera to notFound', () => {
    // Browsers report a vanished `deviceId: { exact }` as an over-constrained deviceId.
    const gone = Object.assign(domError('OverconstrainedError'), { constraint: 'deviceId' });

    expect(describeCameraFailure({ kind: 'openFailed', error: gone }).kind).toBe('notFound');
  });

  it('maps a constraint the camera cannot meet to overConstrained', () => {
    const error = Object.assign(domError('OverconstrainedError'), { constraint: 'width' });

    expect(describeCameraFailure({ kind: 'openFailed', error })).toEqual({
      kind: 'overConstrained',
      message: 'The camera does not support the requested video settings. Try another camera.',
    });
  });

  it('maps an ended track (camera unplugged or access revoked) to disconnected', () => {
    expect(describeCameraFailure({ kind: 'trackEnded' })).toEqual({
      kind: 'disconnected',
      message: 'Camera disconnected.',
    });
  });

  it('maps any other failure to unknown, keeping its explanation', () => {
    const error = describeCameraFailure({
      kind: 'openFailed',
      error: domError('AbortError', 'Starting videoinput failed'),
    });

    expect(error).toEqual({
      kind: 'unknown',
      message: 'The camera could not be opened (Starting videoinput failed).',
    });
  });
});
