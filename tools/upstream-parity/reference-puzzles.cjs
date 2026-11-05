#!/usr/bin/env node
// Execute the original seeded plane-partition and Aztec-diamond generators.
const fs = require("node:fs");
const { upstream } = require("./reference-upstream.cjs");
const base = upstream();
const esbuild = require(base + "/node_modules/esbuild");

// These named functions are extracted after TypeScript removal. The pinned
// functions contain no braces in string literals that disturb this scan.
function extract(source, name) {
  const start = source.indexOf("function " + name + "(");
  if (start < 0) throw new Error(`Missing pinned function ${name}`);
  let at = source.indexOf("(", start);
  let depth = 1;
  let i = at + 1;
  for (; depth && i < source.length; i++) {
    if (source[i] === "(") depth++;
    if (source[i] === ")") depth--;
  }
  at = source.indexOf("{", i);
  depth = 1;
  i = at + 1;
  for (; depth && i < source.length; i++) {
    if (source[i] === "{") depth++;
    if (source[i] === "}") depth--;
  }
  if (depth) throw new Error(`Unterminated pinned function ${name}`);
  return source.slice(start, i);
}

const cases = [
  {
    file: "plane-partition",
    names: [
      "heights",
      "removableCubeIds",
      "addablePositions",
      "mulberry32",
      "makeInitialState",
    ],
    call: `
      const state = makeInitialState({a:3,b:3,c:3,seed:123});
      module.exports = {file:"plane", cubes:Object.values(state.cubes).map(c=>[c.i,c.j,c.k]).sort()};
    `,
  },
  {
    file: "aztec-diamond",
    names: [
      "orientation",
      "inDiamond",
      "cells",
      "cellsOf",
      "coverage",
      "allHorizontal",
      "allVertical",
      "blocksOf",
      "blockCenter",
      "applyTurn",
      "settle",
      "mulberry32",
      "makeInitialState",
      "heightFunction",
      "screen",
      "dominoCenter",
    ],
    call: `
      const state = makeInitialState({n:2,seed:123});
      module.exports = {
        file:"aztec",
        dominoes:Object.values(state.dominoes).map(d=>[d.x,d.y,d.angle/90]),
        height:[...heightFunction(state)],
      };
    `,
  },
];
const fixtures = cases.map(({ file, names, call }) => {
  const source = esbuild.transformSync(
    fs.readFileSync(`${base}/src/demos/${file}.tsx`, "utf8"),
    {
      loader: "tsx",
    },
  ).code;
  // UI-independent source helpers retain their original math and immutable
  // update dependencies. IDs only label records; fixture comparisons use values.
  const contents =
    `
    import {produce} from "immer";
    import * as _ from "lodash-es";
    import {Vec2} from "./src/math/vec2";
    let counter=0;
    function makeId(){ return "id-" + counter++; }
    const CELL=32;
    const cellKey=(x,y)=>x+","+y;
  ` +
    names.map((name) => extract(source, name)).join("\n") +
    "\n" +
    call;
  const built = esbuild.buildSync({
    stdin: { contents, loader: "js", resolveDir: base },
    bundle: true,
    platform: "node",
    format: "cjs",
    write: false,
  });
  const module_ = { exports: {} };
  new Function("module", "exports", "require", built.outputFiles[0].text)(
    module_,
    module_.exports,
    require,
  );
  return module_.exports;
});
console.log(JSON.stringify(fixtures, null, 2));
