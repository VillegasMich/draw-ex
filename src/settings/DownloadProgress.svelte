<!-- A model download's progress bar with its percentage. -->
<script lang="ts">
  interface Props {
    /** How far the download has got, from 0 to 1. */
    fraction: number;
  }

  let { fraction }: Props = $props();
</script>

<div class="progress">
  <progress max="1" value={fraction} aria-label="Model download"></progress>
  <span>{Math.floor(fraction * 100)}%</span>
</div>

<style>
  .progress {
    display: flex;
    gap: var(--space-2);
    align-items: center;
    font-size: var(--font-size-sm);
    color: var(--color-text-muted);
  }

  progress {
    flex: 1;
    min-width: 6rem;
    height: 0.5rem;
    accent-color: var(--color-accent);
  }

  span {
    min-width: 3ch;
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
</style>
