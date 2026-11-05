#!/usr/bin/env node
// Execute the actual one-based Nlp.trstlp implementation at the pinned checkout.
// Node is pinned too: V8's Math.hypot rounding affects degenerate active sets.
const fs = require("node:fs");
const { upstream } = require("./reference-upstream.cjs");
const base = upstream();
if (process.version !== "v22.14.0") {
  throw new Error(
    `Use Node22.14.0 for these numeric references; found ${process.version}`,
  );
}
const source = fs.readFileSync(base + "/src/math/cobyla.js", "utf8");
if (!source.includes("export function FindMinimum")) {
  throw new Error("Missing the pinned COBYLA export");
}
const exports_ = {};
new Function(
  "exports",
  source.replace("export function FindMinimum", "function FindMinimum") +
    "\nexports.trstlp=Nlp.trstlp.bind(Nlp);",
)(exports_);

let seed = 918237;
const rand = () => {
  seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0;
  return seed / 4294967296;
};
const scenarios = [];
const add = (name, n, m, a, b, rho) => {
  const dx = Array(n + 1).fill(0);
  const full = exports_.trstlp(n, m, a, b, rho, dx);
  scenarios.push({ name, n, m, a, b, rho, full, dx });
};
add(
  "unconstrained",
  2,
  0,
  [
    [0, 0],
    [0, 3],
    [0, 4],
  ],
  [0, 0],
  1,
);
add(
  "zero_objective",
  2,
  0,
  [
    [0, 0],
    [0, 0],
    [0, 0],
  ],
  [0, 0],
  1,
);
add(
  "active_half_plane",
  2,
  1,
  [
    [0, 0, 0],
    [0, 1, -1],
    [0, 0, 2],
  ],
  [0, 0.3, 0],
  1,
);
add(
  "infeasible_opposites",
  2,
  2,
  [
    [0, 0, 0, 0],
    [0, 1, -1, 0],
    [0, 0, 0, 1],
  ],
  [0, 1, 1, 0],
  0.5,
);
add(
  "finite_constrained_optimum",
  1,
  1,
  [
    [0, 0, 0],
    [0, -1, 1],
  ],
  [0, -0.2, 0],
  1,
);
add(
  "box_optimum_inside_radius",
  2,
  2,
  [
    [0, 0, 0, 0],
    [0, -1, 0, 1],
    [0, 0, -1, 1],
  ],
  [0, -0.2, -0.3, 0],
  1,
);
for (let test = 0; test < 250; test++) {
  const n = 1 + (test % 5);
  const m = Math.floor(rand() * 10);
  const rho = 10 ** (-2 + rand() * 3);
  const a = Array.from({ length: n + 1 }, () => Array(m + 2).fill(0));
  const b = Array(m + 2).fill(0);
  for (let i = 1; i <= n; i++) {
    for (let k = 1; k <= m + 1; k++) {
      a[i][k] = Math.round((rand() - 0.5) * 16) / 4;
    }
  }
  for (let k = 1; k <= m; k++) b[k] = Math.round((rand() - 0.5) * 12) / 4;
  if (test % 7 === 0 && m >= 2) {
    for (let i = 1; i <= n; i++) a[i][2] = a[i][1] * 2;
    b[2] = b[1] * 2;
  }
  if (test % 11 === 0 && m >= 2) {
    for (let i = 1; i <= n; i++) a[i][2] = -a[i][1];
    b[2] = b[1];
  }
  add("generated_" + test, n, m, a, b, rho);
}
console.log(JSON.stringify(scenarios, null, 2));
