// Single source for the dev server port, shared by vite.config.ts and scripts/tauri.js.

export const DEFAULT_DEV_PORT = 1420;

/**
 * Resolves the dev server port from `DRAW_EX_DEV_PORT`, defaulting to 1420.
 *
 * @param {string | undefined} raw value of `DRAW_EX_DEV_PORT`
 * @returns {number}
 * @throws {Error} when the value is set but is not a port number (1–65535)
 */
export function resolveDevPort(raw) {
  if (raw === undefined || raw === '') return DEFAULT_DEV_PORT;
  const port = Number(raw);
  if (!Number.isInteger(port) || port < 1 || port > 65535) {
    throw new Error(`DRAW_EX_DEV_PORT must be a port number (1-65535), got "${raw}"`);
  }
  return port;
}
