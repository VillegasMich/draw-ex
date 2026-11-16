<!--
  The Setup modal: the operator picks the camera setup, the camera(s), and the player
  where it applies. Saving hands the selection to the parent, which applies it.
-->
<script lang="ts">
  import { untrack } from 'svelte';
  import type { CameraId, Player } from '../lib/bindings';
  import type { CameraDevice } from './devices';
  import type { SetupSelection } from './setup';

  interface Props {
    open: boolean;
    cameras: CameraDevice[];
    /** The setup in use, to start from. */
    current: SetupSelection | null;
    onsave: (selection: SetupSelection) => void;
    onclose: () => void;
  }

  let { open, cameras, current, onsave, onclose }: Props = $props();

  type Kind = SetupSelection['kind'];

  const KINDS: { kind: Kind; name: string; hint: string }[] = [
    { kind: 'oneSide', name: 'One side', hint: "One camera on one player's half of the table." },
    {
      kind: 'splitLine',
      name: 'Both sides',
      hint: 'One camera over the whole table. The split line starts across the middle: Player A below it, Player B above.',
    },
    { kind: 'twoCameras', name: 'Two cameras', hint: 'One camera for each player.' },
  ];

  let dialog = $state<HTMLDialogElement>();
  let kind = $state<Kind>('oneSide');
  let camera = $state<CameraId>('');
  let player = $state<Player>('a');
  let cameraA = $state<CameraId>('');
  let cameraB = $state<CameraId>('');

  // Start from the current setup each time the modal opens, falling back to the first
  // cameras for anything it does not set (or that is no longer plugged in).
  function reset() {
    const ids = cameras.map((device) => device.id);
    const pick = (id: CameraId | undefined, fallback: CameraId | undefined) =>
      id && ids.includes(id) ? id : (fallback ?? '');
    kind = current?.kind ?? 'oneSide';
    camera = pick(current && current.kind !== 'twoCameras' ? current.camera : undefined, ids[0]);
    player = current?.kind === 'oneSide' ? current.player : 'a';
    cameraA = pick(current?.kind === 'twoCameras' ? current.a : undefined, ids[0]);
    cameraB = pick(current?.kind === 'twoCameras' ? current.b : undefined, ids[1] ?? ids[0]);
  }

  $effect(() => {
    if (!dialog) {
      return;
    }
    if (open && !dialog.open) {
      untrack(reset);
      dialog.showModal();
    } else if (!open && dialog.open) {
      dialog.close();
    }
  });

  let selection = $derived.by((): SetupSelection | null => {
    switch (kind) {
      case 'oneSide':
        return camera ? { kind, camera, player } : null;
      case 'splitLine':
        return camera ? { kind, camera } : null;
      case 'twoCameras':
        return cameraA && cameraB ? { kind, a: cameraA, b: cameraB } : null;
    }
  });

  function save(event: SubmitEvent) {
    event.preventDefault();
    if (selection) {
      onsave(selection);
    }
  }
</script>

<dialog bind:this={dialog} aria-labelledby="setup-title" {onclose}>
  <form onsubmit={save}>
    <h2 id="setup-title">Camera setup</h2>

    <fieldset>
      <legend>Setup</legend>
      {#each KINDS as option (option.kind)}
        <label class="choice">
          <input type="radio" name="kind" value={option.kind} bind:group={kind} />
          <span>
            <strong>{option.name}</strong>
            <small>{option.hint}</small>
          </span>
        </label>
      {/each}
    </fieldset>

    {#if cameras.length === 0}
      <p class="note">No cameras found. Plug one in, then open Setup again.</p>
    {:else if kind === 'twoCameras'}
      <label class="field">
        Player A's camera
        <select bind:value={cameraA}>
          {#each cameras as device (device.id)}
            <option value={device.id}>{device.label}</option>
          {/each}
        </select>
      </label>
      <label class="field">
        Player B's camera
        <select bind:value={cameraB}>
          {#each cameras as device (device.id)}
            <option value={device.id}>{device.label}</option>
          {/each}
        </select>
      </label>
    {:else}
      <label class="field">
        Camera
        <select bind:value={camera}>
          {#each cameras as device (device.id)}
            <option value={device.id}>{device.label}</option>
          {/each}
        </select>
      </label>
      {#if kind === 'oneSide'}
        <fieldset class="players">
          <legend>Player</legend>
          <label><input type="radio" name="player" value="a" bind:group={player} /> Player A</label>
          <label><input type="radio" name="player" value="b" bind:group={player} /> Player B</label>
        </fieldset>
      {/if}
    {/if}

    <div class="actions">
      <button type="button" onclick={onclose}>Cancel</button>
      <button type="submit" class="primary" disabled={!selection}>Save</button>
    </div>
  </form>
</dialog>

<style>
  dialog {
    width: min(28rem, calc(100vw - 2 * var(--space-6)));
    padding: 0;
    color: var(--color-text);
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
  }

  dialog::backdrop {
    background: rgb(0 0 0 / 60%);
  }

  form {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    padding: var(--space-6);
  }

  h2 {
    margin: 0;
    font-size: var(--font-size-md);
  }

  fieldset {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: 0;
    margin: 0;
    border: 0;
  }

  legend,
  .field {
    font-size: var(--font-size-sm);
    color: var(--color-text-muted);
  }

  legend {
    padding: 0;
    margin-bottom: var(--space-2);
  }

  .choice {
    display: flex;
    gap: var(--space-3);
    align-items: flex-start;
    padding: var(--space-3);
    cursor: pointer;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
  }

  .choice:has(input:checked) {
    border-color: var(--color-accent);
  }

  .choice span {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .choice small {
    font-size: var(--font-size-sm);
    color: var(--color-text-muted);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  select,
  button {
    padding: var(--space-2) var(--space-3);
    font: inherit;
    color: var(--color-text);
    background: var(--color-surface-raised);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
  }

  .players {
    flex-direction: row;
    flex-wrap: wrap;
    gap: var(--space-4);
  }

  .players legend {
    width: 100%;
  }

  .note {
    margin: 0;
    font-size: var(--font-size-sm);
    color: var(--color-text-muted);
  }

  .actions {
    display: flex;
    gap: var(--space-2);
    justify-content: flex-end;
  }

  button {
    cursor: pointer;
  }

  button:hover:not(:disabled) {
    border-color: var(--color-accent);
  }

  button:disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }

  .primary {
    color: var(--color-bg);
    background: var(--color-accent);
    border-color: var(--color-accent);
  }
</style>
