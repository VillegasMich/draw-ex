import { describe, expect, it } from 'vitest';
import { loadSetup, saveSetup } from './persistence';
import type { SetupSelection } from './setup';

/** A `Storage` backed by a plain map, standing in for `localStorage`. */
function memoryStorage(): Storage {
  const items = new Map<string, string>();
  return {
    get length() {
      return items.size;
    },
    clear: () => items.clear(),
    getItem: (key) => items.get(key) ?? null,
    key: (index) => [...items.keys()][index] ?? null,
    removeItem: (key) => void items.delete(key),
    setItem: (key, value) => void items.set(key, String(value)),
  };
}

describe('setup persistence', () => {
  it('remembers the last setup', () => {
    const storage = memoryStorage();
    const setup: SetupSelection = { kind: 'twoCameras', a: 'cam-a', b: 'cam-b' };

    saveSetup(storage, setup);

    expect(loadSetup(storage, ['cam-a', 'cam-b'])).toEqual(setup);
  });

  it('forgets a setup whose camera is gone', () => {
    const storage = memoryStorage();
    saveSetup(storage, { kind: 'oneSide', camera: 'cam-gone', player: 'a' });

    expect(loadSetup(storage, ['cam-other'])).toBeNull();
  });

  it('forgets a Two cameras setup when either camera is gone', () => {
    const storage = memoryStorage();
    saveSetup(storage, { kind: 'twoCameras', a: 'cam-a', b: 'cam-gone' });

    expect(loadSetup(storage, ['cam-a'])).toBeNull();
  });

  it('has no setup when storage is unavailable', () => {
    expect(loadSetup(null, ['cam-1'])).toBeNull();
    expect(() => saveSetup(null, { kind: 'splitLine', camera: 'cam-1' })).not.toThrow();
  });

  it('has no setup when storage throws, and saving does not throw', () => {
    const storage = memoryStorage();
    storage.getItem = () => {
      throw new DOMException('denied', 'SecurityError');
    };
    storage.setItem = () => {
      throw new DOMException('full', 'QuotaExceededError');
    };

    expect(loadSetup(storage, ['cam-1'])).toBeNull();
    expect(() => saveSetup(storage, { kind: 'splitLine', camera: 'cam-1' })).not.toThrow();
  });

  it.each([
    ['not JSON', '{oops'],
    ['an unknown kind', JSON.stringify({ kind: 'threeCameras', camera: 'cam-1' })],
    ['a missing player', JSON.stringify({ kind: 'oneSide', camera: 'cam-1' })],
    ['an invalid player', JSON.stringify({ kind: 'oneSide', camera: 'cam-1', player: 'c' })],
    ['a non-string camera', JSON.stringify({ kind: 'splitLine', camera: 7 })],
  ])('has no setup when the saved value has %s', (_case, saved) => {
    const storage = memoryStorage();
    storage.setItem('draw-ex.camera-setup', saved);

    expect(loadSetup(storage, ['cam-1'])).toBeNull();
  });
});
