import { svelte } from '@sveltejs/vite-plugin-svelte';
import { svelteTesting } from '@testing-library/svelte/vite';
import { defineConfig } from 'vitest/config';
import { resolveDevPort } from './scripts/dev-port.js';

export default defineConfig({
  plugins: [svelte(), svelteTesting()],
  // Keep Rust errors from `tauri dev` visible.
  clearScreen: false,
  server: {
    port: resolveDevPort(process.env.DRAW_EX_DEV_PORT),
    // Tauri waits for this exact port, so fail instead of picking another one.
    strictPort: true,
    host: 'localhost',
    watch: {
      ignored: ['**/src-tauri/**'],
    },
  },
  test: {
    environment: 'jsdom',
    include: ['src/**/*.test.ts'],
  },
});
