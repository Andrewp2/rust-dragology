# Basics port review

The 28 demos in `examples/gallery/src/basics.rs` and `examples/gallery/src/basics/` follow [Dragology revision b145bdd56530f7cfa9a61bf6a7170459d2f140e1](https://github.com/declarative-dragging/dragology/tree/b145bdd56530f7cfa9a61bf6a7170459d2f140e1). The upstream paths below refer to that revision. Entry titles use the original demo IDs. The deliberately broken identity example retains its error.

The demos use the core source operators and the Rust port of the upstream numerical solver. The shared runtime evaluates dragging once per frame, evaluates initialization without chaining, starts transitions when the active branch path changes, and evaluates release once. Chaining and transition policies come from the drag specifications. Click-capable shapes use the source’s two-pixel pending-drag threshold.

## Demo behavior

Port paths in this table are relative to `examples/gallery/src/`.

| Demo and upstream file | Port file | Implemented behavior |
|---|---|---|
| `linear-track` — `src/demos/linear-track.tsx` | `basics/switches.rs` | A black 40 px square follows the source track on a 150 × 80 canvas. Targets retain true-before-false ordering, a 10 px snap radius, and a 200 ms elastic-out drop transition. |
| `linear-track-chained` — `src/demos/linear-track-chained.tsx` | `basics/switches.rs` | The 400 × 100 zigzag uses adjacent `between` branches. A 10 px snap radius triggers chaining at endpoints. |
| `simple-triangle` — `src/demos/simple-triangle.tsx` | `basics/switches.rs` | Four independent modes appear in source order: closest, closest with floating, between, and between with floating. Each diagram preserves the source positions, labels, outlines, and indicator paint order. |
| `simple-triangle-custom` — `src/demos/simple-triangle-custom.tsx` | `basics/switches.rs` | The custom nearest-state operator uses the source 200 × 150 diagram and reports the selected active branch. |
| `simplest-clicker` — `src/demos/simplest-clicker.tsx` | `basics/switches.rs` | Clicking cycles a 100 px square through CSS red, green, blue, and yellow while moving it in 40 px increments. CSS green is `#008000`. |
| `click-and-drag` — `src/demos/click-and-drag.tsx` | `basics/switches.rs` | Clicking changes color independently of the position changed by dragging. The source four-color palette, 60 px track, target ordering, and elastic-out drop policy are preserved. |
| `switch-with-sharpness` — `src/demos/switch-with-sharpness.tsx` | `basics/switches.rs` | Six switches share one state and use the source interpolation exponents. The 520 × 210 arrangement includes the original labels, colors, and thumb shadows. |
| `study-switch` — `src/demos/study-switch.tsx` | `basics/switches.rs` | The switch starts off and blends between the source off/on states. Its 200 × 200 canvas contains the 120 × 60 track, white thumb, border, and shadow. |
| `study-switch-clickable` — `src/demos/study-switch-clickable.tsx` | `basics/switches.rs` | The track and thumb toggle on click; the thumb also supports `between` dragging. Source geometry and pointer cursors are preserved. |
| `study-three-way-switch` — `src/demos/study-three-way-switch.tsx` | `basics/switches.rs` | The source positions, colors, translucent targets, outline, and shadow appear on a 200 × 200 canvas. Floating and clockwise-only options default to false. Clockwise movement follows r → b → g → r with a 15 px chained snap radius. |
| `study-reorderable-list` — `src/demos/study-reorderable-list.tsx` | `basics/studies.rs` | The source rows, labels, and controls support interpolated reordering or `closest.withFloating`, with optional 10 px snapping. Floating mode accepts distant drops; it has no `whenFar` fallback. |
| `study-slider` — `src/demos/study-slider.tsx` | `basics/studies.rs` | The value starts at 100 and stays within 0–240. The track, fill, thumb, shadow, and centered readout follow the source; rounding matches JavaScript `Math.round`. |
| `study-rotary-dial` — `src/demos/study-rotary-dial.tsx` | `basics/studies.rs` | An unbounded angle controls the source dial, pointer, ticks, and degree readout. The face radius is 60 px, the pointer is 46 px long, and ticks span radii 66–76 px. |
| `study-timeline` — `src/demos/study-timeline.tsx` | `basics/studies.rs` | Three named blocks move continuously within horizontal bounds 0–260 and discretely among four tracks. The dragged block paints last; branch transitions use 100 ms cubic-out easing. |
| `study-timeline-substate` — `src/demos/study-timeline-substate.tsx` | `basics/studies.rs` | The same timeline geometry and constraints use a block-specific drag specification lifted through Rust `substate`. |
| `study-timeline-embed` — `src/demos/study-timeline-embed.tsx` | `basics/studies.rs` | Reusable block rendering and drag helpers provide the source timeline behavior, including 100 ms branch transitions. Native helpers and `substate` perform the lifting represented by the upstream `embed()` API. |
| `error-study-reorderable-list` — `src/demos/error-study-reorderable-list.tsx` | `basics/studies.rs` | Positional identities deliberately reproduce the source coincident-target error. The row geometry, colors, and labels remain intact. |
| `linked-sliders` — `src/demos/linked-sliders.tsx` | `basics/studies.rs` | Each slider moves itself and the suffix below it while every value remains within its track. The 340 × 204 layout includes the source labels, fills, outlines, shadows, and rounded readouts. |
| `lerping-keys` — `src/demos/lerping-keys.tsx` | `basics/studies.rs` | The upper dots use positional identities; the lower dots use stable identities. Source colors, notches, row positions, and labels expose their different interpolation behavior. |
| `lerping-text` — `src/demos/lerping-text.tsx` | `basics/switches.rs` | The mark blends among the three corners while its text comes from the strongest-weight branch. The 300 × 270 diagram includes dashed targets, source label offsets, and a bold white mark label. |
| `between-with-vary` — `src/demos/between-with-vary.tsx` | `basics/composition.rs` | `between` combines a continuously varying slider, constrained to 0–100, with two fixed islands. Active/inactive paints, labels, track, and knob follow the source. |
| `stickiness` — `src/demos/stickiness.tsx` | `basics/composition.rs` | The source black square and three targets use `closest` with stickiness choices 0, 10, 30, and 60, defaulting to 0. |
| `lock-past` — `src/demos/lock-past.tsx` | `basics/composition.rs` | The source hub and five spokes use branch locking with choices off, 10, 20, and 30, defaulting to 20. `with_chaining()` allows continuation when the selected state changes. |
| `controlled` — `src/demos/controlled.tsx` | `basics/controlled.rs` | Both `onDropState` and `onDragState` columns contain simple, double-switch, rejection, override, and double-slider examples. Ten switches and four sliders have independent widget engines and animation clocks. Accepted switch drops retain elastic easing; rejected or overridden state corrections use the default cubic transition. The external slider changes its circle immediately. |
| `with-pointer-delay` — `src/demos/with-pointer-delay.tsx` | `basics/composition.rs` | A black 15 px circle starts at (200, 200) on a 400 × 400 canvas. The default delay is 250 ms, adjustable from 0–1000 ms in 10 ms steps. A deterministic host queue applies the delay once, and release preserves the delayed position. |
| `change-frame` — `src/demos/change-frame.tsx` | `basics/composition.rs` | The point starts at −45° on the source circular track. Reflecting the pointer with y → 300 − y makes the point follow the cursor’s reflection. |
| `component-elements` — `src/demos/component-elements.tsx` | `basics/studies.rs` | Four cards contain inline `+1` buttons, editable notes, counters, mount times, grips, and context-controlled accents. Controls follow their card identities during floating and reordering. The source 50 px `whenFar` fallback and dragged-group layering are preserved. |
| `component-elements-lerp` — `src/demos/component-elements-lerp.tsx` | `basics/composition.rs` | Both six-bar charts retain independent selection state. The left chart blends source data and colors; the right displays the nearest preset. The knob and left chart use the source 200 ms elastic-out drop transition. |

## Verification

All 20 focused basics tests pass, including in the latest full gallery test run. Coverage includes:

- Unique demo and object identities with finite initial geometry across all 28 demos.
- Snap-triggered chaining, continuation before reaching a spoke endpoint, custom branch identity, and elastic drop policies.
- Equivalent timeline constraints and branch timing across all three variants, plus linked-slider suffix movement and bounds.
- Different distant-drop policies for the study list and component cards, persistence of card state, control parenting, and the intentional coincident-target error.
- Controlled-widget parent callbacks, independent animation clocks, held-pointer retargeting, rejected and overridden drop corrections, external toggles, and immediate external-slider changes. Coalesced pointer events cannot make the rejection parent accept an intermediate state that was never evaluated in a frame.
- Reflected-pointer movement and the two charts’ different interpolation settings.

Native GPUI checks confirmed that component-card buttons increment their counters, note inputs accept text including the `R` key without resetting the demo, and the controlled page scrolls to its lower examples and external slider. These checks and the automated tests do not establish pixel-for-pixel equality with the browser gallery or cover every possible gesture.

## Native adaptations and remaining limitations

The four-mode triangle and controlled examples combine separate browser canvases into scrolling native scenes. Diagram coordinates follow the source, but HTML page spacing, responsive layout, text shaping, native control appearance, and shadow rasterization can differ. The controlled page preserves independent widget clocks despite this layout adaptation.

Rust helpers and `substate` replace the React-specific `embed()` mechanism. Native component state replaces React hooks, and scene geometry/color interpolation represents the bar chart’s component-property interpolation. Card mount times use a local 12-hour format rather than browser locale negotiation.

The source debug inspector, operator tree, and overlays are not implemented for these demos. In particular, the buffered-pointer overlay for `with-pointer-delay` and the reflected-pointer overlay for `change-frame` are absent. Pointer delay runs in the native host rather than a public custom `Modifier` subclass. Its native control labels the unit as `Delay (ms)` instead of formatting the value as, for example, `250ms`.

The generated source notes in `examples/gallery/data/upstream-notes.json` cover all 119 listed demos, with 98 notes and 14 preserved links. Extraction has no unresolved expressions at the pinned revision. Original Markdown remains intact; documented `native_markdown` adaptations clarify platform-specific claims and unavailable debug views. Demos without source notes receive no invented instructions.
