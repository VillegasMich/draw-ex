// Dev only: a scripted duel on a 1920×1080 "both sides" camera, with the three canned
// cards the stub `get_card` knows. The default split line puts Player A at the bottom.
import type {
  CandidateDto,
  DetectionDto,
  Passcode,
  Point,
  Quad,
  RecognitionDto,
  TrackEvent,
} from '../../lib/bindings';

export const DEMO_CAMERA = 'demo-camera';
export const DEMO_FRAME = { width: 1920, height: 1080 };

const MONSTER_REBORN = 83764719;
const ASH_BLOSSOM = 14558127;
const POT_OF_GREED = 55144522;

/** One moment of the script: the detections from then on, and the track events then. */
export interface DemoStep {
  /** Milliseconds after the demo starts. */
  atMs: number;
  detections: DetectionDto[];
  /** Track events without `timeMs`, which is stamped when the step plays. */
  events: DemoEvent[];
}

type Untimed<T> = T extends unknown ? Omit<T, 'timeMs'> : never;
export type DemoEvent = Untimed<TrackEvent>;

/** A card-sized quad (59:86) centered at `center`, turned by `degrees` clockwise. */
function card(center: Point, degrees = 0, height = 300): Quad {
  const halfW = (height * 59) / 86 / 2;
  const halfH = height / 2;
  const angle = (degrees * Math.PI) / 180;
  const [cos, sin] = [Math.cos(angle), Math.sin(angle)];
  const corner = (x: number, y: number): Point => ({
    x: center.x + x * cos - y * sin,
    y: center.y + x * sin + y * cos,
  });
  return [
    corner(-halfW, -halfH),
    corner(halfW, -halfH),
    corner(halfW, halfH),
    corner(-halfW, halfH),
  ];
}

function seen(quad: Quad, passcode: Passcode | null, score = 0.9): DetectionDto {
  const candidates: CandidateDto[] = passcode === null ? [] : [{ passcode, score }];
  const recognition: RecognitionDto =
    passcode === null
      ? { kind: 'unreadable' }
      : score < 0.5
        ? { kind: 'lowConfidence' }
        : { kind: 'recognized', passcode, score };
  return { quad, detectorConfidence: 0.9, candidates, recognition };
}

// Player A's cards are below the middle of the frame, Player B's above it.
const REBORN_A = card({ x: 520, y: 800 }, -8);
const POT_A = card({ x: 1380, y: 790 }, 4);
const ASH_B = card({ x: 960, y: 270 }, 180);
const POT_B = card({ x: 1450, y: 260 }, 172);
const FACE_DOWN = card({ x: 420, y: 260 }, 90, 260);

export const DEMO_SCRIPT: DemoStep[] = [
  { atMs: 0, detections: [seen(REBORN_A, MONSTER_REBORN, 0.3), seen(FACE_DOWN, null)], events: [] },
  {
    atMs: 1200,
    detections: [seen(REBORN_A, MONSTER_REBORN), seen(FACE_DOWN, null)],
    events: [{ kind: 'confirmed', track: 1, player: 'a', passcode: MONSTER_REBORN }],
  },
  {
    atMs: 2600,
    detections: [seen(REBORN_A, MONSTER_REBORN), seen(ASH_B, ASH_BLOSSOM), seen(FACE_DOWN, null)],
    events: [{ kind: 'confirmed', track: 2, player: 'b', passcode: ASH_BLOSSOM }],
  },
  {
    // A misread: Player A's new card is first confirmed as Ash Blossom...
    atMs: 4000,
    detections: [
      seen(REBORN_A, MONSTER_REBORN),
      seen(ASH_B, ASH_BLOSSOM),
      seen(POT_A, ASH_BLOSSOM),
      seen(FACE_DOWN, null),
    ],
    events: [{ kind: 'confirmed', track: 3, player: 'a', passcode: ASH_BLOSSOM }],
  },
  {
    // ...then corrected to Pot of Greed. It stays featured.
    atMs: 5600,
    detections: [
      seen(REBORN_A, MONSTER_REBORN),
      seen(ASH_B, ASH_BLOSSOM),
      seen(POT_A, POT_OF_GREED),
      seen(FACE_DOWN, null),
    ],
    events: [{ kind: 'changed', track: 3, player: 'a', from: ASH_BLOSSOM, to: POT_OF_GREED }],
  },
  {
    atMs: 7000,
    detections: [
      seen(REBORN_A, MONSTER_REBORN),
      seen(ASH_B, ASH_BLOSSOM),
      seen(POT_A, POT_OF_GREED),
      seen(POT_B, POT_OF_GREED),
      seen(FACE_DOWN, null),
    ],
    events: [{ kind: 'confirmed', track: 4, player: 'b', passcode: POT_OF_GREED }],
  },
  {
    // Player B's Ash Blossom is picked up; once the grace period passes, it is lost.
    atMs: 8200,
    detections: [
      seen(REBORN_A, MONSTER_REBORN),
      seen(POT_A, POT_OF_GREED),
      seen(POT_B, POT_OF_GREED),
      seen(FACE_DOWN, null),
    ],
    events: [],
  },
  {
    atMs: 9400,
    detections: [
      seen(REBORN_A, MONSTER_REBORN),
      seen(POT_A, POT_OF_GREED),
      seen(POT_B, POT_OF_GREED),
      seen(FACE_DOWN, null),
    ],
    events: [{ kind: 'lost', track: 2, player: 'b', passcode: ASH_BLOSSOM }],
  },
  {
    // Player A's featured Pot of Greed leaves too: Monster Reborn is featured again.
    atMs: 10800,
    detections: [seen(REBORN_A, MONSTER_REBORN), seen(POT_B, POT_OF_GREED), seen(FACE_DOWN, null)],
    events: [],
  },
  {
    atMs: 12000,
    detections: [seen(REBORN_A, MONSTER_REBORN), seen(POT_B, POT_OF_GREED), seen(FACE_DOWN, null)],
    events: [{ kind: 'lost', track: 3, player: 'a', passcode: POT_OF_GREED }],
  },
];
