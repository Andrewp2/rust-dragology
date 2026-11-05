# Editors, tableaux, and simulation

These seven GPUI demos port the models, interactions, controls and presentation of [Dragology at commit `b145bdd56530f7cfa9a61bf6a7170459d2f140e1`](https://github.com/declarative-dragging/dragology/tree/b145bdd56530f7cfa9a61bf6a7170459d2f140e1). The native entry points are in `examples/gallery/src/editors.rs`; the implementations and reference fixtures are under `examples/gallery/src/editors/`.

The comparisons below distinguish tested model behavior from native rendering differences. Passing the tests does not establish pixel equivalence or cover every reachable edited state.

## Source mapping

Source paths are relative to the pinned upstream repository. Rust paths are relative to `examples/gallery/src/editors/`.

| Demo | Upstream source | Rust implementation |
| --- | --- | --- |
| `regex-nfa-sync` | `src/demos/regex-nfa/regex.ts`, `sync.tsx` | `regex.rs` |
| `gantt` | `src/demos/gantt.tsx` | `gantt.rs` |
| `tableaux` | `src/demos/tableaux.tsx` | `tableaux.rs`, `tableaux-article.json` |
| `layout-editor` | `src/demos/layout-editor.tsx` | `layout.rs` |
| `git-graph-surgery` | `src/demos/git-graph-surgery.tsx` | `git.rs` |
| `pivot-table` | `src/demos/pivot-table.tsx` | `pivot.rs` |
| `stock-and-flow` | `src/demos/stock-and-flow/model.ts`, `presets.ts`, `index.tsx` | `stock.rs`, `stock/` |

## Regex and automata

The demo starts with `(a|b)*ab` and includes the letter/star palette, trash, normalized tree rewrites, matching words through length four, Thompson NFA and optional DFA. Tree and automaton scenes retain source dimensions, colors, edge routes, directional heads, start/accept markings, epsilon dashes, semantic edge identities, active-subtree highlighting and label halos.

Drag candidates follow source ordering and eligibility. The root cannot be moved; a star cannot wrap another star or a star's immediate child. Moving a star offers its original state and eligible destinations other than its original child. Palette drags offer new placements, with the source floating fallback and cancellation behavior.

DFA construction uses reachable subsets, merges a/b labels and omits a dead state. The Rust Dagre 0.1.2 library computes layouts for edited trees using the source Dagre 0.8.5 settings: left-to-right ranks, 44-pixel node separation, 72-pixel rank separation and 22×22 nodes. Graph construction and tie handling preserve the compound/multigraph behavior needed for reversed parallel edges.

Evidence includes 61 source-generated cases comparing NFA dimensions and node coordinates and every DFA coordinate. Further tests cover recognition through word length seven, AST normalization and identity, and floating palette/star gestures. `regex/generate-fixtures.mjs` requires the pinned revision, a clean source implementation and Dagre 0.8.5. Layout agreement outside these cases has not been exhaustively established.

## Gantt chart

The source's seven tasks, four lanes and eight dependencies drive derived schedules, critical-path colors, slack/lag annotations and the project-end marker. Tasks can change duration, lag, lane and prerequisites; dependency arrows can be deleted. The 574×216 scene uses the source grid, curves, handles, target highlights and dimming.

Dependency edits take one schedule snapshot after changing the dependency set, then assign every task's new lag from that snapshot, matching the source's downstream scheduling behavior. Lag and duration use constrained numerical variation with the source minimum and horizon constraints. Specs carry the 0.35 floating ghost, 120 ms branch transition and lane snap transition.

Tests cover dependency-removal semantics, downstream propagation and duration gestures at both bounds. Constraint accuracy is measured in projected pixels rather than requiring identical optimizer bits.

## Young tableaux

The entire source article and all five figures appear on one scrolling page, with independent reset controls. A source JSX extractor retains explanations, captions, references, bold/italic/monospace runs, amber “Try it” spans, the numbered insertion list and the muted technical footer. Text wraps using measured font advances and the source 65ch limit; figure borders include their captions.

- **Shapes:** legal removable/addable corners, live target outlines and partition feedback.
- **Fillings:** swaps, broken-rule edges, all 16 gallery slots, discovery order and completion feedback.
- **Row insertion:** the source tableau and queue, separate insert/step controls and enabled states, pending and bumped entries, end-slot indicators and explanatory messages. Transform transitions use 450 ms and `cubic-bezier(.2,.8,.2,1)`.
- **RSK:** the reorderable permutation, insertion-prefix cursor, P/Q tableaux and longest-increasing-subsequence annotation.
- **Jeu de taquin:** the source skew tableau, inner-corner opening, legal forward/reverse slides, reading-word/P feedback, closing and rectified states. A 10-pixel snap spec requests chaining through core metadata.

Tests cover semistandard insertion, RSK and LIS invariants, preservation of the reading tableau during slides, and independent figure state/reset behavior. The article extractor, `tableaux/generate-article.mjs`, verifies the pinned source revision and produces 35 blocks containing all five figures.

The source animates row-entry opacity for 300 ms separately from its 450 ms transform. The native scene transition uses one duration, so that separate opacity timing remains different.

## Layout editor

The source nested boxes and leaves support flex rows, flex columns and grids. Handles edit size, padding, gap, direction and column count; reparenting/reordering preserves the tree and rejects cycles. Scene geometry, colors, dashed containers, numerical constraints and branch/drop/floating rules follow the source.

All four panels are present: editor, native preview, generated CSS and generated HTML. CSS and HTML use independent read-only controls with their own scrolling. Export produces a self-contained HTML/CSS document, using the source structure and rounded layout values.

The **Native HTML preview** implements this demo's flex/grid subset in Rust. It does not embed a browser CSS engine. The displayed introduction states this distinction; the original upstream wording remains in the source-note data.

Tests cover tree preservation, cycle rejection and generated CSS/grid content. Arbitrary browser CSS lies outside the preview's model.

## Git graph surgery

The source commits and main/feature/hotfix references support reordering, reparenting, cherry-picking, descendant rewrites, old-history ghosts and branch movement. The 480×440 scene retains source row/lane spacing, curves, lane colors, branch chips, monospace hashes, message styles and ghost dashes/strikethrough/opacity.

Object identity allocation is independent of commit sequence/hash allocation, so creating ghosts does not consume commit sequence numbers. Hashing uses UTF-16 code units, matching JavaScript `charCodeAt`. The drag specs preserve source closest stickiness, far fallback and the commit-only 0.3 floating ghost.

Tests cover rebase hashes and descendants, retained old history, cherry-pick behavior and fresh sequence allocation.

## Pivot table

The model contains the source's 36 records, generated with the same floating-point LCG arithmetic, three dimensions and sales/units/count measures. Dimension and measure chips move between axes and the unused area; the “Chips in table headers” option is retained.

The scene follows source chip/table geometry, grouped header spans, stacked measure headers, compact cell widths, numeric-label thresholds and stable cell identities. Heat colors normalize each measure independently and use source channel rounding. Drag specs carry the 0.3 floating ghost, 40/56-pixel far hysteresis and 150 ms branch transitions.

Tests exercise axis movement, aggregation preservation and cell identity across rearrangements.

## Stock and flow

All nine presets come from the source factories: glucose–insulin, Lotka–Volterra, predator–prey enrichment, SIR, fishery, toggle switch, repressilator, damped spring and empty.

The model preserves ordered stocks, parameters, flows and dependencies; all six factor shapes; parameter naming and ranges; symbolic formulas; volumes; 400-step RK4 operation order and divergence rules; cursor sampling; scale fitting; and 41-point sweeps over the last 20% of each run. Caches avoid repeating unchanged simulations and sweeps while rendering.

Editing follows the source spec composition: constrained stock/parameter/chart/duration movement, rectangular stock targets, circular valve targets, free endpoint movement, trash targets and floating behavior. Endpoint candidates exclude the opposite endpoint. Creation follows the new element's identity; a new outflow creates the source-stock linear dependency with `k = (3/T) × volume`. New inflows use the source scale-dependent rate. Creation cancellation, deletion and parameter edits participate in the 100-entry undo history. Cleanup removes orphan parameters and dangling dependencies; chart scales refit on commit.

The 680-pixel scene includes the source stock, pipe, valve and cloud geometry; dependency curves and factor glyphs; parameter ordering/sliders; time and sweep controls; and full equations with subscripts, superscripts and colored stock references. Hovering a flow links its diagram and equation feedback. Clicking a parameter name selects its sweep; double-clicking a label or value opens a native editing control. Clicking the sweep plot chooses a parameter value.

Save/Load replaces browser upload/download. Source-valid JSON models round-trip with ordered dictionaries and correctly parsed floating-point values; optional time range, cursor, time unit and sweep fields receive source defaults. Direct file dropping onto the diagram and the browser file-drag overlay are not implemented. Typed import rejects some malformed JSON that the JavaScript loader accepts.

Evidence includes 18 source-generated cases: each preset at its initial parameters and after a 63%-range parameter perturbation. Comparisons cover sampled trajectories, flow rates, divergence, every flow formula and all sweep bands at a `1e-10` relative tolerance. Independent tests cover SIR conservation, the damped oscillator, creation/undo gestures, endpoint exclusion, trash cancellation, exact parameter entry, import defaults, handle geometry and valve following. `stock/generate-fixtures.mjs` requires the pinned revision and clean model/preset sources.

## Verification and platform limits

All 31 editor tests pass, including within the complete gallery suite. Regression checks include compact-grid column selection, invalid parameter ranges on import, and saving/reloading exact volume edits. The committed reference data contains 61 regex/Dagre cases, 18 stock simulation cases and nine complete source preset models. Ordinary Rust tests consume these fixtures without running JavaScript or accessing the network.

Runtime gesture tests advance a frame after pointer movement, matching the source animation-frame lifecycle. Numerical geometry assertions use projected error bounds; for example, the logarithmic time slider must land within 0.01 pixels of its target.

Native visual checks cover the regex sections, layout's four panels, stock diagrams/charts/equations and representative tableau sections. Final captures verify the layout wording and code panes, plus tableau rich text, numbered insertion steps and caption borders. Stock hover-background behavior has not been confirmed by a targeted native capture. These checks supplement model tests rather than exhaustively comparing every interaction and reachable state.

Fonts, form widgets, line wrapping and mathematical script baselines can differ from browser HTML/SVG. Native text controls replace browser prompts, and native file controls replace upload/download gestures. The layout preview and tableau opacity timing have the additional limits described above. Shared rendering and interpolation boundaries are documented in [Renderer](shell.md) and [interpolation](interpolation.md).
