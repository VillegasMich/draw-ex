// Model install state and the running download, shared by the first-run banner and the
// Models tab.
import {
  commands,
  events,
  type ModelStatus,
  type ModelVariantDto,
  type VariantStatus,
} from '../lib/bindings';
import { errorMessage } from './format';

export class ModelsState {
  /** What is installed, or `null` until the first status arrives. */
  status = $state.raw<ModelStatus | null>(null);
  /** The variant being downloaded, if any. */
  downloading = $state<ModelVariantDto | null>(null);
  /** Bytes downloaded so far per file of the running download. */
  #fileBytes = $state<Record<string, number>>({});
  /** Why the last download failed, if it did. */
  error = $state<string | null>(null);

  /** The status of `variant`, once known. */
  variant(variant: ModelVariantDto): VariantStatus | undefined {
    return this.status?.variants.find((entry) => entry.variant === variant);
  }

  /** The status of the variant recognition uses, once known. */
  get current(): VariantStatus | undefined {
    return this.status ? this.variant(this.status.current) : undefined;
  }

  /** How far the running download has got, from 0 to 1. */
  get fraction(): number {
    const total = this.downloading ? (this.variant(this.downloading)?.bytesTotal ?? 0) : 0;
    if (total === 0) {
      return 0;
    }
    const done = Object.values(this.#fileBytes).reduce((sum, bytes) => sum + bytes, 0);
    return Math.min(done / total, 1);
  }

  /**
   * Loads the status and follows `model-download-progress`. Returns a function that
   * stops following. Outside Tauri both fail with a warning and the status stays `null`.
   */
  connect(): () => void {
    let stopped = false;
    let unlisten: (() => void) | undefined;
    void this.refresh();
    events.modelDownloadProgress
      .listen(({ payload }) => {
        if (payload.variant === this.downloading) {
          this.#fileBytes[payload.file] = payload.bytesDone;
        }
      })
      .then((stop) => {
        if (stopped) stop();
        else unlisten = stop;
      })
      .catch((error: unknown) => {
        console.warn('Not listening for model download progress.', error);
      });
    return () => {
      stopped = true;
      unlisten?.();
    };
  }

  async refresh(): Promise<void> {
    try {
      this.status = await commands.modelStatus();
    } catch (error) {
      console.warn('Could not read the model status.', error);
    }
  }

  /** Downloads `variant`'s files, resuming a partial download. One at a time. */
  async download(variant: ModelVariantDto): Promise<void> {
    if (this.downloading) {
      return;
    }
    this.downloading = variant;
    this.#fileBytes = {};
    this.error = null;
    try {
      await commands.downloadModels(variant);
    } catch (error) {
      this.error = errorMessage(error);
    } finally {
      this.downloading = null;
      await this.refresh();
    }
  }
}
