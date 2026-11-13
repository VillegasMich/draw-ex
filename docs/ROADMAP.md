# Implementation roadmap

This plan splits the work into tickets that separate agents can build at the same time,
each on its own branch. The design itself is in [`../README.md`](../README.md); this file
covers only order, ownership and done-criteria.

## How to read this

- **Phases run in order.** Tickets inside a phase run in parallel unless a ticket lists a
  dependency.
- **Repo**: `draw-ex` (this repo) or `draw2` (the fork
  [`VillegasMich/draw2`](https://github.com/VillegasMich/draw2), checked out at
  `../draw2`). Both repos are run with the Orca CLI: one worktree per ticket branch, in
  whichever repo the ticket names. F2 touches both repos, so it gets a `feat/contracts`
  worktree in each; merge the draw2 side and tag it first, because the draw-ex side pins
  that tag.
- **Owns** lists the paths a ticket may create or change. Tickets in the same phase own
  disjoint paths, so their branches merge without conflicts. Shared files are handled
  in [Conflict hotspots](#conflict-hotspots).
- **Done when** is the merge bar: every item is checked and CI is green.
- **Mocks keep the streams independent**: Phase 0 publishes the interfaces with fake
  implementations, so no Phase 1 ticket waits on another stream's real code.

```
Phase 0  F1 ─► F2a ─► F2b
                      │
Phase 1        ├─ A1 ─┐
  (parallel)   ├─ A2 ─┴─► A3                          draw2 fork
               ├─ B1 ─► B2      B3                    card-db
               ├─ C1    C2                            tracker
               └─ D1 ─► D2      D3    D4              app / frontend
                              │
Phase 2        I1 ─► I2       (needs A3 B1 B2 C1 C2 D2 D3)
Phase 3        R1 ─► R2
```

## Decisions made

- **Frontend framework**: Svelte 5 (F1).
- **TS bindings**: `tauri-specta` generates TypeScript types and command wrappers from
  the Rust IPC module (F2).

---

## Phase 0: Foundation (serial, one agent)

Everything else builds on these two tickets. Merge them to `main` before starting Phase 1.

### F1: Workspace scaffold
- **Repo**: draw-ex. **Branch**: `feat/scaffold`
- **Owns**: everything (first code in the repo).
- **Scope**:
  - Cargo workspace, Tauri v2 app (`src-tauri/`), Svelte 5 + TypeScript + Vite (`src/`).
  - Empty library crates `crates/tracker` and `crates/card-db`.
  - `.gitignore` (including `.cargo/config.toml`).
  - GitHub Actions CI on macOS + Windows: `cargo fmt --check`,
    `cargo clippy -- -D warnings`, `cargo test`, frontend typecheck + lint.
- **Done when**:
  - [ ] `tauri dev` opens a window with the empty three-column layout on macOS and
    Windows.
  - [x] CI is green.
  - [x] Build, test and run commands are added to `CLAUDE.md` and the README's
    Development section.

### F2: Contracts and stubs

F2 is split into two tickets because draw-ex pins a draw2 tag: **F2a blocks F2b**.
Vocabulary is defined in [`GLOSSARY.md`](../GLOSSARY.md). The boundaries are recorded in
[ADR-0001](adr/0001-recognition-lives-in-draw2.md),
[ADR-0002](adr/0002-domain-crate-and-tracker-input.md) and
[ADR-0003](adr/0003-frame-transport.md).

Contract rules for both tickets:

- Units and spaces:
  - coordinates are `f32` source-frame pixels;
  - passcodes are integers;
  - scores and confidences are 0–1;
  - timestamps are milliseconds as `f64` from the frontend's `performance.now()`.
- Every public item's doc comment states the unit or coordinate space it uses.

#### F2a: draw-core stub (draw2)
- **Repo**: draw2. **Branch**: `feat/contracts`. **Depends on**: F1.
- **Owns**:
  - the root `Cargo.toml` and `rust-toolchain.toml`, `crates/draw-core/`;
  - `.github/workflows/rust.yml`, a minimal draw2 `CLAUDE.md`, and Rust entries in
    `.gitignore`.
- **Scope**:
  - A Rust workspace alongside the untouched Python project. Rust 1.100.0, edition
    2024.
  - `rust.yml`: fmt, clippy `-D warnings` and test on macOS + Windows, run when
    `crates/**` or the Cargo files change.
  - Public API only:
    - `FrameRef<'a> { width, height, rgba }`: tightly packed RGBA8, row-major. The
      constructor validates the length.
    - `trait Recognize: Send { fn recognize(&mut self, frame: FrameRef<'_>) -> Result<Vec<Detection>, RecognizeError> }`.
    - `Recognizer::new(model_paths, options)`: returns a "not implemented" error in the
      stub.
    - `RecognizerOptions`: deck list passcodes, detector and score thresholds, top-k,
      model variant.
    - `Detection { quad, detector_confidence, candidates, recognition }`:
      - `quad`: corners top-left, top-right, bottom-right, bottom-left of the card read
        upright;
      - `candidates`: top-k sorted by score, highest first;
      - `recognition`: `Recognized(Candidate) | LowConfidence | NotInDeck | Unreadable`.
    - `Candidate { passcode: Passcode, score }`, with `Passcode(u32)`.
    - `ModelManifest` with **real pinned data**:
      - the HF repo and commit;
      - per `ModelVariant` (`Fp32 | Fp16 | Int8 | YugiscanInt8`), the files with sha256
        and size.
    - `FakeRecognizer` behind cargo feature `fake`: returns configured detections,
      optionally after a configured delay.
  - Merge with a merge commit, then the coordinator tags `draw-core-v0.0.1`.
- **Test seams**:
  - `FrameRef::new` rejects a buffer whose length doesn't match the dimensions;
  - `FakeRecognizer` returns its detections and honours its delay;
  - `Recognizer::new` returns the "not implemented" error;
  - every manifest variant has files, each with a 64-hex sha256 and a nonzero size.
- **Done when**:
  - [ ] `cargo fmt/clippy/test` pass locally and in `rust.yml` on macOS + Windows.
  - [ ] Tag `draw-core-v0.0.1` exists on draw2 `main`.

#### F2b: Contracts in draw-ex
- **Repo**: draw-ex. **Branch**: `feat/contracts`. **Depends on**: F2a (tag).
- **Owns**:
  - `Cargo.toml`, `crates/domain/` (new), `crates/tracker/src/lib.rs`,
    `crates/card-db/src/lib.rs`;
  - `src-tauri/`, `src/lib/` (bindings and the `submit_frame` wrapper), and the empty
    module stubs listed below;
  - `.github/workflows/ci.yml` (bindings check).
- **Scope**:
  - Depend on `draw-core` at tag `draw-core-v0.0.1`, with feature `fake`.
  - **`crates/domain`**: `Passcode(u32)`, `CameraId`, `Player { A, B }`, `Point`,
    `Quad`, all serde with camelCase keys.
  - **`tracker` public API** (it never depends on draw-core):
    - `Observation` (camera, quad, candidates, time), `Tracker`, `TrackEvent`
      (`Confirmed | Changed | Lost`, with player and passcode);
    - `CameraSetup` (`OneSide { player } | SplitLine { line } | TwoCameras { a, b }`);
    - `SplitLine { from, to }`: Player A is on the left of `from → to` as seen on
      screen. The default is a horizontal line through the middle, drawn right to left,
      so Player A is at the bottom.
    - `DuelLog`.
    - All bodies are `todo!()`.
  - **`card-db` public API**: `CardDb`, `Card`, `Language` (`En | Fr | De | It | Pt`),
    `ImageCache`, `ImageSize` (`Full | Small | Cropped`), `Deck`, with `thiserror`
    errors. All bodies are `todo!()`.
  - **IPC** in `src-tauri/src/ipc/`, with stub handlers returning canned data. They
    **never call the crates**.
    - **Commands**:
      - `submit_frame`: raw RGBA body plus headers for camera, width, height and
        timestamp. The response is the detections. It has a hand-written typed wrapper
        in `src/lib/`.
      - `set_camera_setup`, `start_session`, `end_session`, `get_card(passcode, language)`.
      - `list_decks`, `import_deck(path)`, `select_deck(id?)`.
      - `get_settings`, `update_settings`, `model_status`, `download_models(variant)`.
      - `cache_stats`, `clear_cache(kind)`, `sync_card_db`.
    - **Events**: `track-event`, `model-download-progress`, `card-db-sync-progress`.
    - **Settings**: language, model variant, inference fps (default 6), selected deck
      (optional), image cache cap in MB (default 1024).
    - **Card image URL format**: `cardimg://<size>/<passcode>`. Only the format is
      defined here; I1 serves it.
  - **Bindings**: `tauri-specta` exports `src/lib/bindings.ts` from a `cargo test` in
    `src-tauri`. The file is committed, and CI fails if it changes after `cargo test`.
  - **Pre-created empty modules**, one per Phase 1 ticket (see [Conflict
    hotspots](#conflict-hotspots)):
    - `crates/card-db/src/{client,limiter,db,sync,images,deck}`;
    - `crates/tracker/src/{track,vote,players,log}`;
    - `src-tauri/src/{pipeline,models,settings}`;
    - `src/{camera,layout,frames,overlay,panels,settings}`.
- **Test seams**:
  - every command handler returns its fake data through Tauri's mock runtime
    (`tauri::test`);
  - `submit_frame` rejects missing or invalid headers and a wrong body length;
  - `domain` and IPC types round-trip through serde with camelCase keys;
  - CI fails when `bindings.ts` is stale;
  - `pnpm check` type-checks a TS file that calls every command through the bindings.
- **Done when**:
  - [ ] draw-ex builds against the stub.
  - [ ] The frontend can call every command through the generated bindings and receives
    fake data.
  - [ ] The API doc comments state units and coordinate spaces: frame pixels, passcode as
    an integer, score in 0–1.

---

## Phase 1: Parallel workstreams

### Stream A: draw-core (draw2 fork)

Port of `docs/scripts/pipeline.js`. The porting notes and gotchas are in
[`../CLAUDE.md`](../CLAUDE.md#porting-notes-for-draw-core-work-done-in-draw2). The first
A ticket to start moves them into the draw2 repo's docs and leaves a pointer here.

#### A1: Detection
- **Branch**: `feat/draw-core-detect`. **Owns**: `crates/draw-core/src/detect/`, its
  tests and fixtures.
- **Scope**: 640 letterbox preprocessing, YOLO-OBB session, output parsing (both
  layouts), portrait normalization, OBB NMS.
- **Done when**:
  - [ ] On the draw2 sample images, the boxes match the Python pipeline's boxes within a
    set IoU tolerance, in a golden test.

#### A2: Crop and classify
- **Branch**: `feat/draw-core-classify`. **Owns**: `crates/draw-core/src/{warp,classify}/`,
  its tests and fixtures.
- **Scope**:
  - Homography warp to 224×224.
  - 0°/180° orientation with fallback.
  - ViT preprocessing, top-k, and loading both label formats.
  - Deck-list filtering.
- **Approach**: tests use fixture boxes saved from the Python pipeline, so this ticket
  does not wait on A1.
- **Done when**:
  - [ ] `sample_photo.jpg` with its fixture box yields Monster Reborn `83764719` as top-1.
  - [ ] The deck filter picks the best in-deck candidate from a top-15.

#### A3: Runtime, manifest, release
- **Branch**: `feat/draw-core-runtime`. **Depends on**: A1, A2.
- **Owns**: `crates/draw-core/src/{lib,runtime,manifest}.rs`, `examples/`.
- **Scope**:
  - Execution provider selection: CoreML, DirectML, CPU fallback.
  - The real manifest, with a pinned HF revision and sha256 for every variant.
  - Wire A1 + A2 into `Recognizer`.
  - A `bench` example that reports fps per variant on the sample video.
- **Done when**:
  - [ ] End-to-end recognition works on the sample video.
  - [ ] Benchmark numbers are recorded in the draw2 docs for at least one Mac.
  - [ ] Tagged `draw-core-v0.1.0`.

### Stream B: card-db (draw-ex)

#### B1: API client and card database
- **Branch**: `feat/card-db-sync`. **Owns**: `crates/card-db/src/{client,limiter,db,sync}/`,
  `crates/card-db/tests/fixtures/`.
- **Scope**:
  - A `governor` rate limiter shared by every request, with headroom below 20 req/s.
  - `checkDBVer.php`; bulk `cardinfo.php` for EN + fr/de/it/pt.
  - SQLite schema and passcode normalization.
- **Done when**:
  - [ ] Sync runs against recorded fixtures (tests never hit the live API).
  - [ ] A second sync with an unchanged `database_version` makes exactly one request.
  - [ ] Lookup by passcode returns the localized fields.

#### B2: Image cache
- **Branch**: `feat/card-db-images`. **Depends on**: B1 (limiter + client).
- **Owns**: `crates/card-db/src/images/`.
- **Scope**:
  - On-demand fetch of the full, small and cropped images, each fetched once.
  - Disk layout `images/<size>/<passcode>.jpg`.
  - LRU size cap, prefetch for a passcode list, stats and clear.
- **Done when**:
  - [ ] A test proves a second request for the same image makes no network call.
  - [ ] A missing cropped image is handled.
  - [ ] The cap evicts the oldest files first.

#### B3: Deck lists
- **Branch**: `feat/decks`. **Owns**: `crates/card-db/src/deck/`.
- **Scope**: `.ydk` parsing (`#main`, `#extra`, `!side`, comments, leading zeros), saving
  imported decks, listing passcodes.
- **Done when**:
  - [ ] Real `.ydk` exports from YGOPRODeck and EDOPro parse into the expected passcode
    sets.

### Stream C: tracker (draw-ex)

#### C1: Tracks, voting, confirmation
- **Branch**: `feat/tracker-core`. **Owns**: `crates/tracker/src/{track,vote}/`.
- **Scope**:
  - Per-camera association of observations to tracks, by quad overlap and center distance.
  - A weighted vote over candidates.
  - A confirmation rule (minimum votes + margin) and a grace period before closing a
    track.
  - `TrackEvent::{Confirmed, Changed, Lost}`.
- **Done when**:
  - [ ] Synthetic sequences show: flicker does not create duplicate tracks; one misread
    frame does not change a confirmed card; a removed card emits `Lost`.

#### C2: Player assignment and duel log
- **Branch**: `feat/tracker-players`. **Owns**: `crates/tracker/src/{players,log}/`.
- **Scope**:
  - The three setups (single side, split line, two cameras).
  - Split-line geometry: any angle, set in frame coordinates.
  - A JSONL duel-log writer (session header + one event per line).
- **Done when**:
  - [ ] Unit tests cover each setup.
  - [ ] The log round-trips through serde.

### Stream D: app and frontend (draw-ex)

#### D1: Cameras and layout
- **Branch**: `feat/cameras`. **Owns**: `src/camera/`, `src/layout/`,
  `src-tauri/Info.plist`, entitlements.
- **Scope**:
  - Camera enumeration and setup picker (one side / both sides / two cameras).
  - `<video>` preview in the center column.
  - Permission-denied and camera-in-use errors.
  - macOS `NSCameraUsageDescription` + camera entitlement.
- **Done when**:
  - [ ] Each setup shows live preview on macOS and Windows.
  - [ ] Unplugging a camera shows an error instead of a frozen frame.

#### D2: Frame pipeline
- **Branch**: `feat/frame-pipeline`. **Depends on**: D1 (a `<video src=sample.mp4>`
  source is enough to start).
- **Owns**: `src/frames/`, `src-tauri/src/pipeline/`.
- **Scope**:
  - Grab frames at a configurable rate and send RGBA over binary IPC.
  - Keep at most one frame in flight per camera and drop the rest.
  - The Rust handler runs a `Recognize` impl (`FakeRecognizer` from F2a) on a worker thread and emits
    detections tagged with the frame timestamp.
- **Done when**:
  - [ ] With a recognizer that sleeps 500 ms, the preview stays at full fps, no frames
    queue up, and memory stays flat over 10 minutes.

#### D3: Overlay and panels
- **Branch**: `feat/overlay-panels`. **Owns**: `src/overlay/`, `src/panels/`.
- **Scope**:
  - Canvas boxes mapped through the video's letterbox.
  - A draggable split line.
  - Per-player panels: art, name, type, ATK/DEF, level/link, effect text.
  - A duel-log timeline.
- **Approach**: everything runs on fake detections, track events and card data from the
  F2 bindings.
- **Done when**:
  - [ ] Boxes line up with the cards at any window size.
  - [ ] Panels update on fake `Confirmed` and `Lost` events.

#### D4: First run, settings, cache screens
- **Branch**: `feat/settings-models`. **Owns**: `src/settings/`, `src-tauri/src/models/`,
  `src-tauri/src/settings/`.
- **Scope**:
  - A first-run model download driven by the manifest: progress, resume, sha256 check.
  - Settings: card language, ViT variant, inference rate, deck selection.
  - A cache screen: sizes, clear images, re-sync data, delete models.
- **Done when**:
  - [ ] An interrupted download resumes.
  - [ ] A corrupted file is detected and fetched again.
  - [ ] Settings persist across restarts.

---

## Phase 2: Integration

#### I1: Wire the real components
- **Branch**: `feat/integration`.
- **Depends on**: A3, B1, B2, C1, C2, D2, D3.
- **Scope**:
  - Pin `draw-core-v0.1.0`.
  - Replace the fakes with draw-core, `tracker` and `card-db`.
  - Serve card images to the webview from the local cache through a custom protocol.
- **Done when**:
  - [ ] Playing `sample_video.mp4` as the source shows its cards confirmed in the
    correct player's panel, with artwork, after first-run setup.

#### I2: Real-duel tuning
- **Branch**: `feat/real-duel`. **Depends on**: I1, B3, D4.
- **Scope**:
  - Test all three camera setups with real footage.
  - Choose deck lists from the UI.
  - Tune the confirmation thresholds and the inference rate per platform.
  - Pick the default ViT variant.
- **Done when**:
  - [ ] A recorded test duel per setup recognizes most face-up cards with no wrong
    confirmations.
  - [ ] The chosen defaults are documented in the README.

## Phase 3: Release

#### R1: Packaging
- **Branch**: `ci/release`. **Depends on**: I1.
- **Scope**:
  - Tag-triggered GitHub Actions builds of the Windows NSIS `.exe` and the macOS `.dmg`
    (arm64 + x64).
  - Portable downloads, built in the same job:
    - Windows: a `.zip` with the app `.exe` plus the ONNX Runtime/DirectML DLLs;
    - macOS: a `.zip` of the `.app`, made with `ditto -c -k --keepParent` so bundle
      attributes and signatures survive.
  - ONNX Runtime libraries inside every artifact.
  - All artifacts attached to a GitHub Release.
- **Done when**:
  - [ ] Every artifact (installer and portable, per platform) runs on a clean macOS or
    Windows machine with no dev tools installed, through first-run download to live
    recognition.
  - [ ] The Windows portable `.exe` starts from any folder, including a USB drive.

#### R2: Signing (optional)
- **Only needed** for public distribution to non-technical users; see the README's
  "Distribution and signing" section.
- **Depends on**: R1, plus an Apple Developer account and a Windows code-signing
  certificate.
- **Done when**:
  - [ ] The `.dmg` is notarized and opens without Gatekeeper warnings.
  - [ ] The Windows installer is signed.

## Phase 4: Later

Ideas to pick from once the app is released:

- Recording the composited view.
- Mat-zone mapping with calibration.
- A life-point counter.
- Streaming outputs (virtual camera, NDI).

Each becomes its own ticket when picked up.

---

## Conflict hotspots

These files are shared, so parallel branches can collide on them:

- **`src-tauri/src/lib.rs`, command registration**: F2 registers every module and
  command once. Phase 1 tickets change only the inside of their own modules.
- **Generated TS bindings**: regenerate after rebasing and never hand-edit them. When the
  IPC contract itself needs a change, make it in a small PR to `main` that the other
  branches rebase onto.
- **`Cargo.lock` / `package-lock.json`**: on conflict, take `main`'s version and rerun
  the build to regenerate.
- **`CLAUDE.md` / `README.md`**: append-only sections per ticket. The integrator
  reconciles them in I1.
