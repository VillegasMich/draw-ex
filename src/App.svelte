<div class="layout">
  <aside class="panel" aria-labelledby="player-a-title">
    <h2 id="player-a-title" class="title">Player A</h2>
    <p class="empty">No cards yet</p>
  </aside>

  <main class="camera" aria-labelledby="camera-title">
    <h2 id="camera-title" class="title">Camera</h2>
    <div class="placeholder">
      <p class="empty">No camera connected</p>
    </div>
  </main>

  <aside class="panel" aria-labelledby="player-b-title">
    <h2 id="player-b-title" class="title">Player B</h2>
    <p class="empty">No cards yet</p>
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
