#!/usr/bin/env node
// Execute upstream's explicit `none` paint handling, including its alpha rules.
const fs = require("node:fs");
const { pathToFileURL } = require("node:url");
const { upstream, sourceSection } = require("./reference-upstream.cjs");
const base = upstream();
const esbuild = require(base + "/node_modules/esbuild");

(async () => {
  const { interpolateLab } = await import(
    pathToFileURL(base + "/node_modules/d3-interpolate/src/index.js").href
  );
  const { rgb } = await import(
    pathToFileURL(base + "/node_modules/d3-color/src/index.js").href
  );
  const source = fs.readFileSync(base + "/src/svgx/lerp.tsx", "utf8");
  const section = sourceSection(
    source,
    "function lerpValue(",
    "/** Returns a clone",
  );
  const code = esbuild.transformSync(
    section + "\nexports.lerpValue=lerpValue;",
    {
      loader: "ts",
      format: "cjs",
    },
  ).code;
  const exports_ = {};
  new Function(
    "rgb",
    "interpolateColor",
    "COLOR_PROPS",
    "DEFAULT_VALUE_FOR_KEY",
    "exports",
    code,
  )(rgb, interpolateLab, new Set(["fill"]), { opacity: 1 }, exports_);
  const fixtures = [
    "rgba(0,80,220,1)",
    "rgba(0,80,220,0.5)",
    "rgba(0,80,220,0)",
  ].map((color) => ({
    color,
    out: exports_.lerpValue("fill", "none", color, 0.25),
  }));
  console.log(JSON.stringify(fixtures, null, 2));
})().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
