import type { CameraId, Player } from '../lib/bindings';
import { playerName, type SetupSelection } from '../camera/setup';

/** One camera region in the center column. */
export interface CameraSlot {
  /** Stable per position, so the same camera can fill both slots of Two cameras. */
  key: Player | 'both';
  cameraId: CameraId;
  /** The region's accessible name. */
  label: string;
}

/**
 * The camera regions to show for `selection`, top to bottom. Two cameras are stacked
 * with Player B's on top and Player A's below, like looking across the table.
 */
export function cameraSlots(selection: SetupSelection): CameraSlot[] {
  switch (selection.kind) {
    case 'oneSide':
      return [
        {
          key: selection.player,
          cameraId: selection.camera,
          label: `Camera for ${playerName(selection.player)}`,
        },
      ];
    case 'splitLine':
      return [{ key: 'both', cameraId: selection.camera, label: 'Camera for both players' }];
    case 'twoCameras':
      return [
        { key: 'b', cameraId: selection.b, label: 'Camera for Player B' },
        { key: 'a', cameraId: selection.a, label: 'Camera for Player A' },
      ];
  }
}
