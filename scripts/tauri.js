// Wraps the Tauri CLI so `tauri dev` follows DRAW_EX_DEV_PORT.
//
// Vite reads DRAW_EX_DEV_PORT itself (vite.config.ts). Tauri's devUrl is fixed in
// tauri.conf.json, so for `dev` this wrapper overrides it with `--config`, keeping the
// window and the dev server on the same port. Other subcommands pass through unchanged.

import { run } from '@tauri-apps/cli';
import { resolveDevPort } from './dev-port.js';

const args = process.argv.slice(2);

if (args[0] === 'dev') {
  const port = resolveDevPort(process.env.DRAW_EX_DEV_PORT);
  const config = { build: { devUrl: `http://localhost:${port}` } };
  // Right after `dev`, so it reaches the CLI even when app args follow `--`.
  args.splice(1, 0, '--config', JSON.stringify(config));
}

run(args, 'pnpm tauri').catch((error) => {
  console.error(error);
  process.exit(1);
});
