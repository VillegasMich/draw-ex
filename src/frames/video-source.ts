// Grabs full-resolution RGBA frames from a <video>, reusing one canvas per video.
import type { FrameSource } from './capture.svelte';

/**
 * A frame source over `video`: each call draws the current frame at its full size
 * (`videoWidth` × `videoHeight`) and returns its RGBA pixels, or `null` while the video
 * has no frame yet. The canvas is reused while the size stays the same.
 */
export function videoFrameSource(video: HTMLVideoElement): FrameSource {
  let context: OffscreenCanvasRenderingContext2D | null = null;

  return () => {
    const { videoWidth: width, videoHeight: height } = video;
    if (width === 0 || height === 0 || video.readyState < HTMLMediaElement.HAVE_CURRENT_DATA) {
      return null;
    }
    if (!context || context.canvas.width !== width || context.canvas.height !== height) {
      // Read back on every tick, so ask for a CPU-backed canvas.
      context = new OffscreenCanvas(width, height).getContext('2d', {
        willReadFrequently: true,
      });
      if (!context) return null;
    }
    context.drawImage(video, 0, 0, width, height);
    return context.getImageData(0, 0, width, height);
  };
}
