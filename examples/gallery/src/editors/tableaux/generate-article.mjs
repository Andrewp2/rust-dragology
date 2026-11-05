// node generate-article.mjs /path/to/pinned/dragology
// Extract static JSX prose and its inline styles without evaluating the demo.
import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const upstream = process.argv[2];
if (!upstream) throw new Error('Expected the pinned upstream checkout path');
const pinned = 'b145bdd56530f7cfa9a61bf6a7170459d2f140e1';
const actual = execFileSync('git', ['-C', upstream, 'rev-parse', 'HEAD'], { encoding: 'utf8' }).trim();
if (actual !== pinned) throw new Error(`Article source must be ${pinned}; found ${actual}`);
const relative = 'src/demos/tableaux.tsx';
execFileSync('git', ['-C', upstream, 'diff', '--exit-code', 'HEAD', '--', relative], { stdio: 'pipe' });
const require = createRequire(path.join(upstream, 'package.json'));
const ts = require('typescript');
const source = ts.createSourceFile(relative, fs.readFileSync(path.join(upstream, relative), 'utf8'), ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);
const base = { weight: 400, italic: false, mono: false, color: '1f2937' };
const name = n => n.openingElement.tagName.getText(source);
function jsxText(text) {
  // React's JSX whitespace transform: trim indentation, preserve intentional
  // same-line spaces and insert one space between nonempty source lines.
  const lines = text.replace(/\r/g, '').split('\n');
  return lines.map((line, i) => {
    line = line.replace(/\t/g, ' ');
    if (i > 0) line = line.replace(/^ +/, '');
    if (i < lines.length - 1) line = line.replace(/ +$/, '');
    return line;
  }).filter(Boolean).join(' ');
}
function runs(node, style) {
  if (ts.isJsxText(node)) return [{ ...style, text: jsxText(node.text) }];
  if (ts.isJsxExpression(node)) {
    if (!node.expression) return [];
    if (ts.isStringLiteral(node.expression)) return [{ ...style, text: node.expression.text }];
    return runs(node.expression, style);
  }
  if (ts.isJsxFragment(node)) return node.children.flatMap(n => runs(n, style));
  if (!ts.isJsxElement(node)) throw new Error(`Unexpected prose node ${ts.SyntaxKind[node.kind]}`);
  const tag = name(node);
  const next = { ...style };
  if (tag === 'b') next.weight = 700;
  else if (tag === 'i') next.italic = true;
  else if (tag === 'code') next.mono = true;
  else if (tag === 'Try') Object.assign(next, { weight: 600, italic: false, color: '92400e' });
  else if (!['P', 'H2', 'li'].includes(tag)) throw new Error(`Unexpected inline element ${tag}`);
  return [...(tag === 'Try' ? [{ ...next, text: 'Try it: ' }] : []), ...node.children.flatMap(n => runs(n, next))];
}
function normalized(rs) {
  const out = [];
  for (const run of rs) {
    run.text = run.text.replace(/\s+/g, ' ');
    if (!run.text) continue;
    const last = out.at(-1);
    if (last && ['weight', 'italic', 'mono', 'color'].every(k => last[k] === run[k])) last.text += run.text;
    else out.push(run);
  }
  if (out.length) { out[0].text = out[0].text.trimStart(); out.at(-1).text = out.at(-1).text.trimEnd(); }
  return out.filter(r => r.text.length);
}
let article;
function visit(n) {
  if (ts.isJsxElement(n) && name(n) === 'div' && n.openingElement.attributes.properties.some(a => a.name?.getText(source) === 'className' && a.initializer?.text === 'flex flex-col')) article = n;
  ts.forEachChild(n, visit);
}
visit(source);
if (!article) throw new Error('Could not find the source article');
const blocks = [];
let figure = 0;
function block(kind, rs, extra = {}) {
  const spans = normalized(rs);
  blocks.push({ kind, text: spans.map(r => r.text).join(''), runs: spans, ...extra });
}
for (const node of article.children) {
  if (!ts.isJsxElement(node)) continue;
  const tag = name(node);
  if (tag === 'P' || tag === 'H2') {
    const style = { ...base };
    if (node.openingElement.attributes.properties.some(a => a.initializer?.text === 'text-gray-500')) style.color = '6b7280';
    block(tag === 'P' ? 'paragraph' : 'heading', runs(node, style));
  } else if (tag === 'Figure') {
    blocks.push({ kind: 'figure', text: String(figure++) });
    const caption = node.openingElement.attributes.properties.find(a => a.name?.getText(source) === 'caption');
    if (caption) block('caption', runs(caption.initializer, { ...base, italic: true, color: '4b5563' }));
  } else if (tag === 'ol') {
    let number = 0;
    for (const item of node.children.filter(ts.isJsxElement)) block('list-item', runs(item, base), { number: ++number });
  } else throw new Error(`Unexpected article element ${tag}`);
}
if (figure !== 5) throw new Error(`Expected all five figures, got ${figure}`);
fs.writeFileSync(path.join(path.dirname(fileURLToPath(import.meta.url)), '../tableaux-article.json'), JSON.stringify(blocks, null, 2) + '\n');
console.log(`Extracted ${blocks.length} blocks and ${figure} figures from ${pinned}`);
