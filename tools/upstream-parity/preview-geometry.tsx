// Execute pinned upstream rendering with nested120-degree rotations.
import { DragSpecBuilder } from '@upstream/DragSpec';
import { dragSpecToBehavior, getElementPosition } from '@upstream/DragBehavior';
import { Vec2 } from '@upstream/math/vec2';
const start={angle:0}; const d=new DragSpecBuilder(start);
const ctx={draggable:({state}:any)=><g><circle id="dot" transform={`rotate(${state.angle})`} r={5}/></g>,draggedPath:'dot/',draggedId:'dot',anchorPos:Vec2(10,0),startState:start,debug:{varyVisualizer:false}};
const specs={inner:d.between([{angle:0},{angle:120}]),outer:d.between([d.between([{angle:0},{angle:120}]),d.fixed({angle:-90})]),floating:d.between([{angle:0},{angle:120}]).withFloating()};
const traces=Object.entries(specs).map(([name,spec])=>{const behavior=dragSpecToBehavior(spec,ctx);return {name,frames:[[0,0],[2,0],[-2,0]].map(([x,y])=>{const r=behavior({pointer:Vec2(x,y)});const p=getElementPosition(ctx,r.preview());return {pointer:[x,y],drop:r.dropState,position:p&&[p.x,p.y],grab:r.grabPos&&[r.grabPos.x,r.grabPos.y],gap:r.gap}})}});
console.log(JSON.stringify({revision:"b145bdd56530f7cfa9a61bf6a7170459d2f140e1",cases:traces},null,2));
