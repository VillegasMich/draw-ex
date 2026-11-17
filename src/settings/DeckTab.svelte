<!-- The deck list recognition prefers. The deck commands are stubs until I1. -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { commands, type DeckId, type DeckSummary } from '../lib/bindings';
  import { errorMessage } from './format';
  import type { SettingsState } from './settings.svelte';

  interface Props {
    settings: SettingsState;
  }

  let { settings }: Props = $props();

  let decks = $state.raw<DeckSummary[] | null>(null);
  // Starts from the settings once; after that it follows the operator's choice.
  // svelte-ignore state_referenced_locally
  let selected = $state<DeckId | null>(settings.current?.selectedDeck ?? null);
  let error = $state<string | null>(null);

  async function select(id: DeckId | null) {
    error = null;
    try {
      await commands.selectDeck(id);
      selected = id;
      // The backend may keep the choice in the settings (I1): reload them, so a later
      // save from another tab does not send a stale deck back.
      await settings.load();
    } catch (reason) {
      error = errorMessage(reason);
    }
  }

  onMount(() => {
    commands.listDecks().then(
      (list) => (decks = list),
      (reason: unknown) => (error = errorMessage(reason)),
    );
  });
</script>

{#if decks}
  <fieldset>
    <legend>Deck list recognition prefers</legend>
    <label class="choice">
      <input
        type="radio"
        name="deck"
        checked={selected === null}
        onchange={() => void select(null)}
      />
      <span><strong>None</strong><small>Recognize any card.</small></span>
    </label>
    {#each decks as deck (deck.id)}
      <label class="choice">
        <input
          type="radio"
          name="deck"
          checked={selected === deck.id}
          onchange={() => void select(deck.id)}
        />
        <span><strong>{deck.name}</strong><small>{deck.cardCount} cards</small></span>
      </label>
    {/each}
  </fieldset>
{:else if !error}
  <p class="note">Loading decks…</p>
{/if}
{#if error}
  <p class="error" role="alert">{error}</p>
{/if}
