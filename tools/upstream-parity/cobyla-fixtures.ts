// Execute the pinned upstream solver and DistanceMinimizer; no solver is copied here.
import { FindMinimum } from '@upstream/math/cobyla';
import { DistanceMinimizer } from '@upstream/math/optimization';
import { Vec2 } from '@upstream/math/vec2';

const capture = globalThis as any;
const output: any[] = [];
function record(name: string, run: () => void) {
  capture.cobylaTraceName = name;
  run();
  output.push(capture.lastCobylaTrace);
}
function direct(name: string, initial: number[], m: number, radius: number, budget: number,
    calc: (x: number[], c: number[]) => number) {
  record(name, () => FindMinimum((_n, _m, x, c) => calc(x, c), initial.length, m,
    initial.slice(), radius, 1e-3, 0, budget));
}
direct('scalar', [100], 0, 140, 200, x => (x[0]-240)**2);
direct('two-dimensional', [10,20], 0, 140, 200, x => (x[0]-140)**2+(x[1]+35)**2);
direct('bounded-scalar', [100], 2, 200, 200, (x,c) => {
  c[0]=x[0]; c[1]=240-x[0]; return (x[0]-300)**2;
});
direct('line-equality', [20,10], 2, 100, 200, (x,c) => {
  c[0]=x[0]+x[1]-60; c[1]=-c[0]; return (x[0]-120)**2+(x[1]-85)**2;
});
direct('circle-equality', [30,40], 2, 80, 200, (x,c) => {
  c[0]=x[0]*x[0]+x[1]*x[1]-2500; c[1]=-c[0]; return (x[0]-110)**2+(x[1]-20)**2;
});
direct('impossible-inequalities', [1], 2, 20, 200, (x,c) => {
  c[0]=x[0]-5; c[1]=-x[0]-5; return (x[0]-3)**2;
});
direct('redundant-constraints', [1,2,3], 4, 10, 200, (x,c) => {
  c[0]=x[0]+x[1]-5; c[1]=-c[0]; c[2]=2*c[0]; c[3]=x[2];
  return (x[0]-8)**2+(x[1]-9)**2+(x[2]+2)**2;
});
for (const budget of [0,1,2,3,17]) {
  direct(`evaluation-budget-${budget}`, [10,20], 0, 140, budget,
    x => (x[0]-140)**2+(x[1]+35)**2);
}
const zero = new DistanceMinimizer([0,0], 0);
for (const [i,target] of [[200,150],[20,-40],[0,0]].entries()) {
  record(`distance-zero-seed-frame-${i}`, () => zero.minimize(Vec2(...target), p => Vec2(p[0],p[1])));
}
// Exact gears.tsx model: x/y/angle, radial equality, tooth-phase equality.
const teethA=22, teethB=14;
const dockDistance=(teethA*12)/(2*Math.PI)+(teethB*12)/(2*Math.PI);
const b=Vec2(140,180).add(Vec2.polarRad(dockDistance,0.4));
const alpha=Vec2(140,180).angleToDeg(b);
const bAngle=alpha+180-180/teethB-(0-alpha)*(teethA/teethB);
const gear=new DistanceMinimizer([140,180,0],4);
for (const [i,target] of [[160,130],[235,170],[220,260]].entries()) {
  record(`gear-docking-frame-${i}`, () => gear.minimize(Vec2(...target), p=>Vec2(p[0],p[1]), {
    constraints:p=>{
      const a=Vec2(p[0],p[1]); const alpha=a.angleToDeg(b);
      const radial=a.dist(b)-dockDistance;
      const phase=(p[2]-alpha)*teethA+(bAngle-(alpha+180))*teethB+180;
      return [radial,-radial,phase,-phase];
    }
  }));
}
console.log(JSON.stringify({revision:'b145bdd56530f7cfa9a61bf6a7170459d2f140e1',node:process.version,cases:output},null,2));
