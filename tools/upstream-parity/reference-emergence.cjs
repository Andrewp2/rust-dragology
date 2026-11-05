#!/usr/bin/env node
// Execute the original direct/wrapped rectangle emergence implementation.
const fs = require("node:fs");
const { upstream, sourceSection } = require("./reference-upstream.cjs");
const base = upstream();
const esbuild = require(base + "/node_modules/esbuild");
const React = require(base + "/node_modules/react");
const source = fs.readFileSync(base + "/src/svgx/lerp.tsx", "utf8");
const section = sourceSection(
  source,
  "type EmergeBounds =",
  "/**\n * For each element",
);
const code = esbuild.transformSync(
  section + "\nexports.synthetic=createSyntheticBefore;",
  {
    loader: "tsx",
    format: "cjs",
  },
).code;
const exports_ = {};
new Function("React", "cloneElement", "exports", code)(
  React,
  React.cloneElement,
  exports_,
);

const node = (mode, wrapped) => {
  const rect = React.createElement("rect", {
    x: 2,
    y: 3,
    width: 12,
    height: 8,
  });
  return React.createElement(
    "g",
    {
      transform: "translate(100, 70)",
      opacity: 0.6,
      dragologyEmergeMode: mode,
    },
    wrapped ? React.createElement("g", {}, rect) : rect,
    React.createElement("text", { x: 4, y: 7 }, "copy"),
  );
};
const origin = React.createElement(
  "g",
  { transform: "translate(20, 50)" },
  React.createElement("rect", { x: 0, y: 0, width: 40, height: 20 }),
  React.createElement("text", { x: 0, y: 12 }, "origin"),
);
const fixtures = [undefined, "clone", "scale"].flatMap((mode) =>
  [false, true].map((wrapped) => {
    const out = exports_.synthetic(node(mode, wrapped), origin);
    return {
      mode: mode ?? "auto",
      wrapped,
      transform: out.props.transform,
      opacity: out.props.opacity,
      children: React.Children.toArray(out.props.children).map((child) => ({
        type: child.type,
        props: child.props,
      })),
    };
  }),
);
console.log(JSON.stringify(fixtures, null, 2));
