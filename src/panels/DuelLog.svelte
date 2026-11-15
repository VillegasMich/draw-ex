<!--
  The duel-log strip: the session's confirmed, changed and lost events, newest first,
  with the time since the session started and the player. Collapsible.
-->
<script lang="ts">
  import '../overlay/player-colors.css';
  import type { TrackEvent } from '../lib/bindings';
  import { cardData, cardName } from './card-data.svelte';

  interface Props {
    /** The session's track events, newest first. */
    entries: TrackEvent[];
    /** The session's start, in milliseconds from `performance.now()`. */
    originMs: number;
  }

  let { entries, originMs }: Props = $props();

  let open = $state(true);
  const uid = $props.id();

  function sessionTime(timeMs: number): string {
    const seconds = Math.max(0, Math.floor((timeMs - originMs) / 1000));
    const minutes = Math.floor(seconds / 60);
    return `${minutes}:${String(seconds % 60).padStart(2, '0')}`;
  }

  const name = (passcode: number) => cardName(passcode, cardData.get(passcode));

  function describe(event: TrackEvent): string {
    switch (event.kind) {
      case 'confirmed':
        return `Confirmed ${name(event.passcode)}`;
      case 'changed':
        return `Changed ${name(event.from)} → ${name(event.to)}`;
      case 'lost':
        return `${name(event.passcode)} left the table`;
    }
  }
</script>

<section class="duel-log">
  <button
    type="button"
    class="header"
    aria-expanded={open}
    aria-controls="{uid}-entries"
    onclick={() => {
      open = !open;
    }}
  >
    <span class="chevron" aria-hidden="true">{open ? '▾' : '▸'}</span>
    Duel log
    <span class="count">{entries.length}</span>
  </button>

  {#if open}
    <ol class="entries" id="{uid}-entries">
      {#each entries as entry, index (entries.length - index)}
        <li class="entry {entry.kind}">
          <time class="time">{sessionTime(entry.timeMs)}</time>
          <span class="player player-{entry.player}">Player {entry.player.toUpperCase()}</span>
          <span class="text">{describe(entry)}</span>
        </li>
      {:else}
        <li class="none">No events yet</li>
      {/each}
    </ol>
  {/if}
</section>

<style>
  .duel-log {
    display: flex;
    flex-direction: column;
    min-height: 0;
    font-size: var(--font-size-sm);
  }

  .header {
    display: flex;
    gap: var(--space-2);
    align-items: center;
    padding: var(--space-1) 0;
    font: inherit;
    font-weight: 600;
    color: var(--color-text-muted);
    text-transform: uppercase;
    letter-spacing: 0.08em;
    cursor: pointer;
    background: none;
    border: none;
  }

  .count {
    padding: 0 var(--space-2);
    font-weight: 400;
    letter-spacing: 0;
    background: var(--color-surface-raised);
    border-radius: var(--radius-sm);
  }

  .entries {
    max-height: 7.5rem;
    margin: 0;
    padding: 0;
    overflow-y: auto;
    list-style: none;
  }

  .entry,
  .none {
    display: flex;
    gap: var(--space-3);
    padding: 2px 0;
  }

  .none {
    color: var(--color-text-muted);
  }

  .time {
    flex: none;
    width: 3.5em;
    color: var(--color-text-muted);
    font-variant-numeric: tabular-nums;
  }

  .player {
    flex: none;
    width: 5.5em;
    font-weight: 600;
  }

  .player-a {
    color: var(--player-a-color);
  }

  .player-b {
    color: var(--player-b-color);
  }

  .entry.lost .text {
    color: var(--color-text-muted);
  }
</style>
