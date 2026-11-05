# Native Dragology gallery

A GPUI port of all 119 demos in the [upstream gallery](https://joshuahhh.com/dragology/#/demos), including its 19 Nursery experiments. The source reference is [Dragology revision b145bdd](https://github.com/declarative-dragging/dragology/tree/b145bdd56530f7cfa9a61bf6a7170459d2f140e1). The original algorithms, interactions, and demo designs are the work of the Dragology contributors; this crate is a Rust port under the [MIT license](LICENSE).

## Run

From the workspace root, use an optimized build for interactive use:

```sh
cargo run --release -p dragology-gallery
cargo run --release -p dragology-gallery -- --demo graspable-math
cargo run --release -p dragology-gallery -- --demo stock-and-flow
cargo run -p dragology-gallery -- --list
```

GPUI requires the platform's native development libraries; see the [Ubuntu setup](../../README.md#native-integrations). Beta 1 (`0.1.0-beta.1`) pins Rust 1.97.1 and the tested dependency graph. The native gallery has been exercised on Ubuntu 24.04, x86-64, with X11; other native platforms and Wayland sessions are outside this beta's tested scope. The [beta release policy](../../README.md#beta-release-policy) defines API changes and version pinning.

The first release build takes longer. Omitting `--release` runs unoptimized code and can add noticeable latency to geometry-heavy demos.

## Controls

- Search the sidebar by title, ID, or description; Ctrl+F focuses search and Enter selects its first result.
- Drag objects in the stage. Each demo supplies its legal moves and any additional mouse or keyboard behavior.
- Escape cancels a drag, including all steps of a chained gesture. R resets the demo when focus is outside a text field.
- Use each demo’s inline controls and settings panel for presets, modes, parameters, and actions. Text fields support ordinary editing and clipboard shortcuts. Multi-figure pages scroll vertically.
- Alt, Shift, and Control reach the active demo. Changing Alt during a drag updates the gesture where the original demo uses it.
- Use Save/Load in stock-and-flow to exchange JSON models with the source format; Save in the layout editor exports HTML.
- Hide the demo sidebar to give dense diagrams more room; Ctrl+F reopens it.
- Disable motion in the footer or start with `--reduced-motion` to finish transitions immediately.

## Structure

The gallery keeps model rules separate from GPUI. An `Example` supplies a typed state, scene, and `DragSpec`; the shared runtime interprets engine previews, groups floating children, blends keyed geometry, and handles capture, cancellation, chaining, modifier changes, and transitions. The GPUI shell handles native input, text fields, drawing, and window controls.

Demo families live in `src/basics`, `lists`, `puzzles`, `continuous`, `trees_algebra`, `topology`, and `editors`. Graspable Math, knots, tree balancing, and the drag-spec designer have separate modules. The registry follows the [pinned source inventory](data/upstream-demo-ids.txt), including the source's comparison variants.

## Native presentation

Source behavior and design are the target. The gallery retains upstream geometry, colors, labels, presets, multi-figure composition, and gesture feedback. Its renderer supports transformed text, source transform lists, Lab color interpolation, path morphing, emergence, clipping, dashes, shadows, blur, floating ghosts, and source transition curves. Native controls replace browser inputs; the surrounding window, search sidebar, and navigation remain GPUI.

Original explanatory notes are extracted from the pinned TypeScript source into [upstream-notes.json](data/upstream-notes.json). Documented native wording overrides correct references to React or browser-only debugging without replacing the original text in the data. The extraction script records source locations and rejects an unexpected upstream revision.

This is not a claim of complete visual or API equivalence. See [PORTING.md](../../PORTING.md) for remaining boundaries and the per-family audit.

## Verification

```sh
cargo test --locked -p dragology-gallery --lib --tests
cargo build --locked -p dragology-gallery --bin dragology-gallery
```

Tests exercise legal gestures and model invariants: list identities, puzzle moves, constrained geometry, algebraic equivalence, capture-avoiding substitution, proof chaining, tree balancing, automata, tableaux, knots, and diagram rewrites. Inventory checks guard against missing or duplicate demos, invalid initial scenes, and broken preset or toggle actions. The algebra builder startup test runs in its own process to catch ID collisions that prior activity could hide. The [gallery CI job](../../.github/workflows/ci.yml) runs these checks without a display. Native screenshots and input checks supplement the tests; they do not exhaust every reachable demo state. Reference fixtures execute selected original TypeScript algorithms, including core operators, regex layouts, and stock-and-flow trajectories.
