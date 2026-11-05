# Collections, braids and permutations

These 21 demos port the behavior and presentation of [Dragology revision `b145bdd56530f7cfa9a61bf6a7170459d2f140e1`](https://github.com/declarative-dragging/dragology/tree/b145bdd56530f7cfa9a61bf6a7170459d2f140e1). The source paths below are relative to `src/demos/` at that revision.

The collection models live in `examples/gallery/src/lists.rs`, `lists/components.rs` and `lists/nested.rs`. Braid and permutation models live in `examples/gallery/src/topology/braids.rs`, with drawing code in `braids_geometry.rs` and `permutation_geometry.rs`. Shared text layout in `examples/gallery/src/text_layout.rs` measures and draws text through the same font database and shaping engine.

## Collection demos

| Demo | Source | Implemented behavior and presentation |
| --- | --- | --- |
| `perm` | `perm.tsx:12–75` | A–E on a 350 × 100 canvas. White 50 px square tiles have black 2 px borders and centered 20 px letters. Reordering uses `between`; the grabbed tile rises 10 px. |
| `perm-floating` | `perm-floating.tsx:14–75` | The same initial tiles use `closest`, `whenFar` and floating. Pulling away cancels the reordering on release. |
| `perm-double` | `perm-double.tsx:12–104` | Three rows on a 250 × 200 canvas. Dragging reorders a tile within its row and moves that entire row vertically. The candidates include every column/row combination, with a 10 px snap radius and the source stacking order. |
| `lists-2d` | `lists-2d.tsx:17–176` | Source row palette, padding, rounded borders, labels and six-dot grips. Tiles move between rows; rows reorder independently. Both use closest/floating previews and allow removal when pulled away. An empty row remains 26 × 66 px. |
| `lists-2d-sizes` | `lists-2d-sizes.tsx:17–180` | The source 400 × 450 canvas and special 108 × 100, 108 × 50 and 50 × 108 px tiles. Row dimensions follow their contents; tile transfer and row reordering preserve item identities. |
| `canvas-of-lists` | `canvas-of-lists.tsx:33–211` | Source 600 × 400 canvas, initial row positions and eight-color palette. Rows move freely. Tiles transfer between rows or detach into a new row beyond the 50 px far threshold. New rows use the smallest available row number and source color cycle. |
| `lists-in-lists` | `lists-in-lists.tsx:1–267` | The source tree has two roots. Tiles and complete subtrees move between nesting levels. Removing the subtree before enumerating destinations prevents cycles. The 60 px far branch creates a freely positioned root row. |
| `lists-in-lists-vanilla` | `lists-in-lists-vanilla.tsx:29–559` | The separate imperative comparison starts with three roots on a 600 × 400 canvas. It keeps the destination layout fixed during dragging, scores full insertion lines, and shows a 70%-opaque floating item and 3 px insertion indicator. It preserves the 50 px threshold, recursive tie order, far-drop positioning and absence of drop animation. |
| `kanban` | `kanban.tsx:24–175` | Source 400 × 200 canvas, gray rounded columns, headers and 110 × 30 px cards. Cards transfer with a 0.3-opacity ghost and blue drag feedback. Columns interpolate between orders with a 20 px snap transition; the unchanged candidate comes first. A grabbed column and its cards rise together. |
| `ring-of-beads` | `ring-of-beads.tsx:18–138` | Five pastel beads on a radius-100 ring centered at (150, 150). All four Stage options preserve their operator sequences, drag shadow and stacking. The 65 px far branch previews removal but restores the original state on release. |
| `insert-and-remove` | `insert-and-remove.tsx:16–183` | Source fruit palette, toolbar, list and trash. Pressing a store item creates and follows a fresh item while retaining the stock item. All insertion slots, trash targeting and far-away creation cancellation are available. Fresh IDs are deterministic rather than UUIDs. |
| `insert-and-remove-original` | `insert-and-remove-original.tsx:17–198` | Preserves the original stock-renaming and floating technique, including the tile shown over the trash before release. It also preserves the source's double-splice deletion: deleting B from A, B, C leaves A. The modern variant leaves A, C. |
| `todo` | `todo.tsx:26–232` | Source tasks, completion states, 400 × 450 canvas and fixed 55 px item stride. Supports adding, checkbox toggling, continuous reordering, and floating reordering/removal. Draft identity carries into the new card. Only an empty string is rejected; whitespace remains valid, and Enter does not add a task. Native input and Add controls occupy the source positions. |
| `text-box` | `text-box.tsx:10–65` | Source initial text, 400 × 150 canvas, input at (20, 20), and 20 px output text on baseline (20, 100). Unicode edits update immediately without a transition. The native input supports IME editing. |
| `carousel` | `carousel.tsx:10–161` | Source 450 × 300 outer canvas and 400 × 250 carousel, four slide titles/colors, arrows and navigation dots. Supports adjacent-slide swiping, dragging the active dot across all slides, and clicking arrows or inactive dots. Inactive slides retain zero opacity; the source's lack of clipping is preserved. |
| `bluefish-static` | `bluefish/static.tsx:9–64` | Static planet diagram with source background, four planets, Mercury reference group and label. Background bounds are (10, 10), 450 × 156 px; planet centers are x = 65, 166, 290, 399 at y = 88, with radii 15, 36, 38 and 21 px. Mercury's surround uses the actual measured width of its 14 px bold label. |
| `bluefish-perm` | `bluefish/perm.tsx:12–84` | Source 300 × 120 canvas and Bluefish layout: 10 px outer padding, initial tile positions (10 + 50i, 30), and a 20 px drag lift to y = 10. Preserves continuous reordering, alphabetic text baselines and the requested Alegreya Sans font. |
| `bluefish-lists-2d-sizes` | `bluefish/lists-2d-sizes.tsx:29–181` | Source variable-sized tiles and row operations with Bluefish layout. Initial row origins are y = 10, 136, 212, with 10 px outer padding and 7 px grip spacing. Empty rows measure 34 × 33 px. |

Todo text follows CSS whitespace collapse and Unicode line-break opportunities. Line widths and baselines use actual font metrics; completed text uses shaped line-through decoration. Cards grow with wrapped text while their list origins retain the source's fixed 55 px spacing. This preserves the source's visible overflow and possible overlap instead of changing the list layout.

Bluefish positions were checked by executing the pinned layout library with controlled font measurements. The resulting layouts preserve its viewBox padding, drag lift and empty-row dimensions. Text measurement at runtime uses the same font selection as drawing.

## Braid and permutation demos

| Demo | Source | Implemented behavior and presentation |
| --- | --- | --- |
| `braid-old` | `braid-old.tsx:14–173` | Source 250 × 400 vertical monochrome diagram, initial 50 px straight segment, 2 px wires, white underpass circles and small gray endpoints. Clicking a crossing flips its sign. Endpoint drags append or remove crossings with a 1 px chained snap radius. |
| `braid` | `braid.tsx:147–1892` | Source 680 px diagram, open/closed origins, capacity-based column widths, 28 px row spacing, six-color palette, white underpass halos, cross-section projection, closure loops and component colors. Includes all 11 presets and source n, closure, slice and reset behavior. |
| `permutation-lab` | `permutation-lab.tsx:75–1745` | All five source views: two-line notation, cycles, matrix, permutohedron and word/wiring diagram. Preserves their geometry, colors, typography, transparent handles, hover feedback, source initial permutations for n = 3–6 and reset-to-identity behavior. |

The modern braid implements signed commutation and Reidemeister III moves, inverse-pair insertion and cancellation, projected half-turn gestures, forward and backward conjugation around closure loops, stabilization at either end, and their inverse moves. Retained zero-width columns let cancellation animate as a horizontal collapse with stable drawing identities. The slice marker follows the full padded-wire hover region. Capacity limits and unavailable-move feedback follow the source.

The permutation lab supports adjacent and free tile modes, cycle insertion and splitting, matrix row swaps constrained to a column, and chained walks on the permutohedron. The n = 4 solid uses the source quaternion view and 14 faces; its Schlegel projection uses inverse-permutation coordinates. Face turning retains silhouette constraints. Walk corners, spurs and hexagons implement the corresponding rewrites, alongside wiring commutation, Reidemeister III, continuous double-crossing insertion/cancellation and endpoint generators. Logical walk-vertex identities remain stable through rewrites.

All six source chained-snap sites use engine metadata with `SnapOptions { chain: true, transition: None }`. The cross-section clamps the pointer through `change_frame`. Chaining depends on preview/drop geometry, so it can continue when the pointer lies outside the candidate hull. Gesture behavior runs once per frame; release performs its final evaluation.

## Verification and limits

The full gallery test run passes, including 11 collection tests, 17 braid/permutation tests and 3 shared text-layout tests. Focused coverage includes:

- Item identity during reordering, transfer, detachment, creation and cancellation; subtree exclusion; full-height insertion-line scoring; and the distinct original/modern fruit deletion results.
- Source origins, empty-row dimensions and Bluefish drag lift; proportional-font wrapping, whitespace and nonbreaking spaces; growing Todo cards at fixed row origins; and floating task removal.
- Signed crossing invariants, both stabilization inverses, half-turn direction/cancellation/chaining, a 51-point closure-conjugation trace, and whole-diagram insertion/cancellation motion.
- Permutation word/cycle round trips, matrix row swaps, the 24-vertex/36-edge planar Schlegel projection, walk rewrites, capacity limits and configuration resets.
- Chaining with the pointer 1,000 px outside a permutation endpoint's candidate hull, plus a distant-pointer case for the old braid. Text metrics are checked against shaped anchor displacement, proportional glyph widths and trailing-space advances.

Native screenshot inspection covered the default Todo, Text box, Bluefish permutation and Permutation lab scenes. The captures show the controls, card content, tile row and five permutation views rendering visibly without clipped scene content. These captures do not establish browser/native pixel equality or verify every interaction state.

Native widgets preserve control values and actions but differ from HTML controls in styling and grouping. The bead and permutation configuration radios use native choice controls; braid presets use a selector instead of two button groups. Platform font availability and rasterization can also change appearance, including the fallback used when Alegreya Sans is unavailable. No exhaustive visual comparison across every state is claimed.

Vanilla-comparison notes use the recorded `native_markdown` adaptations in `native-note-adaptations.json`. Original source Markdown and provenance remain intact, while the displayed notes describe the native implementation accurately.
