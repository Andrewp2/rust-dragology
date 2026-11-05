#!/usr/bin/env node
const fs = require("node:fs");
const path = require("node:path");
const os = require("node:os");
const { execFileSync } = require("node:child_process");
const { upstream } = require("./reference-upstream.cjs");
const base = upstream();
const esbuild = require(path.join(base, "node_modules/esbuild"));
const temporary = fs.mkdtempSync(path.join(os.tmpdir(), "dragology-fifteen-"));
(async () => {
  try {
    const bundle = path.join(temporary, "reference.mjs");
    await esbuild.build({
      entryPoints: [path.join(__dirname, "reference-fifteen.tsx")],
      outfile: bundle,
      bundle: true,
      platform: "node",
      format: "esm",
      jsx: "automatic",
      alias: { "@upstream": path.join(base, "src") },
      nodePaths: [path.join(base, "node_modules")],
      plugins: [
        {
          name: "expose-original-demo-and-frame-lifecycle",
          setup(build) {
            build.onLoad({ filter: /\/fifteen\.tsx$/ }, (args) => {
              const source = fs.readFileSync(args.path, "utf8");
              const end = source.indexOf("export default demo(");
              if (end < 0)
                throw new Error("Missing pinned Fifteen demo export");
              return {
                contents:
                  source.slice(0, end) + "\nexport {initialState,draggable};",
                loader: "tsx",
              };
            });
            build.onLoad({ filter: /\/DraggableRenderer\.tsx$/ }, (args) => ({
              contents:
                fs.readFileSync(args.path, "utf8") +
                "\nexport {initDrag,advanceFrame,runSpring};",
              loader: "tsx",
            }));
          },
        },
      ],
    });
    const output = execFileSync(process.execPath, [bundle], {
      encoding: "utf8",
    });
    const fixture = JSON.parse(output);
    // Keep one frame per line so numeric traces remain reviewable and compact.
    const formatted =
      JSON.stringify({ ...fixture, frames: [] }).replace(
        '"frames":[]',
        '"frames":[\n' +
          fixture.frames.map((frame) => JSON.stringify(frame)).join(",\n") +
          "\n]",
      ) + "\n";
    fs.writeFileSync(
      path.join(
        __dirname,
        "../../examples/gallery/tests/fixtures/fifteen-frames.json",
      ),
      formatted,
    );
  } finally {
    fs.rmSync(temporary, { recursive: true, force: true });
  }
})().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
