<!--
  One camera's live preview. The <video> shows the stream at the camera's native rate;
  when the camera cannot be opened or its track ends, the video is removed and the error
  is shown instead, so a dead camera never looks like a frozen frame.
-->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { CameraId } from '../lib/bindings';
  import { openCamera, stopStream } from './devices';
  import { describeCameraFailure, type CameraError } from './errors';
  import type { VideoSize } from './setup';

  interface Props {
    cameraId: CameraId;
    /** The accessible name of the camera's region, e.g. "Camera for Player A". */
    label: string;
    /** Called with the video's frame size once known, and whenever it changes. */
    onvideosize?: (size: VideoSize) => void;
    /** Drawn over the live video, e.g. detection boxes (D3). */
    overlay?: Snippet<[HTMLVideoElement]>;
  }

  let { cameraId, label, onvideosize, overlay }: Props = $props();

  type CameraState =
    | { status: 'opening' }
    | { status: 'live'; stream: MediaStream }
    | { status: 'error'; error: CameraError };

  let camera = $state.raw<CameraState>({ status: 'opening' });
  let attempt = $state(0);
  let video = $state<HTMLVideoElement>();

  $effect(() => {
    const id = cameraId;
    // Retry bumps `attempt`; reading it makes this effect reopen the camera.
    void attempt;
    let stream: MediaStream | undefined;
    let closed = false;

    const fail = (error: CameraError) => {
      if (stream) {
        release(stream);
        stream = undefined;
      }
      camera = { status: 'error', error };
    };
    const onEnded = () => fail(describeCameraFailure({ kind: 'trackEnded' }));
    const release = (s: MediaStream) => {
      for (const track of s.getVideoTracks()) {
        track.removeEventListener('ended', onEnded);
      }
      stopStream(s);
    };

    camera = { status: 'opening' };
    openCamera(id).then(
      (opened) => {
        if (closed) {
          stopStream(opened);
          return;
        }
        stream = opened;
        const tracks = opened.getVideoTracks();
        for (const track of tracks) {
          track.addEventListener('ended', onEnded);
        }
        // The track can end between getUserMedia resolving and the listener above.
        if (tracks.some((track) => track.readyState === 'ended')) {
          onEnded();
          return;
        }
        camera = { status: 'live', stream: opened };
      },
      (error: unknown) => {
        if (!closed) {
          fail(describeCameraFailure({ kind: 'openFailed', error }));
        }
      },
    );

    return () => {
      closed = true;
      if (stream) {
        release(stream);
      }
    };
  });

  $effect(() => {
    // Detach a stopped stream at once: a <video> keeps showing its last frame otherwise,
    // which would look like a frozen camera while another one opens.
    if (video) {
      video.srcObject = camera.status === 'live' ? camera.stream : null;
    }
  });

  function reportSize() {
    if (video && video.videoWidth > 0 && video.videoHeight > 0) {
      onvideosize?.({ width: video.videoWidth, height: video.videoHeight });
    }
  }
</script>

<section class="view" aria-label={label}>
  {#if camera.status === 'error'}
    <div class="message" role="alert">
      <p>{camera.error.message}</p>
      <button type="button" onclick={() => attempt++}>Retry</button>
    </div>
  {:else}
    <video
      bind:this={video}
      autoplay
      muted
      playsinline
      onloadedmetadata={reportSize}
      onresize={reportSize}
    ></video>
    {#if camera.status === 'opening'}
      <p class="message">Opening camera…</p>
    {:else if overlay && video}
      {@render overlay(video)}
    {/if}
  {/if}
</section>

<style>
  .view {
    position: relative;
    display: grid;
    flex: 1;
    place-items: center;
    min-height: 0;
    overflow: hidden;
    background: #000;
    border-radius: var(--radius-sm);
  }

  video {
    grid-area: 1 / 1;
    width: 100%;
    height: 100%;
    object-fit: contain;
  }

  .message {
    display: flex;
    flex-direction: column;
    grid-area: 1 / 1;
    gap: var(--space-3);
    align-items: center;
    max-width: 32rem;
    margin: 0;
    padding: var(--space-4);
    font-size: var(--font-size-sm);
    color: var(--color-text-muted);
    text-align: center;
  }

  .message p {
    margin: 0;
  }

  button {
    padding: var(--space-1) var(--space-3);
    font: inherit;
    color: var(--color-text);
    cursor: pointer;
    background: var(--color-surface-raised);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
  }

  button:hover {
    border-color: var(--color-accent);
  }
</style>
