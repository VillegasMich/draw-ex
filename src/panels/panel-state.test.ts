import { describe, expect, it } from 'vitest';
import type { TrackEvent } from '../lib/bindings';
import { applyTrackEvent, emptyPanels, featureTrack } from './panel-state';

const MONSTER_REBORN = 83764719;
const ASH_BLOSSOM = 14558127;
const POT_OF_GREED = 55144522;

function lost(track: number, player: 'a' | 'b', passcode: number, timeMs: number): TrackEvent {
  return { kind: 'lost', track, player, passcode, timeMs };
}

function confirmed(track: number, player: 'a' | 'b', passcode: number, timeMs: number): TrackEvent {
  return { kind: 'confirmed', track, player, passcode, timeMs };
}

describe('panel state', () => {
  it('a confirmed card goes on its player’s table and becomes the featured card', () => {
    const state = applyTrackEvent(emptyPanels(), confirmed(1, 'a', MONSTER_REBORN, 100));

    expect(state.a.onTable).toEqual([{ track: 1, passcode: MONSTER_REBORN, timeMs: 100 }]);
    expect(state.a.featured).toBe(1);
  });

  it('the newest confirmed card is featured and listed first', () => {
    let state = applyTrackEvent(emptyPanels(), confirmed(1, 'a', MONSTER_REBORN, 100));
    state = applyTrackEvent(state, confirmed(2, 'a', ASH_BLOSSOM, 200));

    expect(state.a.onTable.map((card) => card.track)).toEqual([2, 1]);
    expect(state.a.featured).toBe(2);
  });

  it('a changed track keeps its place and the feature, with the new passcode', () => {
    let state = applyTrackEvent(emptyPanels(), confirmed(1, 'a', MONSTER_REBORN, 100));
    state = applyTrackEvent(state, confirmed(2, 'a', ASH_BLOSSOM, 200));
    state = applyTrackEvent(state, {
      kind: 'changed',
      track: 2,
      player: 'a',
      from: ASH_BLOSSOM,
      to: POT_OF_GREED,
      timeMs: 300,
    });

    expect(state.a.onTable).toEqual([
      { track: 2, passcode: POT_OF_GREED, timeMs: 200 },
      { track: 1, passcode: MONSTER_REBORN, timeMs: 100 },
    ]);
    expect(state.a.featured).toBe(2);
  });

  it('a changed track that is not featured does not take the feature', () => {
    let state = applyTrackEvent(emptyPanels(), confirmed(1, 'a', MONSTER_REBORN, 100));
    state = applyTrackEvent(state, confirmed(2, 'a', ASH_BLOSSOM, 200));
    state = applyTrackEvent(state, {
      kind: 'changed',
      track: 1,
      player: 'a',
      from: MONSTER_REBORN,
      to: POT_OF_GREED,
      timeMs: 300,
    });

    expect(state.a.onTable[1].passcode).toBe(POT_OF_GREED);
    expect(state.a.featured).toBe(2);
  });

  it('a lost card leaves the table and goes to "Left the table"', () => {
    let state = applyTrackEvent(emptyPanels(), confirmed(1, 'a', MONSTER_REBORN, 100));
    state = applyTrackEvent(state, confirmed(2, 'a', ASH_BLOSSOM, 200));
    state = applyTrackEvent(state, lost(1, 'a', MONSTER_REBORN, 300));

    expect(state.a.onTable.map((card) => card.track)).toEqual([2]);
    expect(state.a.leftTheTable).toEqual([{ track: 1, passcode: MONSTER_REBORN, timeMs: 300 }]);
    expect(state.a.featured).toBe(2);
  });

  it('"Left the table" keeps the last 5 lost cards, newest first', () => {
    let state = emptyPanels();
    for (let track = 1; track <= 7; track += 1) {
      state = applyTrackEvent(state, confirmed(track, 'b', POT_OF_GREED, track));
      state = applyTrackEvent(state, lost(track, 'b', POT_OF_GREED, 100 + track));
    }

    expect(state.b.leftTheTable.map((card) => card.track)).toEqual([7, 6, 5, 4, 3]);
  });

  it('when the featured card is lost, the newest remaining card is featured', () => {
    let state = applyTrackEvent(emptyPanels(), confirmed(1, 'a', MONSTER_REBORN, 100));
    state = applyTrackEvent(state, confirmed(2, 'a', ASH_BLOSSOM, 200));
    state = applyTrackEvent(state, confirmed(3, 'a', POT_OF_GREED, 300));
    state = applyTrackEvent(state, lost(3, 'a', POT_OF_GREED, 400));

    expect(state.a.featured).toBe(2);
  });

  it('when the last card is lost, nothing is featured', () => {
    let state = applyTrackEvent(emptyPanels(), confirmed(1, 'a', MONSTER_REBORN, 100));
    state = applyTrackEvent(state, lost(1, 'a', MONSTER_REBORN, 200));

    expect(state.a.onTable).toEqual([]);
    expect(state.a.featured).toBeNull();
  });

  it('featuring a card from the list makes it the featured card', () => {
    let state = applyTrackEvent(emptyPanels(), confirmed(1, 'a', MONSTER_REBORN, 100));
    state = applyTrackEvent(state, confirmed(2, 'a', ASH_BLOSSOM, 200));
    state = featureTrack(state, 'a', 1);

    expect(state.a.featured).toBe(1);
    expect(state.a.onTable.map((card) => card.track)).toEqual([2, 1]);
  });

  it('featuring a track that is not on the player’s table changes nothing', () => {
    const state = applyTrackEvent(emptyPanels(), confirmed(1, 'a', MONSTER_REBORN, 100));

    expect(featureTrack(state, 'a', 9)).toEqual(state);
    expect(featureTrack(state, 'b', 1)).toEqual(state);
  });

  it('events for Player A never touch Player B’s panel', () => {
    let state = applyTrackEvent(emptyPanels(), confirmed(1, 'b', ASH_BLOSSOM, 50));
    const playerB = state.b;
    state = applyTrackEvent(state, confirmed(2, 'a', MONSTER_REBORN, 100));
    state = applyTrackEvent(state, {
      kind: 'changed',
      track: 2,
      player: 'a',
      from: MONSTER_REBORN,
      to: POT_OF_GREED,
      timeMs: 150,
    });
    state = applyTrackEvent(state, lost(2, 'a', POT_OF_GREED, 200));
    state = featureTrack(state, 'a', 1);

    expect(state.b).toBe(playerB);
    expect(state.a.leftTheTable.map((card) => card.track)).toEqual([2]);
  });
});
