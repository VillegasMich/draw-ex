// Not imported by the app. `pnpm check` type-checks this file, which proves the
// frontend can reach the whole IPC contract: every command with correctly typed
// arguments, every event, and the hand-typed `submit_frame` wrapper.
import { commands, events } from './bindings';
import type { CameraSetup, DetectionDto, Settings, TrackEvent } from './bindings';
import { cardImageUrl } from './card-image';
import { submitFrame } from './frames';

export async function checkContract(): Promise<void> {
  const setup: CameraSetup = {
    kind: 'splitLine',
    camera: 'cam-1',
    line: { from: { x: 1280, y: 360 }, to: { x: 0, y: 360 } },
  };
  await commands.setCameraSetup(setup);
  const session = await commands.startSession();
  const summary = await commands.endSession();
  const card = await commands.getCard(83764719, 'en');
  const decks = await commands.listDecks();
  const imported = await commands.importDeck('/path/chosen/by/operator.ydk');
  await commands.selectDeck(imported.id);
  await commands.selectDeck(null);
  const settings: Settings = await commands.getSettings();
  await commands.updateSettings({ ...settings, inferenceFps: 8 });
  const status = await commands.modelStatus();
  await commands.downloadModels(status.current);
  const stats = await commands.cacheStats();
  await commands.clearCache('images');
  await commands.syncCardDb();

  const detections: DetectionDto[] = await submitFrame(
    'cam-1',
    new ImageData(2, 2),
    performance.now(),
  );

  await events.trackEvent.listen(({ payload }) => {
    const event: TrackEvent = payload;
    void event;
  });
  await events.modelDownloadProgress.listen(({ payload }) => void payload.bytesDone);
  await events.cardDbSyncProgress.listen(({ payload }) => void payload.stage);

  void [
    session.startedAtMs,
    summary.logPath,
    card?.name,
    decks,
    stats.imagesBytes,
    detections,
    cardImageUrl(14558127, 'small'),
  ];
}
