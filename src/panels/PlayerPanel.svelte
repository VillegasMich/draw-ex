<!--
  One player's panel: the featured card large, the player's other cards on the table as
  a compact list (click to feature), and a dimmed "Left the table" section. Only
  confirmed cards appear here.
-->
<script lang="ts">
  import type { TrackId } from '../lib/bindings';
  import CardImage from './CardImage.svelte';
  import { cardData, cardName, cardStats, cardTypeLine } from './card-data.svelte';
  import type { PlayerPanelState } from './panel-state';

  interface Props {
    panel: PlayerPanelState;
    /** Called with a listed card's track when the operator picks it. */
    onfeature: (track: TrackId) => void;
  }

  let { panel, onfeature }: Props = $props();

  const featured = $derived(panel.onTable.find((card) => card.track === panel.featured));
  const others = $derived(panel.onTable.filter((card) => card.track !== panel.featured));
  const featuredCard = $derived(featured ? cardData.get(featured.passcode) : undefined);
  const uid = $props.id();
</script>

<div class="player-panel">
  {#if panel.onTable.length === 0 && panel.leftTheTable.length === 0}
    <p class="empty">No cards yet</p>
  {/if}

  {#if featured}
    <article class="featured" aria-label="Featured card">
      <div class="featured-image">
        <CardImage
          passcode={featured.passcode}
          size="full"
          alt={cardName(featured.passcode, featuredCard)}
        />
      </div>
      <h3 class="name">{cardName(featured.passcode, featuredCard)}</h3>
      {#if featuredCard}
        {@const stats = cardStats(featuredCard)}
        <p class="type">{cardTypeLine(featuredCard)}</p>
        {#if stats}
          <p class="stats">{stats}</p>
        {/if}
        <p class="description">{featuredCard.description}</p>
      {/if}
    </article>
  {/if}

  {#if others.length > 0}
    <ul class="card-list" aria-label="On the table">
      {#each others as card (card.track)}
        {@const name = cardName(card.passcode, cardData.get(card.passcode))}
        <li>
          <button type="button" class="card-row" onclick={() => onfeature(card.track)}>
            <span class="thumb"><CardImage passcode={card.passcode} size="small" alt="" /></span>
            <span class="row-name">{name}</span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}

  {#if panel.leftTheTable.length > 0}
    <section class="left">
      <h4 class="section-title" id="{uid}-left">Left the table</h4>
      <ul class="card-list" aria-labelledby="{uid}-left">
        {#each panel.leftTheTable as card (card.track)}
          <li class="card-row">
            <span class="thumb"><CardImage passcode={card.passcode} size="small" alt="" /></span>
            <span class="row-name">{cardName(card.passcode, cardData.get(card.passcode))}</span>
          </li>
        {/each}
      </ul>
    </section>
  {/if}
</div>

<style>
  .player-panel {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: var(--space-4);
    min-height: 0;
    overflow-y: auto;
  }

  .empty {
    margin: 0;
    font-size: var(--font-size-sm);
    color: var(--color-text-muted);
  }

  .featured {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .featured-image {
    width: 72%;
    margin: 0 auto var(--space-2);
  }

  .name {
    margin: 0;
    font-size: 1.125rem;
    line-height: 1.25;
  }

  .type,
  .stats {
    margin: 0;
    font-size: var(--font-size-sm);
    color: var(--color-text-muted);
  }

  .stats {
    color: var(--color-text);
    font-variant-numeric: tabular-nums;
  }

  .description {
    margin: 0;
    padding: var(--space-2) var(--space-3);
    font-size: var(--font-size-sm);
    line-height: 1.45;
    white-space: pre-line;
    background: var(--color-surface-raised);
    border-radius: var(--radius-sm);
  }

  .card-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .card-row {
    display: flex;
    gap: var(--space-3);
    align-items: center;
    width: 100%;
    padding: var(--space-1) var(--space-2);
    font: inherit;
    font-size: var(--font-size-sm);
    color: inherit;
    text-align: left;
    background: none;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
  }

  button.card-row {
    cursor: pointer;
  }

  button.card-row:hover,
  button.card-row:focus-visible {
    background: var(--color-surface-raised);
    border-color: var(--color-border);
  }

  .thumb {
    flex: none;
    width: 2rem;
  }

  .row-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .left {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding-top: var(--space-3);
    border-top: 1px solid var(--color-border);
    opacity: 0.55;
  }

  .section-title {
    margin: 0;
    font-size: var(--font-size-sm);
    font-weight: 600;
    color: var(--color-text-muted);
  }
</style>
