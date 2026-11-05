# Topology and target demo port

The port follows Dragology commit [`b145bdd56530f7cfa9a61bf6a7170459d2f140e1`](https://github.com/declarative-dragging/dragology/tree/b145bdd56530f7cfa9a61bf6a7170459d2f140e1). Upstream demo paths below are relative to `src/demos/`; Rust demo paths are relative to `examples/gallery/src/`.

This report covers 14 demos: nine copying/target examples, Catalan Explorer, ZX String Diagram, both Frobenius examples, and Knot. It records the source models, interactions, rendering adaptations, and verification limits. It does not claim pixel-identical GPUI rendering or exhaustive equivalence across all pointer trajectories.

## Copying and targets

All nine examples are implemented in `topology/targets.rs`.

| Upstream demo | Model and interaction mapping | Rendering and verification |
|---|---|---|
| `drag-to-copy.tsx` | Dragging inserts and follows a fresh dot while preserving the original dot's identity, position, and color. Double click deletes. Copy color follows the source count-based palette. | Source six-color palette, positions, 20 px radius, and 400 × 300 canvas. Tests cover fresh identity, original position, and cancellation. |
| `alt-to-copy.tsx` | Plain dragging moves; Alt-dragging creates and follows a copy. Modifier changes rebuild from the current step's original press context through `begin_drag`. | Shares the source dot rendering. Tests cover plain and Alt starts. Deterministic IDs replace source timestamp IDs in both copying examples. |
| `graph.tsx` | Preserves four initial nodes, ten edges, circle port spreading, and sibling, antiparallel, and self-loop geometry. Supports node creation/deletion, edge creation, endpoint reconnection, and loose-endpoint deletion. Newly created elements follow the drag, including nested Alt creation. Node targets use a 35 px radius and 15 px fallback distance. | Source arrowhead angle and attachment-derived length, invisible hoverable tail handles, and 350 × 350 canvas. Tests cover directed connections and loose-endpoint removal. Monotonic IDs preserve source insertion order without wall-clock timestamps. |
| `weird-floating-vs-fixed.tsx` | Integer state selects four candidates that alternate fixed and floating behavior. Uses the core floating operator with 0.5 ghost opacity. | Source black 100 × 100 rectangle and 500 × 150 canvas. Covered by source inspection and shared operator tests; no exhaustive pointer comparison. |
| `snap-to-islands.tsx` | Preserves the on-island/floating state union and `closest → floating → whenFar(vary)` composition with the default 50 px cutoff. | Source island positions, labels, occupied/unoccupied rings, colors, and 400 × 300 canvas. Shared operator coverage; no exhaustive trajectory comparison. |
| `snap-to-islands-in-out.tsx` | Preserves the dedicated on-island/floating state and 40 px re-entry / 80 px exit hysteresis. | Source 200 × 200 canvas. A repeated in/out trajectory checks branch history. |
| `magnetic-targets.tsx` | Preserves three centers, strengths 0.6 / 1 / 2.5, and gap reweighting through the core `change_gap` operator. | Source colors, faint `55 / strength` halos, dashed target rings, names, and puck geometry. Scene/source comparison; no exhaustive drop-zone image comparison. |
| `emerge-from.tsx` | Preserves main/emerging dots and the `Snap (20px)` toggle. Emergence uses the shared operator and renderer property. | Source coordinates, colors, and 300 × 200 canvas. Model and controls checked against source; shared emergence rendering has separate coverage. |
| `floating-dynamic-switch.tsx` | Preserves off/on state and the core floating tether `sqrt(distance) / 4`. | Source track, knob, colors, white stroke, and shadow. The regression checks that tethering clears cached `grab_position` and that the host renders the knob at `(160, 102.5)` for pointer `(160, 200)`. Shadow rasterization remains renderer-specific. |

Copying dots and clickable Frobenius nodes use the shared 2 px click/drag threshold. The runtime preserves the prepress cancellation snapshot and rebuilds modifier-sensitive behavior from the current chained step's basis state.

## Catalan Explorer

`topology/catalan.rs` follows `catalan-explorer.tsx`: a full binary tree with persistent node/edge identities, rotations, both diagonal flip orientations, whole-hump Dyck moves, parenthesis moves, and locked Tamari-edge walks. The source's n = 3 / 4 / 5 shapes and controls are retained.

Tree nodes, Dyck humps, pairing arcs, and the lattice token use source `withSnapRadius(1, {chain: true})` metadata. Diagonals and parentheses do not chain. The lattice walk locks its branch after 20 px. Chaining is driven by operator metadata; the runtime has no legacy pointer-distance chaining hooks.

Rendering preserves the source's pastel fills, dark outlines, ten Loday edge-class colors and legend order, polygon orientation and root edge, tree/leaf geometry, Dyck grids and tile bands, affine semicircle pairing shapes, endpoint dots, parenthesis typography, and miniature triangulations. The canvas is 700 × (380 + lattice height). Internal tree edges are decorative. Diagonal grabs retain the uniform-scale transform, 16 px hit stroke, and 3.5 px visible stroke. Postorder iteration controls painting and branch order, and dragged tree nodes render in front.

`topology/catalan_layouts.json` stores coordinates produced by the actual upstream `getLattice` for `(n, width, height, vertex radius)` values `(3, 560, 138, 14)`, `(4, 560, 276, 14)`, and `(5, 560, 460, 11)`. This preserves deterministic source layouts where strict floating-point hill-climb comparisons otherwise produce different coordinates. The source's `height - projectedY` orientation is retained, including its disagreement with the explanatory prose. Tree enumeration, moves, edge classes, and connectivity are computed in Rust.

Tests cover Catalan counts, identity preservation under rotations, the two Loday coordinates associated with each edge class, balanced Dyck words, and grabs at five fractions of every non-root diagonal across all 14 n = 4 trees. A numerical guard handles degenerate between-target grabs without adding legal moves.

## ZX String Diagram

`topology/strings/zx.rs` follows `zx-string-diagram.tsx`, including all four puzzles and goals. The model preserves boundary/spider/bend ports, phase arithmetic, same-color fusion, bare identity removal, Alt-unfusion with leg partitioning, phase transfer, Hopf twin cancellation, snake yanking, and parallel-wire separation. Alt-unfusion follows a fresh spider while retaining the parent's identity and phase. Free movement uses constrained `vary`, and branches retain source 150 / 200 ms transitions. Goal checking uses the source four-round graph key.

The scene preserves pastel Z/X colors, node and hit radii, negative phase notation, phase-label pills, target halos, cup/cap arcs, wires, rails, boundaries, the 470 × 500 extent, and the goal/proved panel. Ordinary non-Hopf beads are invisible until hovered or dragged but remain hit-testable; goal previews contain no bead handles. Preset buttons, reset, puzzle blurbs, and the experimental annotation follow source. Dragged spiders and bends render in front.

Tests cover CNOT cancellation, phase transport through CNOT via unfusion, phased snake reduction, parent identity and cancellation during Alt-unfusion, and runtime hover on an invisible ordinary bead. Deterministic IDs and native font rasterization are adaptations. These checks do not establish equivalence for every graph or drag trajectory.

## Frobenius diagrams

`topology/strings/frobenius.rs` and `frobenius_presets.rs` implement both source examples.

`frobenius-algebra.tsx` retains six presets/goals, ordered input/output ports, units/counits, associativity/coassociativity, the three Frobenius forms, and interchange layout. Partner selection preserves source priority. Free movement uses constrained `vary` and source relayout. Planar ports use 9 px spacing at the baseline and source Bézier stiffness; goal checking uses the four-round port-labeled graph key. The scene retains 7 px nodes, gray unit halos, serif italic μ/δ/η/ε labels, rails, the 470 × 540 extent, and the GOAL/PROVED panel.

`frobenius-tangle.tsx` retains five presets, generated normal-form goals, commutative port swaps, adjacent legal associativity/Frobenius rewrites, directed-cycle rejection, and genus preservation. Rendering keeps the 35° port splay, 7 px port placement, 22 px banks/borders, 630 × 360 canvas, and source wire stiffness. Yellow dashed legal-target halos persist through previews, cancellation, and drop. Goal labels retain `GOAL · N loop(s)` and `DONE ∎`.

Both examples retain preset buttons, source blurbs, source neighbor ordering, and dragged-generator foreground order. Tests cover noncommutative M₂(Z) tensor preservation across ordered rewrites, boundary-port validity, genus, planar snake reduction, and runtime target halos. This is structural and interaction coverage, not an exhaustive comparison of graph-isomorphism or normal-form behavior.

## Knot

`knot.rs` and `knot/{geometry,model,moves}.rs` follow upstream `knot/index.tsx`, `knot/knot.ts`, `knot/geometry.ts`, `knot/presets.ts`, `knot/tangles.json`, and the baked hard layouts. The model retains visits, control lengths/rotations, R1 twist/untwist, R2 push/pull with Alt-under behavior, R3 strand/crossing slides, geometry validity clamps, cleanup, 100-entry undo, and tricolor/writhe/Jones invariants. Free self-loop bending varies one endpoint rotation, and candidate ordering places other moves before R1 targets as source does.

The scene retains five preset buttons, disabled empty undo, invisible hover/drag handles, source line caps, and monospace invariant text. Three 80 × 62 before/after move illustrations preserve orange dashed arrows, white over-strand gaps, and original captions. The diagram remains 560 × 470; three native illustration cards extend the natural-size scrolling scene to 560 × 730. Card wrapping and padding adapt the source HTML flex layout.

Tests cover all five preset invariants, R1/R2 reversibility, R3 invariants, Jones preservation, Culprit move availability, invalid-geometry clamping, runtime twist/drop/undo, and modifier changes. Knot passes with the integrated upstream solver. Invariant checks do not establish visual equality of every bend.

## Shared geometry and solver behavior

`DragSession::with_preview_geometry` lets a host supply the rendered canvas position of an anchor, or authoritative absence. `between` measures rendered child previews; `with_floating` uses rendered anchors and retains the last visible appearance when an element disappears. Locking and snapping use cached geometry when present and rendered geometry otherwise. Tethered floating previews clear the cached grab position. Substate lifts child previews through the parent model. Without a callback, the library remains renderer independent and estimates positions recursively.

`change_result` returns a partial `DragResultPatch`. Changing a preview invalidates cached geometry unless the patch explicitly supplies it; omitted fields remain unchanged. Focused tests cover nested rotations, floating drift/disappearance, parent transforms, cache invalidation, explicit-cache precedence, and absent/non-finite geometry. The committed preview-geometry fixture executes the actual upstream renderer/operators on nested 120° rotations.

`src/cobyla.rs` and `src/cobyla_trstlp.rs` directly translate upstream `src/math/cobyla.js`, and `vary` uses this solver. The translation preserves one-based work arrays, control flow, arithmetic/evaluation order, constraint signs, simplex construction, merit updates, budgets, exit statuses, and final-iterate selection. A zero budget still evaluates the initial point. The trust-region helper retains reference V8 `Math.hypot` summation because platform `hypot` can change a rank-deficient decision. JSXGraph/jcobyla notices and the V8 BSD notice are retained. The private API follows Dragology's silent call path with its experimental rounding check disabled; internal check/status logic remains translated.

Eighteen actual-source solve traces containing 580 callback evaluations match exactly at every evaluated coordinate, final iterate, status, and count in debug and release builds. Cases include inequalities, linear/nonlinear equalities, redundant and impossible constraints, evaluation limits, zero-seed warm starts, and gear radial/phase docking. Replay uses source callback values to isolate solver arithmetic from host transcendental functions; it does not prove bit-identical native scene geometry. Live Rust callbacks separately check constrained outcomes. Eleven embedded trust-region cases pass, and a wider 256-case source comparison produced exact results. Core tests and strict Clippy pass. Reproduction instructions and generators are in [`tools/upstream-parity/README.md`](../../tools/upstream-parity/README.md).

## Native evidence and limits

Native idle images of Catalan Explorer, ZX, Knot, and both Frobenius examples were inspected. Both Frobenius scenes showed complete goal panels, source generators, preset selection, and notes without overlap. Catalan showed all six representations and the full lattice. ZX showed the CNOT diagram, hidden ordinary beads, and two-wire goal. Knot showed complete illustrations and invariants. These observations cover the inspected idle scenes, not every transition or gesture.

Knot uses natural-size scrolling to preserve legible source text. The notes area provides scroll controls. An equivalent screenshot comparison of that scrolling layout is not part of this report's evidence.

Native control placement, font rasterization, shadows, and responsive wrapping can differ from the React page. Multi-button preset semantics remain intact even where GPUI places controls in a sidebar. Shared runtime lifecycle and renderer features are covered in their respective reports; this document does not assess upstream developer diagnostic viewers.
