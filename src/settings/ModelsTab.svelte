<!-- The model variant in use, what each variant takes on disk, and its download. -->
<script lang="ts">
  import type { ModelVariantDto } from '../lib/bindings';
  import DownloadProgress from './DownloadProgress.svelte';
  import { formatBytes, VARIANT_NAMES } from './format';
  import type { ModelsState } from './models.svelte';
  import type { SettingsState } from './settings.svelte';

  interface Props {
    settings: SettingsState;
    models: ModelsState;
  }

  let { settings, models }: Props = $props();

  let selected = $derived(settings.current?.modelVariant);
  let selectedStatus = $derived(selected ? models.variant(selected) : undefined);

  async function choose(variant: ModelVariantDto) {
    await settings.update({ modelVariant: variant });
    await models.refresh();
  }
</script>

{#if models.status && settings.current}
  <fieldset>
    <legend>Classifier variant</legend>
    {#each models.status.variants as entry (entry.variant)}
      <label class="choice">
        <input
          type="radio"
          name="variant"
          value={entry.variant}
          checked={entry.variant === selected}
          disabled={models.downloading !== null}
          onchange={() => void choose(entry.variant)}
        />
        <span>
          <strong>{VARIANT_NAMES[entry.variant].name}</strong>
          <small>{VARIANT_NAMES[entry.variant].hint}</small>
          <small>
            {formatBytes(entry.bytesTotal)} ·
            {#if entry.installed}<em>Installed</em>{:else}Not installed{/if}
          </small>
        </span>
      </label>
    {/each}
  </fieldset>

  {#if selectedStatus && models.downloading === selectedStatus.variant}
    <DownloadProgress fraction={models.fraction} />
  {:else if selectedStatus && !selectedStatus.installed}
    <button
      type="button"
      class="primary"
      disabled={models.downloading !== null}
      onclick={() => void models.download(selectedStatus.variant)}
    >
      Download ({formatBytes(selectedStatus.bytesTotal)})
    </button>
  {/if}
  {#if models.error}
    <p class="error" role="alert">{models.error}</p>
  {/if}
{:else}
  <p class="note">Loading models…</p>
{/if}
