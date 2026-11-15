import { render, screen, within } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import App from './App.svelte';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(async () => null) }));

/** A live video track that can be ended the way an unplugged camera ends it. */
class FakeTrack extends EventTarget {
  readonly kind = 'video';
  readyState: MediaStreamTrackState = 'live';
  stop = vi.fn(() => {
    this.readyState = 'ended';
  });
  unplug() {
    this.readyState = 'ended';
    this.dispatchEvent(new Event('ended'));
  }
}

interface FakeMedia {
  getUserMedia: ReturnType<typeof vi.fn>;
  tracks: FakeTrack[];
}

function camera(deviceId: string, label: string): MediaDeviceInfo {
  return { deviceId, label, kind: 'videoinput', groupId: '', toJSON: () => ({}) };
}

const CAMERAS = [camera('cam-a', 'Desk camera'), camera('cam-b', 'Overhead camera')];

function installMedia(devices: MediaDeviceInfo[] = CAMERAS): FakeMedia {
  const tracks: FakeTrack[] = [];
  const getUserMedia = vi.fn(async () => {
    const track = new FakeTrack();
    tracks.push(track);
    return { getTracks: () => [track], getVideoTracks: () => [track] };
  });
  const events = new EventTarget();
  Object.defineProperty(navigator, 'mediaDevices', {
    configurable: true,
    value: {
      getUserMedia,
      enumerateDevices: vi.fn(async () => devices),
      addEventListener: events.addEventListener.bind(events),
      removeEventListener: events.removeEventListener.bind(events),
    },
  });
  return { getUserMedia, tracks };
}

function rememberSetup(setup: object) {
  localStorage.setItem('draw-ex.camera-setup', JSON.stringify(setup));
}

async function cameraRegions(): Promise<HTMLElement[]> {
  return screen.findAllByRole('region', { name: /^Camera for / });
}

describe('App layout', () => {
  beforeEach(() => localStorage.clear());
  afterEach(() => vi.clearAllMocks());

  it('shows the Player A, Camera and Player B regions in that order', () => {
    installMedia();
    render(App);

    const playerA = screen.getByRole('complementary', { name: 'Player A' });
    const camera = screen.getByRole('main', { name: 'Camera' });
    const playerB = screen.getByRole('complementary', { name: 'Player B' });

    expect(playerA.compareDocumentPosition(camera)).toBe(Node.DOCUMENT_POSITION_FOLLOWING);
    expect(camera.compareDocumentPosition(playerB)).toBe(Node.DOCUMENT_POSITION_FOLLOWING);
    expect(within(camera).getByRole('button', { name: 'Setup' })).toBeTruthy();
  });

  it('shows one video region for a one-camera setup', async () => {
    installMedia();
    rememberSetup({ kind: 'oneSide', camera: 'cam-a', player: 'b' });
    render(App);

    const regions = await cameraRegions();

    expect(regions.map((region) => region.getAttribute('aria-label'))).toEqual([
      'Camera for Player B',
    ]);
    expect(regions[0].querySelector('video')).not.toBeNull();
  });

  it("stacks two cameras with Player B's on top", async () => {
    installMedia();
    rememberSetup({ kind: 'twoCameras', a: 'cam-a', b: 'cam-b' });
    render(App);

    const regions = await cameraRegions();

    expect(regions.map((region) => region.getAttribute('aria-label'))).toEqual([
      'Camera for Player B',
      'Camera for Player A',
    ]);
    expect(regions.every((region) => region.querySelector('video'))).toBe(true);
  });

  it('shows why the camera cannot be opened instead of a video', async () => {
    const media = installMedia();
    media.getUserMedia.mockRejectedValue(new DOMException('', 'NotAllowedError'));
    rememberSetup({ kind: 'splitLine', camera: 'cam-a' });
    render(App);

    expect(await screen.findByText(/Camera access was denied/)).toBeTruthy();
    expect(document.querySelector('video')).toBeNull();
  });

  it('replaces the video with "Camera disconnected" and a retry when the camera is unplugged', async () => {
    const media = installMedia();
    rememberSetup({ kind: 'splitLine', camera: 'cam-a' });
    render(App);
    const [region] = await cameraRegions();
    await vi.waitFor(() => expect(region.querySelector('video')).not.toBeNull());

    media.tracks.at(-1)?.unplug();

    expect(await within(region).findByText('Camera disconnected.')).toBeTruthy();
    expect(within(region).getByRole('button', { name: 'Retry' })).toBeTruthy();
    expect(region.querySelector('video')).toBeNull();
  });
});
