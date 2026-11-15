<!--
  Dev only (`import.meta.env.DEV`): a "Demo" toggle that plays a scripted duel through
  the same panel store and overlay state the real track events and detections use. The
  app imports it only in dev builds, so it is absent from production.
-->
<script lang="ts">
  import { onDestroy } from 'svelte';
  import { panels } from '../../panels/panels.svelte';
  import { defaultSplitLine } from '../split-line';
  import { liveView } from '../live-view.svelte';
  import { DEMO_CAMERA, DEMO_FRAME, DEMO_SCRIPT } from './script';

  let playing = $state(false);
  let timers: ReturnType<typeof setTimeout>[] = [];

  function start() {
    panels.reset(performance.now());
    liveView.camera = DEMO_CAMERA;
    liveView.frame = DEMO_FRAME;
    liveView.setup = { kind: 'splitLine', camera: DEMO_CAMERA, line: defaultSplitLine(DEMO_FRAME) };
    timers = DEMO_SCRIPT.map((step) =>
      setTimeout(() => {
        liveView.detections = step.detections;
        const timeMs = performance.now();
        for (const event of step.events) {
          panels.apply({ ...event, timeMs });
        }
      }, step.atMs),
    );
    playing = true;
  }

  function stop() {
    timers.forEach(clearTimeout);
    timers = [];
    liveView.clear();
    panels.reset();
    playing = false;
  }

  onDestroy(() => {
    if (playing) stop();
  });
</script>

{#if playing}
  <!-- Stands in for the camera: the scripted cards, letterboxed like the video
       (`xMidYMid meet` is SVG's `object-fit: contain`), so the overlay can be checked
       against them at any window size. -->
  <svg
    class="stand-in"
    viewBox="0 0 {DEMO_FRAME.width} {DEMO_FRAME.height}"
    preserveAspectRatio="xMidYMid meet"
    aria-hidden="true"
  >
    <rect class="table" width={DEMO_FRAME.width} height={DEMO_FRAME.height} />
    {#each liveView.detections as detection, index (index)}
      <polygon class="card" points={detection.quad.map(({ x, y }) => `${x},${y}`).join(' ')} />
    {/each}
  </svg>
{/if}

<button
  type="button"
  class="demo"
  aria-pressed={playing}
  title="Dev only: play a scripted duel"
  onclick={() => (playing ? stop() : start())}>Demo</button
>

<style>
  .stand-in {
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

  .demo {
    position: absolute;
    top: var(--space-2);
    left: var(--space-2);
    pointer-events: auto;
    padding: var(--space-1) var(--space-3);
    font: inherit;
    font-size: var(--font-size-sm);
    color: var(--color-text-muted);
    cursor: pointer;
    background: var(--color-surface);
    border: 1px dashed var(--color-border);
    border-radius: var(--radius-sm);
  }

  .demo[aria-pressed='true'] {
    color: var(--color-text);
    border-color: var(--color-accent);
  }
</style>
