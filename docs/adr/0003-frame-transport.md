# The webview captures cameras; frames reach Rust as raw bytes, one in flight

The webview owns the cameras (`getUserMedia`) and shows them in `<video>` at native
rate. At the inference rate, it sends one frame's raw RGBA8 bytes as the body of a
`submit_frame` invoke, with camera, size and timestamp in headers. **The response is
that frame's detections.** The frontend awaits it before sending that camera's next
frame. Awaiting the response is the backpressure, so no queue ever forms and display
never waits for inference.

## Considered Options

- **Capture in Rust (`nokhwa`) and stream preview frames to the webview.** Preview
  smoothness would depend on IPC bandwidth, and it is more code. It stays the fallback if
  webview capture proves unreliable.
- **Detections pushed as events.** That needs a separate mechanism to stop frames
  queueing.
- **JPEG-encoding frames.** It costs an encode/decode pair and loses detail on the card
  crops.

## Consequences

`submit_frame` is the one command that `tauri-specta` cannot type, so it has a
hand-written TypeScript wrapper.
