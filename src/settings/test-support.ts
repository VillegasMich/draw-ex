// Shared fixtures for the settings tests: settings, a model status, and a stand-in for
// the `model-download-progress` event.
import type { EventCallback, UnlistenFn } from '@tauri-apps/api/event';
import type {
  ModelDownloadProgress,
  ModelStatus,
  ModelVariantDto,
  Settings,
} from '../lib/bindings';

export const SETTINGS: Settings = {
  language: 'en',
  modelVariant: 'int8',
  inferenceFps: 6,
  selectedDeck: null,
  imageCacheMb: 1024,
};

const SIZES: Record<ModelVariantDto, number> = {
  fp32: 427_139_361,
  fp16: 233_862_810,
  int8: 138_556_436,
  yugiscanInt8: 137_172_532,
};

/** Every variant, with only `installed` on disk, and `current` in use. */
export function modelStatus(
  current: ModelVariantDto,
  installed: ModelVariantDto[] = [],
): ModelStatus {
  return {
    current,
    variants: (['fp32', 'fp16', 'int8', 'yugiscanInt8'] as const).map((variant) => ({
      variant,
      installed: installed.includes(variant),
      bytesOnDisk: installed.includes(variant) ? SIZES[variant] : 0,
      bytesTotal: SIZES[variant],
    })),
  };
}

/** Collects the listener registered for an event, so a test can emit payloads to it. */
export function fakeEvent<T>() {
  let handler: EventCallback<T> | null = null;
  return {
    listen: async (next: EventCallback<T>): Promise<UnlistenFn> => {
      handler = next;
      return () => {
        handler = null;
      };
    },
    emit(payload: T) {
      handler?.({ event: 'test-event', id: 0, payload });
    },
  };
}

export type FakeProgressEvent = ReturnType<typeof fakeEvent<ModelDownloadProgress>>;

/** jsdom has no `showModal`: make `<dialog>` open and close as a browser would. */
export function installDialog(): void {
  HTMLDialogElement.prototype.showModal = function (this: HTMLDialogElement) {
    this.setAttribute('open', '');
  };
  HTMLDialogElement.prototype.close = function (this: HTMLDialogElement) {
    this.removeAttribute('open');
    this.dispatchEvent(new Event('close'));
  };
}
