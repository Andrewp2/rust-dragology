# Puzzle and balancing ports

The GPUI gallery ports 13 puzzle demos and the four independent diagrams in `rotations-rebalancing` from [Dragology revision `b145bdd56530f7cfa9a61bf6a7170459d2f140e1`](https://github.com/declarative-dragging/dragology/tree/b145bdd56530f7cfa9a61bf6a7170459d2f140e1). The implementations preserve source models, legal moves, presets, controls, geometry, colors, and interaction feedback. They use the shared Rust drag engine for previews, floating objects, snapping, chaining, and transitions.

The puzzle implementations are in [boards.rs](../../examples/gallery/src/puzzles/boards.rs), [collections.rs](../../examples/gallery/src/puzzles/collections.rs), and [tilings.rs](../../examples/gallery/src/puzzles/tilings.rs). Balancing diagrams are in [tree_balancing.rs](../../examples/gallery/src/tree_balancing.rs) and its algorithm modules. Upstream files below are relative to `src/demos/` at the pinned revision.

## Source mapping

| Gallery entry | Upstream source | Preserved behavior and presentation |
|---|---|---|
| `tiles` | `tiles.tsx` | Two independent 300 × 300 figures, with one and two tiles respectively. Source 50 px cells, gray grid, tile paint and labels; 3 px animated chained snapping with branch transitions disabled. |
| `grid-poly` | `grid-poly.tsx` | Original polygon, 350 × 350 figure, gray grid points and black handles. Discrete candidates exclude grid points already occupied by another vertex. |
| `grid-poly-vary` | `grid-poly-vary.tsx` | Continuous two-parameter vertex movement, clamping to the 0–5 grid range during dragging, rounding on drop, and 10 px snapping. Vertex collisions remain permitted. |
| `rush-hour` | `rush-hour.tsx` | Original eight cars, lane and collision rules, and right-edge exit behavior. Source car colors, full-cell rectangles, black outlines, gray border and exit opening in a 350 × 350 figure. |
| `fifteen` | `fifteen.tsx` | Original shuffled permutation and legal exchanges with the adjacent blank. The 250 × 250 board uses 50 px tiles and 10 px chained snapping; the transparent blank remains draggable. |
| `hanoi` | `hanoi.tsx` | Independent three-disk and four-disk figures at 500 × 200 and 600 × 200. Only a peg's top disk moves, and it can land only on an empty peg or a larger disk. Source peg and disk geometry, colors, labels, and floating preview with ghost opacity 0.5. |
| `sokoban` | `sokoban.tsx` | Original board, single-box push rules, editable mode, `lockPast(20)`, and 3 px animated chained movement. Source 500 × 500 figure with 50 px cells, black walls, brown boxes, orange goals above boxes, and the standing-person glyph. |
| `spinny` | `spinny.tsx` | Four labeled circles on the source 100 px orbit, clockwise and counterclockwise branches, and 10 px chained snapping. Retained translate/rotate/translate/counter-rotate operations preserve the orbit and upright labels during interpolation. |
| `tromino` | `tromino.tsx` | Levels 1–4, all snappy/maze combinations, recursive L-shaped tiling, and legal single-rotation maze moves. Source padding, pastel colors, black missing square, size cap, and floating ghost opacity 0.2. |
| `card-piles` | `card-piles.tsx` | Original six card positions, strict overlap, transitive pile connectivity, group movement, and splitting when a card leaves. Source 500 × 400 figure, 70 × 100 cards, pile opacity 0.4 or 0.7 while held, and foreground dragged cards. |
| `tactile-tessellations` | `tactile-tessellations.tsx` | Palette creation with persistent shape identities, center anchoring, compatible-edge snapping with least rotation, and overlap rejection with tolerance 2. Source 600 × 450 figure, palette and polygon styling, dragged-object ordering, and dashed trash target. |
| `plane-partition` | `plane-partition.tsx` | Dimensions `a`, `b`, and `c` from 1–6; hints, Randomize, and Reset. Legal corner additions/removals, original staircase and seeded layouts, 30 px isometric geometry, ghost faces, and a bottomless spare supply. Far drops cancel creation using the source 50 px threshold. |
| `aztec-diamond` | `aztec-diamond.tsx` | Orders 1–8, height display, dimming of stuck dominoes, Shuffle, and Reset. Source block ordering, signed winding angles, quarter-turn settling with JavaScript tie rounding, colors and height labels. Shuffle starts from the horizontal tiling; changing order preserves the seed. Pivot/spin/offset transforms and 120 ms branch transitions remain explicit. |
| `rotations-rebalancing` | `rotations-rebalancing/{index.tsx,bst.ts,render.tsx,splay.tsx,balanced.tsx,two-three.tsx,heap.tsx}` | Four simultaneous independent sections: splay, AVL/red-black, 2–3 tree, and heap. Source zig/zig-zig/zig-zag steps, AVL rotations, Okasaki red-black insertion, 2–3 split promotion, and heap append/sift/extract behavior. Original decks, trays, captions, labels, controls, reset buttons, and diagram dimensions. |

Tiles, Hanoi, and balancing sections have independent state and controls. The balanced-tree section alone switches between AVL and red-black modes. Reset buttons retain the source click behavior and 300 ms transition; changing the balanced-tree mode remounts its initial state without a transition. The 2–3 containers use clone emergence with wrapped rectangle geometry, as in the source.

The shared renderer implements source text anchors, baselines, font styling, affine glyph transforms, stroke caps and joins, dashes, clipping, and scrolling. Source notes use the native rich-text surface. See [scene interpolation](interpolation.md) for color, transform, path, and emergence behavior.

## Verification

Focused puzzle, balancing, interpolation, and runtime tests pass. Puzzle and balancing coverage includes:

- Legal drag specifications through actual drag sessions, including all board variants.
- Independent Tiles, Hanoi, and balancing sections under pointer gestures and control changes.
- Sokoban push restrictions, Rush Hour lane/collision invariants, and Fifteen blank adjacency.
- Transitive card groups, group splitting, polygon edge snaps, and center anchoring during palette creation.
- Plane-partition corner legality, bottomless supply during creation, and canceled far drops.
- Domino coverage, single-vertex height changes of ±4, and seed-preserving order changes.
- AVL and red-black invariants across 67 insertion orders; 2–3 ordering, arity, and equal leaf depth; all splay step types; heap insertion, sifting, extraction, and reset.

The seeded tiling tests compare against executed upstream JavaScript for a 3 × 3 × 3 plane partition and an order-2 Aztec diamond, both with seed 123. The [reference generator](../../tools/upstream-parity/reference-puzzles.cjs) reads the pinned source and uses its original dependencies. [Regeneration instructions](../../tools/upstream-parity/README.md) describe checkout validation and fixture locations. Ordinary Rust tests are offline:

```sh
cargo test -p dragology-gallery --lib puzzles::
cargo test -p dragology-gallery --lib tree_balancing::
```

Native samples have been inspected in GPUI. Automated model checks and sampled visual inspection do not establish exhaustive pixel equivalence across every state, platform, or font configuration.

## Remaining differences

- Native controls, text shaping, font fallback, and emoji rendering can differ from the browser. The source's figure sizes and styles are retained, but the surrounding application and note layout use native widgets.
- The plane-partition spare-bin hit region includes an approximate label descent, ending at local y = 54.5 px. Its cube and ellipse extents are exact; browser text metrics can change the original group's final hit bounds slightly.
- New random seeds come from the platform's randomized hash state rather than `Math.random`. Both choose from 1 through 1,000,000,000; once selected, the source Mulberry32 generation and layout algorithms are preserved.
- Card piles are derived from absolute card positions instead of stored as nested pile offsets. Tests cover the resulting connectivity, movement, and splitting behavior.
- The native scene representation has the interpolation limits documented in [interpolation.md](interpolation.md), including 8-bit alpha and normalized SVG path commands.

The upstream demos are MIT-licensed by the Dragology contributors. Attribution is retained in [LICENSE](../../LICENSE) and [THIRD_PARTY_NOTICES.md](../../THIRD_PARTY_NOTICES.md).
