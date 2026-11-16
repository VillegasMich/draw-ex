<!--
  The first-run banner: shown while the variant in use is not installed, with a Download
  button and the download's progress. It never blocks the camera.
-->
<script lang="ts">
  import DownloadProgress from './DownloadProgress.svelte';
  import { formatBytes } from './format';
  import type { ModelsState } from './models.svelte';

  interface Props {
    models: ModelsState;
  }

  let { models }: Props = $props();

  let current = $derived(models.current);
</script>

{#if current && !current.installed}
  <div class="banner" role="region" aria-label="Recognition models">
    {#if models.downloading}
      <p>Downloading recognition models…</p>
      <DownloadProgress fraction={models.fraction} />
    {:else}
      <p>Recognition models not installed.</p>
      <button type="button" onclick={() => void models.download(current.variant)}>
        Download ({formatBytes(current.bytesTotal)})
      </button>
    {/if}
    {#if models.error}
      <p class="error" role="alert">{models.error}</p>
    {/if}
  </div>
{/if}

<style>
  .banner {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2) var(--space-3);
    align-items: center;
    padding: var(--space-2) var(--space-3);
    font-size: var(--font-size-sm);
    background: var(--color-surface-raised);
    border: 1px solid var(--color-accent);
    border-radius: var(--radius-sm);
  }

  p {
    margin: 0;
  }

  .banner > :global(.progress) {
    flex: 1;
  }

  .error {
    flex-basis: 100%;
    color: var(--color-accent);
  }

  button {
    padding: var(--space-1) var(--space-3);
    font: inherit;
    color: var(--color-bg);
    cursor: pointer;
    background: var(--color-accent);
    border: 1px solid var(--color-accent);
    border-radius: var(--radius-sm);
  }
</style>
