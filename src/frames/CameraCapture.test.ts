import { render } from '@testing-library/svelte';
import { flushSync } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { commands } from '../lib/bindings';
import { appSettings } from '../settings';
import CameraCapture from './CameraCapture.svelte';
import { frameCapture } from './capture.svelte';

vi.mock('../lib/bindings', () => ({
  commands: { getSettings: vi.fn(), updateSettings: vi.fn() },
  events: {},
}));

vi.mock('./capture.svelte', async (importOriginal) => {
  const original = await importOriginal<typeof import('./capture.svelte')>();
  return {
    ...original,
    frameCapture: {
      start: vi.fn(() => () => {}),
      setFps: vi.fn(),
      stats: vi.fn(() => ({
        sent: 0,
        skipped: 0,
        inFlight: 0,
        maxInFlight: 0,
        failed: 0,
        lastRoundTripMs: null,
      })),
    },
  };
});

const SETTINGS = {
  language: 'en',
  modelVariant: 'int8',
  inferenceFps: 6,
  selectedDeck: null,
  imageCacheMb: 1024,
} as const;

describe('CameraCapture', () => {
  afterEach(() => vi.clearAllMocks());

  it('applies an inference rate saved in Settings to the running capture', async () => {
    vi.mocked(commands.getSettings).mockResolvedValue({ ...SETTINGS });
    vi.mocked(commands.updateSettings).mockImplementation(async (settings) => settings);
    await appSettings.load();
    render(CameraCapture, { camera: 'cam-1', video: document.createElement('video') });
    expect(frameCapture.start).toHaveBeenCalledWith('cam-1', expect.any(Function), 6);

    await appSettings.update({ inferenceFps: 12 });
    flushSync();

    expect(frameCapture.setFps).toHaveBeenLastCalledWith('cam-1', 12);
    expect(frameCapture.start).toHaveBeenCalledTimes(1);
  });
});
