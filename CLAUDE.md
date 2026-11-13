# CLAUDE.md

draw-ex: a Tauri v2 desktop app (Windows + macOS) that shows live camera footage of a
real _Yu-Gi-Oh!_ duel with side panels of the cards the DRAW 2 models recognize.
`README.md` is the design source of truth: architecture, pipeline, cache, roadmap. Read it
before any design-level change, and update it in the same change whenever a decision
moves.

Status: design phase, no code yet. When you add the scaffold, put build, test and run
commands here.

Implementing a ticket: read its entry in `docs/ROADMAP.md` first, and change only the
paths listed under its **Owns**. Its **Done when** list is the merge bar.

## Layout (planned)

- `crates/tracker/`: tracks, voting, confirmation, player assignment. No Tauri
  dependency.
- `crates/card-db/`: YGOPRODeck client, SQLite store, image cache. No Tauri dependency.
- `src-tauri/`: the app shell. Keep commands thin and put the logic in the crates.
- `src/`: Svelte 5 + TypeScript frontend. The IPC types come from `tauri-specta`:
  regenerate them after changing a command or event, rather than editing the generated
  file.

## Recognition lives in the draw2 fork, not here

Card recognition is the `draw-core` crate in the draw2 fork
(https://github.com/VillegasMich/draw2, `crates/draw-core/`; local checkout at `../draw2`).
draw-ex consumes it as a pinned Cargo git dependency.

- **Recognition changes go in draw2.** That covers preprocessing, ONNX sessions,
  orientation, classification, deck filtering and the model manifest. Make the change
  there, tag a release, and bump the pin here. draw-ex code only calls draw-core's public
  API.
- **Stateless frame → cards is draw-core; anything across frames or duel-specific is
  draw-ex** (`tracker`). If a fix seems to need draw-core internals, the fix belongs in
  draw2.
- **Model files come from draw-core's manifest.** draw-ex downloads exactly what the
  manifest lists. Keep model names, revisions and hashes out of this repo.
- **Local co-development** uses the gitignored `.cargo/config.toml` `[patch]` override
  described in the README. Before committing a dependency bump, confirm the build passes
  with the override removed.

### Porting notes for draw-core (work done in `../draw2`)

These notes are kept here during the design phase. When implementation of
`crates/draw-core/` starts in the draw2 fork, move them into that repo's docs and leave a
pointer here.

The reference implementation is draw2's **browser** pipeline, `docs/scripts/pipeline.js`.
Port from it, not from the Python `src/draw/draw.py`: the Python version's text-box
contour orientation step is the main source of `orientation_unknown` failures.

Port map, pipeline.js → draw-core:

- `preprocessYOLO`, `parseYOLOOutput`, `xywhrToCorners`, `nmsOBB`: detection.
- `solveHomography`, `warpPerspective`: the 224×224 crop.
- `correctRotation`, `classifyWithRotationFallback`: orientation.
- `preprocessViT`, `topK`: classification.

Gotchas that are easy to drop during a port:

- **Portrait normalization**: when a box has `w > h`, swap w/h and add π/2 to the angle
  *before* warping. Without it, the crop comes out squeezed and confidence stays low in
  every rotation.
- **YOLO output layout varies**: check whether features come first (`dims[1] < dims[2]`)
  instead of hard-coding the layout.
- **`topK` softmax**: apply softmax only when the outputs are logits (max > 1).
- **Two label files**: the `yugiscan` ViT uses `card_labels_yugiscan.json`, with plain
  `Name-<passcode>` strings. The other ViTs use `cardnames_onnx.json`, keyed by output
  index.
- **Deck filtering needs a deep top-k**: Python uses 15. A top-3 rarely contains the deck
  card.

Golden tests: run draw2 (`uv run python -m draw --source <img> -v --log-json out.jsonl`)
and compare its results with draw-core on the same images.
`docs/samples/sample_photo.jpg` should give Monster Reborn `83764719`.

## Invariants

- **Display never waits for inference.** The `<video>` element shows the camera at its
  native rate. Inference takes at most one frame per camera in flight, and new frames
  are skipped while it is busy.
- **Only confirmed tracks reach the UI panels and the duel log.** Raw per-frame
  detections are used only for drawing boxes on the overlay.
- **Passcode is the card identity everywhere.** Normalize it to an integer, because
  sources pad it with leading zeros.

## YGOPRODeck usage (IP-ban risk)

The API blocks an IP for 1 hour above 20 requests/second, and blacklists clients that
hotlink images or keep re-fetching data (https://ygoprodeck.com/api-guide/). draw2 once
needed an emergency fix for exactly this. So:

- Send every YGOPRODeck request through `card-db`'s single shared rate limiter.
- Fetch each image once, store it on disk, and serve it to the webview from the local
  cache. Frontend code references local image URLs only.
- Bulk-sync `cardinfo.php` into SQLite, and re-sync only when `checkDBVer.php` reports a
  new `database_version`.
- Tests and dev loops use recorded fixtures, never the live API.

## Platform notes

- macOS camera access in the webview needs `NSCameraUsageDescription` in `Info.plist`
  plus the `com.apple.security.device.camera` entitlement for hardened-runtime and
  notarized builds.
- ONNX Runtime (via draw-core's `ort` dependency): CoreML on macOS, DirectML on Windows,
  CPU fallback. The runtime library must end up inside the Tauri bundle. Verify a
  packaged build on a clean machine, not only `tauri dev`.

## License

AGPL-3.0, inherited from DRAW 2. Code from draw2 is fine; code copied from projects with
incompatible licenses is not.

## Agent skills

### Issue tracker

GitHub Issues on `VillegasMich/draw-ex`, via the `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

The five default labels (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`). See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: `GLOSSARY.md` + `docs/adr/` at the repo root, created as terms and decisions are resolved. See `docs/agents/domain.md`.
