import { render, screen } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { DetectionDto } from '../lib/bindings';
import CameraOverlay from './CameraOverlay.svelte';
import { liveView } from './live-view.svelte';

vi.mock('../lib/bindings', () => ({
  commands: { getCard: vi.fn(async () => null), setCameraSetup: vi.fn(async () => null) },
}));

// jsdom has no ResizeObserver, which `bind:clientWidth` uses. Sizes stay 0 here.
vi.stubGlobal(
  'ResizeObserver',
  class {
    observe() {}
    unobserve() {}
    disconnect() {}
  },
);

/** A `<video>` reporting a `width` × `height` stream, as `CameraView` passes it. */
function fakeVideo(width: number, height: number) {
  const video = document.createElement('video');
  const size = { width, height };
  Object.defineProperty(video, 'videoWidth', { get: () => size.width });
  Object.defineProperty(video, 'videoHeight', { get: () => size.height });
  return {
    video,
    resize(nextWidth: number, nextHeight: number) {
      size.width = nextWidth;
      size.height = nextHeight;
      video.dispatchEvent(new Event('resize'));
    },
  };
}

const UNREADABLE: DetectionDto = {
  quad: [
    { x: 10, y: 10 },
    { x: 60, y: 10 },
    { x: 60, y: 80 },
    { x: 10, y: 80 },
  ],
  detectorConfidence: 0.5,
  candidates: [],
  recognition: { kind: 'unreadable' },
};

beforeEach(() => {
  liveView.setup = null;
});

afterEach(() => {
  for (const camera of ['cam-a', 'cam-b']) liveView.forget(camera);
});

describe('CameraOverlay', () => {
  it('takes the frame size from the camera’s video, and follows its resize', () => {
    const { video, resize } = fakeVideo(1280, 720);
    render(CameraOverlay, { camera: 'cam-a', video });

    expect(liveView.frames['cam-a']).toEqual({ width: 1280, height: 720 });

    resize(1920, 1080);
    expect(liveView.frames['cam-a']).toEqual({ width: 1920, height: 1080 });
  });

  it('draws nothing until the video knows its size', () => {
    const { video } = fakeVideo(0, 0);
    const { container } = render(CameraOverlay, { camera: 'cam-a', video });

    expect(container.querySelector('svg')).toBeNull();
  });

  it('with two cameras, each overlay draws its own camera’s detections', () => {
    liveView.setup = { kind: 'twoCameras', a: 'cam-a', b: 'cam-b' };
    liveView.setDetections('cam-a', [UNREADABLE, UNREADABLE]);
    liveView.setDetections('cam-b', [UNREADABLE]);

    const a = render(CameraOverlay, { camera: 'cam-a', video: fakeVideo(1280, 720).video });
    const b = render(CameraOverlay, { camera: 'cam-b', video: fakeVideo(640, 480).video });

    expect(a.container.querySelectorAll('polygon')).toHaveLength(2);
    expect(b.container.querySelectorAll('polygon')).toHaveLength(1);
    expect(liveView.frames['cam-b']).toEqual({ width: 640, height: 480 });
  });

  it('shows the split line only over the camera of a both-sides setup', () => {
    liveView.setup = {
      kind: 'splitLine',
      camera: 'cam-a',
      line: { from: { x: 1280, y: 360 }, to: { x: 0, y: 360 } },
    };

    render(CameraOverlay, { camera: 'cam-a', video: fakeVideo(1280, 720).video });
    render(CameraOverlay, { camera: 'cam-b', video: fakeVideo(1280, 720).video });

    expect(screen.getAllByRole('group', { name: 'Split line' })).toHaveLength(1);
    expect(screen.getByText('Player A')).toBeTruthy();
    expect(screen.getByText('Player B')).toBeTruthy();
  });
});
