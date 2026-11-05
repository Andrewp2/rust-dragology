// Run with Node 22+: node --experimental-strip-types generate-fixtures.mjs /path/to/pinned/dragology
// Source revision: b145bdd56530f7cfa9a61bf6a7170459d2f140e1; Dagre 0.8.5.
import fs from 'node:fs';
import {execFileSync} from 'node:child_process';
import path from 'node:path';
import {pathToFileURL,fileURLToPath} from 'node:url';
const upstream=process.argv[2];
if(!upstream) throw new Error('Expected the pinned upstream checkout path');
const pinned='b145bdd56530f7cfa9a61bf6a7170459d2f140e1';
const actual=execFileSync('git',['-C',upstream,'rev-parse','HEAD'],{encoding:'utf8'}).trim();
if(actual!==pinned) throw new Error(`Fixture source must be ${pinned}; found ${actual}`);
execFileSync('git',['-C',upstream,'diff','--exit-code','HEAD','--',...["src/demos/regex-nfa/regex.ts"]],{stdio:'pipe'});

const dagrePackage=JSON.parse(fs.readFileSync(path.join(upstream,'node_modules/dagre/package.json'),'utf8'));
if(dagrePackage.version!=='0.8.5') throw new Error('Fixture source requires dagre 0.8.5');
const {cat,star,alt,chr,allNodes,removeNode,placementTargets,buildNfa,buildDfa}=await import(pathToFileURL(path.join(upstream,'src/demos/regex-nfa/regex.ts')));
const initial=cat('n0',star('n1',alt('n2',chr('n3','a'),chr('n4','b'))),chr('n5','a'),chr('n6','b'));
const examples=[initial,null,chr('n0','a'),alt('n0',chr('n1','a'),chr('n2','b')),star('n0',chr('n1','a'))];
for(const node of allNodes(initial)) {
 const removed=removeNode(initial,node.id);examples.push(removed);
 examples.push(...placementTargets(removed,node,{concat:'n20',alt:'n21'}).slice(0,8));
}
const here=path.dirname(fileURLToPath(import.meta.url));
fs.writeFileSync(path.join(here,'upstream-fixtures.json'),JSON.stringify(examples.map(root=>({root,nfa:buildNfa(root),dfa:buildDfa(buildNfa(root),['a','b'])})))+'\n');
