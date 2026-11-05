# dragology-scene

Shared scene, hit testing, preview interpolation, and animation for the Rust port of [Dragology](https://github.com/declarative-dragging/dragology).

```sh
cargo add dragology-scene@=0.1.0-beta.2
```

Implement `Diagram` to supply a model's layout and the drag specification for each item. Keep a `Canvas` between frames to manage pointer input, previews, and committed state. Items have stable IDs and filled rectangle, rounded rectangle, or circle geometry. The egui, GPUI, and wgpu adapters re-export this API as `scene`.

See the [API documentation](https://docs.rs/dragology-scene) and [integration guide](https://github.com/Andrewp2/rust-dragology#use-an-adapter-in-your-app). For custom paths, text, or controls, use the renderer-independent `dragology` core with your own renderer.

This beta's core and shared scene are tested on Linux, Windows, and macOS with Rust 1.97.1. See the [beta policy and tested scope](https://github.com/Andrewp2/rust-dragology#status-and-tested-platforms).

MIT licensed. This is an independent Rust port; original algorithms and demo designs are by the Dragology contributors. See `LICENSE` and `THIRD_PARTY_NOTICES.md`.
