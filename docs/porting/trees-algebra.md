# Trees and algebra ports

These nine demos follow the rules, presets, controls, geometry, and drag feedback in upstream Dragology at commit [`b145bdd56530f7cfa9a61bf6a7170459d2f140e1`](https://github.com/declarative-dragging/dragology/tree/b145bdd56530f7cfa9a61bf6a7170459d2f140e1). The ports use the shared Rust drag engine and GPUI gallery renderer. Model and interaction tests pass; this report does not claim pixel equivalence with the browser implementation.

## Source mapping

Upstream paths below are relative to `src/demos/`. Rust paths are relative to `examples/gallery/src/`. Files listed after a directory-qualified file share that file's directory.

| Demo | Upstream source | Rust implementation |
| --- | --- | --- |
| `twisted-trees` | `twisted-trees/index.tsx`, `trees.ts`, `layout.ts` | `trees_algebra/structural.rs`, `panels.rs` |
| `two-kinds-of-tree` | `two-kinds-of-tree.tsx` | `trees_algebra/structural.rs` |
| `outline` | `outline.tsx` | `trees_algebra/structural.rs`, `panels.rs` |
| `animate-algebra-tree` | `animate-algebra/tree.tsx`, `asts.ts` | `trees_algebra/algebra.rs`, `tree.rs` |
| `animate-algebra-stage-builder` | `animate-algebra/stage-builder.tsx`, `nool-tree.ts` | `trees_algebra/algebra.rs`, `editor_layout.rs`, `tree.rs` |
| `animate-algebra-tree-macro` | `animate-algebra/tree-macro.tsx`, `nool-tree.ts`, `asts.ts` | `trees_algebra/algebra.rs`, `editor_layout.rs`, `tree.rs` |
| `lambda-beta` | `lambda/beta.tsx`, `terms.ts`, `examples.ts` | `trees_algebra/lambda.rs` |
| `proof-by-dragging` | `proof-by-dragging/index.tsx`, `logic.ts` | `trees_algebra/proof.rs` |
| `graspable-math` | `graspable-math.tsx` | `graspable_math.rs`, `graspable_math/algebra.rs`, `layout.rs` |

## Implemented behavior

### Twisted trees

Both independent 3 → 3 and 7 → 7 canvases appear together, with the source sizes, italic headings, option panel, and compact selectors. The scene preserves the unlabeled foreground circles, background circles and thick edges, cardinal curves, traditional representation, and draggable mapping arrowheads.

Mapping enumeration follows the source depth-first order. Minimum-movement choices preserve the ancestor-order constraint and stable tie ordering. The one-node and multi-node modes, interpolation methods, quality settings, and fixed-root option remain available. Good mode uses a 20 px snap radius with transitions; bad mode uses a 300 ms drop transition; worse mode disables that transition. The 50 px fallback and blue grabbed-node and target feedback follow the source.

Tests enumerate legal morphisms for both tree sizes and check circle containment.

### Two kinds of tree

The 600 × 400 canvas preserves the source colors, nested-box dimension formulas, grip positions, edges, and corner radii. Labels use an alphabetic baseline at 13 px, an 11 px font, and weight 500. Group origins follow subtree origins so that resizing a centered parent box preserves the point grabbed. Held subtrees paint above other boxes.

Reparenting considers both child-list kinds and every insertion position. Far detachment starts the source `vary` model at `(0, 0)` and uses the 50 px fallback. Sampled gestures for every movable subtree preserve the full forest, unique identities, and the held subtree. Detachment tests check that the grabbed point stays within the optimizer's subpixel tolerance.

### Outline

The simple, nested, and tree-of-life canvases remain independent and appear simultaneously. They preserve the source trees, labels, emoji, canvas sizes, 100 × 25 blocks, 20 px indentation, 2 px gaps, 13 px text, and connectors. A shared floating toggle controls the source behavior, including the 50 px fallback and foreground painting of the held subtree.

The panel adapter namespaces object identities and preserves parent links, emergence origins, ghost opacity, transitions, and chaining metadata. Tests check uniqueness and cycle prevention after reparenting, and exercise a drag that changes one panel while leaving the other two unchanged.

### Algebra tree

All 12 rewrite groups retain their source defaults. The 300 × 350 nested-box canvas follows the source depth colors, alpha, strokes, radii, and 18 px labels. Rule examples include colored operators, first-trigger highlights, titles, subtitles, and the conflict warning. Both animation controls are available.

The drag specification uses the closest of the interpolated rewrite branches, locks beyond 20 px, and snaps within 1 px with chaining. Pattern matching follows source operator-binding precedence. Repeated wildcard subtrees use clone emergence; unspecified emergence uses automatic selection. The **Force transform: scale()** control overrides only unspecified emergence and preserves explicit clone modes.

Tests instantiate every rule and check semantic value, unique identities, and preservation of the grabbed identity. Control tests exercise drag construction. Generated identities use the separate `-generated-` namespace to avoid collisions with initial nodes.

### Algebra stage builder

The 600 × 800 canvas follows the source menu, brush, palette, stage, and trash layout formulas, including bucket order, paired atoms, separators, hole and invalid-node colors, and 24 px labels. Menu and palette toggles retain their immediate behavior and source hover colors.

The builder supports insertion into holes and variadic operators, sibling swaps, Alt copying, palette placement and reordering, stage capacity constraints, deletion preview, and the ten-item recovery stack. Alt copies preserve the source emergence behavior and insertion restrictions. Hidden trash trees retain their hierarchy at zero opacity; only the recoverable root remains hit-testable while transparent. The invisible `empty-stage` object serves as an integration-test locator without adding a visible drop zone. Clickable icons use the shared click-versus-drag threshold.

Tests check parent arity and stage capacity. A standalone integration test opens and resets the demo, constructs an expression, fills its holes, deletes it, and recovers it. Keeping this test in its own executable also checks first-open generated identities.

### Algebra macro recorder

The 400 × 400 canvas preserves the source normal and recording layouts, toolkit, gutter, trash, dashed purple border, 20 px labels, validity colors, and disabled toolkit opacity. Inline controls provide Record, Stop, Cancel, per-rule deletion, and emergence settings. Stop remains disabled for malformed operators.

Toolkit insertion targets are limited to the main tree's operator slots. Gutter items support source reordering, return moves, and sibling swaps. Stopping recording preserves the gutter; applying a normal rewrite clears it. Rule derivation uses stable subtree identities to produce forward and reverse rules while ignoring animation annotations. Normal mode chooses between the original interpolated state and fixed rewrite candidates, with the source drag modifiers.

Tests cover operand generalization, stability under animation annotations, and rewrite identity preservation.

### Lambda beta

The port includes capture-avoiding substitution, alpha-renaming, reverse beta reduction, merging of repeated occurrences, eta expansion and reduction, and all 12 source presets. Preset buttons reset their expression even when already selected. Their descriptions and rewrite options follow the source.

The 680 × 420 term canvas preserves the source color assignment order, monospace and italic styling, central baselines, nested-box dimensions, translucent idle fills, opaque held fills, and held-group stacking. Nested binders use the source grouped-lambda notation. Discarded beta arguments leave an unoutlined hole, and collapse takes 450 ms. Automatic, clone, and scale emergence modes follow the source annotations.

Tests cover capture avoidance, shadowing, free-variable restrictions, beta waypoint identities, subtree copying and merging, preset round trips and resets, and a complete beta gesture through `ModelDemo`.

### Proof by dragging

All seven source problems support backward and forward links, simplification, connective click actions, undo history, and continuation identities. The 660 × 356 canvas follows the source row and formula dimensions, serif baselines, italic atoms, pill colors and strokes, hover and held feedback, disabled undo appearance, and sequent emergence origins. The Problem selector sits above the canvas.

Successful targets carry chaining metadata in the drag specification, including continuation to the undo button when a sequent closes. Branch transitions take 200 ms; click actions take 400 ms; the fallback distance is 50 px. Chaining uses the shared engine metadata.

Tests check entailment with truth tables across compound formula and link cases, unique identities after case splits, two-link modus ponens, and a continuous `p → q → goal` gesture through `ModelDemo`. Gesture tests advance one host frame per pointer move to match the source animation-frame lifecycle. Native verification also exercised selection from the Problem dropdown.

### Graspable math

All 11 source equations retain their normal forms and term-versus-factor restrictions. Supported operations include combining like terms, distribution, cancellation, fraction, power, and logarithm transformations, and the source restrictions on exact roots.

The 640 × 240 equation canvas uses serif text, alphabetic baselines, centered glyph anchors, italic variables, and the source fraction, script, parenthesis, and hover-box dimensions. Eleven inline preset buttons and Reset retain the source selection behavior and colors. Page offsets apply consistently to rendered objects and algebra drop bounds; the fallback distance is 50 px.

The merge puff preserves its five-bump outline, three wisps, grabbed identity, and preview-only lifetime. Solved feedback appears only while idle and uses two filled rounded rectangles with blur radii of 14 and 5.

Tests cover every parsed solution fixture, preservation of solutions across sampled candidate gestures, a four-gesture solution of `2x + 3 = 11`, even-root and negative-power restrictions, preset controls, and feedback geometry and visibility.

## Verification

The focused tests and standalone integration test pass:

```sh
cargo +stable test -p dragology-gallery --lib trees_algebra
cargo +stable test -p dragology-gallery --lib graspable_math
cargo +stable test -p dragology-gallery --test algebra_builder
```

These tests also pass in the full gallery test run. They cover semantic invariants, identities, controls, and complete engine gestures. Scene geometry and styling were reviewed against the pinned source, and exported model scenes were visually inspected. Native checks exercised the compact selectors, including changing the Proof problem. The shared renderer implements the text, stroke, blur, emergence, and control metadata used by these demos.

## Remaining differences and verification limits

- Graspable preset labels use Unicode superscripts and subscripts instead of mixed HTML text runs.
- The macro recorder's recording dot is static; the source applies a pulsing CSS animation.
- The builder's hover color changes immediately; the source uses a 100 ms CSS transition.
- Native controls use explicit dimensions in place of CSS intrinsic sizing. Font shaping, emoji fallback, blur rasterization, responsive wrapping, and transition timing have not been exhaustively compared with browser output across all states.

No model operation or preset is intentionally omitted from these nine demos. Passing tests establishes the covered behavior, not frame-by-frame visual equivalence.
