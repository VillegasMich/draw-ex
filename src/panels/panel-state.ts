// What each player's panel shows, as a pure reducer over the tracker's `TrackEvent`s.
// Only confirmed cards reach this state (CLAUDE.md invariants); raw detections never do.
import type { Passcode, Player, TrackEvent, TrackId } from '../lib/bindings';

/** A confirmed card on the table: one track and the card it is confirmed as. */
export interface TableCard {
  track: TrackId;
  passcode: Passcode;
  /**
   * Milliseconds from `performance.now()`: when the track was confirmed for a card on
   * the table, when it was lost for a card in "Left the table".
   */
  timeMs: number;
}

/** One player's panel. */
export interface PlayerPanelState {
  /** The player's confirmed cards on the table, newest first. */
  onTable: TableCard[];
  /** The track of the featured card, or `null` when the player has none on the table. */
  featured: TrackId | null;
  /** Cards whose track was lost, newest first, at most {@link LEFT_THE_TABLE_LIMIT}. */
  leftTheTable: TableCard[];
}

export type PanelState = Record<Player, PlayerPanelState>;

/** How many lost cards the "Left the table" section keeps. */
export const LEFT_THE_TABLE_LIMIT = 5;

export function emptyPanels(): PanelState {
  return { a: emptyPlayer(), b: emptyPlayer() };
}

function emptyPlayer(): PlayerPanelState {
  return { onTable: [], featured: null, leftTheTable: [] };
}

/** The panel state after `event`. Events for one player never touch the other. */
export function applyTrackEvent(state: PanelState, event: TrackEvent): PanelState {
  return { ...state, [event.player]: applyToPlayer(state[event.player], event) };
}

function applyToPlayer(panel: PlayerPanelState, event: TrackEvent): PlayerPanelState {
  switch (event.kind) {
    case 'confirmed': {
      const card = { track: event.track, passcode: event.passcode, timeMs: event.timeMs };
      const others = panel.onTable.filter((existing) => existing.track !== event.track);
      return { ...panel, onTable: [card, ...others], featured: event.track };
    }
    case 'changed':
      return {
        ...panel,
        onTable: panel.onTable.map((card) =>
          card.track === event.track ? { ...card, passcode: event.to } : card,
        ),
      };
    case 'lost': {
      const onTable = panel.onTable.filter((card) => card.track !== event.track);
      const left = { track: event.track, passcode: event.passcode, timeMs: event.timeMs };
      const featured =
        panel.featured === event.track ? (onTable[0]?.track ?? null) : panel.featured;
      return {
        onTable,
        featured,
        leftTheTable: [
          left,
          ...panel.leftTheTable.filter((card) => card.track !== event.track),
        ].slice(0, LEFT_THE_TABLE_LIMIT),
      };
    }
  }
}

/**
 * The panel state after the operator picks `track` from `player`'s list. Only a card on
 * that player's table can be featured; anything else leaves the state unchanged.
 */
export function featureTrack(state: PanelState, player: Player, track: TrackId): PanelState {
  const panel = state[player];
  if (!panel.onTable.some((card) => card.track === track)) {
    return state;
  }
  return { ...state, [player]: { ...panel, featured: track } };
}

/** The player's featured card, or `undefined` when nothing is on their table. */
export function featuredCard(panel: PlayerPanelState): TableCard | undefined {
  return panel.onTable.find((card) => card.track === panel.featured);
}

/** The player's other cards on the table, newest first: the panel's compact list. */
export function listedCards(panel: PlayerPanelState): TableCard[] {
  return panel.onTable.filter((card) => card.track !== panel.featured);
}
