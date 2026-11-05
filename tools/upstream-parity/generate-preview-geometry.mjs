import { createRequire } from 'node:module';
import { execFileSync } from 'node:child_process';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const revision = 'b145bdd56530f7cfa9a61bf6a7170459d2f140e1';
const upstream = resolve(process.argv[2] ?? '');
if (execFileSync('git', ['rev-parse', 'HEAD'], {cwd: upstream, encoding:'utf8'}).trim() !== revision) {
  throw new Error(`Expected upstream revision ${revision}`);
}
const require = createRequire(join(upstream, 'package.json'));
const esbuild = require('esbuild');
const here = dirname(fileURLToPath(import.meta.url));
const temporary = mkdtempSync(join(tmpdir(), 'dragology-parity-'));
try {
  const bundle = join(temporary, 'generate.mjs');
  await esbuild.build({
    entryPoints:[join(here,'preview-geometry.tsx')], outfile:bundle,
    bundle:true, platform:'node', format:'esm', jsx:'automatic',
    alias:{'@upstream':join(upstream,'src')}, nodePaths:[join(upstream,'node_modules')],
  });
  const fixtures = execFileSync(process.execPath,[bundle],{encoding:'utf8'});
  writeFileSync(join(here,'../../tests/fixtures/upstream-preview-geometry.json'),fixtures);
} finally {
  rmSync(temporary,{recursive:true,force:true});
}
