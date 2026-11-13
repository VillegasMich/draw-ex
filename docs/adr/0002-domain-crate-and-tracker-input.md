# Shared `domain` crate; the tracker owns its input type

draw-ex keeps the identifiers and geometry that its crates share (passcode, camera,
player, point, quad) in a small `crates/domain` crate. `tracker` takes its own
`Observation` input instead of draw-core's `Detection`. The app shell (`src-tauri`) maps
draw-core detections to observations in a single place.

This keeps `tracker` and `card-db` free of draw-core, which will pull in ONNX Runtime.
It also lets the tracker be tested with synthetic observations, so Phase 1 streams build
independently.

## Consequences

draw-core and `domain` each define a `Passcode` and a `Quad`. That duplication is
deliberate: the mapping in `src-tauri` is the only place where both appear. Do not make
`tracker` or `card-db` depend on draw-core to remove it.
