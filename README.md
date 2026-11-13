# draw-ex

A desktop app (Windows `.exe`, macOS `.dmg`) that shows a live, augmented view of a
real-life _Yu-Gi-Oh!_ duel. The camera footage sits in the center of the window; the side
panels show the cards the [DRAW 2](https://github.com/HichTala/draw2) model recognizes on
the table, with their artwork, type, stats and effect text.

> **Status:** design phase. This README records the agreed design; no code exists yet.

---

## Goals

- **Live view**: camera footage in the center at the camera's native frame rate, never
  held back by inference.
- **Recognition**: detect and identify cards on the mat with the DRAW 2 models, running
  locally (no Python, no server).
- **Card info**: show name, artwork, type, ATK/DEF, level/link and effect text for every
  confirmed card, per player.
- **Offline-first**: card data and images are downloaded once and cached; a duel runs
  without internet after the first sync.
- **Camera setups**, all supported from the first version:
  - one camera covering **one player's side** only;
  - one camera covering **both sides** (cards assigned to a player by which side of a
    split line they are on);
  - **two cameras**, one per player.
- **Duel log**: a timeline of every confirmed card (who, when, which card), saved per
  session.
- **Recording** (later milestone): save the composited view (footage + panels) as a video.

### Non-goals for now

- Streaming outputs (virtual camera, NDI, OBS plugin). The app's own window is the output;
  OBS window capture works but is not designed for.
- Rules enforcement, life-point automation or game-state reasoning.
- Mapping cards to specific mat zones (monster zone, GY, …); see [Roadmap](#roadmap).

---

## Tech stack

| Area | Technology | Why |
|---|---|---|
| App shell and packaging | [Tauri v2](https://v2.tauri.app/) | Rust backend + web UI; bundles Windows NSIS `.exe` and macOS `.dmg` |
| Backend language | Rust (stable), `tokio` async runtime | Performance for inference and I/O; Tauri's native language |
| Frontend | [Svelte 5](https://svelte.dev/) + TypeScript + Vite | Small, fast reactive panels; polished overlay with web tech |
| IPC types | [`tauri-specta`](https://github.com/specta-rs/tauri-specta) | Generates TS types and command wrappers from Rust, so frontend and backend cannot drift apart |
| Camera capture | `getUserMedia` in the webview; `nokhwa` as fallback | Smooth native-rate preview; multiple cameras by device ID |
| Card recognition | `draw-core` crate from the draw2 fork | Single owner of the recognition logic; see [Recognition](#recognition-draw-core-owned-by-the-draw2-fork) |
| Inference runtime | [ONNX Runtime](https://onnxruntime.ai/) via [`ort`](https://ort.pyke.io/): CoreML (macOS), DirectML (Windows), CPU fallback | Runs the DRAW 2 ONNX exports with GPU acceleration on both platforms |
| Models | DRAW 2 YOLO-OBB detector + ViT classifier, ONNX, from [Hugging Face](https://huggingface.co/HichTala/draw2) | Already trained and exported for the web demo |
| Card data | [YGOPRODeck API v7](https://ygoprodeck.com/api-guide/), `reqwest` HTTP client | Free, complete card database with images |
| Local storage | SQLite via `rusqlite`; images as files on disk | Offline-first card DB and image cache |
| Rate limiting | `governor` | One shared limiter keeps YGOPRODeck traffic under its 20 req/s limit |
| Serialization | `serde`, `serde_json` | IPC payloads, settings, duel logs |
| Duel log | JSON Lines files | Append-only, easy to inspect with `jq` |
| Recording (later) | Canvas compositing + `MediaRecorder` in the webview | Records footage and panels exactly as shown |
| CI and releases | GitHub Actions on macOS and Windows runners | Native builds, signing and notarization |

---

## Architecture

```
┌──────────────────────────── Tauri v2 app ────────────────────────────┐
│                                                                      │
│  Frontend (webview, TypeScript)              Rust backend            │
│  ┌──────────────────────────────┐            ┌─────────────────────┐ │
│  │ getUserMedia ─► <video>      │  frame     │ draw-core  (dep,    │ │
│  │   (display, native fps)      │──(RGBA)──► │  from draw2 fork)   │ │
│  │                              │  ~5–10 fps │  YOLO ─► warp ─► ViT│ │
│  │ overlay canvas (boxes)       │◄─────────  │ tracker + voting    │ │
│  │ side panels (per player)     │ detections │ side assignment     │ │
│  │ duel log / settings          │  + events  ├─────────────────────┤ │
│  │                              │            │ card-db             │ │
│  │ card images ◄────────────────┼─ local ────│  SQLite card data   │ │
│  │   (served from disk cache)   │  protocol  │  image cache        │ │
│  └──────────────────────────────┘            │  YGOPRODeck client  │ │
│                                              └─────────────────────┘ │
└──────────────────────────────────────────────────────────────────────┘
```

### Why Tauri v2

Rust owns the heavy work (inference, tracking, cache), and a web frontend makes the
overlay panels easy to make look polished. Tauri's bundler produces the Windows installer
and the macOS `.dmg` directly.

### Video and frame flow

- The **webview captures the camera** (`getUserMedia`, one stream per camera) and shows it
  in a `<video>` element, so display stays smooth no matter how slow inference is.
- At the inference rate (target 5–10 fps per camera), the frontend grabs the current
  frame and sends its raw pixels to Rust over Tauri's binary IPC.
- **Backpressure**: at most one frame per camera is in flight. While Rust is busy, frames
  are skipped rather than queued, so results never fall behind the live feed.
- Rust replies with detections in source-frame pixel coordinates plus the frame's
  timestamp. The frontend draws boxes on a canvas over the video, and the panels update
  from track events.
- _Fallback if webview capture proves unreliable_: capture in Rust (`nokhwa`) and push
  preview frames to the webview. This is more work and costs more bandwidth, so it is not
  the default.

### Recognition: `draw-core`, owned by the draw2 fork

Card recognition **does not live in this repo**. It is the `draw-core` Rust crate in the
draw2 fork ([`VillegasMich/draw2`](https://github.com/VillegasMich/draw2), under
`crates/draw-core/`). That keeps the model and the code that runs it together, in one
place. draw-ex uses it as a Cargo git dependency pinned to a commit or tag:

```toml
draw-core = { git = "https://github.com/VillegasMich/draw2", tag = "draw-core-v0.1.0" }
```

End users never see draw2. The crate is compiled into the draw-ex binary, and the model
weights download on first run (see [Models](#models)).

**Responsibilities**

| draw-core (draw2 fork) | draw-ex (this repo) |
|---|---|
| Load the ONNX sessions and pick the execution provider | Download models with progress UI, store them on disk |
| Model manifest: HF repo, pinned revision, file names, SHA-256 per variant | Camera capture, frame IPC, backpressure |
| Frame in → recognized cards out (stateless, one frame at a time) | Tracking and voting across frames, player assignment |
| Deck-list filtering of candidates | Card data, image cache, UI, duel log |
| Golden tests against the Python and JS pipelines | End-to-end tests of the app |

draw-core keeps the manifest because each crate version only works with the model files
it was written for. draw-ex reads the manifest to know what to download, so the two can
never drift apart.

**Pipeline**

draw-core ports the DRAW 2 **browser** pipeline (`docs/scripts/pipeline.js` in draw2), not
the Python one. The browser version is newer, does not depend on fragile text-box contour
detection, and already runs the ONNX exports.

1. **Detect**: letterbox the frame to 640×640 and run `ygo_yolo.onnx` (YOLO-OBB). This
   outputs rotated boxes; keep those with confidence ≥ 0.20, force them into the card's
   portrait orientation, and apply NMS.
2. **Warp**: perspective-warp each box to a 224×224 crop.
3. **Orient**: guess 0° or 180° from which end holds the bright text box. If the top-1
   confidence is below 0.15, also try the other rotation and keep it only if it wins by
   at least 1.25×.
4. **Classify**: run the ViT (normalize with mean = std = 0.5, softmax) and take the top-k
   candidates.
5. **Filter**: when a deck list (`.ydk`) is loaded, take the best candidate that is in the
   deck. Deck lists improve accuracy a lot.

Inference runs on [ONNX Runtime](https://onnxruntime.ai/) through the
[`ort`](https://ort.pyke.io/) crate. It uses **CoreML** on macOS and **DirectML** on
Windows (CUDA optional), and falls back to CPU.

### Tracking and confirmation (draw-ex)

A single frame is noisy: a card may be misread for a frame or briefly not detected at
all. Detections are therefore associated across frames into **tracks**, by box overlap
and center distance, separately for each camera. Each track keeps a weighted vote over
candidate card IDs. A card is **confirmed** when its leading label has enough votes and a
clear margin over the runner-up. Only confirmed cards reach the side panels and the duel
log. A track that disappears for a grace period is closed.

### Player assignment

| Setup | Rule |
|---|---|
| 1 camera, one side | Every card belongs to the configured player |
| 1 camera, both sides | A split line (draggable in the UI, horizontal by default) divides the frame between player A and player B |
| 2 cameras | Each camera is assigned to a player |

### Models

The ONNX files are hosted on Hugging Face at
[`HichTala/draw2`](https://huggingface.co/HichTala/draw2/tree/main/onnx) under `onnx/`:

| File | Size | Use |
|---|---|---|
| `ygo_yolo.onnx` | 39 MB | Detector (always needed) |
| `vit_fp32.onnx` | 386 MB | Classifier, full precision |
| `vit_fp16.onnx` | 193 MB | Classifier, half precision |
| `vit_int8.onnx` | 98 MB | Classifier, INT8 (CPU-friendly) |
| `vit_yugiscan_int8.onnx` | 98 MB | Alternative classifier; labels in `card_labels_yugiscan.json` |
| `cardnames_onnx.json` | 1.8 MB | ViT output index → `{EN, FR, JA, card_id, label}` |

- Models are **downloaded on first run**, not bundled in the installer, into the app's
  data directory. A progress UI shows the download.
- Which files to fetch comes from **draw-core's model manifest**. Downloads are pinned to
  a specific Hugging Face **commit**, not `main`, and checked against a SHA-256.
- The default ViT variant will be chosen from benchmarks on real hardware. The user can
  switch variants in settings.

### Card data and cache (`card-db`)

Card information comes from the free [YGOPRODeck API v7](https://ygoprodeck.com/api-guide/).
Its rules shape the design:

- **Rate limit**: 20 requests/second. Exceeding it blocks the IP for 1 hour.
- **Store data locally** and keep API calls to a minimum, or the IP gets blacklisted.
- **Images**: download each image once, store it locally, and never hotlink. Images
  exist only in English.

Design:

- **Card database**: one bulk download of `cardinfo.php` into a local SQLite file,
  including the localized names and text (`language=fr|de|it|pt`). On startup (at most
  once a day) the app calls `checkDBVer.php` and re-syncs only when `database_version`
  has changed.
- **Images**: fetched on demand the first time a card is shown, then served from disk
  forever. Three sizes are available: `cards/`, `cards_small/` and `cards_cropped/` (the
  cropped artwork is missing for some cards). When a deck list is loaded, its card images
  are prefetched.
- **One global rate limiter** guards every request to YGOPRODeck, with headroom below
  the 20/s limit. The frontend never loads YGOPRODeck URLs directly: images reach the
  webview from the local cache.
- **Cache management**: a settings screen shows the cache size, with buttons to clear
  images, re-sync data, or delete models. The image cache has a size cap with LRU
  eviction.
- **Card identity**: everything is keyed by **passcode** (the 8-digit number on the card,
  which is YGOPRODeck's `id` and the suffix of the ViT label `Card-Name-<passcode>`).
  Passcodes are normalized as integers, because some sources pad them with leading zeros.

### Data on disk

All paths are under the OS app-data directory resolved by Tauri:

```
models/      ONNX files + label JSON + manifest (revision, sha256)
cards.db     SQLite: card data, sync metadata
images/      full/ small/ cropped/  <passcode>.jpg
decks/       imported .ydk files
duels/       one JSONL log per session (+ recordings, later)
settings.json
```

---

## Planned repository layout

```
Cargo.toml          workspace; depends on draw-core (git, pinned)
crates/
  tracker/          tracks, voting, confirmation, player assignment
  card-db/          YGOPRODeck client, SQLite store, image cache, rate limiter
src-tauri/          Tauri app: commands, state, model download, packaging config
src/                frontend (TypeScript + Vite)

draw2 fork (separate repo)
crates/draw-core/   inference pipeline + model manifest + golden tests
```

`tracker` and `card-db` stay independent of Tauri, so they can be tested and run
headless.

---

## Roadmap

The full ticket plan, built for several agents working in parallel branches, is in
[`docs/ROADMAP.md`](docs/ROADMAP.md). It lists each ticket's owned paths, dependencies
and done-criteria. In short:

0. **Foundation** (serial): workspace scaffold, then contracts and stubs (the draw-core
   API, the IPC commands and events, and the crate APIs, all with fake implementations).
1. **Parallel streams**: draw-core port (draw2 fork), card-db, tracker, and app/frontend.
   Each stream builds against the Phase 0 fakes.
2. **Integration**: wire in the real components, then tune on real duels.
3. **Release**: packaged installers from CI, then signing and notarization.
4. **Later**: recording the composited view, mat-zone mapping (with a calibration step),
   life-point counter, streaming outputs.

---

## Open questions

- **Default ViT variant and inference rate per platform**: decided by benchmarks.
- **Duel log sharing**: whether to also export a human-readable summary.
- **Code signing**: only needed to distribute installers to other people (see
  [Distribution and signing](#distribution-and-signing)). Whether to pay for it is
  undecided.

## Distribution and signing

Signing proves to the operating system who built an installer and that nobody modified
it. Without it, the app still works, but other people's machines warn about it or block
it:

| | Unsigned installer | Signed | Cost |
|---|---|---|---|
| **macOS** | Gatekeeper blocks a downloaded app. Users must allow it under System Settings → Privacy & Security → "Open Anyway" | Opens normally once also **notarized** (scanned by Apple) | Apple Developer Program, $99/year |
| **Windows** | SmartScreen shows "Windows protected your PC". Users click "More info → Run anyway". Some antivirus tools may flag it | No warning, or a warning that fades as downloads build reputation | Code-signing certificate, about $100–400/year depending on the provider |

**Signing is not needed** for development, for builds you make and run on your own
machine, or for a few testers who can click past the warnings. Unsigned macOS builds
still get an ad-hoc signature, which Apple Silicon requires to run at all; Tauri applies
it automatically.

**Signing becomes worth it** when the app is shared publicly with non-technical users.

**Side effect on macOS**: camera permission is tied to the app's signing identity. Ad-hoc
builds can lose the permission after an update, so users get asked again; a stable
Developer ID keeps it across updates.

---

## Development

Not set up yet. Planned prerequisites: Rust (stable), Node.js, and the
[Tauri v2 prerequisites](https://v2.tauri.app/start/prerequisites/) for your OS. Commands
will be documented here once the scaffold lands.

**Working on draw-core and draw-ex together**: clone the draw2 fork next to this repo
(`../draw2`) and point Cargo at the local copy. Use a `.cargo/config.toml`, which is
gitignored and therefore never committed:

```toml
[patch."https://github.com/VillegasMich/draw2"]
draw-core = { path = "../draw2/crates/draw-core" }
```

Delete it to build against the pinned version again. Releases always build against the
pinned git dependency.

---

## Credits and legal

- Card detection and recognition models: [DRAW 2](https://github.com/HichTala/draw2) by
  HichTala, AGPL-3.0.
- Card data and images: [YGOPRODeck](https://ygoprodeck.com/) API.
- _Yu-Gi-Oh!_ card names, text and images are © Konami / 4K Media Inc. This is an
  unofficial fan project, not affiliated with or endorsed by Konami, 4K Media or
  YGOPRODeck.

## License

[GNU Affero General Public License v3.0](https://www.gnu.org/licenses/agpl-3.0.html), the
same as DRAW 2, which this app links through `draw-core`. A `LICENSE` file will be added with the
first code.
