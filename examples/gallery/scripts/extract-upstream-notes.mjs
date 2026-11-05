#!/usr/bin/env node
// Copyright (c) 2025 Dragology contributors; port tooling MIT.
// Extract source-authored notes without evaluating or importing a demo.
import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import assert from 'node:assert/strict';

const REVISION = 'b145bdd56530f7cfa9a61bf6a7170459d2f140e1';
const gallery = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const upstream = path.resolve(process.argv.slice(2).find(arg => !arg.startsWith('-')) ?? '/tmp/dragology-gallery-upstream');
const requireUpstream = createRequire(path.join(upstream, 'package.json'));
const ts = requireUpstream('typescript');
const entities = { amp: '&', apos: "'", quot: '"', lt: '<', gt: '>', nbsp: '\u00a0', thinsp: '\u2009', ldquo: '“', rdquo: '”', lsquo: '‘', rsquo: '’' };
function decode(text) {
  return text.replace(/&(#x[\da-f]+|#\d+|[a-z]+);/gi, (entity, key) => {
    if (key.startsWith('#')) return String.fromCodePoint(parseInt(key.slice(key[1] === 'x' ? 2 : 1), key[1] === 'x' ? 16 : 10));
    assert.ok(key in entities, `Unsupported HTML entity ${entity}`);
    return entities[key];
  });
}
function cleanJsxText(text) {
  const lines = text.split(/\r\n|\n|\r/);
  let last = 0;
  lines.forEach((line, i) => { if (/[^ \t]/.test(line)) last = i; });
  return lines.map((line, i) => {
    line = line.replace(/\t/g, ' ');
    if (i !== 0) line = line.replace(/^ +/, '');
    if (i !== lines.length - 1) line = line.replace(/ +$/, '');
    return line ? line + (i !== last ? ' ' : '') : '';
  }).join('');
}
function escape(text) { return text.replace(/[\\`*_[\]<>]/g, '\\$&'); }
function inlineCode(text) {
  const marks = '`'.repeat(Math.max(0, ...[...text.matchAll(/`+/g)].map(m => m[0].length)) + 1);
  const pad = text.startsWith('`') || text.endsWith('`') ? ' ' : '';
  return marks + pad + text + pad + marks;
}
function parse(text, filename = 'fixture.tsx') {
  return ts.createSourceFile(filename, text, ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);
}
function attribute(element, name) {
  const opening = ts.isJsxElement(element) ? element.openingElement : element;
  const attr = opening.attributes.properties.find(p => ts.isJsxAttribute(p) && p.name.text === name);
  if (!attr?.initializer) return undefined;
  return ts.isJsxExpression(attr.initializer) ? attr.initializer.expression : attr.initializer;
}
function stringValue(node) {
  if (!node) return undefined;
  if (ts.isStringLiteral(node) || ts.isNoSubstitutionTemplateLiteral(node)) return node.text;
  if (ts.isParenthesizedExpression(node) || ts.isAsExpression(node)) return stringValue(node.expression);
  if (ts.isBinaryExpression(node) && node.operatorToken.kind === ts.SyntaxKind.PlusToken) {
    const a = stringValue(node.left), b = stringValue(node.right);
    if (a !== undefined && b !== undefined) return a + b;
  }
  return undefined;
}
function renderer(source, unresolved) {
  function render(node, raw = false) {
    if (ts.isJsxText(node)) {
      const text = decode(cleanJsxText(node.text));
      return raw ? text : escape(text);
    }
    if (ts.isJsxExpression(node)) {
      if (!node.expression) return '';
      return render(node.expression, raw);
    }
    const value = stringValue(node);
    if (value !== undefined) return raw ? value : escape(value);
    if (ts.isJsxFragment(node)) return node.children.map(c => render(c, raw)).join('');
    if (ts.isJsxElement(node) || ts.isJsxSelfClosingElement(node)) {
      const tag = (ts.isJsxElement(node) ? node.openingElement : node).tagName.getText(source);
      const children = ts.isJsxElement(node) ? node.children : [];
      const cls = stringValue(attribute(node, 'className')) ?? '';
      const isCode = tag === 'code' || tag === 'kbd' || cls.split(/\s/).includes('font-mono');
      const text = children.map(c => render(c, raw || isCode)).join('');
      if (raw) return text;
      if (isCode) return inlineCode(text);
      if (tag === 'DemoLink' || tag === 'a') {
        const href = stringValue(attribute(node, 'href'));
        if (href === undefined) { unresolved.push(`href: ${attribute(node, 'href')?.getText(source)}`); return text; }
        return `[${text}](<${decode(href)}>)`;
      }
      if (tag === 'b' || tag === 'strong') return `**${text}**`;
      if (tag === 'i' || tag === 'em') return `*${text}*`;
      if (tag === 'sub') {
        const subs = { i: 'ᵢ', j: 'ⱼ' };
        if (text in subs) return subs[text];
        unresolved.push(`subscript: ${text}`); return `<sub>${text}</sub>`;
      }
      if (tag === 'p') return `\n\n${cls.split(/\s/).includes('font-bold') ? `**${text}**` : text}\n\n`;
      if (tag === 'br') return '\n';
      if (tag === 'ol' || tag === 'ul') return '\n\n' + children.filter(ts.isJsxElement).map((child, i) => `${tag === 'ol' ? `${i + 1}.` : '-'} ${render(child).trim()}`).join('\n') + '\n\n';
      if (!['DemoNotes', 'span', 'li'].includes(tag)) unresolved.push(`element: ${tag}`);
      return text;
    }
    unresolved.push(node.getText(source));
    return '';
  }
  return node => render(node).replace(/[ \t]+\n/g, '\n').replace(/\n{3,}/g, '\n\n').trim();
}
function nearestFunctionName(node) {
  for (let p = node.parent; p; p = p.parent) {
    if (ts.isArrowFunction(p) || ts.isFunctionExpression(p) || ts.isFunctionDeclaration(p)) {
      if (p.name) return p.name.text;
      if (ts.isVariableDeclaration(p.parent)) return p.parent.name.getText();
      return null;
    }
  }
  return null;
}
function walk(node, visit) { visit(node); ts.forEachChild(node, child => walk(child, visit)); }
function fileId(filename) { return filename.replace(/\.tsx$/, '').replace(/\/index$/, '').replaceAll('/', '-'); }
function sourceLocation(source, node, filename) { return `src/demos/${filename}:${source.getLineAndCharacterOfPosition(node.getStart(source)).line + 1}`; }
function extract(filename, content) {
  const source = parse(content, filename);
  const notes = [];
  const pathName = `src/demos/${filename}`;
  function add(node, context, title, selector) {
    const unresolved = [];
    const markdown = renderer(source, unresolved)(node);
    notes.push({ markdown, source: sourceLocation(source, node, filename), context, ...(title ? { title } : {}), ...(selector ? { selector } : {}), unresolved });
  }
  walk(source, node => {
    if (!ts.isJsxElement(node) || node.openingElement.tagName.getText(source) !== 'DemoNotes') return;
    const name = nearestFunctionName(node);
    // The helper receives four source-authored JSX arguments below. Expand
    // those arguments instead of emitting an unresolved {notes} placeholder.
    if (filename === 'rotations-rebalancing/index.tsx' && name === 'Section') return;
    const expression = node.children.find(child => ts.isJsxExpression(child) && child.expression)?.expression;
    if (expression?.getText(source) === 'puzzle.blurb') {
      let puzzles;
      walk(source, child => { if (ts.isVariableDeclaration(child) && child.name.getText(source) === 'PUZZLES') puzzles = child.initializer; });
      assert.ok(puzzles && ts.isObjectLiteralExpression(puzzles), `${filename}: PUZZLES shape changed`);
      for (const puzzle of puzzles.properties) {
        assert.ok(ts.isPropertyAssignment(puzzle) && ts.isObjectLiteralExpression(puzzle.initializer));
        const properties = puzzle.initializer.properties;
        const blurb = properties.find(p => ts.isPropertyAssignment(p) && p.name.getText(source) === 'blurb');
        const title = properties.find(p => ts.isPropertyAssignment(p) && p.name.getText(source) === 'title');
        assert.ok(blurb && stringValue(blurb.initializer) !== undefined, `${filename}: dynamic puzzle blurb`);
        add(blurb.initializer, 'specimen', stringValue(title?.initializer), `puzzleId=${puzzle.name.getText(source).replace(/^['"]|['"]$/g, '')}`);
      }
    } else {
      const specimen = filename === 'controlled.tsx' && name !== null;
      const titles = { SimpleDemo: 'simple', DoubleSwitchDemo: 'double switch', RejectionDemo: 'rejection', OverrideDemo: 'override', DoubleSliderDemo: 'double slider', ExternalControlDemo: 'external control' };
      add(node, specimen ? 'specimen' : 'intro', specimen ? titles[name] ?? name : undefined);
    }
  });
  if (filename === 'rotations-rebalancing/index.tsx') {
    walk(source, node => {
      if (ts.isJsxElement(node) && node.openingElement.tagName.getText(source) === 'Section') {
        const note = attribute(node, 'notes');
        assert.ok(note, 'Section notes missing');
        add(note, 'specimen', stringValue(attribute(node, 'title')));
      }
    });
  }
  // URLs are author attribution and useful references, so ensure conversion
  // cannot quietly drop even one source link while retaining nearby text.
  walk(source, node => {
    if (!ts.isJsxElement(node) || !['DemoLink', 'a'].includes(node.openingElement.tagName.getText(source))) return;
    let inNotes = false;
    for (let p = node.parent; p; p = p.parent) {
      if (ts.isJsxElement(p) && p.openingElement.tagName.getText(source) === 'DemoNotes') inNotes = true;
    }
    if (!inNotes) return;
    const href = stringValue(attribute(node, 'href'));
    assert.ok(href !== undefined && notes.some(n => n.markdown.includes(`](<${decode(href)}>)`)), `${filename}: dropped note link ${href}`);
  });
  return { source: pathName, revision: REVISION, notes };
}
function selfTest() {
  const a = extract('example.tsx', `export default demo(() => <DemoNotes><p>After <DemoLink href="https://example.org/a?x=1&amp;y=2">Author</DemoLink>, s<sub>i</sub> and <b>*</b>.</p><p>Use <code>{"<Widget>"}</code>{" "}here &ldquo;now&rdquo;.</p></DemoNotes>);`);
  assert.equal(a.notes[0].markdown, 'After [Author](<https://example.org/a?x=1&y=2>), sᵢ and **\\***.\n\nUse `<Widget>` here “now”.');
  assert.deepEqual(a.notes[0].unresolved, []);
  const b = extract('example.tsx', `const PUZZLES={first:{title:"One",blurb:"A "+"B"},second:{title:"Two",blurb:"C"}}; export default demo(()=><DemoNotes>{puzzle.blurb}</DemoNotes>);`);
  assert.deepEqual(b.notes.map(n => [n.markdown, n.context, n.selector]), [['A B','specimen','puzzleId=first'],['C','specimen','puzzleId=second']]);
  const c = extract('example.tsx', `export default demo(()=><DemoNotes>{unknown.instructions}</DemoNotes>);`);
  assert.deepEqual(c.notes[0].unresolved, ['unknown.instructions']);
}
selfTest();
const revision = execFileSync('git', ['-C', upstream, 'rev-parse', 'HEAD'], { encoding: 'utf8' }).trim();
assert.equal(revision, REVISION, 'Refusing to silently extract from a different revision');
const ids = fs.readFileSync(path.join(gallery, 'data/upstream-demo-ids.txt'), 'utf8').split(/\r?\n/).map(s => s.trim()).filter(s => s && !s.startsWith('#'));
const files = new Map();
function list(dir, prefix = '') {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const filename = prefix + entry.name;
    if (entry.isDirectory()) list(path.join(dir, entry.name), filename + '/');
    else if (filename.endsWith('.tsx')) files.set(fileId(filename), filename);
  }
}
list(path.join(upstream, 'src/demos'));
const data = {};
const adaptations = JSON.parse(fs.readFileSync(path.join(gallery, 'data/native-note-adaptations.json'), 'utf8'));
for (const id of ids) {
  const filename = files.get(id);
  assert.ok(filename, `No source file for listed demo ${id}`);
  data[id] = extract(filename, fs.readFileSync(path.join(upstream, 'src/demos', filename), 'utf8'));
  const nativeIntros = adaptations[id]?.intros ?? (adaptations[id]?.intro ? [adaptations[id].intro] : null);
  if (nativeIntros) {
    const intro = data[id].notes.filter(note => note.context === 'intro');
    assert.equal(intro.length, nativeIntros.length, `${id}: native adaptation count differs from source introductions`);
    intro.forEach((note, i) => { note.native_markdown = nativeIntros[i]; });
  }
}
const unresolved = Object.entries(data).flatMap(([id, entry]) => entry.notes.flatMap(note => note.unresolved.map(expression => ({ id, source: note.source, expression }))));
const serialized = JSON.stringify(data, null, 2) + '\n';
const output = path.join(gallery, 'data/upstream-notes.json');
if (process.argv.includes('--check')) assert.equal(fs.readFileSync(output, 'utf8'), serialized, 'Generated source notes are stale');
else fs.writeFileSync(output, serialized);
console.log(JSON.stringify({ demos: ids.length, notes: Object.values(data).reduce((sum, e) => sum + e.notes.length, 0), demosWithNotes: Object.values(data).filter(e => e.notes.length).length, unresolved, output, checked: process.argv.includes('--check') }, null, 2));
assert.equal(unresolved.length, 0, 'Source notes contain unresolved expressions; inspect them before shipping');
