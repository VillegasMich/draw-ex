<script lang="ts">
  import { onMount } from 'svelte';
  import { CameraOverlay } from './overlay';
  import { DuelLog, PlayerPanel, connectPanels, panels } from './panels';

  onMount(connectPanels);
</script>

<div class="layout">
  <aside class="panel" aria-labelledby="player-a-title">
    <h2 id="player-a-title" class="title">Player A</h2>
    <PlayerPanel panel={panels.state.a} onfeature={(track) => panels.feature('a', track)} />
  </aside>

  <main class="camera" aria-labelledby="camera-title">
    <h2 id="camera-title" class="title">Camera</h2>
    <div class="placeholder">
      <p class="empty">No camera connected</p>
      <CameraOverlay />
    </div>
    <DuelLog entries={panels.log} originMs={panels.originMs} />
  </main>

  <aside class="panel" aria-labelledby="player-b-title">
    <h2 id="player-b-title" class="title">Player B</h2>
    <PlayerPanel panel={panels.state.b} onfeature={(track) => panels.feature('b', track)} />
  </aside>
</div>

<style>
  .layout {
    display: grid;
    grid-template-columns: var(--panel-width) minmax(0, 1fr) var(--panel-width);
    gap: var(--space-4);
    height: 100%;
    padding: var(--space-4);
  }

  .panel,
  .camera {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    min-height: 0;
    padding: var(--space-4);
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
  }

  .title {
    margin: 0;
    font-size: var(--font-size-sm);
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--color-accent);
  }

  .placeholder {
    position: relative;
    display: grid;
    flex: 1;
    place-items: center;
    background: var(--color-surface-raised);
    border: 1px dashed var(--color-border);
    border-radius: var(--radius-sm);
  }

  .empty {
    margin: 0;
    font-size: var(--font-size-sm);
    color: var(--color-text-muted);
  }
</style>
