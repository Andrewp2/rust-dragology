// Execute the original operators, not a second implementation of their rules.
import { DragSpecBuilder } from '@upstream/DragSpec';
import { dragSpecToBehavior, getElementPosition } from '@upstream/DragBehavior';
import { Vec2 } from '@upstream/math/vec2';
import { translate } from '@upstream/svgx/helpers';
import { easeCubicOut, easeElasticOut } from 'd3-ease';

const start = {x:0,y:0};
const sites = [start,{x:100,y:0},{x:0,y:100}];
const anchor = Vec2(2,3);
const ctx = {
  draggable:({state}:any)=><g><circle id="dot" transform={translate(state.x,state.y)} r={5}/></g>,
  draggedPath:'dot/',draggedId:'dot',anchorPos:anchor,startState:start,
  debug:{varyVisualizer:false},
};
const d = new DragSpecBuilder(start);
const pointers = [[2,3],[24,19],[85,5],[52,4],[48,4],[150,110],[3,5]];
const specs = {
  fixed:d.fixed(sites[1]),
  closest:d.closest(sites),
  sticky:d.closest(sites,{stickiness:15}),
  locked:d.closest([d.between([start,sites[1]]),d.between([start,sites[2]])],{lockPast:20}),
  between:d.between(sites),
  sharp:d.between(sites,{sharpness:3}),
  natural:d.between(sites,{interpolation:'natural-neighbor'}),
  far:d.between(sites).whenFar({x:-50,y:-50},{gapIn:10,gapOut:30}),
  snap:d.between(sites).withSnapRadius(20),
  snapChain:d.between(sites).withSnapRadius(20,{transition:true,chain:true}),
  floating:d.closest(sites).withFloating(),
  tether:d.closest(sites).withFloating({ghost:{opacity:.35},tether:(distance:number)=>Math.min(distance,20)}),
  during:d.closest(sites).during((s:any)=>({...s,y:s.y+10})),
  onDrop:d.closest(sites).onDrop((s:any)=>({...s,y:s.y+10})),
  gap:d.closest(sites).changeGap((gap:number)=>gap/2),
  frame:d.closest(sites).changeFrame(({pointer}:any)=>({pointer:pointer.add(Vec2(20,-5))})),
  chain:d.closest(sites).withChaining(),
  vary:d.varyFunc([0],([x]:number[])=>({x,y:0})),
  constrained:d.varyFunc([0],([x]:number[])=>({x,y:0}),{constraint:(s:any)=>[-s.x,s.x-80]}),
  coupled:d.varyFunc([10,20],([x,y]:number[])=>({x,y}),{constraint:(s:any)=>[-s.x,-s.y,s.x+s.y-80]}),
};
const cases = Object.entries(specs).map(([name,spec])=>{
  const behavior = dragSpecToBehavior(spec,ctx);
  return {name, frames:pointers.map(([x,y])=>{
    const result = behavior({pointer:Vec2(x,y)});
    const position = getElementPosition(ctx,result.preview());
    return {pointer:[x,y],drop:[result.dropState.x,result.dropState.y],
      preview:position ? [position.x,position.y]:null,
      gap:Number.isFinite(result.gap)?result.gap:null,
      path:result.activePath,chain:!!result.chainNow};
  })};
});
const easing = [0,.05,.1,.2,.35,.5,.75,.95,1].map(t=>({t,cubic:easeCubicOut(t),elastic:easeElasticOut(t)}));
process.stdout.write(JSON.stringify({revision:'b145bdd56530f7cfa9a61bf6a7170459d2f140e1',cases,easing},null,2)+'\n');
