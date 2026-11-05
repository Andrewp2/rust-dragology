# Source demo notes

`upstream-notes.json` preserves the `DemoNotes` content from all 119 listed Dragology demos at revision `b145bdd56530f7cfa9a61bf6a7170459d2f140e1`. The original text is licensed under the repository’s MIT license and attributed to the Dragology contributors. Entries follow the canonical order in `upstream-demo-ids.txt`.

Regenerate from a checkout of that revision with its TypeScript development dependency installed:

```sh
node examples/gallery/scripts/extract-upstream-notes.mjs /path/to/dragology
node examples/gallery/scripts/extract-upstream-notes.mjs /path/to/dragology --check
```

The script parses TSX with the TypeScript compiler API without executing demo code. It verifies the checkout revision, converts JSX text whitespace and HTML entities, preserves paragraphs, lists, emphasis, code and links, and converts the source’s `i` and `j` subscripts to Unicode. It also runs small extraction regressions and verifies that no source note hyperlink disappeared. Unknown expressions are retained in `unresolved` and cause the command to fail.

Every demo maps to `source`, `revision` and `notes`. A note has `markdown`, `source` (file and line), `context` and `unresolved`. The `intro` context is ordinary page explanation. The `specimen` context marks controlled-demo subsection notes, the four rotations/rebalancing sections, and dynamic puzzle descriptions. Specimens also have a `title`; puzzle descriptions have a `selector`, such as `puzzleId=unit`. The shell should show the active specimen at the appropriate place rather than combining every puzzle instruction into one introduction.

The current extraction contains 98 notes across 71 demos, including all 14 source links. The other 48 demos have no `DemoNotes`; the extractor does not invent instructions for them. The three dynamic `puzzle.blurb` references expand to all 15 statically authored puzzle descriptions. The rotations/rebalancing helper expands to its four JSX `notes` arguments. There are no unresolved expressions at the pinned revision.

This is a transcription of author notes, not a certification of native behavior. It retains source React/API names and references to the source debug tools. `native-note-adaptations.json` provides explicitly authored native introductions where React-specific API claims or currently absent debug views would otherwise mislead. The extractor adds these as `native_markdown` without modifying `markdown`; the shell prefers the native field when present. These adaptations do not assert full behavioral parity. Notes outside `DemoNotes` or the explicit helper arguments above, HTML page layout, widget labels, comments and other documentation are not included. Subscripts and textual mathematics are preserved in readable form; this file does not provide a mathematical typesetter.
