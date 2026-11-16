<!-- General settings: card language and inference rate. Each change is saved at once. -->
<script lang="ts">
  import type { Language } from '../lib/bindings';
  import { LANGUAGE_NAMES } from './format';
  import type { SettingsState } from './settings.svelte';

  interface Props {
    settings: SettingsState;
  }

  let { settings }: Props = $props();

  const LANGUAGES = Object.entries(LANGUAGE_NAMES) as [Language, string][];

  function setFps(value: string) {
    const fps = Number(value);
    if (value !== '' && Number.isInteger(fps)) {
      void settings.update({ inferenceFps: fps });
    }
  }
</script>

{#if settings.current}
  <label class="field">
    Card language
    <select
      value={settings.current.language}
      onchange={(event) =>
        void settings.update({ language: event.currentTarget.value as Language })}
    >
      {#each LANGUAGES as [code, name] (code)}
        <option value={code}>{name}</option>
      {/each}
    </select>
  </label>
  <label class="field">
    Inference rate (frames per second, per camera)
    <input
      type="number"
      min="1"
      max="30"
      step="1"
      value={settings.current.inferenceFps}
      onchange={(event) => setFps(event.currentTarget.value)}
    />
    <small>1 to 30. Higher finds cards sooner but uses more CPU or GPU.</small>
  </label>
{:else}
  <p class="note">Loading settings…</p>
{/if}
