<!--
  The split line of a "both sides" camera setup, drawn over that camera with two
  draggable end points. On drag end it sends the new line, in frame pixels, with
  `set_camera_setup`. Player A's side is the left of `from → to` as seen on screen.
-->
<script lang="ts">
  import './player-colors.css';
  import { commands, type CameraId, type Point, type SplitLine } from '../lib/bindings';
  import { frameToElement, letterbox, type Size } from './geometry';
  import { splitLineFromDrag } from './split-line';

  interface Props {
    /** The camera's frame size, in pixels. */
    frame: Size;
    camera: CameraId;
    /** The current split line, in frame pixels. */
    line: SplitLine;
    /** Called with the new line after `set_camera_setup` accepts it. */
    onchange?: (line: SplitLine) => void;
  }

  let { frame, camera, line, onchange }: Props = $props();

  let width = $state(0);
  let height = $state(0);
  /** The end points while one is being dragged, in element pixels. */
  let dragged = $state<{ end: 'from' | 'to'; from: Point; to: Point } | null>(null);

  const box = $derived(letterbox(frame, { width, height }));
  const ends = $derived(
    dragged ?? { from: frameToElement(line.from, box), to: frameToElement(line.to, box) },
  );
  /** The frame image's rectangle in element pixels; the line is clipped to it. */
  const image = $derived({
    x: box.offsetX,
    y: box.offsetY,
    width: frame.width * box.scale,
    height: frame.height * box.scale,
  });

  const geometry = $derived.by(() => {
    const dx = ends.to.x - ends.from.x;
    const dy = ends.to.y - ends.from.y;
    const length = Math.hypot(dx, dy) || 1;
    const [ux, uy] = [dx / length, dy / length];
    // Long enough to cross the whole image from either end point.
    const reach = Math.hypot(image.width, image.height) * 2;
    const mid = { x: (ends.from.x + ends.to.x) / 2, y: (ends.from.y + ends.to.y) / 2 };
    // The left of `from → to` as seen on screen (y down) is the direction (uy, -ux).
    const offset = 28;
    return {
      x1: ends.from.x - ux * reach,
      y1: ends.from.y - uy * reach,
      x2: ends.to.x + ux * reach,
      y2: ends.to.y + uy * reach,
      sideA: { x: mid.x + uy * offset, y: mid.y - ux * offset },
      sideB: { x: mid.x - uy * offset, y: mid.y + ux * offset },
    };
  });

  const uid = $props.id();
  const clipId = `split-clip-${uid}`;

  function clampToImage(point: Point): Point {
    return {
      x: Math.min(Math.max(point.x, image.x), image.x + image.width),
      y: Math.min(Math.max(point.y, image.y), image.y + image.height),
    };
  }

  function elementPoint(event: PointerEvent): Point {
    const rect = (event.currentTarget as Element).closest('svg')!.getBoundingClientRect();
    return clampToImage({ x: event.clientX - rect.left, y: event.clientY - rect.top });
  }

  function startDrag(end: 'from' | 'to', event: PointerEvent) {
    (event.currentTarget as Element).setPointerCapture(event.pointerId);
    dragged = { end, from: ends.from, to: ends.to };
  }

  function drag(event: PointerEvent) {
    if (dragged) {
      dragged = { ...dragged, [dragged.end]: elementPoint(event) };
    }
  }

  async function endDrag() {
    if (!dragged) return;
    const { from, to } = dragged;
    dragged = null;
    if (box.scale === 0 || (from.x === to.x && from.y === to.y)) {
      return;
    }
    const next = splitLineFromDrag(from, to, box);
    try {
      await commands.setCameraSetup({ kind: 'splitLine', camera, line: next });
      onchange?.(next);
    } catch (error) {
      console.error('Could not update the split line.', error);
    }
  }
</script>

<div class="split-line" bind:clientWidth={width} bind:clientHeight={height}>
  <svg {width} {height} role="group" aria-label="Split line">
    <defs>
      <clipPath id={clipId}>
        <rect x={image.x} y={image.y} width={image.width} height={image.height} />
      </clipPath>
    </defs>
    <g clip-path="url(#{clipId})">
      <line class="line" x1={geometry.x1} y1={geometry.y1} x2={geometry.x2} y2={geometry.y2} />
      <text class="side player-a" x={geometry.sideA.x} y={geometry.sideA.y}>Player A</text>
      <text class="side player-b" x={geometry.sideB.x} y={geometry.sideB.y}>Player B</text>
    </g>
    {#each ['from', 'to'] as const as end (end)}
      <circle
        class="handle"
        cx={ends[end].x}
        cy={ends[end].y}
        r="9"
        role="slider"
        tabindex="-1"
        aria-label="Split line {end === 'from' ? 'start' : 'end'}"
        aria-valuetext="{Math.round(ends[end].x)}, {Math.round(ends[end].y)}"
        aria-valuenow={Math.round(ends[end].x)}
        onpointerdown={(event) => startDrag(end, event)}
        onpointermove={drag}
        onpointerup={endDrag}
        onpointercancel={endDrag}
      />
    {/each}
  </svg>
</div>

<style>
  .split-line {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }

  svg {
    position: absolute;
    inset: 0;
  }

  .line {
    stroke: var(--color-accent);
    stroke-width: 2;
    stroke-dasharray: 10 6;
  }

  .side {
    font-size: var(--font-size-sm);
    font-weight: 700;
    text-anchor: middle;
    dominant-baseline: middle;
    paint-order: stroke;
    stroke: var(--overlay-label-halo);
    stroke-width: 4;
    stroke-linejoin: round;
  }

  .side.player-a {
    fill: var(--player-a-color);
  }

  .side.player-b {
    fill: var(--player-b-color);
  }

  .handle {
    fill: var(--color-surface);
    stroke: var(--color-accent);
    stroke-width: 3;
    cursor: grab;
    pointer-events: auto;
    touch-action: none;
  }

  .handle:active {
    cursor: grabbing;
  }
</style>
