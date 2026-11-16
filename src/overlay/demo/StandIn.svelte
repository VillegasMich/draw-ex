<!--
  Dev only: while the demo plays with no camera set up, a stand-in for the camera. It
  draws the scripted cards letterboxed like the video (`xMidYMid meet` is SVG's
  `object-fit: contain`) under the real overlay, so the outlines can be checked against
  them at any window size.
-->
<script lang="ts">
  import CameraOverlay from '../CameraOverlay.svelte';
  import { liveView } from '../live-view.svelte';
  import { DEMO_CAMERA, DEMO_FRAME } from './script';
  import { demoState } from './state.svelte';
</script>

{#if demoState.standIn}
  <div class="stand-in">
    <svg
      viewBox="0 0 {DEMO_FRAME.width} {DEMO_FRAME.height}"
      preserveAspectRatio="xMidYMid meet"
      aria-hidden="true"
    >
      <rect class="table" width={DEMO_FRAME.width} height={DEMO_FRAME.height} />
      {#each liveView.detections(DEMO_CAMERA) as detection, index (index)}
        <polygon class="card" points={detection.quad.map(({ x, y }) => `${x},${y}`).join(' ')} />
      {/each}
    </svg>
    <CameraOverlay camera={DEMO_CAMERA} />
  </div>
{/if}

<style>
  .stand-in {
    position: absolute;
    inset: 0;
  }

  svg {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }

  .table {
    fill: #1d3b2a;
  }

  .card {
    fill: #b8945a;
    stroke: #3a2c18;
    stroke-width: 4;
  }
</style>
