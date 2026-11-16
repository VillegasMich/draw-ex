<!--
  Detection outlines over one camera's video. It fills its positioned parent, the same
  area as the `<video>`, which must use `object-fit: contain`: frame pixels are mapped
  through that letterbox so outlines stay on the cards at any size. Recognized cards are
  outlined in their player's color and labelled; the rest get a thin neutral outline.
  Its toggle hides only these outlines: the split line is a setup control and stays.
-->
<script lang="ts">
  import './player-colors.css';
  import type { CameraId, CameraSetup, DetectionDto, Player } from '../lib/bindings';
  import { cardData, cardName } from '../panels/card-data.svelte';
  import { frameToElement, letterbox, type Size } from './geometry';
  import { playerForQuad } from './split-line';

  interface Props {
    /** The camera's frame size, in pixels. */
    frame: Size;
    camera: CameraId;
    setup: CameraSetup | null;
    /** The camera's latest detections, in frame pixels. Raw: not only confirmed cards. */
    detections: DetectionDto[];
  }

  let { frame, camera, setup, detections }: Props = $props();

  let width = $state(0);
  let height = $state(0);
  let visible = $state(true);

  const fit = $derived(letterbox(frame, { width, height }));

  /** How an outline is drawn: in a player's color, or as unassigned or unrecognized. */
  type Tone = Player | 'unassigned' | 'unrecognized';

  /** One detection as drawn. Positions are in element pixels. */
  interface Outline {
    /** The quad's corners, as an SVG `points` list. */
    points: string;
    /** The CSS class giving the outline's and label's color. */
    tone: string;
    /** The card name, centered above the quad, once recognized. */
    label: { text: string; x: number; y: number } | null;
  }

  const toneClass = (tone: Tone) => (tone === 'a' || tone === 'b' ? `player-${tone}` : tone);

  const outlines: Outline[] = $derived(
    detections.map((detection) => {
      const corners = detection.quad.map((corner) => frameToElement(corner, fit));
      const points = corners.map(({ x, y }) => `${x},${y}`).join(' ');
      const { recognition } = detection;
      if (recognition.kind !== 'recognized') {
        return { points, tone: toneClass('unrecognized'), label: null };
      }
      const top = Math.min(...corners.map((corner) => corner.y));
      const left = Math.min(...corners.map((corner) => corner.x));
      const right = Math.max(...corners.map((corner) => corner.x));
      return {
        points,
        tone: toneClass(playerForQuad(setup, camera, detection.quad) ?? 'unassigned'),
        label: {
          text: cardName(recognition.passcode, cardData.get(recognition.passcode)),
          x: (left + right) / 2,
          y: Math.max(top - 6, 14),
        },
      };
    }),
  );
</script>

<div class="overlay" bind:clientWidth={width} bind:clientHeight={height}>
  {#if visible}
    <svg class="outlines" {width} {height} aria-hidden="true">
      {#each outlines as outline, index (index)}
        <polygon class="outline {outline.tone}" points={outline.points} />
        {#if outline.label}
          <text class="label {outline.tone}" x={outline.label.x} y={outline.label.y}
            >{outline.label.text}</text
          >
        {/if}
      {/each}
    </svg>
  {/if}

  <button
    type="button"
    class="toggle"
    aria-pressed={visible}
    onclick={() => {
      visible = !visible;
    }}>Overlay</button
  >
</div>

<style>
  .overlay {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }

  .outlines {
    position: absolute;
    inset: 0;
    overflow: visible;
  }

  .outline {
    fill: none;
    stroke-width: 3;
    stroke-linejoin: round;
  }

  .outline.player-a {
    stroke: var(--player-a-color);
  }

  .outline.player-b {
    stroke: var(--player-b-color);
  }

  .outline.unassigned {
    stroke: var(--color-accent);
  }

  .outline.unrecognized {
    stroke: var(--overlay-neutral-color);
    stroke-width: 1;
    stroke-dasharray: 6 4;
  }

  .label {
    font-size: var(--font-size-sm);
    font-weight: 600;
    text-anchor: middle;
    paint-order: stroke;
    stroke: var(--overlay-label-halo);
    stroke-width: 4;
    stroke-linejoin: round;
  }

  .label.player-a {
    fill: var(--player-a-color);
  }

  .label.player-b {
    fill: var(--player-b-color);
  }

  .label.unassigned {
    fill: var(--color-accent);
  }

  .toggle {
    position: absolute;
    top: var(--space-2);
    right: var(--space-2);
    padding: var(--space-1) var(--space-3);
    font: inherit;
    font-size: var(--font-size-sm);
    color: var(--color-text-muted);
    cursor: pointer;
    pointer-events: auto;
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
  }

  .toggle[aria-pressed='true'] {
    color: var(--color-text);
    border-color: var(--color-accent);
  }
</style>
