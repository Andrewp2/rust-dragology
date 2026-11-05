// Shared checkout validation for optional JavaScript reference generators.
const fs = require("node:fs");
const path = require("node:path");
const { execFileSync } = require("node:child_process");

const PINNED_REVISION = "b145bdd56530f7cfa9a61bf6a7170459d2f140e1";

function upstream() {
  if (process.argv.length !== 3) {
    throw new Error(
      `Usage: node ${path.basename(process.argv[1])} /path/to/upstream/dragology`,
    );
  }
  const base = fs.realpathSync(path.resolve(process.argv[2]));
  const revision = execFileSync("git", ["-C", base, "rev-parse", "HEAD"], {
    encoding: "utf8",
  }).trim();
  if (revision !== PINNED_REVISION) {
    throw new Error(`Expected upstream ${PINNED_REVISION}, found ${revision}`);
  }
  // HEAD alone is insufficient if the source was subsequently edited locally.
  execFileSync(
    "git",
    [
      "-C",
      base,
      "diff",
      "--exit-code",
      "HEAD",
      "--",
      "src",
      "package.json",
      "pnpm-lock.yaml",
    ],
    {
      stdio: "pipe",
    },
  );
  if (!fs.existsSync(path.join(base, "node_modules", "esbuild"))) {
    throw new Error(
      "Install the pinned checkout's dependencies: pnpm install --frozen-lockfile --ignore-scripts",
    );
  }
  return base;
}

// Source sections are read from the verified checkout, never copied into the
// generator. Fail loudly when a marker cannot be found.
function sourceSection(source, startMarker, endMarker) {
  const start = source.indexOf(startMarker);
  const end = source.indexOf(endMarker, start + startMarker.length);
  if (start < 0 || end < 0) {
    throw new Error(
      `Missing pinned source markers: ${startMarker} / ${endMarker}`,
    );
  }
  return source.slice(start, end);
}

module.exports = { upstream, sourceSection, PINNED_REVISION };
