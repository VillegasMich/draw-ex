import { describe, expect, it } from 'vitest';
import { toCameraSetup } from './setup';

describe('toCameraSetup', () => {
  it('sends one camera and its player for One side', () => {
    expect(toCameraSetup({ kind: 'oneSide', camera: 'cam-1', player: 'b' }, null)).toEqual({
      kind: 'oneSide',
      camera: 'cam-1',
      player: 'b',
    });
  });

  it('sends the default split line through the middle, right to left, for Both sides', () => {
    expect(
      toCameraSetup({ kind: 'splitLine', camera: 'cam-1' }, { width: 1920, height: 1080 }),
    ).toEqual({
      kind: 'splitLine',
      camera: 'cam-1',
      line: { from: { x: 1920, y: 540 }, to: { x: 0, y: 540 } },
    });
  });

  it('waits for the video size before sending Both sides', () => {
    expect(toCameraSetup({ kind: 'splitLine', camera: 'cam-1' }, null)).toBeNull();
  });

  it('treats a video with no frame yet (0 × 0) as an unknown size', () => {
    expect(
      toCameraSetup({ kind: 'splitLine', camera: 'cam-1' }, { width: 0, height: 0 }),
    ).toBeNull();
  });

  it('sends the camera for each player for Two cameras', () => {
    expect(toCameraSetup({ kind: 'twoCameras', a: 'cam-a', b: 'cam-b' }, null)).toEqual({
      kind: 'twoCameras',
      a: 'cam-a',
      b: 'cam-b',
    });
  });
});
