import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { CameraId, DetectionDto } from '../lib/bindings';
import { submitFrame, type FramePixels } from '../lib/frames';
import { liveView } from '../overlay/live-view.svelte';
import { FrameCapture } from './capture.svelte';

vi.mock('../lib/frames', () => ({ submitFrame: vi.fn() }));

const submit = vi.mocked(submitFrame);

/** A `submitFrame` call the test answers when it wants to. */
interface PendingSubmit {
  camera: CameraId;
  frame: FramePixels;
  resolve: (detections: DetectionDto[]) => void;
  reject: (error: unknown) => void;
}

let calls: PendingSubmit[];

/** A frame source that always has a 2 × 1 frame ready. */
function source(): () => FramePixels {
  return () => ({ width: 2, height: 1, data: new Uint8ClampedArray(8) });
}

function detection(x: number): DetectionDto {
  return {
    quad: [
      { x, y: 0 },
      { x: x + 1, y: 0 },
      { x: x + 1, y: 1 },
      { x, y: 1 },
    ],
    detectorConfidence: 0.9,
    candidates: [],
    recognition: { kind: 'unreadable' },
  };
}

/** Lets the promise callbacks of answered submits run. */
async function settle() {
  await vi.advanceTimersByTimeAsync(0);
}

beforeEach(() => {
  vi.useFakeTimers();
  calls = [];
  submit.mockImplementation(
    (camera, frame) =>
      new Promise((resolve, reject) => {
        calls.push({ camera, frame, resolve, reject });
      }),
  );
});

afterEach(() => {
  vi.useRealTimers();
  vi.clearAllMocks();
});

