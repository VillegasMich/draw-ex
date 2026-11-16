<!--
  The Settings modal: General, Models, Cache and Deck tabs. Changes are saved as they are
  made, so there is only a Close button.
-->
<script lang="ts">
  import CacheTab from './CacheTab.svelte';
  import DeckTab from './DeckTab.svelte';
  import GeneralTab from './GeneralTab.svelte';
  import type { ModelsState } from './models.svelte';
  import ModelsTab from './ModelsTab.svelte';
  import type { SettingsState } from './settings.svelte';

  interface Props {
    open: boolean;
    settings: SettingsState;
    models: ModelsState;
    onclose: () => void;
  }

  let { open, settings, models, onclose }: Props = $props();

  const TABS = ['General', 'Models', 'Cache', 'Deck'] as const;
  type Tab = (typeof TABS)[number];

  let dialog = $state<HTMLDialogElement>();
  let tab = $state<Tab>('General');

  $effect(() => {
    if (!dialog) {
      return;
    }
    if (open && !dialog.open) {
      tab = 'General';
      void settings.load();
      void models.refresh();
      dialog.showModal();
    } else if (!open && dialog.open) {
      dialog.close();
    }
  });

  const id = (name: Tab, part: 'tab' | 'panel') => `settings-${part}-${name.toLowerCase()}`;

  /** Arrow keys move between tabs, as in any tab list. */
  function onkeydown(event: KeyboardEvent) {
    const step = event.key === 'ArrowRight' ? 1 : event.key === 'ArrowLeft' ? -1 : 0;
    if (step !== 0) {
      tab = TABS[(TABS.indexOf(tab) + step + TABS.length) % TABS.length];
      document.getElementById(id(tab, 'tab'))?.focus();
    }
  }
</script>

<dialog bind:this={dialog} aria-labelledby="settings-title" {onclose}>
  <div class="content">
    <h2 id="settings-title">Settings</h2>

    <div class="tabs" role="tablist" aria-label="Settings sections" tabindex="-1" {onkeydown}>
      {#each TABS as name (name)}
        <button
          type="button"
          role="tab"
          id={id(name, 'tab')}
          aria-selected={tab === name}
          aria-controls={id(name, 'panel')}
          tabindex={tab === name ? 0 : -1}
          onclick={() => (tab = name)}
        >
          {name}
        </button>
      {/each}
    </div>

    <div class="panel" role="tabpanel" id={id(tab, 'panel')} aria-labelledby={id(tab, 'tab')}>
      {#if tab === 'General'}
        <GeneralTab {settings} />
      {:else if tab === 'Models'}
        <ModelsTab {settings} {models} />
      {:else if tab === 'Cache'}
        <CacheTab {models} />
      {:else}
        <DeckTab initial={settings.current?.selectedDeck ?? null} />
      {/if}
      {#if settings.error}
        <p class="error" role="alert">{settings.error}</p>
      {/if}
    </div>

    <div class="actions">
      <button type="button" onclick={onclose}>Close</button>
    </div>
  </div>
</dialog>

<style>
  dialog {
    width: min(32rem, calc(100vw - 2 * var(--space-6)));
    padding: 0;
    color: var(--color-text);
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
  }

  dialog::backdrop {
    background: rgb(0 0 0 / 60%);
  }

  .content {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    padding: var(--space-6);
  }

  h2 {
    margin: 0;
    font-size: var(--font-size-md);
  }

  .tabs {
    display: flex;
    gap: var(--space-1);
    border-bottom: 1px solid var(--color-border);
  }

  .tabs button {
    padding: var(--space-2) var(--space-3);
    margin-bottom: -1px;
    color: var(--color-text-muted);
    background: none;
    border: 0;
    border-bottom: 2px solid transparent;
    border-radius: 0;
  }

  .tabs button[aria-selected='true'] {
    color: var(--color-text);
    border-bottom-color: var(--color-accent);
  }

  .panel {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    min-height: 14rem;
  }

  /* Shared by the tabs' contents. */
  .panel :global(.field) {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    font-size: var(--font-size-sm);
    color: var(--color-text-muted);
  }

  .panel :global(fieldset) {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: 0;
    margin: 0;
    border: 0;
  }

  .panel :global(legend) {
    padding: 0;
    margin-bottom: var(--space-2);
    font-size: var(--font-size-sm);
    color: var(--color-text-muted);
  }

  .panel :global(.choice) {
    display: flex;
    gap: var(--space-3);
    align-items: flex-start;
    padding: var(--space-3);
    cursor: pointer;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
  }

  .panel :global(.choice:has(input:checked)) {
    border-color: var(--color-accent);
  }

  .panel :global(.choice span) {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .panel :global(small) {
    font-size: var(--font-size-sm);
    color: var(--color-text-muted);
  }

  .panel :global(em) {
    font-style: normal;
    color: var(--color-accent);
  }

  .panel :global(.note) {
    margin: 0;
    font-size: var(--font-size-sm);
    color: var(--color-text-muted);
  }

  .panel :global(.error) {
    margin: 0;
    font-size: var(--font-size-sm);
    color: var(--color-accent);
  }

  .content :global(select),
  .content :global(input[type='number']),
  .content :global(button) {
    padding: var(--space-2) var(--space-3);
    font: inherit;
    color: var(--color-text);
    background: var(--color-surface-raised);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
  }

  .content :global(button) {
    cursor: pointer;
  }

  .content :global(button:hover:not(:disabled)) {
    border-color: var(--color-accent);
  }

  .content :global(button:disabled) {
    cursor: not-allowed;
    opacity: 0.5;
  }

  .content :global(.primary) {
    align-self: flex-start;
    color: var(--color-bg);
    background: var(--color-accent);
    border-color: var(--color-accent);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
  }
</style>
