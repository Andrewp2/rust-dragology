# dragology-gpui

A GPUI adapter for the Rust port of [Dragology](https://github.com/declarative-dragging/dragology).

```sh
cargo add dragology-gpui@=0.1.0-beta.2
```

Implement `scene::Diagram`, create a `scene::Canvas`, and embed a `CanvasView` entity in your GPUI view. Read its public `canvas` to access the committed model. The adapter handles pointer input, release, cancellation, and preview drawing for filled rectangles, rounded rectangles, and circles.

See the [API documentation](https://docs.rs/dragology-gpui) and [integration guide](https://github.com/Andrewp2/rust-dragology#use-an-adapter-in-your-app). Run the [Hanoi example](https://github.com/Andrewp2/rust-dragology/blob/v0.1.0-beta.2/crates/gpui/examples/hanoi-gpui.rs) from a repository checkout with `cargo run -p dragology-gpui --example hanoi-gpui`. The full 119-demo gallery also lives in the repository and uses a richer GPUI scene interpreter; the native demos and their helper are not part of the published crate.

This beta targets GPUI 0.2.2 and is verified on Ubuntu 24.04 x86-64/X11 with Rust 1.97.1. Install the [native build dependencies](https://github.com/Andrewp2/rust-dragology#native-integrations) before building. See the [beta policy and tested scope](https://github.com/Andrewp2/rust-dragology#status-and-tested-platforms).

MIT licensed. This is an independent Rust port; original algorithms and demo designs are by the Dragology contributors. See `LICENSE` and `THIRD_PARTY_NOTICES.md`.
