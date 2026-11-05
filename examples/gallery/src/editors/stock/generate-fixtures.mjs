// Run with Node 22+: node --experimental-strip-types generate-fixtures.mjs /path/to/pinned/dragology
// Source revision: b145bdd56530f7cfa9a61bf6a7170459d2f140e1.
import fs from 'node:fs';
import {execFileSync} from 'node:child_process';
import {stripTypeScriptTypes} from 'node:module';
import path from 'node:path';
import {pathToFileURL,fileURLToPath} from 'node:url';
const upstream=process.argv[2];
if(!upstream) throw new Error('Expected the pinned upstream checkout path');
const pinned='b145bdd56530f7cfa9a61bf6a7170459d2f140e1';
const actual=execFileSync('git',['-C',upstream,'rev-parse','HEAD'],{encoding:'utf8'}).trim();
if(actual!==pinned) throw new Error(`Fixture source must be ${pinned}; found ${actual}`);
execFileSync('git',['-C',upstream,'diff','--exit-code','HEAD','--',...["src/demos/stock-and-flow/model.ts", "src/demos/stock-and-flow/presets.ts"]],{stdio:'pipe'});

const {simulate,sweep,flowFormula}=await import(pathToFileURL(path.join(upstream,'src/demos/stock-and-flow/model.ts')));
const here=path.dirname(fileURLToPath(import.meta.url));
const modelUrl=pathToFileURL(path.join(upstream,'src/demos/stock-and-flow/model.ts')).href;
const source=fs.readFileSync(path.join(upstream,'src/demos/stock-and-flow/presets.ts'),'utf8').replace('import { End, fit, Link, Model, Shape } from "./model";',`import {fit} from ${JSON.stringify(modelUrl)};`);
const {presets:makers}=await import('data:text/javascript;base64,'+Buffer.from(stripTypeScriptTypes(source)).toString('base64'));
const presets=makers.map(({label,make})=>({label,model:make()}));
fs.writeFileSync(path.join(here,'presets.json'),JSON.stringify(presets,null,2)+'\n');
const fixtures=[];
for(let preset=0;preset<presets.length;preset++) {
  for(const changed of [false,true]) {
    const m=structuredClone(presets[preset].model);
    if(changed) for(const p of Object.values(m.params)) p.value=p.min+0.63*(p.max-p.min);
    const run=simulate(m);
    const sample=values=>Object.fromEntries(Object.entries(values).map(([id,vs])=>[id,[...new Set([0,1,20,100,200,400,vs.length-1])].filter(i=>i<vs.length).map(i=>[i,vs[i]])]));
    fixtures.push({preset,changed,model:m,diverged:run.diverged,values:sample(run.values),rates:sample(run.rates),formulas:Object.fromEntries(Object.keys(m.flows).map(id=>[id,flowFormula(m,id)])),bands:m.sweep?sweep(m,m.sweep).bands:null});
  }
}
fs.writeFileSync(path.join(here,'upstream-fixtures.json'),JSON.stringify(fixtures)+'\n');
