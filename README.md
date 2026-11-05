# Dragology for Rust

Declare the model states a drag can reach and supply the same layout transform you use to draw the grabbed element. Dragology follows the point where the press landed, previews the move, and tells you which model to commit on release.

This is a Rust port of the model-driven operators from [Dragology](https://github.com/declarative-dragging/dragology/tree/b145bdd56530f7cfa9a61bf6a7170459d2f140e1). It works with ordinary Rust types. Your application owns rendering, element identity, hit testing, pointer capture, and animation.

## Status and tested platforms

**Beta 1, version 0.1.0-beta.1.** Ready for application integration and feedback within the scope below. This is an independent Rust port; the original algorithms and demo designs are the work of the Dragology contributors.

| Surface | Beta verification |
| --- | --- |
| Renderer-independent core and shared scene | Tests, Clippy, and API documentation on Linux, Windows, and macOS |
| egui, GPUI, and wgpu adapters | Ubuntu 24.04 builds of all three Hanoi examples; egui input tests and headless wgpu rendering tests |
| Full GPUI gallery | All 119 demo IDs; model, source-reference, and interaction regressions; native X11 screen and gesture checks |

The supported compiler baseline is **Rust 1.97.1**, pinned in `rust-toolchain.toml`; the lockfile pins dependency versions. Earlier compilers have not been qualified. Interactive native verification uses Ubuntu 24.04, x86-64, with X11. Native UI behavior on macOS/Windows, Wayland sessions, and WebAssembly is outside this beta's tested scope.

The [CI workflow](.github/workflows/ci.yml) checks these build and test targets. GPU tests use Mesa's software Vulkan adapter. Native screen and gesture checks supplement CI; they do not establish complete parity across every reachable state. The [porting notes](PORTING.md) describe the source reference and remaining differences.

### Beta release policy

The current operators, preview model, host boundary, and adapter APIs form the beta integration baseline. Work toward stable 0.1.0 focuses on correctness, usability, and feedback from applications. Any deliberate breaking API change requires a new beta version and migration notes in its [GitHub release](https://github.com/Andrewp2/rust-dragology/releases). Pin a release tag when integrating; `main` remains the development branch.

The full source-gallery lifecycle is implemented in GPUI. The three small adapters support the documented rectangle/circle scene API. Browser debugging tools and pixel-identical browser rendering are outside the beta scope. Original Nursery and teaching experiments retain their upstream behavior and wording.

## A toggle

The model is a boolean. Its layout places the knob at either end of a track. During a drag, the knob can appear halfway across while the model still has just two legal values.

```rust
use dragology::{Affine, DragError, DragSession, Point, between};

fn main() -> Result<(), DragError> {
    let layout = |on: &bool| {
        Some(Affine::translate((if *on { 90.0 } else { 30.0 }, 30.0)))
    };

    // Press three units to the right of the knob's center.
    let mut drag = DragSession::begin(false, (33.0, 30.0), layout, between([false, true]))?;
    let frame = drag.frame((48.0, 30.0))?;
    assert_eq!(frame.grab_position, Some(Point::new(48.0, 30.0)));
    assert_eq!(*frame.drop_state, false);

    let on = drag.finish((88.0, 30.0))?;
    assert!(on);
    Ok(())
}
```

Start a fresh session on each press with a snapshot of your current model. Capture the selected element's stable ID in the layout closure so reordering the model still tracks the same item. Use `None` when that state removes the item. Call `cancel()` to recover the starting model.

## Drag specifications

| Behavior | Rust API |
| --- | --- |
| Hold one candidate model | `fixed(state)` |
| Preview the branch closest to the pointer | `closest(targets)` |
| Blend across discrete targets, then drop on the nearest | `between(targets)` |
| Drag a field inside its parent layout | `substate(parent, field, spec)` |
| Solve for continuous model parameters | `vary(initial, model)`, `vary_constrained(initial, model, constraints)` |
| Accept a drop within a canvas rectangle | `drop_target(state, bounds)` |
| Use a fallback beyond a distance | `.when_far(background, radius)` |
| Float the grabbed item over the selected layout | `.with_floating()` |
| Snap the preview near the drop position | `.with_snap_radius(radius)` |
| Transform the candidate or the final drop model | `.during(map)`, `.on_drop(map)` |
| Rebuild a child when an external value changes | `react_to(getter, build)` |
| Request source-compatible branch and drop animation | `.with_branch_transition(...)`, `.with_drop_transition(...)` |
| Request another step while the pointer is held | `.with_chaining_options(...)` |
| Change pointer coordinates or branch scoring | `.change_frame(map)`, `.change_gap(map)` |

Both `closest` and `between` accept model states or nested drag specifications. For example, `closest([track_a, track_b])` can choose between two continuous sliders. Use `closest_with` for stickiness or branch locking and `between_with` to choose natural-neighbor interpolation or adjust sharpness. The default blend uses Delaunay interpolation and projects the pointer onto the hull of reachable positions.

A bounded slider can declare its numeric range directly. Each constraint below must be at most zero. Constraints use the original numerical solver and its final-iterate semantics, so they are approximate.

```rust
use dragology::vary_constrained;

let slider = vary_constrained([60.0], |[value]| value, |&value| [-value, value - 240.0]);
```

## Drawing a preview

A frame contains a real drop model and a separate `Preview`. Draw a `State` leaf using your usual rendering function. For a `Blend`, render the child models and interpolate corresponding visual properties with the supplied weights. For a `Floating` preview, draw the backdrop without the grabbed element, retain an optional ghost at its candidate position, then draw the floating element from the source with the supplied canvas-space translation.

The Rust API also accepts custom operators. Each frame carries its active operator path and optional branch, drop, and chaining settings. The host uses those signals to manage gesture restarts and animation; `Transition` supplies the source easing curves. The GPUI gallery implements this lifecycle and visual interpolation for native scenes.

## Full GPUI demo gallery

The native gallery includes all **119 demos** from the pinned upstream list, including the **19 Nursery demos**. It covers the small drag experiments, list and tree editors, puzzles, constrained geometry, algebra, knots, string diagrams, and stock-and-flow models.

```sh
git clone --branch v0.1.0-beta.1 https://github.com/Andrewp2/rust-dragology.git
cd rust-dragology
cargo run --release -p dragology-gallery
cargo run --release -p dragology-gallery -- --demo knot
cargo run -p dragology-gallery -- --list
cargo run --release -p dragology-gallery -- --reduced-motion
```

Use the sidebar to select a demo or press Ctrl+F to search. Each demo has its own settings and instructions. Escape cancels the current drag; R resets the selected demo when a text field is not focused. Demos that use Alt respond even when you change the modifier during a drag. Stock-and-flow models can be saved and loaded as JSON.

The demos aim to preserve upstream rules, presets, geometry, colors, controls, and feedback. GPUI draws the source scenes and native equivalents of browser controls. Multi-figure demos retain independent canvases, and text follows the same transforms as its shapes. The [gallery notes](examples/gallery/README.md) and [porting notes](PORTING.md) distinguish verified behavior from remaining gaps. The MIT attribution to the Dragology contributors also applies to these demo ports.

## Native integrations

Each adapter includes the same Tower of Hanoi demo. Drag a top ring toward another peg. The receiving pedestal rises and the ring snaps into alignment near a legal landing. Release to commit the move and watch it settle, or press Escape to return it. Press R to restart. The move count updates after each completed move, and finishing the stack on the right marks the goal.

```sh
cargo run -p dragology-egui --example hanoi-egui
cargo run -p dragology-gpui --example hanoi-gpui
cargo run -p dragology-wgpu --example hanoi-wgpu
```

On Ubuntu 24.04, install the native build dependencies for the GPUI gallery and examples:

```sh
sudo apt-get update
sudo apt-get install --no-install-recommends -y \
  build-essential pkg-config libfontconfig1-dev \
  libwayland-dev libxkbcommon-x11-dev fonts-dejavu-core \
  libvulkan1 mesa-vulkan-drivers
```

The workspace lockfile records the dependency versions used by the demos.

The GPUI demo provides a draggable Linux title bar with minimize, maximize/restore and close controls. All three demos accept `--reduced-motion` after Cargo's `--`, for example `cargo run -p dragology-gpui --example hanoi-gpui -- --reduced-motion`.

### Use an adapter in your app

Add the adapter directory as a path dependency, for example `cargo add --path ../rust-dragology/crates/egui`. Each adapter re-exports the shared scene API as `scene`. Implement its `Diagram` trait with two functions:

- `layout(&state)` returns items in paint order. Give each item a stable ID, bounds, corner radius and sRGBA fill color.
- `drag(&state, &id)` returns a core drag specification for that item or `None` for a decoration or disabled item.

Construct a `Canvas::new(diagram, initial_state, design_size)` and keep it between frames. It hit-tests the layout, matches IDs when blending previews, and paints a floating item above its backdrop. Read the committed state with `canvas.model()`, change it with `canvas.set_model(...)`, and inspect a failed gesture with `canvas.last_error()`. See the [shared Hanoi diagram](examples/hanoi/src/lib.rs) for an implementation.

Call `canvas.set_animated(true)` to enable brief transitions between layouts and on release or cancellation. The grabbed item follows the pointer immediately, and a drop commits the model immediately. egui and GPUI advance visual time automatically. A wgpu host calls `canvas.advance(elapsed)` once per frame and requests another redraw while it returns true. Call `set_animated(false)` to finish pending motion immediately. A diagram can override `paint(item, dragging)` to add render-only shadows or highlights to its displayed shapes.

| Host | Integration point |
| --- | --- |
| [egui](crates/egui/examples/hanoi-egui.rs) | Call `dragology_egui::show(ui, &mut canvas, size)` inside any UI. The returned response marks a successful drop as changed. |
| [GPUI](crates/gpui/examples/hanoi-gpui.rs) | Create an entity with `cx.new(\|cx\| CanvasView::new(canvas, window, cx))` and embed it as a child. Read its public `canvas` through the entity. |
| [wgpu](crates/wgpu/examples/hanoi-wgpu.rs) | Construct a `Renderer` for your attachment format and sample count. Call `prepare` with the canvas, drawing area and target size in physical pixels, then `draw` inside your color pass. Use the same `Viewport::fit` to map window input into `canvas.handle(...)`. The example uses winit. |

The shared scene supports axis-aligned filled rectangles, rounded rectangles and circles. For custom paths, text or native controls, you can use the renderer-independent core and interpret its previews in your own renderer.

## Run the core locally

```sh
cargo run --example reorder
cargo test
```

The headless example prints the candidate order and floating row position at each pointer move. To use the library from a neighboring crate, add it with `cargo add --path ../rust-dragology`.

To depend on the library directly from GitHub:

```sh
cargo add dragology --git https://github.com/Andrewp2/rust-dragology --tag v0.1.0-beta.1
```

To run the library checks used by CI:

```sh
cargo fmt --all --check
cargo test --locked -p dragology -p dragology-scene
cargo clippy --locked -p dragology -p dragology-scene --all-targets -- -D warnings
RUSTDOCFLAGS='-D warnings' cargo doc --locked -p dragology -p dragology-scene --no-deps
```

After installing the Linux native dependencies, run the remaining CI checks:

```sh
cargo test --locked -p dragology-egui -p dragology-wgpu --tests
cargo test --locked -p dragology-gallery --lib --tests
cargo build --locked --workspace --examples --bins
```

Adapter tests exercise egui release/cancellation and actual wgpu drawing, clipping, color, and viewport reset through software Vulkan without a window. Gallery checks include a separate startup regression for the algebra builder, so other demos cannot hide an ID collision by consuming IDs first.

## License and attribution

This crate is a Rust port of the original Dragology library by the Dragology contributors. It is distributed under the [MIT license](LICENSE) and carries the upstream notice, `Copyright (c) 2025 Dragology contributors`. Translated solver, interpolation, and easing routines retain their additional notices in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
