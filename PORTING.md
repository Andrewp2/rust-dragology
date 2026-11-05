# Porting Dragology

This project is a Rust port of [Dragology](https://github.com/declarative-dragging/dragology/tree/b145bdd56530f7cfa9a61bf6a7170459d2f140e1), by the Dragology contributors. The source reference is commit `b145bdd56530f7cfa9a61bf6a7170459d2f140e1`.

The target is source fidelity: the same models, legal moves, candidate ordering, presets, defaults, feedback, and animation rules. Source quirks are preserved where they affect behavior, including deliberately broken teaching examples. Differences needed to replace React, SVG, HTML, or browser events with Rust and GPUI are documented. A demo being present does not establish that every state renders identically.

## Engine and host boundary

The core remains renderer independent. It ports the model operators and the original JSXGraph/jcobyla numerical solver, and supplies previews, branch identities, transition settings, and chaining requests. A host supplies element geometry and owns input, rendering, and the animation clock. `DragSession::with_preview_geometry` lets a host measure its actual interpolated preview; nested operators then use rendered geometry instead of assuming that every transform interpolates linearly.

The GPUI gallery implements the source renderer lifecycle: initialization without immediate chaining, one evaluation per animation frame, source click/drag threshold, branch changes, chained restarts, release evaluation, and the source transition curves. The source’s SVG interpolation is translated into keyed native scenes, including Lab colors, path morphing, ordered transforms, and emergence. Chaining uses the operators’ own metadata; the earlier pointer-distance heuristics have been removed.

Rust APIs and state representations are not literal TypeScript syntax translations. In particular, native begin-drag hooks provide the element-identity changes used by `switchToStateAndFollow`; child-model helpers provide the gallery’s `embed` examples; native widget state replaces React component state. The small egui/GPUI/wgpu rectangle adapters retain their simpler host lifecycle. The full source-gallery lifecycle is currently implemented in the GPUI gallery.

## Demo coverage

All **119 source demo IDs**, including **19 Nursery experiments**, appear in the registry in source order. [The source inventory](examples/gallery/data/upstream-demo-ids.txt) records the IDs. [Extracted notes](examples/gallery/data/upstream-notes.json) map every ID to its original file and retain source locations, original wording, and explicit native wording adaptations.

The source-comparison reports record the changes and limits for each family:

| Area | Demos | Report |
| --- | ---: | --- |
| Basic interactions and component studies | 28 | [Basics](docs/porting/basics.md) |
| Continuous geometry and mechanisms | 25 | [Continuous](docs/porting/continuous.md) |
| Lists, braids, and permutation views | 21 | [Collections](docs/porting/lists.md) |
| Puzzles and tree balancing | 14 | [Puzzles](docs/porting/puzzles.md) |
| Copying, targets, Catalan, string diagrams, and knots | 14 | [Topology](docs/porting/topology.md) |
| Trees, algebra, proofs, and Graspable Math | 9 | [Trees and algebra](docs/porting/trees-algebra.md) |
| Editors, tableaux, and simulation | 7 | [Editors](docs/porting/editors.md) |
| Drag-spec designer | 1 | [Designer](docs/porting/designer.md) |
| Shared drawing and native controls | — | [Renderer](docs/porting/shell.md), [interpolation](docs/porting/interpolation.md) |

## Evidence

[Executable reference tools](tools/upstream-parity/README.md) run the original TypeScript code at the pinned revision. Ordinary Rust tests read committed fixtures and require no JavaScript installation or network access.

- Core traces compare 20 operator scenarios over 140 pointer frames: drop models, preview positions, gaps, active paths, and chaining. Easing samples include elastic overshoot. Continuous operator traces allow a 0.01 canvas-unit tolerance for host geometry calculations. Separate solver traces compare 18 original solves and all 580 objective evaluations exactly, including evaluation limits, impossible constraints, and gear docking.
- Gallery references cover original regex automata and Dagre layouts, stock-and-flow trajectories and formulas, seeded tilings, finite Catalan layouts, and selected color/path/transform/emergence calculations.
- Model and runtime tests exercise identity, constraints, rewriting, cancellation, copying, independent canvases, callback timing, and stateful operator evaluation.
- Native X11 screenshots and input checks supplement the model tests. They cover representative families and actual editable controls; they are not an exhaustive visual comparison of all reachable states.

## Remaining limits

The project should continue to be described as an **experimental Rust port**, without claiming complete API or pixel equivalence.

- Upstream’s operator-tree inspector, debug overlays, drop-zone visualizer, and browser fuzzing interface are not ported. Their absence matters for demos teaching those tools.
- SVG is represented by native objects, rather than an arbitrary SVG DOM. Raw path commands are normalized to absolute curves; inherited paint, nested group compositing, some property transitions, and unsupported transform-list combinations can differ. The interpolation report identifies these boundaries.
- Fonts and form widgets use native rendering. Color emoji and native inputs have transform limitations; rich math/button labels may use native text equivalents. Browser typography and responsive layout are not guaranteed pixel identical.
- The layout editor implements its source flex/grid subset in Rust; it does not embed a browser CSS engine. Some source CSS-only effects, such as pulsing recording indicators, still need native equivalents.
- The full Tableaux article takes about 6.4 seconds to lay out initially in the tested native build. Long-form text layout still needs performance work.
- Host floating-point geometry and layout engines can differ. The original solver is translated directly, and its recorded evaluation traces match exactly; that does not prove all host geometry callbacks are identical. Tests cover selected original traces, not every possible input. Source-computed cached coordinates are retained for the finite Catalan presets to avoid a measured layout drift.

The [MIT license](LICENSE) credits the Dragology contributors. [Third-party notices](THIRD_PARTY_NOTICES.md) retain the licenses of translated solver, interpolation, and easing routines.