describe('FrameCapture', () => {
  it('keeps one frame in flight per camera and skips ticks while it is pending', async () => {
    const capture = new FrameCapture();
    const stop = capture.start('cam-1', source(), 10);

    await vi.advanceTimersByTimeAsync(100);
    expect(calls).toHaveLength(1);
    expect(capture.stats('cam-1')).toMatchObject({ sent: 1, skipped: 0, inFlight: 1 });

    await vi.advanceTimersByTimeAsync(300);
    expect(calls).toHaveLength(1);
    expect(capture.stats('cam-1')).toMatchObject({ sent: 1, skipped: 3, inFlight: 1 });

    calls[0].resolve([]);
    await settle();
    expect(capture.stats('cam-1').inFlight).toBe(0);

    await vi.advanceTimersByTimeAsync(100);
    expect(calls).toHaveLength(2);
    expect(capture.stats('cam-1')).toMatchObject({ sent: 2, skipped: 3, inFlight: 1 });
    stop();
  });

  it('captures cameras independently', async () => {
    const capture = new FrameCapture();
    const stopA = capture.start('cam-a', source(), 10);
    const stopB = capture.start('cam-b', source(), 10);

    await vi.advanceTimersByTimeAsync(100);
    expect(calls.map((call) => call.camera)).toEqual(['cam-a', 'cam-b']);

    // Answering cam-b frees only cam-b; cam-a keeps skipping until its own answer.
    calls[1].resolve([]);
    await vi.advanceTimersByTimeAsync(100);
    expect(calls.map((call) => call.camera)).toEqual(['cam-a', 'cam-b', 'cam-b']);
    expect(capture.stats('cam-a')).toMatchObject({ sent: 1, skipped: 1, inFlight: 1 });
    expect(capture.stats('cam-b')).toMatchObject({ sent: 2, skipped: 0, inFlight: 1 });
    stopA();
    stopB();
  });

  it('sends frames at the given rate when every frame is answered at once', async () => {
    submit.mockResolvedValue([]);
    const capture = new FrameCapture();
    const stop = capture.start('cam-1', source(), 6);

    await vi.advanceTimersByTimeAsync(1000);
    expect(capture.stats('cam-1')).toMatchObject({ sent: 6, skipped: 0, inFlight: 0 });

    await vi.advanceTimersByTimeAsync(9000);
    expect(capture.stats('cam-1').sent).toBe(60);
    stop();
  });

  it('falls back to the default rate for a rate that makes no sense', async () => {
    submit.mockResolvedValue([]);
    const capture = new FrameCapture();
    const stop = capture.start('cam-1', source(), 0);

    await vi.advanceTimersByTimeAsync(1000);
    expect(capture.stats('cam-1').sent).toBe(6);
    stop();
  });

  it('sends each frame at full size with its capture time', async () => {
    const frame = { width: 1920, height: 1080, data: new Uint8ClampedArray(1920 * 1080 * 4) };
    const capture = new FrameCapture();
    const stop = capture.start('cam-1', () => frame, 10);

    await vi.advanceTimersByTimeAsync(100);
    expect(submit).toHaveBeenCalledWith('cam-1', frame, expect.any(Number));
    stop();
  });

  it('sends nothing while the source has no frame', async () => {
    const capture = new FrameCapture();
    const stop = capture.start('cam-1', () => null, 10);

    await vi.advanceTimersByTimeAsync(500);
    expect(calls).toHaveLength(0);
    expect(capture.stats('cam-1')).toMatchObject({ sent: 0, skipped: 0 });
    stop();
  });

  it('puts each response into liveView under the camera that sent the frame', async () => {
    const capture = new FrameCapture();
    const stopA = capture.start('cam-a', source(), 10);
    const stopB = capture.start('cam-b', source(), 10);
    await vi.advanceTimersByTimeAsync(100);

    calls[1].resolve([detection(20)]);
    calls[0].resolve([detection(10)]);
    await settle();

    expect(liveView.detections('cam-a')).toEqual([detection(10)]);
    expect(liveView.detections('cam-b')).toEqual([detection(20)]);
    expect(capture.stats('cam-a').lastRoundTripMs).toEqual(expect.any(Number));
    stopA();
    stopB();
    liveView.forget('cam-a');
    liveView.forget('cam-b');
  });

  it('stops ticking when stopped and ignores a response that arrives afterwards', async () => {
    const capture = new FrameCapture();
    const stop = capture.start('cam-1', source(), 10);
    await vi.advanceTimersByTimeAsync(100);

    stop();
    calls[0].resolve([detection(10)]);
    await vi.advanceTimersByTimeAsync(1000);

    expect(calls).toHaveLength(1);
    expect(liveView.detections('cam-1')).toEqual([]);
    expect(capture.stats('cam-1')).toMatchObject({ sent: 1, skipped: 0, inFlight: 0 });
  });

  it('counts a failed frame and keeps capturing', async () => {
    const capture = new FrameCapture();
    const stop = capture.start('cam-1', source(), 10);
    await vi.advanceTimersByTimeAsync(100);

    calls[0].reject({ kind: 'internal', message: 'recognition failed on this frame' });
    await vi.advanceTimersByTimeAsync(100);

    expect(capture.stats('cam-1')).toMatchObject({ sent: 2, failed: 1, inFlight: 1 });
    stop();
  });

  it('waits for a frame still in flight from before a restart', async () => {
    const capture = new FrameCapture();
    capture.start('cam-1', source(), 10)();
    // Stopping between ticks sent nothing, so start again and send one frame.
    const stopFirst = capture.start('cam-1', source(), 10);
    await vi.advanceTimersByTimeAsync(100);
    stopFirst();

    const stop = capture.start('cam-1', source(), 10);
    await vi.advanceTimersByTimeAsync(300);
    expect(calls).toHaveLength(1);
    expect(capture.stats('cam-1')).toMatchObject({ skipped: 3, maxInFlight: 1 });

    calls[0].resolve([]);
    await vi.advanceTimersByTimeAsync(100);
    expect(calls).toHaveLength(2);
    stop();
  });

  it('shares one in-flight frame when the same camera is shown twice', async () => {
    const capture = new FrameCapture();
    const stopTop = capture.start('cam-1', source(), 10);
    const stopBottom = capture.start('cam-1', source(), 10);

    await vi.advanceTimersByTimeAsync(300);
    expect(calls).toHaveLength(1);

    // Removing one view keeps capturing from the other.
    stopTop();
    calls[0].resolve([]);
    await vi.advanceTimersByTimeAsync(100);
    expect(calls).toHaveLength(2);
    expect(capture.stats('cam-1').maxInFlight).toBe(1);
    stopBottom();
  });
});
