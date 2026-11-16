// Owned by D3 (docs/ROADMAP.md): per-player card panels and the duel-log timeline.
export { default as DuelLog } from './DuelLog.svelte';
export { default as PlayerPanel } from './PlayerPanel.svelte';
// `panels.reset(originMs)` starts a session's panels and duel log (call it with `start_session`).
export { connectPanels, panels } from './panels.svelte';
