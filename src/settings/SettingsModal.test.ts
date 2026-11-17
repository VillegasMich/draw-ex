import { fireEvent, render, screen, within } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { commands, events, type ModelDownloadProgress } from '../lib/bindings';
import { ModelsState } from './models.svelte';
import { SettingsState } from './settings.svelte';
import SettingsModal from './SettingsModal.svelte';
import { fakeEvent, installDialog, modelStatus, SETTINGS } from './test-support';

vi.mock('../lib/bindings', () => ({
  commands: {
    getSettings: vi.fn(),
    updateSettings: vi.fn(),
    modelStatus: vi.fn(),
    downloadModels: vi.fn(),
    cacheStats: vi.fn(),
    clearCache: vi.fn(),
    syncCardDb: vi.fn(),
    listDecks: vi.fn(),
    selectDeck: vi.fn(),
  },
  events: { modelDownloadProgress: { listen: vi.fn() } },
}));

function open() {
  const settings = new SettingsState();
  const models = new ModelsState();
  models.connect();
  render(SettingsModal, { open: true, settings, models, onclose: () => {} });
  return { settings, models };
}

describe('SettingsModal', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    installDialog();
    vi.mocked(events.modelDownloadProgress.listen).mockImplementation(
      fakeEvent<ModelDownloadProgress>().listen,
    );
    vi.mocked(commands.getSettings).mockResolvedValue(SETTINGS);
    vi.mocked(commands.updateSettings).mockImplementation(async (settings) => settings);
    vi.mocked(commands.modelStatus).mockResolvedValue(modelStatus('int8', ['fp16']));
    vi.mocked(commands.cacheStats).mockResolvedValue({
      imagesBytes: 450_000,
      imageCount: 3,
      cardDbBytes: 48_000_000,
      cardDbVersion: 'canned',
      modelsBytes: 233_862_810,
    });
    vi.mocked(commands.clearCache).mockResolvedValue(null);
    vi.mocked(commands.syncCardDb).mockResolvedValue(null);
    vi.mocked(commands.listDecks).mockResolvedValue([
      { id: 'canned-deck', name: 'Canned Deck', cardCount: 40 },
    ]);
    vi.mocked(commands.selectDeck).mockResolvedValue(null);
  });

  it('has the General, Models, Cache and Deck tabs, General first', async () => {
    open();

    const tabs = screen.getAllByRole('tab');
    expect(tabs.map((tab) => tab.textContent?.trim())).toEqual([
      'General',
      'Models',
      'Cache',
      'Deck',
    ]);
    expect(tabs[0].getAttribute('aria-selected')).toBe('true');
    expect(screen.getByRole('tabpanel', { name: 'General' })).toBeTruthy();
  });

  it('saves a new inference rate with updateSettings', async () => {
    open();
    const fps = await screen.findByRole('spinbutton', { name: /Inference rate/ });

    await fireEvent.change(fps, { target: { value: '12' } });

    expect(commands.updateSettings).toHaveBeenCalledWith({ ...SETTINGS, inferenceFps: 12 });
  });

  it('saves a new card language with updateSettings', async () => {
    open();
    const language = await screen.findByRole('combobox', { name: /Card language/ });

    await fireEvent.change(language, { target: { value: 'fr' } });

    expect(commands.updateSettings).toHaveBeenCalledWith({ ...SETTINGS, language: 'fr' });
  });

  it('saves quick successive changes in order, each on top of the last', async () => {
    let release = () => {};
    vi.mocked(commands.updateSettings)
      .mockImplementationOnce(
        (settings) => new Promise((resolve) => (release = () => resolve(settings))),
      )
      .mockImplementation(async (settings) => settings);
    open();
    const language = await screen.findByRole('combobox', { name: /Card language/ });
    const fps = screen.getByRole('spinbutton', { name: /Inference rate/ });

    await fireEvent.change(language, { target: { value: 'fr' } });
    await fireEvent.change(fps, { target: { value: '12' } });
    expect(commands.updateSettings).toHaveBeenCalledTimes(1);
    release();

    await vi.waitFor(() => expect(commands.updateSettings).toHaveBeenCalledTimes(2));
    expect(commands.updateSettings).toHaveBeenLastCalledWith({
      ...SETTINGS,
      language: 'fr',
      inferenceFps: 12,
    });
  });

  it('shows why the settings were rejected', async () => {
    vi.mocked(commands.updateSettings).mockRejectedValue({
      kind: 'invalidArgument',
      message: 'The inference rate must be 1 to 30 frames per second, not 40.',
    });
    open();
    const fps = await screen.findByRole('spinbutton', { name: /Inference rate/ });

    await fireEvent.change(fps, { target: { value: '40' } });

    expect((await screen.findByRole('alert')).textContent).toContain('1 to 30');
  });

  it('lists the variants with their sizes and install state, and switches variant', async () => {
    open();
    await fireEvent.click(screen.getByRole('tab', { name: 'Models' }));
    const panel = screen.getByRole('tabpanel', { name: 'Models' });

    const int8 = await within(panel).findByRole('radio', { name: /Integer/ });
    expect(int8).toHaveProperty('checked', true);
    expect(
      within(panel)
        .getByText(/Half precision/)
        .closest('label')?.textContent,
    ).toMatch(/234 MB.*Installed/s);
    expect(within(panel).getByRole('button', { name: 'Download (139 MB)' })).toBeTruthy();
    expect(
      within(panel)
        .getByRole('radio', { name: /Integer/ })
        .closest('label')?.textContent,
    ).toMatch(/Default/);
    // Still selectable, but flagged: its labels file does not match the model (A3).
    expect(
      within(panel)
        .getByRole('radio', { name: /Half precision/ })
        .closest('label')?.textContent,
    ).toMatch(/Not recommended — labels mismatch/);

    await fireEvent.click(within(panel).getByRole('radio', { name: /Half precision/ }));

    expect(commands.updateSettings).toHaveBeenCalledWith({ ...SETTINGS, modelVariant: 'fp16' });
  });

  it('checks the files of an installed variant on demand', async () => {
    vi.mocked(commands.getSettings).mockResolvedValue({ ...SETTINGS, modelVariant: 'fp16' });
    vi.mocked(commands.downloadModels).mockResolvedValue(null);
    open();
    await fireEvent.click(screen.getByRole('tab', { name: 'Models' }));
    const panel = screen.getByRole('tabpanel', { name: 'Models' });

    await fireEvent.click(await within(panel).findByRole('button', { name: 'Check files' }));

    expect(commands.downloadModels).toHaveBeenCalledWith('fp16');
  });

  it('shows the cache sizes and wires the clear and sync buttons', async () => {
    open();
    await fireEvent.click(screen.getByRole('tab', { name: 'Cache' }));
    const panel = screen.getByRole('tabpanel', { name: 'Cache' });

    expect(await within(panel).findByText(/3 images/)).toBeTruthy();
    await fireEvent.click(within(panel).getByRole('button', { name: 'Clear images' }));
    await fireEvent.click(within(panel).getByRole('button', { name: 'Clear card data' }));
    await fireEvent.click(within(panel).getByRole('button', { name: 'Delete models' }));
    await fireEvent.click(within(panel).getByRole('button', { name: 'Sync now' }));

    expect(vi.mocked(commands.clearCache).mock.calls).toEqual([
      ['images'],
      ['cardData'],
      ['models'],
    ]);
    expect(commands.syncCardDb).toHaveBeenCalled();
  });

  it('lists the decks and selects one', async () => {
    open();
    await fireEvent.click(screen.getByRole('tab', { name: 'Deck' }));
    const panel = screen.getByRole('tabpanel', { name: 'Deck' });

    await fireEvent.click(await within(panel).findByRole('radio', { name: /Canned Deck/ }));

    expect(commands.selectDeck).toHaveBeenCalledWith('canned-deck');
  });
});
