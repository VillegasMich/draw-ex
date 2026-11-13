# Recognition lives in the draw2 fork as the draw-core crate

Card recognition (model loading, preprocessing, orientation, classification, deck
filtering and the model manifest) is a Rust crate, `draw-core`, in the draw2 fork
(`VillegasMich/draw2`). draw-ex depends on it through a pinned git tag. We wanted one
owner for recognition, next to the model it runs, without shipping Python in the app.

## Considered Options

- **Run draw2 as a Python sidecar.** This reuses the code fully, but makes installers
  1–3 GB, starts slowly, has no GPU on macOS without changes, and brings in the weaker
  Python orientation step.
- **Docker.** Users would need Docker Desktop, containers have no camera access on
  macOS/Windows, and there is no Metal GPU.
- **Port the pipeline into draw-ex.** This duplicates recognition logic in a second repo.

## Consequences

- Recognition changes go to draw2 first: tag a release, then bump the pin in draw-ex.
- draw2 now implements the pipeline three times (Python, JS demo, Rust).
- The draw-core version and the model files it expects change together through the
  manifest.
