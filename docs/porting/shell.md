# Shared GPUI renderer and native controls

The gallery ports the presentation and interaction rules of [Dragology at commit `b145bdd56530f7cfa9a61bf6a7170459d2f140e1`](https://github.com/declarative-dragging/dragology/tree/b145bdd56530f7cfa9a61bf6a7170459d2f140e1) into GPUI. Its shared scene API is in `examples/gallery/src/api.rs`; native rendering and controls are in `examples/gallery/src/shell/` and `main.rs`. These capabilities support the source demos, but do not provide an arbitrary SVG or HTML renderer.

## Drawing and text

Text uses SVG shaping through usvg and a shared system-font database. Glyph outlines follow the object's full affine transform, including rotation, shear and nonuniform scale. `TextStyle` represents source anchors, baselines, font family, weight, italic style, letter spacing and strikethrough. Text measurement and painting share `text_layout`, so measured wrapping and mixed-script labels use the same font advances as the drawing. An optional outline paints behind glyph fills for source text halos.

Strokes are outlined in local coordinates before transformation. This preserves scaling behavior, cap and join styles, miter limits, dash patterns and dash offsets. Non-scaling strokes transform the centerline first. Shapes support nonzero and even-odd fills, local rectangle/path clips, Gaussian blur and drop shadows. Shadows use visible ink: a transparent fill casts no shadow. Source blurred halos use a filled shape with blur.

The painter rejects distant article content before shaping its visible glyphs. The stage caches unchanged scenes, but bypasses that cache in the same frame when the displayed scene changes. This keeps unrelated search edits from repainting a dense diagram without delaying drag feedback.

## Controls and input

Inline controls use source scene coordinates or the final transform of their parent object. Their identity persists while a parent floats or changes position. Styling includes foreground, background, border, font family, font size and placeholders. Inline text fields and sliders do not receive an extra caption.

Buttons preserve enabled and selected states. Explicit `ControlPlacement.select()` placements use compact dropdowns for source HTML selects; open menus support Up, Down, Enter and Escape. Other choices can use grouped buttons. Read-only code panes have independent horizontal and vertical scrolling.

Mouse movement and modifier changes record input; rendering advances the drag runtime on the animation clock. Pointer capture handles movement and release outside an object, and loss of the held button cancels the gesture. Source cursor overrides, hover tooltips, transparent hit targets and click-handler annotations pass through the shared API. The runtime uses click-handler annotations for the source click/drag threshold.

Text inputs support native typing, selection, clipboard and input-method handling. Gallery shortcuts do not reset a demo while its text field is being edited. Scrolling scenes support wheel input and Home, End, Page Up and Page Down outside text editing or dragging.

## Gallery presentation

The sidebar and demo headings use source `demo.id` values, matching upstream `src/demo/ui.tsx`. The native wrapper supplies search, window controls, reset, motion preferences and import/export where supported.

The extracted `DemoNotes` retain source wording, locations, paragraph order and links. Explicit `native_markdown` overrides explain platform differences while keeping the original wording in the data. The gallery displays introductory notes with wrapping, emphasis, code styling and active links; specimen notes remain beside their figures. A demo with no source introduction receives no invented description. Long introductions have a bounded scroll region with a visible scrollbar.

Long articles use scrolling scenes to retain readable source dimensions. The layout editor's preview identifies itself as a native implementation of the demo's flex/grid subset. Its generated CSS and HTML panes do not imply an embedded browser engine.

## Verification

Focused tests cover literal XML escaping, text anchors and baselines, rotated glyph bounds, measured text advances, nonuniform stroke/cap geometry, and Unicode Markdown link ranges. Native Linux checks at a 1280×900 viewport supplement these tests:

- A component counter and editable note retained their values and mount time through a floating drag, reorder and drop. Its native input and button followed the displayed card.
- Selecting Commutativity in the proof dropdown changed both the selected option and the displayed sequent; all options were visible without clipping.
- Linear dragging committed at the expected position, reset correctly, and returned to its initial state after Escape cancellation. One uninterrupted held drag traversed all three chained-track endpoints, with stationary pauses between moves.
- The final layout editor showed its native-preview wording and both code panes. Tableaux showed the full article, source emphasis, numbered insertion instructions, captions and later RSK/jeu de taquin figures. Stock-and-flow showed its source diagram, equations, parameter controls and plots.
- Representative captures also checked rotated Spinny labels, serif Bluefish labels, inline text fields, the controlled-demo columns, tree and algebra figures, Hanoi panels, and topology diagrams. Native window movement, maximize/restore and close were exercised.

The locked native build passed. The final editor/drag captures produced no application errors or panics. These checks cover selected states and gestures; they do not establish pixel equivalence across the gallery.

Startup remains a performance limitation for text-heavy pages. In the final software-rendered Linux run, the first nonblank Tableaux capture took about 6.4 seconds after window sizing; the sampled layout and stock pages took about 0.3–0.5 seconds. Subsequent article scrolling produced visible updates in about 0.45 seconds in that harness. These measurements describe that test environment, not a hardware-independent performance guarantee.

## Native rendering limits

- System fonts and fallback choices can differ from a browser's fonts.
- Bitmap-only color emoji use native glyph sprites to retain color. Their native line boxes are positioned using the source text anchor and baseline, so centered symbols stay inside their hit targets. Those fallback strings remain upright; ordinary glyph outlines receive the full affine transform.
- Native widgets use transformed axis-aligned bounds. Their text remains upright under a rotated or sheared parent, and their form appearance differs from HTML. GPUI's closed-hand cursor substitutes for CSS `move`.
- Mathematical notation in introductory notes is preserved as text, without a TeX renderer. Scene mathematics uses positioned text and vector objects.
- Group opacity is flattened per drawable, so overlapping translucent shapes can differ from browser group compositing. Blur and arbitrary clip effects use raster masks and can differ under extreme scaling.
- Code panes are read-only native controls. The layout preview implements the documented subset rather than arbitrary browser CSS.

See [the interpolation report](interpolation.md) for path, color, transform and emergence boundaries, and [the overall porting report](../../PORTING.md) for gallery-wide limits.
