#!/usr/bin/env node
// Reproduce the fixtures embedded in runtime/scene.rs from pinned upstream code.
const fs = require("fs");
const { upstream } = require("./reference-upstream.cjs");
const { pathToFileURL } = require("node:url");
const base = upstream();
const esbuild = require(base + "/node_modules/esbuild");
const { interpolatePath } = require(base + "/node_modules/d3-interpolate-path");
(async () => {
  const { interpolateLab } = await import(
    pathToFileURL(base + "/node_modules/d3-interpolate/src/index.js").href
  );
  const { rgb } = await import(
    pathToFileURL(base + "/node_modules/d3-color/src/index.js").href
  );
  const css = (c) => `rgba(${c[0]},${c[1]},${c[2]},${c[3] / 255})`;
  const colorCases = [
    [[255, 0, 0, 255], [0, 0, 255, 255], 0.5],
    [[0, 0, 0, 255], [255, 255, 255, 255], 0.5],
    [[0, 255, 0, 255], [0, 0, 255, 255], 0.25],
    [[250, 100, 15, 128], [20, 50, 240, 64], 0.6],
    [[255, 0, 0, 0], [0, 80, 220, 255], 0.25],
    [[220, 20, 100, 255], [0, 0, 0, 0], 0.75],
    [[3, 5, 8, 255], [25, 17, 4, 255], 0.5],
    [[255, 100, 200, 255], [10, 210, 50, 255], 1.25],
    [[0, 0, 0, 0], [0, 0, 0, 0], 0.5],
  ];
  const colors = colorCases.map(([a, b, t]) => {
    const c = rgb(interpolateLab(css(a), css(b))(t));
    return { a, b, t, out: [c.r, c.g, c.b, Math.round(c.opacity * 255)] };
  });
  const pathCases = [
    ["M0,0L12,0", "M0,10L4,10L8,14L12,10", 0.5],
    [
      "M0,0C0,12,12,12,12,0",
      "M0,6C0,8,2,8,4,6C6,8,8,8,10,6C10,8,12,8,12,6",
      0.25,
    ],
    ["M0,0Q6,12,12,0", "M0,6L6,8L12,6", 0.5],
    ["M0,0L12,12", "M0,2C2,4,8,10,12,14", 0],
    ["M0,0Q6,12,12,0", "M0,2C2,4,8,10,12,14", 0.5],
    ["", "M4,8L12,20L20,8Z", 0.5],
    ["M4,8L12,20L20,8Z", "", 0.75],
    ["M0,0L12,0Z", "M0,2L6,8L12,2Z", 0.5],
    ["M0,0L12,0Z", "M0,2L6,8L12,2", 0.5],
    ["M0,0L12,0L12,12", "M0,2L12,2", 1],
    ["M0,0L12,0M20,0L30,0", "M0,4L12,4M20,4L30,4", 0.25],
    ["M0,0L12,0", "M0,10L4,10L8,14L12,10", -0.25],
    ["M2,3", "M0,0Q6,12,12,0", 0.5],
    ["M0,0L6,8L12,0", "M0,0L3,2L6,4L9,2L12,0", 0.25],
    ["M0,0Q6,12,12,0", "M0,0Q2,2,4,0Q6,2,8,0Q10,2,12,0", 0.3],
    ["M0,0C0,12,12,12,12,0", "M0,2Q6,8,12,2", 0.4],
    ["M0,0L12,0", "M0,2L6,8L12,2", 0],
    ["M0,0L12,0M20,0L30,0", "M0,4L6,4L12,4M20,4L25,4L30,4", 0.5],
  ];
  const paths = pathCases.map(([a, b, t]) => ({
    a,
    b,
    t,
    out: interpolatePath(a, b)(t),
  }));
  // Evaluate the pinned transform source itself. Imports are removed only to avoid
  // its React error-rendering dependency; numeric helpers are identical to source.
  let ts = fs
    .readFileSync(base + "/src/svgx/transform.ts", "utf8")
    .replace(/^import .*;\n/gm, "");
  ts =
    'const lerp=(a:number,b:number,t:number)=>a+(b-a)*t; const assert=(x:unknown)=>{if(!x)throw Error("assert")}; class ErrorWithJSX extends Error{}; const createElement=()=>null;\n' +
    ts;
  const js = esbuild.transformSync(ts, { loader: "ts", format: "cjs" }).code;
  const module = { exports: {} };
  new Function("module", "exports", js)(module, module.exports);
  const transforms = [
    ["rotate(0)", "rotate(180)", 0.5],
    ["rotate(180)", "rotate(0)", 0.5],
    ["rotate(350)", "rotate(10)", 0.25],
    [
      "translate(20,30) rotate(170) scale(2,3)",
      "translate(60,10) rotate(-170) scale(4,1)",
      0.5,
    ],
    [
      "translate(2,3) rotate(0) translate(20,0)",
      "translate(2,3) rotate(90) translate(0,20)",
      0.5,
    ],
  ].map(([a, b, t]) => ({
    a,
    b,
    t,
    out: module.exports.lerpTransformString(a, b, t),
  }));
  console.log(JSON.stringify({ colors, paths, transforms }, null, 2));
})().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
