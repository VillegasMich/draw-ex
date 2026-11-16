import type { CameraId, CameraSetup, Player, SplitLine } from '../lib/bindings';

/** What the operator picked in the Setup modal. */
export type SetupSelection =
  | { kind: 'oneSide'; camera: CameraId; player: Player }
  | { kind: 'splitLine'; camera: CameraId }
  | { kind: 'twoCameras'; a: CameraId; b: CameraId };

/** A video's intrinsic size (`videoWidth` × `videoHeight`), in frame pixels. */
export interface VideoSize {
  width: number;
  height: number;
}

/**
 * Builds the `CameraSetup` sent to `setCameraSetup` from the operator's selection.
 *
 * `videoSize` is the size of the Both sides camera's video. Returns `null` for Both sides
 * until that size is known, because the split line is in that camera's frame pixels.
 */
export function toCameraSetup(
  selection: SetupSelection,
  videoSize: VideoSize | null,
): CameraSetup | null {
  switch (selection.kind) {
    case 'oneSide':
      return { kind: 'oneSide', camera: selection.camera, player: selection.player };
    case 'splitLine':
      // `videoWidth` and `videoHeight` read 0 until the first frame's metadata arrives.
      if (!videoSize || videoSize.width <= 0 || videoSize.height <= 0) {
        return null;
      }
      return { kind: 'splitLine', camera: selection.camera, line: defaultSplitLine(videoSize) };
    case 'twoCameras':
      return { kind: 'twoCameras', a: selection.a, b: selection.b };
  }
}

/**
 * The horizontal line through the middle of the frame, drawn right → left, so Player A
 * (left of `from → to`) gets the bottom half. Mirrors `SplitLine::default_for` in
 * `crates/tracker/src/players.rs`; keep the two in sync.
 */
function defaultSplitLine({ width, height }: VideoSize): SplitLine {
  const middle = height / 2;
  return { from: { x: width, y: middle }, to: { x: 0, y: middle } };
}

/** The cameras a selection uses, without duplicates. */
export function setupCameras(selection: SetupSelection): CameraId[] {
  return selection.kind === 'twoCameras'
    ? [...new Set([selection.a, selection.b])]
    : [selection.camera];
}

/** The interface name of a player: "Player A" or "Player B". */
export function playerName(player: Player): string {
  return player === 'a' ? 'Player A' : 'Player B';
}

/**
 * A one-line description of `selection` for the toolbar, naming cameras with
 * `cameraLabel` (their device labels).
 */
export function describeSelection(
  selection: SetupSelection | null,
  cameraLabel: (id: CameraId) => string,
): string {
  if (!selection) {
    return 'No camera setup';
  }
  switch (selection.kind) {
    case 'oneSide':
      return `One side: ${cameraLabel(selection.camera)} on ${playerName(selection.player)}`;
    case 'splitLine':
      return `Both sides: ${cameraLabel(selection.camera)}`;
    case 'twoCameras':
      return `Two cameras: ${cameraLabel(selection.a)} on Player A, ${cameraLabel(selection.b)} on Player B`;
  }
}
