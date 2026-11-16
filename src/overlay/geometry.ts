// Pure geometry for the overlay: the `object-fit: contain` letterbox between a camera's
// frame pixels and the on-screen element's pixels, and which player a point belongs to.
import type { Point } from '../lib/bindings';

/** A width and height in pixels. */
export interface Size {
  width: number;
  height: number;
}

/**
 * Where a frame sits inside an element under `object-fit: contain`: scaled uniformly to
 * fit, centered, with bars on the two sides that have room left over.
 */
export interface Letterbox {
  /** Element pixels per frame pixel. */
  scale: number;
  /** Left edge of the frame image, in element pixels. */
  offsetX: number;
  /** Top edge of the frame image, in element pixels. */
  offsetY: number;
}

/** The letterbox of a `frame`-sized image shown with `object-fit: contain` in `element`. */
export function letterbox(frame: Size, element: Size): Letterbox {
  const scale = Math.min(element.width / frame.width, element.height / frame.height);
  return {
    scale,
    offsetX: (element.width - frame.width * scale) / 2,
    offsetY: (element.height - frame.height * scale) / 2,
  };
}

/** A point in frame pixels, in element pixels. */
export function frameToElement(point: Point, fit: Letterbox): Point {
  return { x: fit.offsetX + point.x * fit.scale, y: fit.offsetY + point.y * fit.scale };
}

/** A point in element pixels, in frame pixels. The inverse of {@link frameToElement}. */
export function elementToFrame(point: Point, fit: Letterbox): Point {
  return { x: (point.x - fit.offsetX) / fit.scale, y: (point.y - fit.offsetY) / fit.scale };
}
