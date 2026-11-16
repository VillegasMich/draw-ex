<!--
  Dev only (`import.meta.env.DEV`): a "Demo" toggle that plays a scripted duel through
  the same panel store and overlay state the real track events and detections use.
  With a camera on screen, its detections are drawn over the first camera (scaled to
  its frame) under the real camera setup. With none, it plays on a stand-in table in
  the camera area. The app imports it only in dev builds, so it is absent from
  production.
-->
<script lang="ts">
  import { onDestroy } from 'svelte';
  import type { CameraId, DetectionDto } from '../../lib/bindings';
  import { panels } from '../../panels/panels.svelte';
  import type { Size } from '../geometry';
  import { liveView } from '../live-view.svelte';
  import { defaultSplitLine } from '../split-line';
  import { DEMO_CAMERA, DEMO_FRAME, DEMO_SCRIPT } from './script';
  import { demoState } from './state.svelte';

  interface Props {
    /** The cameras on screen, top to bottom. */
    cameras: CameraId[];
  }

  let { cameras }: Props = $props();

  let playing = $state(false);
  let target: CameraId = DEMO_CAMERA;
  let timers: ReturnType<typeof setTimeout>[] = [];

  /** The scripted detections, from the script's 1920×1080 frame to `frame`. */
  function scaled(detections: DetectionDto[], frame: Size): DetectionDto[] {
    const sx = frame.width / DEMO_FRAME.width;
    const sy = frame.height / DEMO_FRAME.height;
    return detections.map((detection) => ({
      ...detection,
      quad: detection.quad.map(({ x, y }) => ({ x: x * sx, y: y * sy })) as DetectionDto['quad'],
    }));
  }

  function start() {
    panels.reset(performance.now());
    target = cameras[0] ?? DEMO_CAMERA;
    demoState.standIn = cameras.length === 0;
    if (demoState.standIn) {
      liveView.setFrame(DEMO_CAMERA, DEMO_FRAME);
      liveView.setup = {
        kind: 'splitLine',
        camera: DEMO_CAMERA,
        line: defaultSplitLine(DEMO_FRAME),
      };
    }
    timers = DEMO_SCRIPT.map((step) =>
      setTimeout(() => {
        liveView.setDetections(
          target,
          scaled(step.detections, liveView.frames[target] ?? DEMO_FRAME),
        );
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
    liveView.setDetections(target, []);
    if (demoState.standIn) {
      liveView.forget(DEMO_CAMERA);
      liveView.setup = null;
      demoState.standIn = false;
    }
    panels.reset();
    playing = false;
  }

  // A new camera setup ends the demo rather than drawing it over the wrong camera.
  const camerasKey = $derived(cameras.join('\n'));
  let shownFor: string | undefined;
  $effect(() => {
    const key = camerasKey;
    if (shownFor !== undefined && key !== shownFor && playing) stop();
    shownFor = key;
  });

  onDestroy(() => {
    if (playing) stop();
  });
</script>

<button
  type="button"
  class="demo"
  aria-pressed={playing}
  title="Dev only: play a scripted duel"
  onclick={() => (playing ? stop() : start())}>Demo</button
>

<style>
  .demo {
    position: fixed;
    bottom: var(--space-2);
    left: 50%;
    z-index: 10;
    padding: var(--space-1) var(--space-3);
    font: inherit;
    font-size: var(--font-size-sm);
    color: var(--color-text-muted);
    cursor: pointer;
    background: var(--color-surface);
    border: 1px dashed var(--color-border);
    border-radius: var(--radius-sm);
    transform: translateX(-50%);
  }

  .demo[aria-pressed='true'] {
    color: var(--color-text);
    border-color: var(--color-accent);
  }
</style>
