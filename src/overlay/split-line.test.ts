import { describe, expect, it } from 'vitest';
import type { CameraSetup, Quad } from '../lib/bindings';
import { letterbox } from './geometry';
import { defaultSplitLine, playerAt, playerForQuad, splitLineFromDrag } from './split-line';

const FRAME = { width: 1920, height: 1080 };

function square(cx: number, cy: number): Quad {
  return [
    { x: cx - 10, y: cy - 10 },
    { x: cx + 10, y: cy - 10 },
    { x: cx + 10, y: cy + 10 },
    { x: cx - 10, y: cy + 10 },
  ];
}

describe('split line from drag', () => {
  it('turns element-space end points into a split line in frame pixels', () => {
    const fit = letterbox(FRAME, { width: 1200, height: 540 });

    expect(splitLineFromDrag({ x: 1080, y: 270 }, { x: 120, y: 270 }, fit)).toEqual({
      from: { x: 1920, y: 540 },
      to: { x: 0, y: 540 },
    });
  });

  it('maps through the bars of a taller element', () => {
    const fit = letterbox(FRAME, { width: 960, height: 960 });

    expect(splitLineFromDrag({ x: 0, y: 210 }, { x: 960, y: 750 }, fit)).toEqual({
      from: { x: 0, y: 0 },
      to: { x: 1920, y: 1080 },
    });
  });
});

describe('which side is Player A', () => {
  it('the default line runs right to left through the middle, so Player A is at the bottom', () => {
    const line = defaultSplitLine(FRAME);

    expect(line).toEqual({ from: { x: 1920, y: 540 }, to: { x: 0, y: 540 } });
    expect(playerAt({ x: 960, y: 900 }, line)).toBe('a');
    expect(playerAt({ x: 960, y: 100 }, line)).toBe('b');
  });

  it('a line drawn left to right puts Player A at the top', () => {
    const line = { from: { x: 0, y: 540 }, to: { x: 1920, y: 540 } };

    expect(playerAt({ x: 960, y: 100 }, line)).toBe('a');
    expect(playerAt({ x: 960, y: 900 }, line)).toBe('b');
  });

  it('a line drawn top to bottom puts Player A on the right, as seen on screen', () => {
    const line = { from: { x: 960, y: 0 }, to: { x: 960, y: 1080 } };

    expect(playerAt({ x: 1500, y: 540 }, line)).toBe('a');
    expect(playerAt({ x: 300, y: 540 }, line)).toBe('b');
  });
});

describe('player of a detection', () => {
  it('one camera on one side: every quad belongs to that player', () => {
    const setup: CameraSetup = { kind: 'oneSide', camera: 'cam-1', player: 'b' };

    expect(playerForQuad(setup, 'cam-1', square(100, 100))).toBe('b');
  });

  it('two cameras: the quad belongs to the camera’s player', () => {
    const setup: CameraSetup = { kind: 'twoCameras', a: 'cam-a', b: 'cam-b' };

    expect(playerForQuad(setup, 'cam-a', square(100, 100))).toBe('a');
    expect(playerForQuad(setup, 'cam-b', square(100, 100))).toBe('b');
  });

  it('split line: the quad belongs to the side its center is on', () => {
    const setup: CameraSetup = {
      kind: 'splitLine',
      camera: 'cam-1',
      line: defaultSplitLine(FRAME),
    };

    expect(playerForQuad(setup, 'cam-1', square(960, 800))).toBe('a');
    expect(playerForQuad(setup, 'cam-1', square(960, 300))).toBe('b');
  });

  it('a camera the setup does not use has no player', () => {
    const setup: CameraSetup = { kind: 'oneSide', camera: 'cam-1', player: 'a' };

    expect(playerForQuad(setup, 'cam-2', square(100, 100))).toBeNull();
    expect(playerForQuad(null, 'cam-1', square(100, 100))).toBeNull();
  });
});
