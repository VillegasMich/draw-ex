<!--
  The split line of a one-camera, both-players setup, drawn over that camera with two
  end points to drag (or move with the arrow keys). On drop it sends the new line, in
  frame pixels, with `set_camera_setup`. Player A's half of the table is the left of
  `from → to` as seen on screen.
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
    /**
     * Called with the new line as soon as an end point is dropped, and with the previous
     * line again if `set_camera_setup` rejects it.
     */
    onchange?: (line: SplitLine) => void;
  }

  let { frame, camera, line, onchange }: Props = $props();

  type End = 'from' | 'to';

  let width = $state(0);
  let height = $state(0);
  /** The end points while one is being dragged, in element pixels. */
  let dragged = $state<{ end: End; from: Point; to: Point } | null>(null);

  const fit = $derived(letterbox(frame, { width, height }));
  /** The two end points, in element pixels. */
  const ends = $derived(
    dragged ?? { from: frameToElement(line.from, fit), to: frameToElement(line.to, fit) },
  );
  /** The frame image's rectangle in element pixels; the line is clipped to it. */
  const imageRect = $derived({
    x: fit.offsetX,
    y: fit.offsetY,
    width: frame.width * fit.scale,
    height: frame.height * fit.scale,
  });

  /** The drawn line, extended across the image, and its half labels, in element pixels. */
  const drawn = $derived.by(() => {
    const dx = ends.to.x - ends.from.x;
    const dy = ends.to.y - ends.from.y;
    const length = Math.hypot(dx, dy) || 1;
    const [ux, uy] = [dx / length, dy / length];
    // Long enough to cross the whole image from either end point.
    const reach = Math.hypot(imageRect.width, imageRect.height) * 2;
    // The half labels sit just inside the `from` end, clear of the cards mid-table, and
    // run toward `to` so they stay inside the image.
    const inset = 14;
    const anchor = { x: ends.from.x + ux * inset, y: ends.from.y + uy * inset };
    const textAnchor = ux < -0.3 ? 'end' : ux > 0.3 ? 'start' : 'middle';
    // The left of `from → to` as seen on screen (y down) is the direction (uy, -ux).
    const offset = 16;
    return {
      x1: ends.from.x - ux * reach,
      y1: ends.from.y - uy * reach,
      x2: ends.to.x + ux * reach,
      y2: ends.to.y + uy * reach,
      halfA: { x: anchor.x + uy * offset, y: anchor.y - ux * offset },
      halfB: { x: anchor.x - uy * offset, y: anchor.y + ux * offset },
      textAnchor,
    };
  });

  const uid = $props.id();
  const clipId = `split-clip-${uid}`;

  /** How far one arrow-key press moves an end point, in element pixels. */
  const KEY_STEP = 8;
  const KEY_MOVES: Record<string, [number, number]> = {
    ArrowLeft: [-1, 0],
    ArrowRight: [1, 0],
    ArrowUp: [0, -1],
    ArrowDown: [0, 1],
  };

  function clampToImage(point: Point): Point {
    return {
      x: Math.min(Math.max(point.x, imageRect.x), imageRect.x + imageRect.width),
      y: Math.min(Math.max(point.y, imageRect.y), imageRect.y + imageRect.height),
    };
  }

  function elementPoint(event: PointerEvent): Point {
    const rect = (event.currentTarget as Element).closest('svg')!.getBoundingClientRect();
    return clampToImage({ x: event.clientX - rect.left, y: event.clientY - rect.top });
  }

  function startDrag(end: End, event: PointerEvent) {
    (event.currentTarget as Element).setPointerCapture(event.pointerId);
    dragged = { end, from: ends.from, to: ends.to };
  }

  function drag(event: PointerEvent) {
    if (dragged) {
      dragged = { ...dragged, [dragged.end]: elementPoint(event) };
    }
  }

  function endDrag() {
    if (!dragged) return;
    const { from, to } = dragged;
    dragged = null;
    void commit(from, to);
  }

  function nudge(end: End, event: KeyboardEvent) {
    const move = KEY_MOVES[event.key];
    if (!move) return;
    event.preventDefault();
    const moved = clampToImage({
      x: ends[end].x + move[0] * KEY_STEP,
      y: ends[end].y + move[1] * KEY_STEP,
    });
    void commit(end === 'from' ? moved : ends.from, end === 'to' ? moved : ends.to);
  }

  /** Shows the line through `from` and `to` (element pixels) and sends it to the backend. */
  async function commit(from: Point, to: Point) {
    if (fit.scale === 0 || (from.x === to.x && from.y === to.y)) {
      return;
    }
    const previous = line;
    const next = splitLineFromDrag(from, to, fit);
    onchange?.(next);
    try {
      await commands.setCameraSetup({ kind: 'splitLine', camera, line: next });
    } catch (error) {
      console.error('Could not update the split line.', error);
      onchange?.(previous);
    }
  }
</script>

<div class="split-line" bind:clientWidth={width} bind:clientHeight={height}>
  <svg {width} {height} role="group" aria-label="Split line">
    <defs>
      <clipPath id={clipId}>
        <rect x={imageRect.x} y={imageRect.y} width={imageRect.width} height={imageRect.height} />
      </clipPath>
    </defs>
    <g clip-path="url(#{clipId})">
      <line class="line" x1={drawn.x1} y1={drawn.y1} x2={drawn.x2} y2={drawn.y2} />
      <text
        class="half-label player-a"
        x={drawn.halfA.x}
        y={drawn.halfA.y}
        text-anchor={drawn.textAnchor}>Player A</text
      >
      <text
        class="half-label player-b"
        x={drawn.halfB.x}
        y={drawn.halfB.y}
        text-anchor={drawn.textAnchor}>Player B</text
      >
    </g>
    {#each ['from', 'to'] as const as end (end)}
      <circle
        class="handle"
        cx={ends[end].x}
        cy={ends[end].y}
        r="9"
        role="slider"
        tabindex="0"
        aria-label="Split line {end === 'from' ? 'start' : 'end'}"
        aria-valuetext="{Math.round(ends[end].x)}, {Math.round(ends[end].y)}"
        aria-valuenow={Math.round(ends[end].x)}
        onpointerdown={(event) => startDrag(end, event)}
        onpointermove={drag}
        onpointerup={endDrag}
        onpointercancel={endDrag}
        onkeydown={(event) => nudge(end, event)}
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
    /* The end handles sit on the image's edge; keep their outer half clickable. */
    overflow: visible;
  }

  .line {
    stroke: var(--color-accent);
    stroke-width: 2;
    stroke-dasharray: 10 6;
  }

  .half-label {
    font-size: var(--font-size-sm);
    font-weight: 700;
    dominant-baseline: middle;
    paint-order: stroke;
    stroke: var(--overlay-label-halo);
    stroke-width: 4;
    stroke-linejoin: round;
  }

  .half-label.player-a {
    fill: var(--player-a-color);
  }

  .half-label.player-b {
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

  .handle:focus-visible {
    outline: none;
    stroke: var(--color-text);
  }

  .handle:active {
    cursor: grabbing;
  }
</style>
