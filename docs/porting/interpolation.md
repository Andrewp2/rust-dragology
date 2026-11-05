# Scene interpolation

The native scene interpolator follows [Dragology revision `b145bdd56530f7cfa9a61bf6a7170459d2f140e1`](https://github.com/declarative-dragging/dragology/tree/b145bdd56530f7cfa9a61bf6a7170459d2f140e1), particularly `src/svgx/lerp.tsx` and `src/svgx/transform.ts`. Its implementation is in [runtime/scene.rs](../../examples/gallery/src/runtime/scene.rs), with retained transform and emergence metadata in [api.rs](../../examples/gallery/src/api.rs).

## Interpolation rules

| Feature | Source | Native behavior |
|---|---|---|
| Colors | `svgx/lerp.tsx`, `d3-color` 3.1.0, `d3-interpolate` 3.0.1 | D50 Lab interpolation with the source conversion constants, sRGB transfer function, transparent-channel handling, and rounded/clamped RGB output. Explicit `none` paint fades using the source's opaque counterpart rule, including its handling of a fully transparent color. |
| Paths | `d3-interpolate-path` 2.3.0 | Segment-count equalization, uniform-parameter De Casteljau splitting, asymmetric command conversion, trailing closure handling, and empty-path growth/retraction. At `t = 1`, the original target commands return; at `t = 0`, the converted and extended source commands remain. Extrapolation follows the same arithmetic. |
| Transforms | `svgx/transform.ts` | Ordered translation, rotation about a center, and scale operations. Adjacent translations collapse; missing identity translations/scales are normalized for emergence. An exact 180° tie rotates in the negative direction. |
| Emergence | `createSyntheticBefore` and `augmentWithEmerging` in `svgx/lerp.tsx` | Auto mode copies the origin's first direct rectangle width/height and first text y position at zero opacity. Clone mode uses those bounds at full destination opacity. Explicit Scale, and Auto without suitable rectangles, start at exact scale zero. Synthetic counterparts work in both directions and do not become origins for other synthetic objects in the same pass. |
| Discrete properties | Unblended-property selection in `svgx/lerp.tsx` | Springs take text, cursor, static metadata, and stacking order from the target throughout the transition. Weighted previews take them from the heaviest state, with ties going to the earlier state. Numeric geometry and its hit region continue interpolating. |

`Object::transforms` retains the source operations and computes an affine matrix for rendering and hit testing. Scene offsets and floating translations preserve both representations through `prepend_translation`. Objects with only a matrix use affine decomposition; component lists that cannot be matched also fall back to matrix interpolation.

Spinny retains `translate(center) rotate(angle) translate(radius) rotate(-angle)`. Aztec retains `translate(pivot) rotate(spin) translate(offset)` plus the domino's inner rotation, including at rest. Retained operations also cover the angle, stretchy, clock, robot, and gear diagrams. These lists preserve trajectories that cannot be recovered from a single flattened matrix.

`Object::emerge_bounds` distinguishes direct rectangle/text children from geometry wrapped inside an SVG group. The 2–3 tree containers disable bounds morphing to preserve the source's wrapped-rectangle clone behavior.

## Verification and reproduction

The gallery's interpolation, emergence, and runtime tests pass. They check source color/path/transform outputs, empty and mixed-curve paths, pivoted trajectories, target and strongest-state metadata policies, direct and wrapped emergence, multiple origins, clone paint order, absent-origin fading, and zero-scale growth. Native samples have been inspected; these checks do not establish exhaustive browser-to-GPUI pixel equivalence.

The retained generators execute the pinned source or its installed dependencies:

| Generator | Reference coverage |
|---|---|
| [reference-interpolation.cjs](../../tools/upstream-parity/reference-interpolation.cjs) | Nine color cases, 18 path cases, and five transform cases. |
| [reference-emergence.cjs](../../tools/upstream-parity/reference-emergence.cjs) | All six Auto/Clone/Scale × direct/wrapped rectangle combinations, evaluated with React. |
| [reference-none.cjs](../../tools/upstream-parity/reference-none.cjs) | Explicit `none` against opaque, partial-alpha, and zero-alpha colors. |

Expected values are embedded in Rust tests. The generators accept an upstream checkout path and validate its pinned revision and relevant source files. [Regeneration instructions](../../tools/upstream-parity/README.md) describe dependency setup and commands. Normal tests require neither JavaScript nor an upstream checkout:

```sh
cargo test -p dragology-gallery --lib runtime::scene::
```

Continuous drag optimization uses the original COBYLA algorithm translated in [cobyla.rs](../../src/cobyla.rs) and [cobyla_trstlp.rs](../../src/cobyla_trstlp.rs). The solver's 18 source traces match all 580 callback evaluations, final iterates, statuses, and evaluation counts; core tests pass. Direct trust-region references cover 256 cases, with 11 representative cases embedded in unit tests. The reference Node 22.14.0 implementation of `Math.hypot` is retained because its rounding affects degenerate active sets. These solver checks are separate from the scene interpolation fixtures above.

## Representation limits

- `BezPath` represents absolute move, line, quadratic, cubic, and close commands (`M/L/Q/C/Z`). Relative commands, shorthand commands, and arcs are normalized before interpolation, so their original SVG command forms are not preserved. A path containing only `Z` safely becomes empty; malformed command correspondences can retain NaN coordinates as in the source.
- Native colors store 8-bit alpha; SVG color strings can retain more alpha precision. The executed Lab fixtures match within the native representation.
- `Option<Color>` treats missing native paint as explicit SVG `none`. It does not distinguish omitted or inherited SVG properties.
- Objects without retained transform components cannot recover their original transform lists from an affine matrix. The flat object/group model also does not represent arbitrary SVG DOM structure.
- Emergence uses the first native rectangle/text shapes and the explicit `emerge_bounds` flag to represent direct-child structure. Arbitrary nested SVG structural interpolation is not supported.
- Upstream removes disappearing elements below opacity 0.001. Native scenes retain them; zero-opacity objects produce no visible output and are not hit-tested unless explicitly enabled.
- Shadow, clip, and stroke-style metadata are selected from the chosen object rather than recursively interpolating every SVG/CSS property. Rendering supports those styles, but rendering support does not imply interpolation of every style field.
- System fonts, antialiasing, compositing, and native widget rendering can differ from browser output even when scene geometry and paint values match.

## Attribution

The translated interpolation code retains the full ISC notices for `d3-color` and `d3-interpolate`, and the BSD-3-Clause notice for `d3-interpolate-path`. The COBYLA translation retains its JSXGraph/jcobyla MIT notices and the V8 notice for `Math.hypot`. Distribution notices are collected in [THIRD_PARTY_NOTICES.md](../../THIRD_PARTY_NOTICES.md); source headers retain the corresponding copyright and license text.
