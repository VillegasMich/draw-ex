import { fireEvent, render, screen, within } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { Card } from '../lib/bindings';
import { commands } from '../lib/bindings';
import type { PlayerPanelState } from './panel-state';
import PlayerPanel from './PlayerPanel.svelte';

vi.mock('../lib/bindings', () => ({
  commands: { getCard: vi.fn(), getSettings: vi.fn() },
}));

const CARDS: Record<number, Card> = {
  14558127: {
    passcode: 14558127,
    name: 'Ash Blossom & Joyous Spring',
    kind: 'Tuner Monster',
    description: 'You can discard this card; negate that activation.',
    attribute: 'FIRE',
    race: 'Zombie',
    level: 3,
    linkRating: null,
    atk: 0,
    def: 1800,
  },
  83764719: {
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
  },
  55144522: {
    passcode: 55144522,
    name: 'Pot of Greed',
    kind: 'Spell Card',
    description: 'Draw 2 cards.',
    attribute: null,
    race: 'Normal',
    level: null,
    linkRating: null,
    atk: null,
    def: null,
  },
};

const PANEL: PlayerPanelState = {
  onTable: [
    { track: 2, passcode: 14558127, timeMs: 200 },
    { track: 1, passcode: 83764719, timeMs: 100 },
  ],
  featured: 2,
  leftTheTable: [{ track: 3, passcode: 55144522, timeMs: 150 }],
};

beforeEach(() => {
  vi.mocked(commands.getCard).mockImplementation(async (passcode) => CARDS[passcode] ?? null);
});

describe('PlayerPanel', () => {
  it('shows the featured card’s name, type, stats and effect text', async () => {
    render(PlayerPanel, { panel: PANEL, onfeature: () => {} });

    const featured = screen.getByRole('article', { name: 'Featured card' });
    expect(
      await within(featured).findByRole('heading', { name: 'Ash Blossom & Joyous Spring' }),
    ).toBeTruthy();
    expect(within(featured).getByText(/Tuner Monster/)).toBeTruthy();
    expect(within(featured).getByText('Level 3 · ATK 0 / DEF 1800')).toBeTruthy();
    expect(within(featured).getByText(/negate that activation/)).toBeTruthy();
  });

  it('lists the other cards on the table and the cards that left it', async () => {
    render(PlayerPanel, { panel: PANEL, onfeature: () => {} });

    const list = screen.getByRole('list', { name: 'On the table' });
    expect(await within(list).findByText('Monster Reborn')).toBeTruthy();
    expect(within(list).queryByText('Ash Blossom & Joyous Spring')).toBeNull();

    const left = screen.getByRole('list', { name: 'Left the table' });
    expect(await within(left).findByText('Pot of Greed')).toBeTruthy();
  });

  it('clicking a listed card features it', async () => {
    const onfeature = vi.fn();
    render(PlayerPanel, { panel: PANEL, onfeature });

    await fireEvent.click(await screen.findByRole('button', { name: /Monster Reborn/ }));

    expect(onfeature).toHaveBeenCalledWith(1);
  });

  it('shows "No cards yet" for an empty panel', () => {
    render(PlayerPanel, {
      panel: { onTable: [], featured: null, leftTheTable: [] },
      onfeature: () => {},
    });

    expect(screen.getByText('No cards yet')).toBeTruthy();
  });

  it('shows a card-shaped placeholder when a card image fails to load', async () => {
    render(PlayerPanel, { panel: PANEL, onfeature: () => {} });

    const featured = screen.getByRole('article', { name: 'Featured card' });
    const image = within(featured).getByRole('img');
    expect(image.tagName).toBe('IMG');

    await fireEvent.error(image);

    const placeholder = within(featured).getByRole('img');
    expect(placeholder.tagName).not.toBe('IMG');
    expect(placeholder.classList.contains('card-placeholder')).toBe(true);
  });
});
