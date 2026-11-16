// The live panel state and duel log: a runes store fed by the tracker's `track-event`
// events. The pure rules live in `panel-state.ts`.
import { events, type Player, type TrackEvent, type TrackId } from '../lib/bindings';
import { cardData } from './card-data.svelte';
import { applyTrackEvent, emptyPanels, featureTrack, type PanelState } from './panel-state';

class Panels {
  /** Both players' panels. */
  state = $state<PanelState>(emptyPanels());
  /** The session's track events, newest first. */
  log = $state<TrackEvent[]>([]);
  /**
   * The session's start, in milliseconds from `performance.now()`. Duel-log times are
   * shown relative to it.
   */
  originMs = $state(performance.now());

  /** Applies one track event to the panels and the duel log. */
  apply(event: TrackEvent): void {
    this.state = applyTrackEvent(this.state, event);
    this.log = [event, ...this.log];
  }

  /** Features `track` in `player`'s panel, when the operator picks it from the list. */
  feature(player: Player, track: TrackId): void {
    this.state = featureTrack(this.state, player, track);
  }

  /**
   * Clears the panels and the duel log for a new session that started at `originMs`
   * (from `performance.now()`). Call it when `start_session` runs.
   */
  reset(originMs: number = performance.now()): void {
    this.state = emptyPanels();
    this.log = [];
    this.originMs = originMs;
  }
}

export const panels = new Panels();

/**
 * Starts feeding the panels: reads the card language from the settings and listens for
 * `track-event`. Returns a function that stops listening. Outside Tauri (plain
 * `pnpm dev`, tests) there is no backend: both fail with a warning, and the panels stay
 * as they are.
 */
export function connectPanels(): () => void {
  let stopped = false;
  let unlisten: (() => void) | undefined;

  void cardData.loadLanguage();
  events.trackEvent
    .listen(({ payload }) => panels.apply(payload))
    .then((stop) => {
      if (stopped) stop();
      else unlisten = stop;
    })
    .catch((error: unknown) => {
      console.warn('Not listening for track events.', error);
    });

  return () => {
    stopped = true;
    unlisten?.();
  };
}
