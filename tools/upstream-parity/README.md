# Upstream reference traces

These fixtures come from executing Dragology's original TypeScript operators at revision `b145bdd56530f7cfa9a61bf6a7170459d2f140e1`. The Rust tests replay the same pointer sequences and compare drop models, rendered grab positions, gaps, active branches, chaining signals, and easing values. Continuous operator traces allow a 0.01 canvas-unit tolerance for host geometry/transcendental differences; the solver itself now uses the direct upstream translation and has exact evaluation-trace checks below.

To regenerate, check out that revision in a separate directory and install its dependencies with `pnpm install --frozen-lockfile --ignore-scripts`. Then, from this repository:

```sh
node tools/upstream-parity/generate.mjs /path/to/upstream/dragology
cargo test -p dragology --test upstream_parity
```

Ordinary Rust test runs use the committed fixture and need neither JavaScript nor network access. These traces cover the named operator scenarios; they do not establish complete library or gallery equivalence.


## Gallery interpolation and puzzle references

The additional reference generators execute the original pinned JavaScript dependencies or extract the actual UI-independent TypeScript functions. Each requires an upstream checkout path, verifies its HEAD matches the revision above, and rejects local modifications under `src`, `package.json`, or `pnpm-lock.yaml`. Install upstream dependencies using the frozen-lockfile command above first. They print JSON to standard output and do not modify either checkout:

```sh
node tools/upstream-parity/reference-interpolation.cjs /path/to/upstream/dragology > /tmp/dragology-interpolation.json
node tools/upstream-parity/reference-emergence.cjs /path/to/upstream/dragology > /tmp/dragology-emergence.json
node tools/upstream-parity/reference-none.cjs /path/to/upstream/dragology > /tmp/dragology-none.json
node tools/upstream-parity/reference-puzzles.cjs /path/to/upstream/dragology > /tmp/dragology-puzzles.json
```

- `reference-interpolation.cjs` generates Lab colors from D3 3.x, paths from d3-interpolate-path 2.3.0, and transform-list interpolation from `src/svgx/transform.ts`. JSON `null` RGB channels for fully transparent D3 colors represent JavaScript NaN; the native 8-bit output uses zero.
- `reference-emergence.cjs` executes `createSyntheticBefore` and its helpers from `src/svgx/lerp.tsx` with React for Auto/Clone/Scale emergence with direct and wrapped rectangles.
- `reference-none.cjs` executes the source `lerpValue` branch for explicit `none` paint, including partial and zero alpha behavior.
- `reference-puzzles.cjs` executes the seeded plane-partition and Aztec-diamond initial-state generators with the original Immer, lodash, and Vec2 dependencies. It emits cube coordinates, domino order and signed quarter turns, and the Aztec height function for seed123.

The expected values are embedded in `examples/gallery/src/runtime/scene.rs` and `examples/gallery/src/puzzles/tilings.rs`. Compare regenerated JSON with those test fixtures before updating either the pinned revision or expected values; these scripts intentionally do not overwrite test code. Ordinary Rust tests remain offline and require neither this upstream checkout nor Node:

```sh
cargo test -p dragology-gallery --lib runtime::scene::
cargo test -p dragology-gallery --lib puzzles::tilings::tests::seeded_tilings_match_the_pinned_typescript_source
```


## COBYLA trust-region references

Use **Node22.14.0** for the direct trust-region reference generator. Besides checking the upstream revision, it checks the Node version because V8's `Math.hypot` normalization/rounding can change a rank-deficient active-set decision. The Rust translation retains the reference engine's normalized Kahan summation rather than using the platform `hypot` implementation.

```sh
node tools/upstream-parity/reference-trust-region.cjs /path/to/upstream/dragology > /tmp/dragology-trust-region.json
cargo test -p dragology trust_region_steps_match_original_javascript
```

The generator evaluates the actual `Nlp.trstlp` implementation in upstream `src/math/cobyla.js`, emitting256 deterministic cases: six named cases and250 seeded cases with dimensions1–5, constraints0–9, repeated/opposing gradients, infeasible constraints, finite constrained optima, and zero objectives. Eleven representative expected results are embedded in `src/cobyla_trstlp.rs`, including the degenerate case that distinguishes V8 and platform `hypot`. The usual Rust tests require neither Node nor the upstream checkout.


## Complete COBYLA solver references

Use Node 22.14.0 and the pinned checkout above:

```sh
node tools/upstream-parity/generate-cobyla.mjs /path/to/upstream/dragology
cargo test -p dragology cobyla::
```

The generator instruments the actual `FindMinimum` export without changing its
algorithm, then executes it directly and through the original `DistanceMinimizer`.
It writes `tests/fixtures/upstream-cobyla.json`: 18 solves and 580 objective
calls covering scalar and planar minimization, inequalities, linear and nonlinear
equalities, redundant constraints, an impossible constraint system, budgets
0/1/2/3/17, three warm-started frames from a zero seed, and three gear docking frames
with the source x/y/angle parameters and radial/tooth-phase equalities.

The Rust test replays the source objective/constraint values and compares **every
evaluated parameter, evaluation count, status, and final iterate exactly**. Replaying
those values isolates the solver from host `atan2`/`hypot` implementations; it does
not claim that every native geometry callback is bit-identical to JavaScript.
Separate live Rust callback tests check inequality/equality handling. Fixtures use
serde_json's `float_roundtrip` feature so decimal parsing cannot alter an active-set
rank decision. The private solver preserves the source's one-based work arrays,
ordered arithmetic, zero-budget first evaluation, and final-iterate selection.
Console logging and the public experimental rounding-check switch are omitted;
the internal rounding check and all three exit statuses are retained.


## Fifteen continuous-motion reference

The Fifteen trace runs the original demo together with `DraggableRenderer`'s actual `initDrag`, `advanceFrame`, and `runSpring` functions. It exposes those private functions during bundling and advances a deterministic clock in 16 ms steps; it does not reimplement their behavior.

```sh
node tools/upstream-parity/generate-fifteen.cjs /path/to/upstream/dragology
cargo test -p dragology-gallery --test fifteen_jitter
```

The generator validates the pinned checkout and writes `examples/gallery/tests/fixtures/fifteen-frames.json`. Its 97 frames cover leaving a snapped home position, a continuous circular lap, a reverse lap, backtracking near snap boundaries, and a stationary hold. Each frame records all 16 tile positions; the source's stable numeric-ID paint order is also checked. The regression preserves branch springs that start while two previews still look identical, because their elapsed time affects motion after the pointer leaves the snap zone. Ordinary test runs use the committed JSON and remain offline.
