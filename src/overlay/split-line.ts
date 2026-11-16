// Which player a point in a camera's frame belongs to, under the camera setup. The rule
// follows the contract's `SplitLine`: Player A is on the left of `from → to` as seen on
// screen (y pointing down).
import type { CameraId, CameraSetup, Player, Point, Quad, SplitLine } from '../lib/bindings';
import { elementToFrame, type Letterbox, type Size } from './geometry';

/**
 * The contract's default split line for a `frame`-sized camera: horizontal through the
 * middle, drawn right to left, so Player A is at the bottom.
 */
export function defaultSplitLine(frame: Size): SplitLine {
  const y = frame.height / 2;
  return { from: { x: frame.width, y }, to: { x: 0, y } };
}

/** The split line, in frame pixels, for end points dragged to in element pixels. */
export function splitLineFromDrag(from: Point, to: Point, fit: Letterbox): SplitLine {
  return { from: elementToFrame(from, fit), to: elementToFrame(to, fit) };
}

/**
 * The player whose half of the table `point` is on. Both are in the same space (frame
 * pixels). A point exactly on the line counts as Player B's.
 */
export function playerAt(point: Point, line: SplitLine): Player {
  const dx = line.to.x - line.from.x;
  const dy = line.to.y - line.from.y;
  // With y pointing down, a negative cross product means the point is on the left of
  // the direction `from → to` as seen on screen.
  const cross = dx * (point.y - line.from.y) - dy * (point.x - line.from.x);
  return cross < 0 ? 'a' : 'b';
}

/**
 * The player a card seen by `camera` at `quad` (frame pixels) belongs to under `setup`,
 * or `null` when there is no setup or it does not use that camera.
 */
export function playerForQuad(
  setup: CameraSetup | null,
  camera: CameraId,
  quad: Quad,
): Player | null {
  if (!setup) {
    return null;
  }
  switch (setup.kind) {
    case 'oneSide':
      return setup.camera === camera ? setup.player : null;
    case 'twoCameras':
      if (setup.a === camera) return 'a';
      if (setup.b === camera) return 'b';
      return null;
    case 'splitLine':
      return setup.camera === camera ? playerAt(quadCenter(quad), setup.line) : null;
  }
}

/** The average of a quad's four corners. */
export function quadCenter(quad: Quad): Point {
  const x = quad.reduce((sum, corner) => sum + corner.x, 0) / quad.length;
  const y = quad.reduce((sum, corner) => sum + corner.y, 0) / quad.length;
  return { x, y };
}
