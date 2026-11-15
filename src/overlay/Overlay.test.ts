import { fireEvent, render, screen } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { CameraSetup, DetectionDto, Quad } from '../lib/bindings';
import { commands } from '../lib/bindings';
import Overlay from './Overlay.svelte';

vi.mock('../lib/bindings', () => ({
  commands: { getCard: vi.fn(), getSettings: vi.fn() },
}));

function quad(left: number, top: number): Quad {
  return [
    { x: left, y: top },
    { x: left + 200, y: top },
    { x: left + 200, y: top + 290 },
    { x: left, y: top + 290 },
  ];
}

const DETECTIONS: DetectionDto[] = [
  {
    quad: quad(300, 650),
    detectorConfidence: 0.9,
    candidates: [{ passcode: 83764719, score: 0.93 }],
    recognition: { kind: 'recognized', passcode: 83764719, score: 0.93 },
  },
  {
    quad: quad(900, 100),
    detectorConfidence: 0.6,
    candidates: [],
    recognition: { kind: 'unreadable' },
  },
];

const SETUP: CameraSetup = {
  kind: 'splitLine',
  camera: 'cam-1',
  line: { from: { x: 1920, y: 540 }, to: { x: 0, y: 540 } },
};

// jsdom has no ResizeObserver, which `bind:clientWidth` uses. Sizes stay 0 here.
vi.stubGlobal(
  'ResizeObserver',
  class {
    observe() {}
    unobserve() {}
    disconnect() {}
  },
);

beforeEach(() => {
  vi.mocked(commands.getCard).mockResolvedValue({
    passcode: 83764719,
    name: 'Monster Reborn',
    kind: 'Spell Card',
    description: 'Target 1 monster in either GY; Special Summon it.',
    attribute: null,
    race: 'Normal',
    level: null,
    linkRating: null,
    atk: null,
    def: null,
  });
});

function renderOverlay() {
  return render(Overlay, {
    frame: { width: 1920, height: 1080 },
    camera: 'cam-1',
    setup: SETUP,
    detections: DETECTIONS,
  });
}

describe('Overlay', () => {
  it('outlines every detection: player color when recognized, neutral otherwise', async () => {
    const { container } = renderOverlay();

    const outlines = container.querySelectorAll('polygon');
    expect(outlines).toHaveLength(2);
    expect(outlines[0].classList.contains('player-a')).toBe(true);
    expect(outlines[1].classList.contains('unrecognized')).toBe(true);
    expect(await screen.findByText('Monster Reborn')).toBeTruthy();
  });

  it('the toggle hides and shows the overlay', async () => {
    const { container } = renderOverlay();

    await fireEvent.click(screen.getByRole('button', { name: 'Overlay' }));
    expect(container.querySelectorAll('polygon')).toHaveLength(0);

    await fireEvent.click(screen.getByRole('button', { name: 'Overlay' }));
    expect(container.querySelectorAll('polygon')).toHaveLength(2);
  });
});
