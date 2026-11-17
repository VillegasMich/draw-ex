// Names and number formats shown in the settings screens.
import type { IpcError, Language, ModelVariantDto } from '../lib/bindings';

export const LANGUAGE_NAMES: Record<Language, string> = {
  en: 'English',
  fr: 'French',
  de: 'German',
  it: 'Italian',
  pt: 'Portuguese',
};

export const VARIANT_NAMES: Record<ModelVariantDto, { name: string; hint: string }> = {
  fp32: { name: 'Full precision (FP32)', hint: 'Largest and most accurate.' },
  // The pinned FP16 file has 13,820 outputs but its labels list 13,968 cards (A3).
  fp16: {
    name: 'Half precision (FP16)',
    hint: 'Not recommended — labels mismatch: it reports wrong cards.',
  },
  int8: { name: 'Integer (INT8)', hint: 'Default. Smallest and fastest on a CPU.' },
  yugiscanInt8: { name: 'Yugiscan (INT8)', hint: 'An alternative classifier build.' },
};

/** `bytes` in decimal units, as download sizes are usually given: "139 MB", "1.2 GB". */
export function formatBytes(bytes: number): string {
  if (bytes >= 1e9) {
    return `${(bytes / 1e9).toFixed(1)} GB`;
  }
  if (bytes >= 1e6) {
    return `${Math.round(bytes / 1e6)} MB`;
  }
  if (bytes >= 1e3) {
    return `${Math.round(bytes / 1e3)} KB`;
  }
  return `${bytes} B`;
}

/** The operator-facing message of a rejected command. */
export function errorMessage(error: unknown): string {
  if (typeof error === 'object' && error !== null && 'message' in error) {
    return String((error as IpcError).message);
  }
  return String(error);
}
