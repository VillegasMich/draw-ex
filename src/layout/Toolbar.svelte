<!-- The slim bar above the camera area: what the camera setup is, and the Setup button. -->
<script lang="ts">
  interface Props {
    /** A short description of the current camera setup. */
    summary: string;
    /** A problem to show next to the summary, e.g. the setup could not be applied. */
    problem?: string | null;
    onsetup: () => void;
  }

  let { summary, problem = null, onsetup }: Props = $props();
</script>

<div class="toolbar" role="toolbar" aria-label="Camera toolbar">
  <p class="summary">{summary}</p>
  {#if problem}
    <p class="problem" role="alert">{problem}</p>
  {/if}
  <button type="button" onclick={onsetup}>Setup</button>
</div>

<style>
  .toolbar {
    display: flex;
    gap: var(--space-3);
    align-items: center;
    min-height: 2rem;
  }

  .summary,
  .problem {
    margin: 0;
    overflow: hidden;
    font-size: var(--font-size-sm);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .summary {
    flex: 1;
    color: var(--color-text-muted);
  }

  .problem {
    color: var(--color-accent);
  }

  button {
    padding: var(--space-1) var(--space-3);
    font: inherit;
    font-size: var(--font-size-sm);
    color: var(--color-text);
    cursor: pointer;
    background: var(--color-surface-raised);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
  }

  button:hover {
    border-color: var(--color-accent);
  }
</style>
