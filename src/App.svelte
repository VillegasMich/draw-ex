<script lang="ts">
  import { onMount } from 'svelte';
  import {
    browserStorage,
    CameraView,
    describeCameraFailure,
    describeSelection,
    listCameras,
    loadSetup,
    onCamerasChanged,
    requestCameraAccess,
    saveSetup,
    SetupModal,
    toCameraSetup,
    type CameraDevice,
    type CameraError,
    type SetupSelection,
    type VideoSize,
  } from './camera';
  import { cameraSlots, Toolbar } from './layout';
  import { CameraCapture } from './frames';
  import { commands, type CameraId, type IpcError } from './lib/bindings';
  import { CameraOverlay, DemoStandIn, DemoToggle, liveView } from './overlay';
  import { connectPanels, DuelLog, panels, PlayerPanel } from './panels';
  import { appSettings, ModelsBanner, ModelsState, SettingsModal } from './settings';

  let cameras = $state.raw<CameraDevice[]>([]);
  let connecting = $state(true);
  /** Set when the webview may not use cameras at all. */
  let accessError = $state.raw<CameraError | null>(null);
  let selection = $state.raw<SetupSelection | null>(null);
  /** A selection not yet sent to `setCameraSetup` (Both sides waits for the video size). */
  let pendingSetup = $state.raw<SetupSelection | null>(null);
  /** Counts `setCameraSetup` calls, so only the latest one's outcome is shown. */
  let setupRequest = 0;
  let videoSizes = $state<Record<CameraId, VideoSize>>({});
  let setupOpen = $state(false);
  let setupProblem = $state<string | null>(null);
  let settingsOpen = $state(false);
  const models = new ModelsState();

  let slots = $derived(selection ? cameraSlots(selection) : []);
  let summary = $derived(
    describeSelection(
      selection,
      (id) => cameras.find((camera) => camera.id === id)?.label ?? 'Camera',
    ),
  );

  async function refreshCameras() {
    try {
      cameras = await listCameras();
    } catch {
      cameras = [];
    }
  }

  /** Applies the remembered setup if there is none yet and its cameras are present. */
  function restoreSetup() {
    if (selection) {
      return;
    }
    const remembered = loadSetup(
      browserStorage(),
      cameras.map((camera) => camera.id),
    );
    if (remembered) {
      apply(remembered);
    }
  }

  /** On plug or unplug: list the cameras again, and restore a setup whose camera is back. */
  async function onDevicesChanged() {
    await refreshCameras();
    if (!accessError) {
      restoreSetup();
    }
  }

  async function connect() {
    connecting = true;
    accessError = null;
    try {
      // Device labels and stable IDs need permission, so ask once before listing.
      await requestCameraAccess();
    } catch (error) {
      const failure = describeCameraFailure({ kind: 'openFailed', error });
      if (failure.kind === 'permissionDenied') {
        accessError = failure;
        connecting = false;
        return;
      }
      // Any other failure (e.g. the default camera is busy) still lets the operator pick.
    }
    await refreshCameras();
    restoreSetup();
    connecting = false;
  }

  // The backend keeps the camera setup in memory only, so a restored setup is sent too.
  function apply(next: SetupSelection) {
    selection = next;
    pendingSetup = next;
  }

  function save(next: SetupSelection) {
    saveSetup(browserStorage(), next);
    apply(next);
    setupOpen = false;
  }

  $effect(() => {
    if (!pendingSetup) {
      return;
    }
    const size =
      pendingSetup.kind === 'splitLine' ? (videoSizes[pendingSetup.camera] ?? null) : null;
    const setup = toCameraSetup(pendingSetup, size);
    if (!setup) {
      return;
    }
    pendingSetup = null;
    liveView.setup = setup;
    const request = ++setupRequest;
    commands.setCameraSetup(setup).then(
      () => {
        if (request === setupRequest) {
          setupProblem = null;
        }
      },
      (error: IpcError) => {
        if (request === setupRequest) {
          setupProblem = `The camera setup was not applied: ${error.message}`;
        }
      },
    );
  });

  onMount(connectPanels);
  onMount(() => models.connect());
  onMount(() => void appSettings.load());

  onMount(() => {
    void connect();
    return onCamerasChanged(() => void onDevicesChanged());
  });
