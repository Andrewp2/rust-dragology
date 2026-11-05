// Run the actual demo and renderer lifecycle; only private exports are exposed
// by the generator. No drag, chaining or spring logic is reimplemented here.
import { draggable, initialState } from "@upstream/demos/fifteen";
import { initDrag, advanceFrame, runSpring } from "@upstream/DraggableRenderer";
import { renderDraggableInertUnlayered } from "@upstream/renderDraggable";
import { getOnDragCallbackOnElement } from "@upstream/draggable";
import { findElement } from "@upstream/svgx";
import { getPath } from "@upstream/svgx/path";
import { localToGlobal } from "@upstream/svgx/transform";
import { Vec2 } from "@upstream/math/vec2";

let now = 0;
Object.defineProperty(globalThis, "performance", { value: { now: () => now } });
const press = [25, 175] as const;
const draggedId = "tile- ";
const ctx: any = {
  draggable,
  draggedId,
  anchorPos: Vec2(25, 25),
  startState: initialState,
  debug: { varyVisualizer: false },
};
const content = renderDraggableInertUnlayered(
  draggable,
  initialState,
  draggedId,
  true,
);
const found = findElement(
  content,
  (element: any) => element.props.id === draggedId,
)!;
ctx.draggedPath = getPath(found.element);
const spec = getOnDragCallbackOnElement(found.element)!();
let status = initDrag(spec, ctx, initialState, { pointer: Vec2(press) }, null);

// Start the upward branch while still snapped at home, then leave that snap
// zone before the branch spring expires. Follow with a circular lap, reverse
// lap, backtracking near a snap boundary, and a stationary hold.
const pointers: number[][] = [
  [25, 175],
  [25, 173.5],
  [25, 164.5],
  [25, 155],
  [25, 145],
  [25, 135],
  [25, 125],
];
for (const direction of [1, -1]) {
  for (let i = 0; i <= 32; i++) {
    const angle = Math.PI + (direction * 2 * Math.PI * i) / 32;
    pointers.push([
      Math.round((75 + 50 * Math.cos(angle)) * 2) / 2,
      Math.round((125 + 50 * Math.sin(angle)) * 2) / 2,
    ]);
  }
}
for (const y of [136, 134, 136, 134, 115, 114, 116, 114, 116, 115])
  pointers.push([25, y]);
for (let i = 0; i < 14; i++) pointers.push([25, 115]);
const frames = pointers.map(([x, y]) => {
  now += 16;
  status = advanceFrame(status, Vec2(x, y), now) ?? status;
  const scene = runSpring(status.springOrigin, status.result.preview());
  const positions = Array.from({ length: 16 }, (_, id) => {
    const layer = scene.byId.get(`tile-${id === 0 ? " " : id}`)!;
    const p = localToGlobal(layer.element.props.transform, Vec2(0, 0));
    return [p.x, p.y];
  });
  const paint_order = [...scene.byId.keys()]
    .filter((id) => id.startsWith("tile-"))
    .map((id) => (id === "tile- " ? "tile-0" : id));
  return { pointer: [x, y], positions, paint_order };
});
const paint_order = frames[0].paint_order;
if (
  frames.some(
    (frame) =>
      JSON.stringify(frame.paint_order) !== JSON.stringify(paint_order),
  )
) {
  throw new Error(
    "Source Fifteen paint order changed during the reference trace",
  );
}
console.log(
  JSON.stringify({
    revision: "b145bdd56530f7cfa9a61bf6a7170459d2f140e1",
    step_ms: 16,
    press,
    paint_order,
    frames: frames.map(({ paint_order, ...frame }) => frame),
  }),
);
