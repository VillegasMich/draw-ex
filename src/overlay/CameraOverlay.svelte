<!--
  Everything drawn over one camera's video: detection outlines and, for a one-camera
  both-players setup on this camera, the split line handle. Mount it in `CameraView`'s
  `overlay` snippet: it fills the view, the same area the `<video>` fills with
  `object-fit: contain`, and reads the frame size from that video.
-->
<script lang="ts">
  import { untrack } from 'svelte';
  import type { CameraId } from '../lib/bindings';
  import DetectionOverlay from './DetectionOverlay.svelte';
  import { liveView } from './live-view.svelte';
  import SplitLineHandle from './SplitLineHandle.svelte';

  interface Props {
    camera: CameraId;
    /** The camera's `<video>`; its `videoWidth` × `videoHeight` is the frame size. */
    video?: HTMLVideoElement;
  }

  let { camera, video }: Props = $props();

  $effect(() => {
    if (!video) return;
    const element = video;
    const id = camera;
    const report = () =>
      liveView.setFrame(id, { width: element.videoWidth, height: element.videoHeight });
    // Untracked: `setFrame` reads the frames, which must not re-run this effect.
    untrack(report);
    element.addEventListener('loadedmetadata', report);
    element.addEventListener('resize', report);
    return () => {
      element.removeEventListener('loadedmetadata', report);
      element.removeEventListener('resize', report);
    };
  });

  const frame = $derived(liveView.frames[camera]);
</script>

{#if frame}
  <div class="camera-overlay">
    <DetectionOverlay
      {frame}
      {camera}
      setup={liveView.setup}
      detections={liveView.detections(camera)}
    />
    {#if liveView.setup?.kind === 'splitLine' && liveView.setup.camera === camera}
      {@const setup = liveView.setup}
      <SplitLineHandle
        {frame}
        {camera}
        line={setup.line}
        onchange={(line) => {
          liveView.setup = { ...setup, line };
        }}
      />
    {/if}
  </div>
{/if}

<style>
  .camera-overlay {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }
</style>
