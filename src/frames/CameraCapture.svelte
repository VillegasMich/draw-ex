<!--
  Captures one camera's frames for recognition while mounted. Mount it in `CameraView`'s
  `overlay` snippet: that only renders while the camera is live, so capture stops when
  the camera errors or leaves the setup. The capture rate follows the inference rate in
  the app's settings, live. In dev builds it also shows the capture counters, the
  capture rate and the preview's frame rate.
-->
<script lang="ts">
  import { untrack } from 'svelte';
  import type { CameraId } from '../lib/bindings';
  import { appSettings } from '../settings';
  import { DEFAULT_INFERENCE_FPS, frameCapture } from './capture.svelte';
  import { videoFrameSource } from './video-source';

  interface Props {
    camera: CameraId;
    /** The camera's live `<video>`. */
    video: HTMLVideoElement;
  }

  let { camera, video }: Props = $props();

  /** The rate from the settings; the default until they load. */
  const fps = $derived(appSettings.current?.inferenceFps ?? DEFAULT_INFERENCE_FPS);

  // Starts capture once per camera and video. The rate is read untracked, so a new rate
  // does not restart capture: the effect below applies it to the running capture.
  $effect(() => {
    return frameCapture.start(
      camera,
      videoFrameSource(video),
      untrack(() => fps),
    );
  });

  $effect(() => {
    frameCapture.setFps(camera, fps);
  });

  const stats = $derived(frameCapture.stats(camera));

  /** Preview frames per second, from `requestVideoFrameCallback` (dev readout only). */
  let previewFps = $state<number | null>(null);

  $effect(() => {
    if (!import.meta.env.DEV || !('requestVideoFrameCallback' in video)) return;
    const element = video;
    let frames = 0;
    let since = performance.now();
    let handle = element.requestVideoFrameCallback(function count(now) {
      frames += 1;
      if (now - since >= 1000) {
        previewFps = Math.round((frames * 1000) / (now - since));
        frames = 0;
        since = now;
      }
      handle = element.requestVideoFrameCallback(count);
    });
    return () => element.cancelVideoFrameCallback(handle);
  });
</script>

{#if import.meta.env.DEV}
  <p class="readout" data-testid="capture-readout">
    preview {previewFps ?? '–'} fps · capture {fps} fps · sent {stats.sent} · skipped {stats.skipped}
    · in flight
    {stats.inFlight} (max {stats.maxInFlight}) · failed {stats.failed} · round trip
    {stats.lastRoundTripMs === null ? '–' : Math.round(stats.lastRoundTripMs)} ms
  </p>
{/if}

<style>
  .readout {
    position: absolute;
    bottom: var(--space-1);
    left: var(--space-1);
    z-index: 1;
    margin: 0;
    padding: 0 var(--space-1);
    font-family: ui-monospace, monospace;
    font-size: var(--font-size-sm);
    color: var(--color-text);
    pointer-events: none;
    background: rgb(0 0 0 / 60%);
    border-radius: var(--radius-sm);
  }
</style>
