import { describe, expect, it } from 'vitest';
import { elementToFrame, frameToElement, letterbox } from './geometry';

const FRAME = { width: 1920, height: 1080 };

describe('letterbox mapping', () => {
  it('maps frame pixels into a wider element with bars left and right', () => {
    const fit = letterbox(FRAME, { width: 1200, height: 540 });

    expect(frameToElement({ x: 0, y: 0 }, fit)).toEqual({ x: 120, y: 0 });
    expect(frameToElement({ x: 1920, y: 1080 }, fit)).toEqual({ x: 1080, y: 540 });
  });

  it('maps frame pixels into a taller element with bars above and below', () => {
    const fit = letterbox(FRAME, { width: 960, height: 960 });

    expect(frameToElement({ x: 0, y: 0 }, fit)).toEqual({ x: 0, y: 210 });
    expect(frameToElement({ x: 960, y: 540 }, fit)).toEqual({ x: 480, y: 480 });
  });

  it('maps frame pixels into an element of the same aspect ratio without bars', () => {
    const fit = letterbox(FRAME, { width: 960, height: 540 });

    expect(frameToElement({ x: 1920, y: 1080 }, fit)).toEqual({ x: 960, y: 540 });
    expect(frameToElement({ x: 480, y: 270 }, fit)).toEqual({ x: 240, y: 135 });
  });

  it('maps element pixels back to frame pixels', () => {
    expect(
      elementToFrame({ x: 120, y: 0 }, letterbox(FRAME, { width: 1200, height: 540 })),
    ).toEqual({ x: 0, y: 0 });
    expect(
      elementToFrame({ x: 480, y: 480 }, letterbox(FRAME, { width: 960, height: 960 })),
    ).toEqual({ x: 960, y: 540 });
    expect(
      elementToFrame({ x: 960, y: 540 }, letterbox(FRAME, { width: 960, height: 540 })),
    ).toEqual({ x: 1920, y: 1080 });
  });

  it('round-trips a frame point at any element size', () => {
    const point = { x: 1234.5, y: 678.25 };
    for (const element of [
      { width: 1200, height: 540 },
      { width: 777, height: 1313 },
      { width: 640, height: 360 },
    ]) {
      const fit = letterbox(FRAME, element);
      const back = elementToFrame(frameToElement(point, fit), fit);
      expect(back.x).toBeCloseTo(point.x, 6);
      expect(back.y).toBeCloseTo(point.y, 6);
    }
  });
});
