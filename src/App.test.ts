import { render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import App from './App.svelte';

describe('App layout', () => {
  it('shows the Player A, Camera and Player B regions in that order', () => {
    render(App);

    const playerA = screen.getByRole('complementary', { name: 'Player A' });
    const camera = screen.getByRole('main', { name: 'Camera' });
    const playerB = screen.getByRole('complementary', { name: 'Player B' });

    expect(playerA.compareDocumentPosition(camera)).toBe(Node.DOCUMENT_POSITION_FOLLOWING);
    expect(camera.compareDocumentPosition(playerB)).toBe(Node.DOCUMENT_POSITION_FOLLOWING);
  });
});