</script>

<div class="layout">
  <aside class="panel" aria-labelledby="player-a-title">
    <h2 id="player-a-title" class="title">Player A</h2>
    <PlayerPanel panel={panels.state.a} onfeature={(track) => panels.feature('a', track)} />
  </aside>

  <main class="camera" aria-labelledby="camera-title">
    <h2 id="camera-title" class="title">Camera</h2>
    <Toolbar
      {summary}
      problem={setupProblem}
      onsetup={() => (setupOpen = true)}
      onsettings={() => (settingsOpen = true)}
    />
    <ModelsBanner {models} />

    {#if slots.length > 0}
      <div class="stack">
        {#each slots as slot (slot.key)}
          <CameraView
            cameraId={slot.cameraId}
            label={slot.label}
            onvideosize={(size) => (videoSizes[slot.cameraId] = size)}
          >
            {#snippet overlay(video)}
              <CameraOverlay camera={slot.cameraId} {video} />
              <CameraCapture camera={slot.cameraId} {video} />
            {/snippet}
          </CameraView>
        {/each}
      </div>
    {:else}
      <div class="placeholder">
        {#if connecting}
          <p class="empty">Looking for cameras…</p>
        {:else if accessError}
          <div class="message" role="alert">
            <p>{accessError.message}</p>
            <button type="button" onclick={() => void connect()}>Retry</button>
          </div>
        {:else if cameras.length === 0}
          <p class="empty">No cameras found. Plug one in, then open Setup.</p>
        {:else}
          <p class="empty">No camera setup yet. Choose one with Setup.</p>
        {/if}
        <DemoStandIn />
      </div>
    {/if}
    <DuelLog entries={panels.log} originMs={panels.originMs} />
    <DemoToggle cameras={slots.map((slot) => slot.cameraId)} />
  </main>

  <aside class="panel" aria-labelledby="player-b-title">
    <h2 id="player-b-title" class="title">Player B</h2>
    <PlayerPanel panel={panels.state.b} onfeature={(track) => panels.feature('b', track)} />
  </aside>
</div>

<SetupModal
  open={setupOpen}
  {cameras}
  current={selection}
  onsave={save}
  onclose={() => (setupOpen = false)}
/>

<SettingsModal
  open={settingsOpen}
  settings={appSettings}
  {models}
  onclose={() => (settingsOpen = false)}
/>

<style>
  .layout {
    display: grid;
    grid-template-columns: var(--panel-width) minmax(0, 1fr) var(--panel-width);
    gap: var(--space-4);
    height: 100%;
    padding: var(--space-4);
  }

  .panel,
  .camera {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    min-height: 0;
    padding: var(--space-4);
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
  }

  .title {
    margin: 0;
    font-size: var(--font-size-sm);
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--color-accent);
  }

  .stack {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: var(--space-2);
    min-height: 0;
  }

  .placeholder {
    position: relative;
    display: grid;
    flex: 1;
    place-items: center;
    background: var(--color-surface-raised);
    border: 1px dashed var(--color-border);
    border-radius: var(--radius-sm);
  }

  .empty {
    margin: 0;
    font-size: var(--font-size-sm);
    color: var(--color-text-muted);
  }

  .message {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    align-items: center;
    max-width: 32rem;
    padding: var(--space-4);
    font-size: var(--font-size-sm);
    color: var(--color-text-muted);
    text-align: center;
  }

  .message p {
    margin: 0;
  }

  .message button {
    padding: var(--space-1) var(--space-3);
    font: inherit;
    color: var(--color-text);
    cursor: pointer;
    background: var(--color-surface-raised);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
  }
</style>
