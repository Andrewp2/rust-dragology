import { createRequire } from 'node:module';
import { execFileSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

if (process.version !== 'v22.14.0') {
  throw new Error('Reference solver fixtures use Node v22.14.0 (V8 Math.hypot arithmetic)');
}
const revision='b145bdd56530f7cfa9a61bf6a7170459d2f140e1';
const upstream=resolve(process.argv[2]??'');
if(execFileSync('git',['rev-parse','HEAD'],{cwd:upstream,encoding:'utf8'}).trim()!==revision) {
  throw new Error(`Expected upstream revision ${revision}`);
}
const require=createRequire(join(upstream,'package.json'));
const esbuild=require('esbuild');
const here=dirname(fileURLToPath(import.meta.url));
const temporary=mkdtempSync(join(tmpdir(),'dragology-cobyla-'));
try {
  const bundle=join(temporary,'generate.mjs');
  await esbuild.build({
    entryPoints:[join(here,'cobyla-fixtures.ts')],outfile:bundle,
    bundle:true,platform:'node',format:'esm',
    alias:{'@upstream':join(upstream,'src')},nodePaths:[join(upstream,'node_modules')],
    plugins:[{name:'record-actual-solver-evaluations',setup(build){
      build.onLoad({filter:/\/math\/cobyla\.js$/},args=>({loader:'js',contents:
        readFileSync(args.path,'utf8').replace('export function FindMinimum','function OriginalFindMinimum')+`
export function FindMinimum(calc,n,m,x,rhobeg,rhoend,iprint,maxfun) {
  const trace={name:globalThis.cobylaTraceName,initial:x.slice(),constraints:m,rhobeg,rhoend,maxfun,samples:[]};
  const status=OriginalFindMinimum((n,m,p,c)=>{
    const f=calc(n,m,p,c); trace.samples.push({x:Array.from(p),f,con:Array.from(c)}); return f;
  },n,m,x,rhobeg,rhoend,iprint,maxfun);
  trace.status=status;trace.result=x.slice();globalThis.lastCobylaTrace=trace;return status;
}
` }));
    }}]
  });
  const fixtures=execFileSync(process.execPath,[bundle],{encoding:'utf8'});
  writeFileSync(join(here,'../../tests/fixtures/upstream-cobyla.json'),fixtures);
} finally {rmSync(temporary,{recursive:true,force:true});}
