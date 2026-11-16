<!--
  What the local caches hold, with buttons to clear them and to sync the card data.
  The cache commands are stubs until I1.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import { commands, type CacheKind, type CacheStats } from '../lib/bindings';
  import { errorMessage, formatBytes } from './format';
  import type { ModelsState } from './models.svelte';

  interface Props {
    models: ModelsState;
  }

  let { models }: Props = $props();

  let stats = $state.raw<CacheStats | null>(null);
  let note = $state<string | null>(null);
  let error = $state<string | null>(null);

  async function load() {
    try {
      stats = await commands.cacheStats();
    } catch (reason) {
      error = errorMessage(reason);
    }
  }

  async function run(action: () => Promise<unknown>, done: string) {
    error = null;
    note = null;
    try {
      await action();
      note = done;
    } catch (reason) {
      error = errorMessage(reason);
    }
    await load();
  }

  function clear(kind: CacheKind, done: string) {
    return run(async () => {
      await commands.clearCache(kind);
      if (kind === 'models') {
        await models.refresh();
      }
    }, done);
  }

  onMount(() => void load());
</script>

{#if stats}
  <dl>
    <dt>Card images</dt>
    <dd>{formatBytes(stats.imagesBytes)}, {stats.imageCount} images</dd>
    <dt>Card data</dt>
    <dd>
      {formatBytes(stats.cardDbBytes)},
      {stats.cardDbVersion ? `version ${stats.cardDbVersion}` : 'not synced yet'}
    </dd>
    <dt>Models</dt>
    <dd>{formatBytes(stats.modelsBytes)}</dd>
  </dl>
{:else if !error}
  <p class="note">Loading cache sizes…</p>
{/if}

<div class="buttons">
  <button type="button" onclick={() => void clear('images', 'Card images cleared.')}>
    Clear images
  </button>
  <button type="button" onclick={() => void clear('cardData', 'Card data cleared.')}>
    Clear card data
  </button>
  <button type="button" onclick={() => void clear('models', 'Models deleted.')}>
    Delete models
  </button>
  <button type="button" onclick={() => void run(() => commands.syncCardDb(), 'Card data synced.')}>
    Sync now
  </button>
</div>
{#if note}
  <p class="note" role="status">{note}</p>
{/if}
{#if error}
  <p class="error" role="alert">{error}</p>
{/if}

<style>
  dl {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: var(--space-2) var(--space-4);
    margin: 0;
    font-size: var(--font-size-sm);
  }

  dt {
    color: var(--color-text-muted);
  }

  dd {
    margin: 0;
  }

  .buttons {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }
</style>
