<!--
  Everything drawn over the camera on screen: detection outlines and, for a "both sides"
  setup, the split line handle. It fills its positioned parent, the box the `<video>`
  fills with `object-fit: contain`. In dev builds it also offers the scripted demo.
-->
<script lang="ts">
  import { liveView } from './live-view.svelte';
  import Overlay from './Overlay.svelte';
  import SplitLineHandle from './SplitLineHandle.svelte';

  // Dev only: in a production build this is `null` and the demo module is not bundled.
  const demo = import.meta.env.DEV ? import('./demo/Demo.svelte') : null;
</script>

<div class="camera-overlay">
  {#if demo}
    {#await demo then { default: Demo }}
      <Demo />
    {/await}
  {/if}

  {#if liveView.camera !== null}
    {@const camera = liveView.camera}
    <Overlay
      frame={liveView.frame}
      {camera}
      setup={liveView.setup}
      detections={liveView.detections}
    />
    {#if liveView.setup?.kind === 'splitLine' && liveView.setup.camera === camera}
      {@const setup = liveView.setup}
      <SplitLineHandle
        frame={liveView.frame}
        {camera}
        line={setup.line}
        onchange={(line) => {
          liveView.setup = { ...setup, line };
        }}
      />
    {/if}
  {/if}
</div>

<style>
  .camera-overlay {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }
</style>
