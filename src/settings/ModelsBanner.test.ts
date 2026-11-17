import { fireEvent, render, screen } from '@testing-library/svelte';
import { tick } from 'svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { commands, events } from '../lib/bindings';
import ModelsBanner from './ModelsBanner.svelte';
import { ModelsState } from './models.svelte';
import { fakeEvent, modelStatus, type FakeProgressEvent } from './test-support';

vi.mock('../lib/bindings', () => ({
  commands: { modelStatus: vi.fn(), downloadModels: vi.fn() },
  events: { modelDownloadProgress: { listen: vi.fn() } },
}));

async function connected(): Promise<ModelsState> {
  const models = new ModelsState();
  models.connect();
  await vi.waitFor(() => expect(models.status).not.toBeNull());
  return models;
}

describe('ModelsBanner', () => {
  let progress: FakeProgressEvent;

  beforeEach(() => {
    vi.clearAllMocks();
    progress = fakeEvent();
    vi.mocked(events.modelDownloadProgress.listen).mockImplementation(progress.listen);
  });

  it('offers the download with its size when the current variant is not installed', async () => {
    vi.mocked(commands.modelStatus).mockResolvedValue(modelStatus('int8', ['fp16']));
    render(ModelsBanner, { models: await connected() });

    expect(screen.getByText(/Recognition models not installed/)).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Download (139 MB)' })).toBeTruthy();
  });

  it('is hidden when the current variant is installed', async () => {
    vi.mocked(commands.modelStatus).mockResolvedValue(modelStatus('int8', ['int8']));
    render(ModelsBanner, { models: await connected() });

    expect(screen.queryByText(/Recognition models not installed/)).toBeNull();
  });

  it('shows progress from the download events, then hides once installed', async () => {
    vi.mocked(commands.modelStatus).mockResolvedValue(modelStatus('int8'));
    let finish = () => {};
    vi.mocked(commands.downloadModels).mockImplementation(
      () => new Promise((resolve) => (finish = () => resolve(null))),
    );
    const models = await connected();
    render(ModelsBanner, { models });

    await fireEvent.click(screen.getByRole('button', { name: /^Download/ }));
    expect(commands.downloadModels).toHaveBeenCalledWith('int8');
    progress.emit({
      variant: 'int8',
      file: 'onnx/ygo_yolo.onnx',
      bytesDone: 39_016_336,
      bytesTotal: 39_016_336,
    });
    progress.emit({
      variant: 'int8',
      file: 'onnx/vit_int8.onnx',
      bytesDone: 30_261_882,
      bytesTotal: 97_771_550,
    });
    await tick();

    const bar = screen.getByRole('progressbar', { name: 'Model download' });
    expect(Number(bar.getAttribute('value'))).toBeCloseTo(0.5, 2);
    expect(screen.getByText(/50%/)).toBeTruthy();

    vi.mocked(commands.modelStatus).mockResolvedValue(modelStatus('int8', ['int8']));
    finish();

    await vi.waitFor(() => expect(screen.queryByRole('progressbar')).toBeNull());
    expect(screen.queryByText(/Recognition models not installed/)).toBeNull();
  });

  it('shows why a download failed and offers it again', async () => {
    vi.mocked(commands.modelStatus).mockResolvedValue(modelStatus('int8'));
    vi.mocked(commands.downloadModels).mockRejectedValue({
      kind: 'network',
      message: 'onnx/vit_int8.onnx could not be downloaded: the server answered 503',
    });
    render(ModelsBanner, { models: await connected() });

    await fireEvent.click(screen.getByRole('button', { name: /^Download/ }));

    expect(await screen.findByRole('alert')).toHaveProperty(
      'textContent',
      expect.stringContaining('the server answered 503'),
    );
    expect(screen.getByRole('button', { name: /^Download/ })).toBeTruthy();
  });
});
