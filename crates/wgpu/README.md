# dragology-wgpu

A wgpu renderer for the Rust port of [Dragology](https://github.com/declarative-dragging/dragology).

```sh
cargo add dragology-wgpu@=0.1.0-beta.2
```

Implement `scene::Diagram` and keep a `scene::Canvas`. Construct `Renderer` for your color attachment format and sample count, call `prepare` with the canvas and drawing area in physical pixels, and call `draw` inside your render pass. Use `scene::Viewport::fit` to map window input into `canvas.handle(...)`. A host that enables animation calls `canvas.advance(elapsed)` each frame and redraws while it returns true. The renderer supports filled rectangles, rounded rectangles, and circles.

See the [API documentation](https://docs.rs/dragology-wgpu) and [integration guide](https://github.com/Andrewp2/rust-dragology#use-an-adapter-in-your-app). Run the [winit Hanoi example](https://github.com/Andrewp2/rust-dragology/blob/v0.1.0-beta.2/crates/wgpu/examples/hanoi-wgpu.rs) from a repository checkout with `cargo run -p dragology-wgpu --example hanoi-wgpu`; demo sources and their shared helper are not part of the published crate.

This beta targets wgpu 30 and is verified on Ubuntu 24.04 x86-64/X11 with Rust 1.97.1. CI also checks offscreen drawing through software Vulkan. See the [beta policy and tested scope](https://github.com/Andrewp2/rust-dragology#status-and-tested-platforms).

MIT licensed. This is an independent Rust port; original algorithms and demo designs are by the Dragology contributors. See `LICENSE` and `THIRD_PARTY_NOTICES.md`.
