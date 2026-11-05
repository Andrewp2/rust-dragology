# dragology-egui

An egui adapter for the Rust port of [Dragology](https://github.com/declarative-dragging/dragology).

```sh
cargo add dragology-egui@=0.1.0-beta.2
```

Implement `scene::Diagram`, keep a `scene::Canvas`, and call `dragology_egui::show(ui, &mut canvas, size)` each frame. The widget handles pointer input, release, cancellation, and preview drawing. A successful drop marks the returned `egui::Response` as changed. The shared scene supports filled rectangles, rounded rectangles, and circles.

See the [API documentation](https://docs.rs/dragology-egui) and [integration guide](https://github.com/Andrewp2/rust-dragology#use-an-adapter-in-your-app). Run the [Hanoi example](https://github.com/Andrewp2/rust-dragology/blob/v0.1.0-beta.2/crates/egui/examples/hanoi-egui.rs) from a repository checkout with `cargo run -p dragology-egui --example hanoi-egui`; demo sources and their shared helper are not part of the published crate.

This beta targets egui 0.37 and is verified on Ubuntu 24.04 x86-64/X11 with Rust 1.97.1. See the [beta policy and tested scope](https://github.com/Andrewp2/rust-dragology#status-and-tested-platforms).

MIT licensed. This is an independent Rust port; original algorithms and demo designs are by the Dragology contributors. See `LICENSE` and `THIRD_PARTY_NOTICES.md`.
