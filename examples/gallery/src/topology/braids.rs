use super::*;
use dragology::{between, closest, fixed};
use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::TAU;

const STRAND_FILLS: [Color; 6] = [
    [254, 202, 202, 255],
    [253, 230, 138, 255],
    [187, 247, 208, 255],
    [191, 219, 254, 255],
    [221, 214, 254, 255],
    [251, 207, 232, 255],
];
const STRAND_STROKES: [Color; 6] = [
    [220, 38, 38, 255],
    [217, 119, 6, 255],
    [22, 163, 74, 255],
    [37, 99, 235, 255],
    [124, 58, 237, 255],
    [219, 39, 119, 255],
];
const DEPTH: f64 = 0.96 * 100. / (2. * 0.8 * 28.);
#[path = "braids_geometry.rs"]
mod geometry;

#[path = "permutation_geometry.rs"]
mod perm_geometry;

pub(super) fn entries() -> Vec<Entry> {
    vec![
        Entry::new(
            "braid-old",
            "Braid construction",
            "Pull a strand end sideways to append a crossing; pull it upward to erase. Click a crossing to swap which strand passes over.",
            || demo(BraidDemo { old: true }),
        ),
        Entry::new(
            "braid",
            "Braids and links",
            "Drag crossings to commute or apply a braid relation. Pull a straight segment across its neighbor to insert an inverse pair. Close the braid to use Markov moves.",
            || demo(BraidDemo { old: false }),
        ),
        Entry::new(
            "permutation-lab",
            "Permutation lab",
            "One permutation in tile, cycle, matrix, wiring and permutohedron views. Reorder tiles or drag crossings to rewrite the generator word.",
            || demo(PermutationDemo),
        ),
    ]
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Letter {
    key: usize,
    i: usize,
    sign: i8,
}
fn permutation(n: usize, word: &[Letter]) -> Vec<usize> {
    let mut p: Vec<_> = (0..n).collect();
    for l in word {
        p.swap(l.i, l.i + 1);
    }
    p
}
fn crossings(n: usize, word: &[Letter]) -> Vec<(usize, usize, usize)> {
    let mut p: Vec<_> = (0..n).collect();
    word.iter()
        .map(|l| {
            let (a, b) = (p[l.i], p[l.i + 1]);
            p.swap(l.i, l.i + 1);
            (a, b, if l.sign > 0 { a } else { b })
        })
        .collect()
}
fn pair(a: usize, b: usize) -> (usize, usize) {
    (a.min(b), a.max(b))
}
fn braid_move(n: usize, word: &[Letter], start: usize, signed: bool) -> Option<Vec<Letter>> {
    let part = word.get(start..start + 3)?;
    if part[0].i != part[2].i || part[0].i.abs_diff(part[1].i) != 1 {
        return None;
    }
    let old = crossings(n, word);
    let cross = &old[start..start + 3];
    if signed
        && !cross
            .iter()
            .any(|x| cross.iter().filter(|y| y.2 == x.2).count() == 2)
    {
        return None;
    }
    let previous: BTreeMap<_, _> = cross
        .iter()
        .zip(part)
        .map(|(&(a, b, over), l)| (pair(a, b), (over, l.key)))
        .collect();
    let mut out = word.to_vec();
    let i = part[0].i;
    let j = part[1].i;
    out[start].i = j;
    out[start + 1].i = i;
    out[start + 2].i = j;
    let next = crossings(n, &out);
    for k in start..start + 3 {
        let (a, b, _) = next[k];
        let (over, key) = previous[&pair(a, b)];
        out[k].key = key;
        out[k].sign = if !signed || a == over { 1 } else { -1 };
    }
    Some(out)
}
fn crossing_movable(n: usize, word: &[Letter], j: usize, signed: bool) -> bool {
    [j.checked_sub(1), (j + 1 < word.len()).then_some(j + 1)]
        .into_iter()
        .flatten()
        .any(|k| word[j].i.abs_diff(word[k].i) >= 2)
        || j > 0 && braid_move(n, word, j - 1, signed).is_some()
}
fn inversions(p: &[usize]) -> usize {
    p.iter()
        .enumerate()
        .map(|(i, a)| p[i + 1..].iter().filter(|b| a > *b).count())
        .sum()
}
fn reduced_word(p: &[usize]) -> Vec<Letter> {
    let mut arr = p.to_vec();
    let mut swaps = vec![];
    for _ in 0..arr.len() {
        for i in 0..arr.len() - 1 {
            if arr[i] > arr[i + 1] {
                arr.swap(i, i + 1);
                swaps.push(i);
            }
        }
    }
    swaps.reverse();
    swaps
        .into_iter()
        .enumerate()
        .map(|(key, i)| Letter { key, i, sign: 1 })
        .collect()
}
fn cycle_list(p: &[usize]) -> Vec<Vec<usize>> {
    let mut seen = BTreeSet::new();
    let mut result = vec![];
    for v in 0..p.len() {
        if seen.contains(&v) {
            continue;
        }
        let mut cycle = vec![];
        let mut x = v;
        while seen.insert(x) {
            cycle.push(x);
            x = p[x];
        }
        result.push(cycle);
    }
    result
}
fn from_cycles(n: usize, cycles: &[Vec<usize>]) -> Vec<usize> {
    let mut p: Vec<_> = (0..n).collect();
    for cycle in cycles {
        for (i, &v) in cycle.iter().enumerate() {
            p[v] = cycle[(i + 1) % cycle.len()];
        }
    }
    p
}

#[derive(Clone, PartialEq)]
struct BraidState {
    n: usize,
    word: Vec<Letter>,
    next: usize,
    closure: bool,
    slice: bool,
    hover: Option<f64>,
    motion: Option<BraidMotion>,
    preset_index: Option<usize>,
    palette: Option<Vec<usize>>,
    by_column: Option<usize>,
    reset_count: usize,
    closed: Option<(usize, [Letter; 2])>,
}
#[derive(Clone, PartialEq)]
enum BraidMotion {
    Turn {
        progress: f64,
    },
    Grow {
        at_end: bool,
        progress: f64,
    },
    Pair {
        start: usize,
        width: f64,
        amplitude: f64,
    },
    Orbit {
        progress: f64,
        key: usize,
    },
}
struct BraidDemo {
    old: bool,
}
impl BraidDemo {
    fn preset(n: usize, values: &[i32], closure: bool) -> BraidState {
        BraidState {
            n,
            word: values
                .iter()
                .enumerate()
                .map(|(key, &v)| Letter {
                    key,
                    i: v.unsigned_abs() as usize - 1,
                    sign: if v > 0 { 1 } else { -1 },
                })
                .collect(),
            next: values.len(),
            closure,
            slice: false,
            hover: None,
            motion: None,
            preset_index: None,
            palette: None,
            by_column: None,
            reset_count: 0,
            closed: None,
        }
    }
    fn source_initial(n: usize, closure: bool) -> BraidState {
        let values: &[i32] = match n {
            1 => &[],
            2 => &[1, 1, -1],
            3 => &[1, 2, 1, -2],
            4 => &[1, -3, 2, 1, 2],
            5 => &[1, -3, 4, 2, 1, 2],
            _ => &[1, -3, 5, 4, 2, 1, 2],
        };
        Self::preset(n, values, closure)
    }
    fn xy(&self, column: f64, row: f64) -> Point {
        if self.old {
            Point::new(row * 50., column * 50.)
        } else {
            Point::new(48. + column * 52., 52. + row * 28.)
        }
    }
    fn capacity(&self, s: &BraidState) -> usize {
        if self.old {
            s.word.len() + 1
        } else if s.closure {
            14
        } else {
            12.max(2 * s.n + 4)
        }
    }
    fn can_append(&self, s: &BraidState, count: usize) -> bool {
        self.old || s.word.len() + count < self.capacity(s)
    }
    fn layout_xy(&self, raw: &BraidState, column: f64, mut row: f64) -> Point {
        if self.old {
            return self.xy(column, row);
        }
        let s = geometry::displayed(raw);
        let orbit = geometry::orbit(self, &s);
        let j = column.floor().max(0.) as usize;
        let x = if column >= self.capacity(&s) as f64 {
            geometry::x0(&s) + self.capacity(&s) as f64 * geometry::cw(&s)
        } else {
            geometry::col_x(&s, j, orbit.as_ref())
                + geometry::col_width(&s, j, orbit.as_ref()) * (column - j as f64)
        };
        if let Some(BraidMotion::Turn { progress }) = s.motion {
            if column > (s.word.len() - 1) as f64 {
                let i = s.word.last().unwrap().i as f64;
                if row == i {
                    row = i + 1. - progress;
                } else if row == i + 1. {
                    row = i + progress;
                }
            }
        }
        Point::new(x, geometry::row_y(&s, row))
    }
    fn slice_origin(&self, _s: &BraidState) -> Point {
        Point::new(564., 52.)
    }
    fn configuration(&self, s: &BraidState) -> Vec<(usize, f64, f64)> {
        let pointer_x = s
            .hover
            .map_or(f64::INFINITY, |t| geometry::x0(s) + t * geometry::cw(s));
        let orbit = geometry::orbit(self, s);
        let mut order = (0..s.n).collect::<Vec<_>>();
        let mut turning = None;
        for (j, letter) in s.word.iter().enumerate() {
            if matches!(s.motion,Some(BraidMotion::Pair{start,amplitude,..})if (j==start||j==start+1)&&amplitude==0.)
            {
                continue;
            }
            if let Some(BraidMotion::Turn { progress }) =
                s.motion.as_ref().filter(|_| j + 1 == s.word.len())
            {
                turning = Some((letter, (1. - 2. * progress).clamp(-1., 1.).acos()));
                break;
            }
            if j + 1 == s.word.len() {
                if let Some(done) = geometry::end_fraction(self, s).filter(|done| *done < 1.) {
                    turning = Some((letter, geometry::theta_at(done)));
                    break;
                }
            }
            let x = geometry::col_x(s, j, orbit.as_ref());
            let w = geometry::col_width(s, j, orbit.as_ref());
            if pointer_x >= x + w {
                order.swap(letter.i, letter.i + 1);
            } else if pointer_x >= x {
                turning = Some((letter, geometry::theta_at((pointer_x - x) / w)));
                break;
            } else {
                break;
            }
        }
        order
            .iter()
            .enumerate()
            .map(|(r, strand)| {
                let (mut row, mut depth) = (r as f64, 0.);
                if let Some((letter, theta)) = turning {
                    if r == letter.i || r == letter.i + 1 {
                        row = letter.i as f64
                            + 0.5
                            + (if r == letter.i { -0.5 } else { 0.5 }) * theta.cos();
                        let over = if letter.sign > 0 {
                            order[letter.i]
                        } else {
                            order[letter.i + 1]
                        };
                        depth = if *strand == over { 0.5 } else { -0.5 } * theta.sin();
                    }
                }
                (*strand, row, depth)
            })
            .collect()
    }
    fn project_slice(&self, s: &BraidState, row: f64, depth: f64) -> (Point, f64) {
        // Same perspective projection as upstream: front is to the right,
        // with resting points vertically aligned and full scale.
        let origin = self.slice_origin(s);
        let plane_depth = DEPTH;
        let u = 0.5 - depth / (2. * plane_depth);
        let scale = 1.25 / (1. + 0.5 * u);
        let mid = (geometry::row_y(s, s.n as f64 - 1.) - 52.) / 2.;
        let x = 100. - 100. * u * 1.5 / (1. + 0.5 * u);
        (
            origin + Vec2::new(x, mid + (geometry::row_y(s, row) - 52. - mid) * scale),
            scale,
        )
    }
    fn turn_spec(
        &self,
        s: &BraidState,
        strand: usize,
        other: usize,
        front: bool,
    ) -> DragSpec<'static, BraidState> {
        let order = permutation(s.n, &s.word);
        let row = order.iter().position(|v| *v == strand).unwrap();
        let mut pending = s.clone();
        pending.hover = None;
        pending.word.push(Letter {
            key: s.next,
            i: row.min(other),
            sign: if (row < other) == front { 1 } else { -1 },
        });
        pending.next += 1;
        let mut done = pending.clone();
        done.motion = None;
        let original = s.clone();
        dragology::vary_constrained(
            [0.],
            move |[progress]| {
                let mut state = pending.clone();
                state.motion = Some(BraidMotion::Turn { progress });
                state
            },
            |state: &BraidState| {
                let progress = match state.motion {
                    Some(BraidMotion::Turn { progress }) => progress,
                    _ => 0.,
                };
                [-progress, progress - 1.]
            },
        )
        .on_drop(move |state| {
            if matches!(state.motion, Some(BraidMotion::Turn { progress }) if progress > 0.5) {
                done.clone()
            } else {
                original.clone()
            }
        })
        .with_snap_radius_options(
            4.,
            dragology::SnapOptions {
                chain: true,
                ..Default::default()
            },
        )
    }
    fn stabilize(&self, s: &BraidState, at_end: bool) -> Option<DragSpec<'static, BraidState>> {
        if !s.closure || s.n >= 6 || !self.can_append(s, 1) {
            return None;
        }
        let mut grown = s.clone();
        let mut palette = geometry::palette(s);
        let end = permutation(s.n, &s.word)[s.n - 1];
        palette.push(palette[if at_end { end } else { s.n - 1 }]);
        grown.palette = Some(palette);
        let letter = Letter {
            key: grown.next,
            i: s.n - 1,
            sign: if at_end { 1 } else { -1 },
        };
        grown.next += 1;
        if at_end {
            grown.word.push(letter);
        } else {
            grown.word.insert(0, letter);
        }
        grown.n += 1;
        grown.hover = None;
        let mut start = grown.clone();
        start.motion = Some(BraidMotion::Grow {
            at_end,
            progress: 0.,
        });
        let original = s.clone();
        Some(between([start, grown]).on_drop(move |state| {
            if state.motion.is_some() {
                original.clone()
            } else {
                state.clone()
            }
        }))
    }
    fn cross_point(&self, s: &BraidState, key: usize) -> Option<Point> {
        if self.old {
            let (j, l) = s.word.iter().enumerate().find(|(_, l)| l.key == key)?;
            return Some(self.xy(j as f64 + 1.5, l.i as f64 + 0.5));
        }
        let shown = geometry::displayed(s);
        let orbit = geometry::orbit(self, &shown);
        if let Some(o) = orbit.as_ref().filter(|o| shown.word[o.j].key == key) {
            return Some(o.center);
        }
        let (j, l) = shown.word.iter().enumerate().find(|(_, l)| l.key == key)?;
        Some(Point::new(
            geometry::col_x(&shown, j, orbit.as_ref())
                + geometry::col_width(&shown, j, orbit.as_ref()) / 2.,
            geometry::row_y(&shown, l.i as f64 + 0.5),
        ))
    }
    fn orbit_spec(&self, s: &BraidState, key: usize, first: bool) -> DragSpec<'static, BraidState> {
        let mut base = s.clone();
        base.hover = None;
        base.by_column = None;
        base.palette = Some(geometry::palette(s));
        if first {
            let first = base.word.remove(0);
            base.palette.as_mut().unwrap().swap(first.i, first.i + 1);
            base.word.push(first);
        }
        dragology::vary_constrained(
            [if first { 1. } else { 0. }],
            move |[progress]| {
                let mut state = base.clone();
                state.motion = Some(BraidMotion::Orbit { progress, key });
                state
            },
            |s: &BraidState| {
                let progress = match s.motion {
                    Some(BraidMotion::Orbit { progress, .. }) => progress,
                    _ => 0.,
                };
                [-progress, progress - 1.]
            },
        )
        .on_drop(|s| {
            let mut state = s.clone();
            if matches!(s.motion,Some(BraidMotion::Orbit{progress,..})if progress>=0.5) {
                geometry::rotate_last(&mut state);
            }
            state.motion = None;
            state
        })
    }
    fn insert_spec(
        &self,
        s: &BraidState,
        col: usize,
        row: usize,
        other: usize,
    ) -> DragSpec<'static, BraidState> {
        let mut inserted = Self::add_pair(s, col, row.min(other), if row < other { 1 } else { -1 });
        inserted.hover = None;
        inserted.by_column = None;
        let mut closed = inserted.clone();
        closed.motion = Some(BraidMotion::Pair {
            start: col,
            width: 0.,
            amplitude: 0.,
        });
        let original = s.clone();
        between([closed, inserted]).on_drop(move |state| {
            if matches!(state.motion, Some(BraidMotion::Pair { .. })) {
                original.clone()
            } else {
                state.clone()
            }
        })
    }
    fn cancel_pair_spec(&self, s: &BraidState, col: usize) -> DragSpec<'static, BraidState> {
        let mut flat = s.clone();
        flat.hover = None;
        flat.by_column = None;
        flat.motion = Some(BraidMotion::Pair {
            start: col,
            width: 1.,
            amplitude: 0.,
        });
        let mut removed = s.clone();
        removed.closed = Some((
            col,
            [removed.word[col].clone(), removed.word[col + 1].clone()],
        ));
        removed.word.drain(col..col + 2);
        removed.motion = None;
        removed.hover = None;
        between([s.clone(), flat]).on_drop(move |state| {
            if matches!(state.motion, Some(BraidMotion::Pair { .. })) {
                removed.clone()
            } else {
                state.clone()
            }
        })
    }
    fn add_pair(s: &BraidState, col: usize, row: usize, sign: i8) -> BraidState {
        let mut n = s.clone();
        n.word.splice(
            col..col,
            [
                Letter {
                    key: n.next,
                    i: row,
                    sign,
                },
                Letter {
                    key: n.next + 1,
                    i: row,
                    sign: -sign,
                },
            ],
        );
        n.next += 2;
        n
    }
}
impl Example for BraidDemo {
    type State = BraidState;
    fn initial(&self) -> BraidState {
        Self::preset(4, if self.old { &[] } else { &[1, -3, 2, 1, 2] }, false)
    }
    fn begin_drag(&self, s: &BraidState, id: &str, _: Modifiers) -> Option<(BraidState, String)> {
        if s.closed.is_some() {
            let mut clean = s.clone();
            clean.closed = None;
            Some((clean, id.to_owned()))
        } else {
            None
        }
    }
    fn scene(&self, s: &BraidState) -> Scene {
        if !self.old {
            let mut scene = geometry::drawing(self, s);
            for o in &mut scene.objects {
                o.cursor = if let Some(key) =
                    o.id.strip_prefix("cross-")
                        .and_then(|x| x.parse::<usize>().ok())
                {
                    let movable = s.word.iter().position(|l| l.key == key).is_some_and(|j| {
                        crossing_movable(s.n, &s.word, j, true)
                            || s.closure && (j == 0 || j + 1 == s.word.len())
                    });
                    if movable {
                        Cursor::Grab
                    } else {
                        Cursor::NotAllowed
                    }
                } else if o.id.starts_with("slice-") && o.hit.is_some() {
                    if self.can_append(s, 1) {
                        Cursor::Grab
                    } else {
                        Cursor::NotAllowed
                    }
                } else if o.id.starts_with("segment-") {
                    Cursor::ResizeVertical
                } else if o.id.starts_with("curl-") && s.n < 6 && self.can_append(s, 1) {
                    Cursor::ResizeVertical
                } else if o.id == "destabilize" && s.n > 1 {
                    let ks = s
                        .word
                        .iter()
                        .enumerate()
                        .filter(|(_, l)| l.i == s.n - 2)
                        .map(|(j, _)| j)
                        .collect::<Vec<_>>();
                    if ks.len() == 1 && (ks[0] == 0 || ks[0] + 1 == s.word.len()) {
                        Cursor::ResizeVertical
                    } else {
                        Cursor::Default
                    }
                } else {
                    Cursor::Default
                };
            }
            return scene;
        }
        let mut scene = Scene::new(250., 400.);
        let mut strands = (0..s.n).collect::<Vec<_>>();
        for row in 0..s.n {
            scene.push(Object::line(
                format!("initial-{row}"),
                self.xy(0., row as f64),
                self.xy(1., row as f64),
                [0, 0, 0, 255],
                2.,
            ));
        }
        for (j, l) in s.word.iter().enumerate() {
            for row in 0..s.n {
                if row == l.i || row == l.i + 1 {
                    continue;
                }
                scene.push(Object::line(
                    format!("strand-{j}-{}", strands[row]),
                    self.xy(j as f64 + 1., row as f64),
                    self.xy(j as f64 + 2., row as f64),
                    [0, 0, 0, 255],
                    2.,
                ));
            }
            let over = if l.sign > 0 { l.i } else { l.i + 1 };
            let under = 2 * l.i + 1 - over;
            scene.push(Object::line(
                format!("strand-{j}-{}", strands[under]),
                self.xy(j as f64 + 1., under as f64),
                self.xy(j as f64 + 2., over as f64),
                [0, 0, 0, 255],
                2.,
            ));
            scene.push(
                Object::circle(
                    format!("gap-{}", l.key),
                    self.xy(j as f64 + 1.5, l.i as f64 + 0.5),
                    50. / 7.,
                    [255; 4],
                )
                .decorative(),
            );
            scene.push(Object::line(
                format!("strand-{j}-{}", strands[over]),
                self.xy(j as f64 + 1., over as f64),
                self.xy(j as f64 + 2., under as f64),
                [0, 0, 0, 255],
                2.,
            ));
            scene.push(Object::circle(
                format!("cross-{}", l.key),
                self.xy(j as f64 + 1.5, l.i as f64 + 0.5),
                50. / 3.,
                [0; 4],
            ));
            strands.swap(l.i, l.i + 1);
        }
        for (row, v) in strands.into_iter().enumerate() {
            let mut end = Object::circle(
                format!("end-{v}"),
                self.xy(s.word.len() as f64 + 1., row as f64),
                20.,
                [0; 4],
            );
            end.shapes.push(Shape::Circle {
                center: Point::ORIGIN,
                radius: 4.,
                fill: [128, 128, 128, 255],
                stroke: None,
            });
            scene.push(end);
        }

        for o in &mut scene.objects {
            if o.id.starts_with("cross-") {
                o.cursor = Cursor::Pointer;
                o.click_handler = true;
            }
        }
        scene
    }
    fn scene_interaction(
        &self,
        s: &BraidState,
        dragged: Option<&str>,
        hovered: Option<&str>,
    ) -> Scene {
        let mut scene = self.scene(s);
        if let Some(id) = dragged.or(hovered) {
            if let Some(object) = scene.objects.iter_mut().find(|o| o.id == id) {
                let c = if id.starts_with("slice-") {
                    let v = id[6..].parse::<usize>().unwrap_or(0);
                    let palette = geometry::palette(s);
                    let mut c = STRAND_FILLS[palette[v] % 6];
                    c[3] = if dragged.is_some() { 204 } else { 89 };
                    c
                } else if self.old {
                    [30, 144, 255, 38]
                } else if id.starts_with("cross-") {
                    let key = id[6..].parse::<usize>().unwrap_or(usize::MAX);
                    let movable = s.word.iter().position(|l| l.key == key).is_some_and(|j| {
                        [j.checked_sub(1), (j + 1 < s.word.len()).then_some(j + 1)]
                            .into_iter()
                            .flatten()
                            .any(|k| s.word[j].i.abs_diff(s.word[k].i) >= 2)
                            || j > 0 && braid_move(s.n, &s.word, j - 1, true).is_some()
                            || s.closure && (j == 0 || j + 1 == s.word.len())
                    });
                    if movable {
                        [37, 99, 235, 89]
                    } else {
                        [156, 163, 175, 89]
                    }
                } else {
                    [220, 38, 38, 89]
                };
                if let Some(Shape::Circle { fill, .. } | Shape::Rect { fill, .. }) =
                    object.shapes.first_mut()
                {
                    *fill = c;
                }
            }
        }
        scene
    }
    fn drag(
        &self,
        s: &BraidState,
        id: &str,
        _: Modifiers,
    ) -> Option<DragSpec<'static, BraidState>> {
        let clean;
        let s = if s.closed.is_some() {
            clean = {
                let mut t = s.clone();
                t.closed = None;
                t
            };
            &clean
        } else {
            s
        };

        if let Some(strand) = id
            .strip_prefix("slice-")
            .and_then(|s| s.parse::<usize>().ok())
        {
            if !self.can_append(s, 1) {
                return Some(fixed(s.clone()));
            }
            let order = permutation(s.n, &s.word);
            let row = order.iter().position(|v| *v == strand)?;
            let specs = [true, false]
                .into_iter()
                .flat_map(|front| {
                    [row.checked_sub(1), (row + 1 < s.n).then_some(row + 1)]
                        .into_iter()
                        .flatten()
                        .map(move |other| (other, front))
                })
                .map(|(other, front)| self.turn_spec(s, strand, other, front))
                .collect::<Vec<_>>();
            let rest_x = self.project_slice(s, 0., 0.).0.x;
            return Some(closest(specs).change_frame(move |pointer| {
                Point::new(pointer.x.clamp(rest_x - 4., rest_x + 4.), pointer.y)
            }));
        }
        if id == "curl-left" || id == "curl-right" {
            return self.stabilize(s, id == "curl-right");
        }
        if let Some(key) = id
            .strip_prefix("cross-")
            .and_then(|x| x.parse::<usize>().ok())
        {
            if self.old {
                return None;
            }
            let j = s.word.iter().position(|l| l.key == key)?;
            let mut specs = vec![fixed(s.clone())];
            for k in [j.checked_sub(1), (j + 1 < s.word.len()).then_some(j + 1)]
                .into_iter()
                .flatten()
            {
                if s.word[j].i.abs_diff(s.word[k].i) >= 2 {
                    let mut n = s.clone();
                    n.word.swap(j, k);
                    specs.push(between([s.clone(), n]));
                }
            }
            if j > 0
                && let Some(word) = braid_move(s.n, &s.word, j - 1, true)
            {
                let mut from = s.clone();
                from.by_column = Some(j - 1);
                let mut to = BraidState { word, ..s.clone() };
                to.by_column = Some(j - 1);
                specs.push(between([from, to]));
            }
            if s.closure {
                if j + 1 == s.word.len() {
                    specs.push(self.orbit_spec(s, key, false));
                }
                if j == 0 && s.word.len() > 1 {
                    specs.push(self.orbit_spec(s, key, true));
                }
            }
            return Some(closest(specs));
        }
        if let Some(strand) = id
            .strip_prefix("end-")
            .and_then(|x| x.parse::<usize>().ok())
        {
            if !self.old {
                return None;
            }
            let p = permutation(s.n, &s.word);
            let row = p.iter().position(|&v| v == strand)?;
            if !self.can_append(s, 1) {
                return Some(fixed(s.clone()));
            }
            let mut states = vec![s.clone()];
            for nextrow in [row.checked_sub(1), (row + 1 < s.n).then_some(row + 1)]
                .into_iter()
                .flatten()
            {
                let mut n = s.clone();
                n.word.push(Letter {
                    key: n.next,
                    i: row.min(nextrow),
                    sign: if nextrow > row { 1 } else { -1 },
                });
                n.next += 1;
                states.push(n);
            }
            let mut specs = vec![between(states)];
            if self.old && !s.word.is_empty() {
                let mut n = s.clone();
                n.word.pop();
                specs.push(between([s.clone(), n]));
            }
            return Some(closest(specs).with_snap_radius_options(
                1.,
                dragology::SnapOptions {
                    chain: true,
                    ..Default::default()
                },
            ));
        }
        if let Some(parts) = id.strip_prefix("segment-") {
            let (col, strand) = parts.split_once('-')?;
            let col = col.parse::<usize>().ok()?;
            let strand = strand.parse::<usize>().ok()?;
            let row = permutation(s.n, &s.word[..col])
                .iter()
                .position(|v| *v == strand)?;
            if let (Some(a), Some(b)) = (s.word.get(col), s.word.get(col + 1)) {
                if a.i == b.i && a.sign != b.sign && (row == a.i || row == a.i + 1) {
                    return Some(self.cancel_pair_spec(s, col));
                }
            }
            let mut braid_side = None;
            let mut braid_spec = None;
            if col > 0 {
                if let Some(word) = braid_move(s.n, &s.word, col - 1, true) {
                    let a = s.word[col - 1].i;
                    let b = s.word[col].i;
                    if (row == a || row == a + 1) && row != b && row != b + 1 {
                        braid_side = Some(if row < b { row + 1 } else { row - 1 });
                        let mut from = s.clone();
                        from.by_column = Some(col - 1);
                        let mut to = BraidState { word, ..s.clone() };
                        to.by_column = Some(col - 1);
                        braid_spec = Some(between([from, to]));
                    }
                }
            }
            let mut specs = Vec::new();
            for other in [row.checked_sub(1), (row + 1 < s.n).then_some(row + 1)]
                .into_iter()
                .flatten()
            {
                if braid_side == Some(other) {
                    specs.extend(braid_spec.take());
                } else if self.can_append(s, 2) {
                    specs.push(self.insert_spec(s, col, row, other));
                }
            }
            return Some(if specs.is_empty() {
                fixed(s.clone())
            } else {
                closest(specs)
            });
        }
        if id == "destabilize" && s.n > 1 {
            let bottom = s.n - 2;
            let indices: Vec<_> = s
                .word
                .iter()
                .enumerate()
                .filter(|(_, l)| l.i == bottom)
                .map(|(i, _)| i)
                .collect();
            if indices.len() == 1 && (indices[0] == 0 || indices[0] + 1 == s.word.len()) {
                let at_end = indices[0] + 1 == s.word.len();
                let mut collapsed = s.clone();
                collapsed.hover = None;
                collapsed.motion = Some(BraidMotion::Grow {
                    at_end,
                    progress: 0.,
                });
                let mut gone = s.clone();
                gone.word.remove(indices[0]);
                gone.n -= 1;
                if let Some(palette) = gone.palette.as_mut() {
                    palette.pop();
                }
                gone.hover = None;
                return Some(between([s.clone(), collapsed]).on_drop(move |state| {
                    if state.motion.is_some() {
                        gone.clone()
                    } else {
                        state.clone()
                    }
                }));
            }
        }
        None
    }
    fn locate(&self, s: &BraidState, id: &str) -> Option<Affine> {
        if let Some(key) = id
            .strip_prefix("cross-")
            .and_then(|s| s.parse::<usize>().ok())
        {
            return self
                .cross_point(s, key)
                .map(|p| Affine::translate(p.to_vec2()));
        }
        if let Some(strand) = id
            .strip_prefix("slice-")
            .and_then(|id| id.parse::<usize>().ok())
        {
            let shown = geometry::displayed(s);
            let (_, row, depth) = self
                .configuration(&shown)
                .into_iter()
                .find(|(v, _, _)| *v == strand)?;
            return Some(Affine::translate(
                self.project_slice(s, row, depth).0.to_vec2(),
            ));
        }
        if id == "curl-left" || id == "curl-right" {
            let column = if id == "curl-right" {
                self.capacity(s) as f64
            } else {
                0.
            };
            return Some(Affine::translate(
                self.layout_xy(s, column, s.n as f64 - 1.).to_vec2(),
            ));
        }
        if id == "destabilize" {
            let p = Point::new(
                geometry::x0(s) + self.capacity(s) as f64 * geometry::cw(s) / 2.,
                geometry::bottom_y(s),
            );
            return Some(Affine::translate(p.to_vec2()));
        }
        self.scene(s).object(id).map(|o| o.transform)
    }
    fn click(
        &self,
        s: &BraidState,
        id: &str,
        _: Point,
        _: Modifiers,
        _: usize,
    ) -> Option<BraidState> {
        if !self.old {
            return None;
        }
        let key = id.strip_prefix("cross-")?.parse::<usize>().ok()?;
        let mut n = s.clone();
        n.word.iter_mut().find(|l| l.key == key)?.sign *= -1;
        Some(n)
    }
    fn controls(&self, s: &BraidState) -> Vec<Control> {
        if self.old {
            return vec![];
        }
        vec![
            Control::Choice {
                id: "n",
                label: "n".into(),
                options: (1..=6).map(|i| i.to_string()).collect(),
                selected: s.n - 1,
            },
            Control::toggle("closure", "close the braid", s.closure),
            Control::toggle("slice", "show the slice", s.slice),
            Control::Choice {
                id: "preset",
                label: "Link preset".into(),
                options: std::iter::once("Choose a link preset".to_string())
                    .chain(PRESETS.iter().map(|p| p.0.to_string()))
                    .collect(),
                selected: s.preset_index.map_or(0, |i| i + 1),
            },
            Control::button("source-reset", "reset"),
        ]
    }
    fn action_transition(&self, _: &BraidState, id: &str) -> Option<dragology::Transition> {
        if self.old && id.starts_with("cross-") {
            Some(dragology::Transition::default())
        } else {
            None
        }
    }
    fn action(&self, s: &BraidState, id: &str, v: Value) -> Option<BraidState> {
        let mut next = match (id, v) {
            ("slice", Value::Bool(slice)) => return Some(BraidState { slice, ..s.clone() }),
            ("closure", Value::Bool(closure)) => {
                if s.reset_count == 0 {
                    Self::source_initial(s.n, closure)
                } else {
                    Self::preset(s.n, &[], closure)
                }
            }
            ("n", Value::Index(i)) => {
                if s.reset_count == 0 {
                    Self::source_initial((i + 1).clamp(1, 6), s.closure)
                } else {
                    Self::preset((i + 1).clamp(1, 6), &[], s.closure)
                }
            }
            ("preset", Value::Index(i)) => {
                let i = i.checked_sub(1)?;
                let p = PRESETS.get(i)?;
                let mut next = Self::preset(p.1, p.2, true);
                next.preset_index = Some(i);
                next.reset_count = s.reset_count + 1;
                next.slice = s.slice;
                return Some(next);
            }
            ("source-reset", _) => {
                let mut next = if let Some(i) = s.preset_index {
                    Self::preset(PRESETS[i].1, PRESETS[i].2, true)
                } else {
                    Self::preset(s.n, &[], s.closure)
                };
                next.preset_index = s.preset_index;
                next.reset_count = s.reset_count + 1;
                next.slice = s.slice;
                return Some(next);
            }
            _ => return None,
        };
        next.slice = s.slice;
        next.reset_count = s.reset_count;
        Some(next)
    }
    fn hover(&self, s: &BraidState, p: Point) -> Option<BraidState> {
        if self.old {
            return None;
        }
        let hover = if p.x >= geometry::x0(s) - 14.
            && p.x <= geometry::x0(s) + self.capacity(s) as f64 * geometry::cw(s)
            && p.y >= geometry::row_y(s, -0.7)
            && p.y <= geometry::row_y(s, s.n as f64 - 0.3)
        {
            Some((p.x - geometry::x0(s)) / geometry::cw(s))
        } else {
            None
        };
        (hover != s.hover).then(|| BraidState { hover, ..s.clone() })
    }
    fn status(&self, s: &BraidState) -> String {
        format!(
            "{} strands · {} crossings · writhe {}",
            s.n,
            s.word.len(),
            s.word.iter().map(|l| l.sign as i32).sum::<i32>()
        )
    }
}
const PRESETS: [(&str, usize, &[i32]); 11] = [
    ("unknot", 1, &[]),
    ("Hopf link", 2, &[1, 1]),
    ("trefoil", 2, &[1, 1, 1]),
    ("cinquefoil", 2, &[1, 1, 1, 1, 1]),
    ("figure-eight", 3, &[1, -2, 1, -2]),
    ("Borromean rings", 3, &[1, -2, 1, -2, 1, -2]),
    ("(3, 4) torus knot", 3, &[1, 2, 1, 2, 1, 2, 1, 2]),
    ("unknot (4 strands)", 4, &[1, -2, 3]),
    ("Hopf link (3 strands)", 3, &[1, 1, 1, 2, -1]),
    ("trefoil (3 strands)", 3, &[1, 2, 1, 2]),
    ("figure-eight (4 strands)", 4, &[2, 1, -2, 1, -2, 3, -2]),
];

#[derive(Clone, PartialEq)]
struct PermState {
    n: usize,
    word: Vec<Letter>,
    next: usize,
    adjacent: bool,
    solid: bool,
    orientation: [f64; 4],
    spin: [f64; 2],
    motion: Option<PermMotion>,
    by_column: bool,
    reset_count: usize,
    closed: Option<(usize, [Letter; 2])>,
}
#[derive(Clone, PartialEq)]
enum PermMotion {
    Pair {
        start: usize,
        width: f64,
        amplitude: f64,
    },
}
struct PermutationDemo;
fn perms(mut p: Vec<usize>) -> Vec<Vec<usize>> {
    fn rec(k: usize, p: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
        if k == p.len() {
            out.push(p.clone());
        } else {
            for j in k..p.len() {
                p.swap(k, j);
                rec(k + 1, p, out);
                p.swap(k, j);
            }
        }
    }
    let mut out = vec![];
    rec(0, &mut p, &mut out);
    out
}
fn projected_permutation(p: &[usize]) -> Point {
    static POINTS: std::sync::OnceLock<BTreeMap<Vec<usize>, Point>> = std::sync::OnceLock::new();
    POINTS.get_or_init(|| {
        let mut points = BTreeMap::new();
        for n in [3, 4] {
            let raw = perms((0..n).collect())
                .into_iter()
                .map(|perm| {
                    let mut v = vec![0.; n];
                    for (i, value) in perm.iter().enumerate() {
                        v[*value] = i as f64 - (n as f64 - 1.) / 2.;
                    }
                    if n == 4 {
                        let c = [-0.5, -0.5, -0.5, 1.5];
                        let eye = c.map(|x| x * 1.6);
                        let chat = c.map(|x| x / 3f64.sqrt());
                        let numerator = (0..4).map(|i| eye[i] * chat[i]).sum::<f64>();
                        let denominator = (0..4).map(|i| (eye[i] - v[i]) * chat[i]).sum::<f64>();
                        let t = numerator / denominator;
                        for i in 0..4 {
                            v[i] = eye[i] + t * (v[i] - eye[i]);
                        }
                    }
                    let point = Point::new(
                        (v[0] - v[1]) / 2f64.sqrt(),
                        (v[0] + v[1] - 2. * v[2]) / 6f64.sqrt(),
                    );
                    (perm, point)
                })
                .collect::<Vec<_>>();
            let radius = raw
                .iter()
                .map(|(_, p)| p.to_vec2().hypot())
                .fold(0., f64::max);
            for (perm, p) in raw {
                points.insert(perm, (p.to_vec2() / radius).to_point());
            }
        }
        points
    })[p]
}
fn perm_vertex(p: &[usize], s: &PermState) -> Point {
    if p.len() == 3 || !s.solid {
        let p = projected_permutation(p);
        return Point::new(580. + p.x * 88., 160. + p.y * 88.);
    }
    let v = quat_rotate(view_rotation(s), poly_point(p));
    let f = 4. / (4. - v[2]);
    Point::new(580. + v[0] * f * 88. * 0.92, 160. - v[1] * f * 88. * 0.92)
}

type V3 = [f64; 3];
fn dot3(a: V3, b: V3) -> f64 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}
fn sub3(a: V3, b: V3) -> V3 {
    std::array::from_fn(|i| a[i] - b[i])
}
fn unit3(a: V3) -> V3 {
    let l = dot3(a, a).sqrt();
    a.map(|v| v / l)
}
fn cross3(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn quat_axis(axis: V3, angle: f64) -> [f64; 4] {
    let t = (angle / 2.).sin();
    [(angle / 2.).cos(), axis[0] * t, axis[1] * t, axis[2] * t]
}
fn quat_mul(a: [f64; 4], b: [f64; 4]) -> [f64; 4] {
    let [aw, ax, ay, az] = a;
    let [bw, bx, by, bz] = b;
    [
        aw * bw - ax * bx - ay * by - az * bz,
        aw * bx + ax * bw + ay * bz - az * by,
        aw * by - ax * bz + ay * bw + az * bx,
        aw * bz + ax * by - ay * bx + az * bw,
    ]
}
fn quat_rotate(q: [f64; 4], v: V3) -> V3 {
    let u = [q[1], q[2], q[3]];
    let t = cross3(u, v).map(|v| 2. * v);
    let c = cross3(u, t);
    std::array::from_fn(|i| v[i] + q[0] * t[i] + c[i])
}
fn view_rotation(s: &PermState) -> [f64; 4] {
    let [x, y] = s.spin;
    let angle = x.hypot(y);
    let q = if angle < 1e-12 {
        s.orientation
    } else {
        quat_mul(quat_axis([x / angle, y / angle, 0.], angle), s.orientation)
    };
    let norm = q.iter().map(|v| v * v).sum::<f64>().sqrt();
    q.map(|v| v / norm)
}
fn poly_point(p: &[usize]) -> V3 {
    let mut inv = [0.; 4];
    for (i, &v) in p.iter().enumerate() {
        inv[v] = i as f64 - 1.5
    }
    let [a, b, c, d] = inv;
    let scale = 2. * 5f64.sqrt();
    [
        (a + b - c - d) / scale,
        (a - b + c - d) / scale,
        (a - b - c + d) / scale,
    ]
}
#[derive(Clone)]
struct Face {
    mask: u8,
    vertices: Vec<Vec<usize>>,
    normal: V3,
}
fn poly_faces() -> &'static [Face] {
    static FACES: std::sync::OnceLock<Vec<Face>> = std::sync::OnceLock::new();
    FACES.get_or_init(|| {
        let all = perms(vec![0, 1, 2, 3]);
        (1u8..15)
            .map(|mask| {
                let count = mask.count_ones() as usize;
                let mut vertices: Vec<_> = all
                    .iter()
                    .filter(|p| p[..count].iter().all(|v| mask & (1 << v) != 0))
                    .cloned()
                    .collect();
                let center: V3 = std::array::from_fn(|i| {
                    vertices.iter().map(|p| poly_point(p)[i]).sum::<f64>() / vertices.len() as f64
                });
                let normal = unit3(center);
                let u = unit3(sub3(poly_point(&vertices[0]), center));
                let w = cross3(normal, u);
                vertices.sort_by(|a, b| {
                    let a = sub3(poly_point(a), center);
                    let b = sub3(poly_point(b), center);
                    dot3(a, w)
                        .atan2(dot3(a, u))
                        .total_cmp(&dot3(b, w).atan2(dot3(b, u)))
                });
                Face {
                    mask,
                    vertices,
                    normal,
                }
            })
            .collect()
    })
}
fn face_facing(s: &PermState, f: &Face) -> f64 {
    let q = view_rotation(s);
    let eye = unit3(sub3(
        [0., 0., 4.],
        quat_rotate(q, poly_point(&f.vertices[0])),
    ));
    dot3(quat_rotate(q, f.normal), eye)
}
fn face_center(s: &PermState, f: &Face) -> Point {
    (f.vertices
        .iter()
        .map(|p| perm_vertex(p, s).to_vec2())
        .fold(Vec2::ZERO, |a, b| a + b)
        / f.vertices.len() as f64)
        .to_point()
}
impl PermutationDemo {
    fn appendable(s: &PermState, count: usize) -> bool {
        s.word.len() + count < perm_geometry::cols(s)
    }
    fn pair_spec(s: &PermState, start: usize, generator: usize) -> DragSpec<'static, PermState> {
        let mut full = s.clone();
        full.motion = None;
        full.by_column = false;
        full.word.splice(
            start..start,
            [
                Letter {
                    key: s.next,
                    i: generator,
                    sign: 1,
                },
                Letter {
                    key: s.next + 1,
                    i: generator,
                    sign: 1,
                },
            ],
        );
        full.next += 2;
        let mut closed = full.clone();
        closed.motion = Some(PermMotion::Pair {
            start,
            width: 0.,
            amplitude: 0.,
        });
        let original = s.clone();
        between([closed, full]).on_drop(move |n| {
            if n.motion.is_some() {
                original.clone()
            } else {
                n.clone()
            }
        })
    }
    fn cancel_spec(s: &PermState, start: usize) -> DragSpec<'static, PermState> {
        let mut flat = s.clone();
        flat.motion = Some(PermMotion::Pair {
            start,
            width: 1.,
            amplitude: 0.,
        });
        flat.by_column = false;
        between([s.clone(), flat]).on_drop(move |n| {
            let mut n = n.clone();
            if n.motion.is_some() {
                n.closed = Some((start, [n.word[start].clone(), n.word[start + 1].clone()]));
                n.word.drain(start..start + 2);
                n.motion = None;
            }
            n
        })
    }
    fn source_perm(n: usize) -> Vec<usize> {
        match n {
            3 => vec![1, 2, 0],
            4 => vec![2, 0, 3, 1],
            5 => vec![2, 4, 0, 3, 1],
            _ => vec![2, 4, 0, 5, 3, 1],
        }
    }
}
impl Example for PermutationDemo {
    type State = PermState;
    fn initial(&self) -> PermState {
        let word = reduced_word(&[2, 0, 3, 1]);
        let next = word.len();
        PermState {
            n: 4,
            word,
            next,
            adjacent: true,
            solid: true,
            orientation: quat_mul(
                quat_axis([1., 0., 0.], -35f64.to_radians()),
                quat_axis([0., 1., 0.], 160f64.to_radians()),
            ),
            spin: [0.; 2],
            motion: None,
            by_column: false,
            reset_count: 0,
            closed: None,
        }
    }
    fn begin_drag(&self, s: &PermState, id: &str, _: Modifiers) -> Option<(PermState, String)> {
        if s.closed.is_some() {
            let mut clean = s.clone();
            clean.closed = None;
            Some((clean, id.to_owned()))
        } else {
            None
        }
    }
    fn scene(&self, s: &PermState) -> Scene {
        let mut scene = perm_geometry::drawing(s);
        for o in &mut scene.objects {
            o.cursor = if o.id.starts_with("tile-")
                || o.id.starts_with("cycle-") && o.hit.is_some()
                || o.id.starts_with("face-")
                || o.id.starts_with("walk-corner-")
                || o.id.starts_with("walk-braid-")
            {
                Cursor::Grab
            } else if o.id.starts_with("matrix-") && o.hit.is_some() || o.id.starts_with("insert-")
            {
                Cursor::ResizeVertical
            } else if o.id == "walk" {
                if Self::appendable(s, 1) {
                    Cursor::Grab
                } else {
                    Cursor::NotAllowed
                }
            } else if o.id.starts_with("wire-end-") {
                if Self::appendable(s, 1) {
                    Cursor::ResizeVertical
                } else {
                    Cursor::NotAllowed
                }
            } else if let Some(key) =
                o.id.strip_prefix("word-")
                    .and_then(|x| x.parse::<usize>().ok())
            {
                if s.word
                    .iter()
                    .position(|l| l.key == key)
                    .is_some_and(|j| crossing_movable(s.n, &s.word, j, false))
                {
                    Cursor::Grab
                } else {
                    Cursor::NotAllowed
                }
            } else {
                Cursor::Default
            };
        }
        scene
    }
    fn scene_interaction(
        &self,
        s: &PermState,
        dragged: Option<&str>,
        hovered: Option<&str>,
    ) -> Scene {
        let mut scene = self.scene(s);
        if let Some(id) = dragged {
            if id.starts_with("tile-") {
                if let Some(o) = scene.objects.iter_mut().find(|o| o.id == id) {
                    o.transform = Affine::translate((0., -6.)) * o.transform;
                }
            }
            if id.starts_with("tile-") || id.starts_with("cycle-") || id.starts_with("matrix-") {
                scene.objects.sort_by_key(|o| o.id == id);
            }
        }
        if let Some(id) = dragged.or(hovered) {
            if let Some(o) = scene.objects.iter_mut().find(|o| o.id == id) {
                let c = if id.starts_with("insert-") {
                    [220, 38, 38, 89]
                } else if let Some(v) = id
                    .strip_prefix("wire-end-")
                    .and_then(|v| v.parse::<usize>().ok())
                {
                    let mut c = STRAND_FILLS[v % 6];
                    c[3] = if dragged.is_some() { 153 } else { 89 };
                    c
                } else if let Some(key) = id
                    .strip_prefix("word-")
                    .and_then(|x| x.parse::<usize>().ok())
                {
                    if s.word
                        .iter()
                        .position(|l| l.key == key)
                        .is_some_and(|j| crossing_movable(s.n, &s.word, j, false))
                    {
                        [37, 99, 235, 89]
                    } else {
                        [156, 163, 175, 89]
                    }
                } else {
                    [37, 99, 235, 89]
                };
                if id.starts_with("insert-")
                    || id.starts_with("word-")
                    || id.starts_with("walk-corner-")
                    || id.starts_with("wire-end-")
                {
                    if let Some(Shape::Rect { fill, .. } | Shape::Circle { fill, .. }) =
                        o.shapes.first_mut()
                    {
                        *fill = c;
                    }
                } else if id.starts_with("walk-braid-") {
                    if let Some(Shape::Path { stroke, .. }) = o.shapes.first_mut() {
                        *stroke = Some((c, 10.));
                    }
                }
            }
        }
        scene
    }
    fn drag(&self, s: &PermState, id: &str, _: Modifiers) -> Option<DragSpec<'static, PermState>> {
        let clean;
        let s = if s.closed.is_some() {
            clean = {
                let mut t = s.clone();
                t.closed = None;
                t
            };
            &clean
        } else {
            s
        };

        let p = permutation(s.n, &s.word);
        if let Some(mask) = id.strip_prefix("face-").and_then(|s| s.parse::<u8>().ok()) {
            let face = poly_faces().iter().find(|f| f.mask == mask)?.clone();
            let min_facing = face_facing(s, &face).min(0.1);
            let mut base = s.clone();
            base.orientation = view_rotation(s);
            base.spin = [0.; 2];
            return Some(
                dragology::vary_constrained(
                    [0.; 2],
                    move |spin| PermState {
                        spin,
                        ..base.clone()
                    },
                    move |state| [min_facing - face_facing(state, &face)],
                )
                .on_drop(|state| {
                    let mut t = state.clone();
                    t.orientation = view_rotation(state);
                    t.spin = [0.; 2];
                    t
                }),
            );
        }
        if let Some(j) = id
            .strip_prefix("walk-braid-")
            .and_then(|s| s.parse::<usize>().ok())
        {
            let word = braid_move(s.n, &s.word, j.checked_sub(1)?, false)?;
            let from = PermState {
                by_column: true,
                ..s.clone()
            };
            return Some(between([from.clone(), PermState { word, ..from }]));
        }
        if let Some(j) = id
            .strip_prefix("walk-corner-")
            .and_then(|s| s.parse::<usize>().ok())
        {
            let start = j.checked_sub(1)?;
            let a = s.word.get(start)?;
            let b = s.word.get(j)?;
            if a.i.abs_diff(b.i) >= 2 {
                let mut t = s.clone();
                t.word.swap(start, j);
                return Some(between([s.clone(), t]));
            }
            if a.i == b.i {
                return Some(Self::cancel_spec(s, start));
            }
        }
        if id == "walk" {
            if !Self::appendable(s, 1) {
                return Some(fixed(s.clone()));
            }
            let mut states = vec![s.clone()];
            for i in 0..s.n - 1 {
                let mut n = s.clone();
                n.word.push(Letter {
                    key: n.next,
                    i,
                    sign: 1,
                });
                n.next += 1;
                states.push(n);
            }
            return Some(
                dragology::closest_with(
                    states
                        .into_iter()
                        .skip(1)
                        .map(|to| between([s.clone(), to])),
                    dragology::ClosestOptions {
                        lock_past: Some(10.),
                        ..Default::default()
                    },
                )
                .with_snap_radius_options(
                    3.,
                    dragology::SnapOptions {
                        chain: true,
                        ..Default::default()
                    },
                ),
            );
        }
        if let Some(value) = id
            .strip_prefix("wire-end-")
            .and_then(|v| v.parse::<usize>().ok())
        {
            let row = p.iter().position(|v| *v == value)?;
            if !Self::appendable(s, 1) {
                return Some(fixed(s.clone()));
            }
            let specs = [row.checked_sub(1), (row + 1 < s.n).then_some(row)]
                .into_iter()
                .flatten()
                .map(|i| {
                    let mut t = s.clone();
                    t.word.push(Letter {
                        key: t.next,
                        i,
                        sign: 1,
                    });
                    t.next += 1;
                    between([s.clone(), t])
                });
            return Some(closest(specs).with_snap_radius_options(
                3.,
                dragology::SnapOptions {
                    chain: true,
                    ..Default::default()
                },
            ));
        }
        if let Some(value) = id
            .strip_prefix("tile-")
            .and_then(|x| x.parse::<usize>().ok())
        {
            let index = p.iter().position(|&v| v == value)?;
            if s.adjacent && !Self::appendable(s, 1) {
                return Some(fixed(s.clone()));
            }
            let mut states = vec![s.clone()];
            for dst in 0..s.n {
                if dst == index || s.adjacent && index.abs_diff(dst) != 1 {
                    continue;
                }
                let mut n = s.clone();
                if s.adjacent {
                    n.word.push(Letter {
                        key: n.next,
                        i: index.min(dst),
                        sign: 1,
                    });
                    n.next += 1;
                } else {
                    let mut q = p.clone();
                    let val = q.remove(index);
                    q.insert(dst, val);
                    n.word = reduced_word(&q);
                    n.next = n.word.len();
                }
                states.push(n);
            }
            return Some(if s.adjacent {
                closest(states.into_iter().skip(1).map(|t| between([s.clone(), t])))
                    .with_snap_radius_options(
                        3.,
                        dragology::SnapOptions {
                            chain: true,
                            ..Default::default()
                        },
                    )
            } else {
                closest(states)
                    .when_far(fixed(s.clone()), 50.)
                    .with_floating()
            });
        }
        if let Some(value) = id
            .strip_prefix("matrix-")
            .and_then(|v| v.parse::<usize>().ok())
        {
            let row = p.iter().position(|v| *v == value)?;
            let states = (0..s.n).map(|to| {
                let mut q = p.clone();
                q.swap(row, to);
                let word = reduced_word(&q);
                PermState {
                    next: word.len(),
                    word,
                    ..s.clone()
                }
            });
            return Some(between(states));
        }
        if let Some(value) = id
            .strip_prefix("cycle-")
            .and_then(|x| x.parse::<usize>().ok())
        {
            let mut rest = cycle_list(&p);
            for c in &mut rest {
                c.retain(|&v| v != value);
            }
            rest.retain(|c| !c.is_empty());
            let mut states = vec![s.clone()];
            for c in 0..=rest.len() {
                let slots = if c == rest.len() { 1 } else { rest[c].len() };
                for slot in 0..slots {
                    let mut cycles = rest.clone();
                    if c == cycles.len() {
                        cycles.push(vec![value]);
                    } else {
                        cycles[c].insert(slot + 1, value);
                    }
                    let word = reduced_word(&from_cycles(s.n, &cycles));
                    let n = PermState {
                        next: word.len(),
                        word,
                        ..s.clone()
                    };
                    if !states.contains(&n) {
                        states.push(n);
                    }
                }
            }
            return Some(closest(states).with_floating());
        }
        if let Some(key) = id
            .strip_prefix("word-")
            .and_then(|x| x.parse::<usize>().ok())
        {
            let j = s.word.iter().position(|l| l.key == key)?;
            let mut specs = Vec::new();
            for k in [j.checked_sub(1), (j + 1 < s.word.len()).then_some(j + 1)]
                .into_iter()
                .flatten()
            {
                if s.word[j].i.abs_diff(s.word[k].i) >= 2 {
                    let mut n = s.clone();
                    n.word.swap(j, k);
                    specs.push(between([s.clone(), n]));
                }
            }
            if j > 0 {
                if let Some(word) = braid_move(s.n, &s.word, j - 1, false) {
                    let from = PermState {
                        by_column: true,
                        ..s.clone()
                    };
                    specs.push(between([from.clone(), PermState { word, ..from }]));
                }
            }
            return Some(if specs.is_empty() {
                fixed(s.clone())
            } else {
                closest(specs).with_snap_radius_options(
                    3.,
                    dragology::SnapOptions {
                        chain: true,
                        ..Default::default()
                    },
                )
            });
        }
        if let Some(rest) = id.strip_prefix("insert-") {
            let (col, strand) = rest.split_once('-')?;
            let col = col.parse::<usize>().ok()?;
            let strand = strand.parse::<usize>().ok()?;
            let row = permutation(s.n, &s.word[..col])
                .iter()
                .position(|v| *v == strand)?;
            if let (Some(a), Some(b)) = (s.word.get(col), s.word.get(col + 1)) {
                if a.i == b.i && (row == a.i || row == a.i + 1) {
                    return Some(Self::cancel_spec(s, col));
                }
            }
            let mut braid_side = None;
            let mut braid_spec = None;
            if col > 0 {
                if let Some(word) = braid_move(s.n, &s.word, col - 1, false) {
                    let a = s.word[col - 1].i;
                    let b = s.word[col].i;
                    if (row == a || row == a + 1) && row != b && row != b + 1 {
                        braid_side = Some(if row < b { row + 1 } else { row - 1 });
                        let from = PermState {
                            by_column: true,
                            ..s.clone()
                        };
                        braid_spec = Some(between([from.clone(), PermState { word, ..from }]));
                    }
                }
            }
            let mut specs = Vec::new();
            for other in [row.checked_sub(1), (row + 1 < s.n).then_some(row + 1)]
                .into_iter()
                .flatten()
            {
                if braid_side == Some(other) {
                    specs.extend(braid_spec.take());
                } else if Self::appendable(s, 2) {
                    specs.push(Self::pair_spec(s, col, row.min(other)));
                }
            }
            return Some(if specs.is_empty() {
                fixed(s.clone())
            } else {
                closest(specs)
            });
        }
        None
    }
    fn locate(&self, s: &PermState, id: &str) -> Option<Affine> {
        if let Some(mask) = id.strip_prefix("face-").and_then(|s| s.parse::<u8>().ok()) {
            let face = poly_faces().iter().find(|f| f.mask == mask)?;
            return Some(Affine::translate(face_center(s, face).to_vec2()));
        }
        self.scene(s).object(id).map(|o| o.transform)
    }
    fn controls(&self, s: &PermState) -> Vec<Control> {
        vec![
            Control::Choice {
                id: "n",
                label: "n".into(),
                options: (3..=6).map(|n| n.to_string()).collect(),
                selected: s.n - 3,
            },
            Control::Choice {
                id: "mode",
                label: "Tile drags".into(),
                options: vec![
                    "free reorder".into(),
                    "adjacent transpositions only (chained)".into(),
                ],
                selected: usize::from(s.adjacent),
            },
            Control::Choice {
                id: "projection",
                label: "Permutohedron (n = 4)".into(),
                options: vec![
                    "solid (drag a face to turn it)".into(),
                    "Schlegel diagram".into(),
                ],
                selected: usize::from(!s.solid),
            },
            Control::button("source-reset", "reset"),
        ]
    }
    fn action_transition(&self, _: &PermState, _: &str) -> Option<dragology::Transition> {
        None
    }
    fn action(&self, s: &PermState, id: &str, v: Value) -> Option<PermState> {
        let mut n = s.clone();
        match (id, v) {
            ("n", Value::Index(i)) => {
                n.n = (i + 3).clamp(3, 6);
                n.word = if s.reset_count == 0 {
                    reduced_word(&Self::source_perm(n.n))
                } else {
                    vec![]
                };
                n.next = n.word.len();
                n.orientation = self.initial().orientation;
                n.spin = [0.; 2];
                n.motion = None;
                n.closed = None;
                n.by_column = false;
            }
            ("mode", Value::Index(i)) => n.adjacent = i == 1,
            ("projection", Value::Index(i)) => n.solid = i == 0,
            ("source-reset", _) => {
                n.word.clear();
                n.next = 0;
                n.reset_count += 1;
                n.orientation = self.initial().orientation;
                n.spin = [0.; 2];
                n.motion = None;
                n.closed = None;
                n.by_column = false;
            }
            _ => return None,
        }
        Some(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn release_braid(state: BraidState, id: &str, target: Point) -> BraidState {
        let example = BraidDemo { old: false };
        let start = example.locate(&state, id).unwrap() * Point::ORIGIN;
        let spec = example.drag(&state, id, Modifiers::default()).unwrap();
        let key = id.to_owned();
        dragology::DragSession::begin(
            state,
            start,
            move |s| BraidDemo { old: false }.locate(s, &key),
            spec,
        )
        .unwrap()
        .finish(target)
        .unwrap()
    }
    #[test]
    fn cross_section_turns_commit_the_selected_over_under_sign() {
        let example = BraidDemo { old: false };
        let state = BraidDemo::preset(3, &[], false);
        for front in [false, true] {
            let start = example.project_slice(&state, 0., 0.).0;
            let end = example.project_slice(&state, 1., 0.).0;
            let mut session = dragology::DragSession::begin(
                state.clone(),
                start,
                |s| BraidDemo { old: false }.locate(s, "slice-0"),
                example
                    .drag(&state, "slice-0", Modifiers::default())
                    .unwrap(),
            )
            .unwrap();
            let side = if front { 4. } else { -4. };
            session
                .frame(Point::new(start.x + side, (start.y + end.y) * 0.5))
                .unwrap();
            let done = session.finish(Point::new(end.x + side, end.y)).unwrap();
            assert_eq!(done.word.len(), 1);
            assert_eq!(done.word[0].i, 0);
            assert_eq!(done.word[0].sign, if front { 1 } else { -1 });
            assert!(done.motion.is_none());
        }
    }
    #[test]
    fn half_turns_can_be_canceled_and_have_continuous_geometry() {
        let example = BraidDemo { old: false };
        let state = BraidDemo::preset(3, &[], false);
        let start = example.project_slice(&state, 0., 0.).0;
        let spec = example.turn_spec(&state, 0, 1, true);
        let mut session = dragology::DragSession::begin(
            state.clone(),
            start,
            |s| BraidDemo { old: false }.locate(s, "slice-0"),
            spec,
        )
        .unwrap();
        let frame = session.frame(start + Vec2::new(4., 2.)).unwrap();
        assert_eq!(frame.drop_state.word.len(), 0);
        assert_eq!(session.finish(start).unwrap().word.len(), 0);
    }
    #[test]
    fn both_markov_stabilizations_have_their_corresponding_inverse() {
        let example = BraidDemo { old: false };
        for at_end in [false, true] {
            let original = BraidDemo::preset(2, &[1, 1], true);
            let id = if at_end { "curl-right" } else { "curl-left" };
            let initial = example.locate(&original, id).unwrap() * Point::ORIGIN;
            let grown = release_braid(original.clone(), id, initial + Vec2::new(0., 28.));
            assert_eq!(grown.n, 3);
            assert_eq!(grown.word.len(), 3);
            let added = if at_end {
                grown.word.last().unwrap()
            } else {
                &grown.word[0]
            };
            assert_eq!(added.i, 1);
            assert_eq!(added.sign, if at_end { 1 } else { -1 });
            assert_eq!(
                cycle_list(&permutation(grown.n, &grown.word)).len(),
                cycle_list(&permutation(original.n, &original.word)).len()
            );
            let loop_point = example.locate(&grown, "destabilize").unwrap() * Point::ORIGIN;
            let back = release_braid(grown, "destabilize", loop_point - Vec2::new(0., 28.));
            assert_eq!(back.n, original.n);
            assert_eq!(back.word, original.word);
        }
    }
    #[test]
    fn projected_cross_section_turns_chain_with_the_pointer_held_to_the_side() {
        let example = BraidDemo { old: false };
        let state = example.initial();
        let order = permutation(state.n, &state.word);
        let id = format!("slice-{}", order[0]);
        let start = example.locate(&state, &id).unwrap() * Point::ORIGIN;
        let one = example.project_slice(&state, 1., 0.).0;
        let two = example.project_slice(&state, 2., 0.).0;
        let mut runtime = crate::runtime::ModelDemo::new(example);
        runtime.set_animated(false);
        runtime.press(start, Modifiers::default(), 1);
        runtime.move_to(Point::new(one.x + 80., one.y), Modifiers::default());
        runtime.advance(std::time::Duration::ZERO);
        runtime.move_to(Point::new(two.x + 80., two.y), Modifiers::default());
        runtime.advance(std::time::Duration::ZERO);
        runtime.release(Point::new(two.x + 80., two.y), Modifiers::default());
        assert!(
            runtime
                .status()
                .contains(&format!("{} crossings", state.word.len() + 2)),
            "{}",
            runtime.status()
        );
    }
    #[test]
    fn source_chaining_uses_snapped_preview_geometry_outside_the_pointer_hull() {
        let example = PermutationDemo;
        let state = example.initial();
        let value = permutation(state.n, &state.word)[0];
        let id = format!("wire-end-{value}");
        let start = example.locate(&state, &id).unwrap() * Point::ORIGIN;
        let outside = start + Vec2::new(1000., 24.);
        let dragged = id.clone();
        let mut session = dragology::DragSession::begin(
            state.clone(),
            start,
            move |s| PermutationDemo.locate(s, &dragged),
            example.drag(&state, &id, Modifiers::default()).unwrap(),
        )
        .unwrap();
        let frame = session.frame(outside).unwrap();
        assert!(
            frame.gap > 900.,
            "the pointer is deliberately far outside the segment hull"
        );
        let chain = frame
            .chain
            .as_ref()
            .expect("the preview itself reached its drop state");
        assert!(
            chain.dragged_id.is_none(),
            "the source continues the same ID"
        );
        assert!(
            chain.transition.is_none(),
            "the source enables chain without a transition"
        );
        assert_eq!(frame.drop_state.word.len(), state.word.len() + 1);

        let mut runtime = crate::runtime::ModelDemo::new(PermutationDemo);
        runtime.set_animated(false);
        runtime.press(start, Modifiers::default(), 1);
        runtime.move_to(outside, Modifiers::default());
        runtime.advance(std::time::Duration::ZERO);
        let second = outside + Vec2::new(0., 24.);
        runtime.move_to(second, Modifiers::default());
        runtime.advance(std::time::Duration::ZERO);
        runtime.release(second, Modifiers::default());
        let end = runtime.scene().object(&id).unwrap().transform * Point::ORIGIN;
        assert_eq!(end, start + Vec2::new(0., 48.));
        assert_eq!(
            runtime
                .scene()
                .objects
                .iter()
                .filter(|o| o.id.starts_with("letter-"))
                .count(),
            state.word.len() + 2
        );

        // The old vertical braid uses the same metadata with a 1px radius.
        // Its finite triangle also snaps at an endpoint when the pointer is far away.
        let old = BraidDemo { old: true };
        let old_state = old.initial();
        let start = old.locate(&old_state, "end-0").unwrap() * Point::ORIGIN;
        let spec = old.drag(&old_state, "end-0", Modifiers::default()).unwrap();
        let frame = dragology::DragSession::begin(
            old_state,
            start,
            |s| BraidDemo { old: true }.locate(s, "end-0"),
            spec,
        )
        .unwrap()
        .frame(Point::new(50., 1000.))
        .unwrap();
        assert!(frame.gap > 800. && frame.chain.is_some());
        assert_eq!(frame.drop_state.word.len(), 1);
    }
    #[test]
    fn matrix_gesture_swaps_rows_without_reordering_intermediate_values() {
        let example = PermutationDemo;
        let state = example.initial();
        let before = permutation(state.n, &state.word);
        let value = before[0];
        let id = format!("matrix-{value}");
        let start = example.locate(&state, &id).unwrap() * Point::ORIGIN;
        let spec = example.drag(&state, &id, Modifiers::default()).unwrap();
        let out = dragology::DragSession::begin(
            state,
            start,
            move |s| PermutationDemo.locate(s, &id),
            spec,
        )
        .unwrap()
        .finish(start + Vec2::new(0., 84.))
        .unwrap();
        let mut expected = before;
        expected.swap(0, 3);
        assert_eq!(permutation(out.n, &out.word), expected);
    }
    #[test]
    fn closed_conjugation_follows_the_loop_continuously_and_commits_at_halfway() {
        let example = BraidDemo { old: false };
        let state = BraidDemo::preset(3, &[1, -2, 1], true);
        let key = state.word.last().unwrap().key;
        let id = format!("cross-{key}");
        let start = example.locate(&state, &id).unwrap() * Point::ORIGIN;
        let spec = example.orbit_spec(&state, key, false);
        let key_id = id.clone();
        let mut session = dragology::DragSession::begin(
            state.clone(),
            start,
            move |s| BraidDemo { old: false }.locate(s, &key_id),
            spec,
        )
        .unwrap();
        for step in 0..=50 {
            let progress = step as f64 / 50.;
            let mut target = state.clone();
            target.motion = Some(BraidMotion::Orbit { progress, key });
            let p = example.cross_point(&target, key).unwrap();
            let frame = session.frame(p).unwrap();
            assert!(
                frame.grab_position.unwrap().distance(p) < 0.2,
                "step {step}"
            );
            if progress < 0.48 {
                assert_eq!(frame.drop_state.word, state.word);
            }
            if progress > 0.52 {
                assert_eq!(frame.drop_state.word[0].key, key);
            }
        }
        let mut before = state.clone();
        before.motion = Some(BraidMotion::Orbit {
            progress: 0.5 - 1e-6,
            key,
        });
        let mut after = before.clone();
        after.motion = Some(BraidMotion::Orbit {
            progress: 0.5 + 1e-6,
            key,
        });
        assert!(
            example
                .cross_point(&before, key)
                .unwrap()
                .distance(example.cross_point(&after, key).unwrap())
                < 0.1
        );
        let mut end = state.clone();
        end.motion = Some(BraidMotion::Orbit { progress: 1., key });
        let out = session
            .finish(example.cross_point(&end, key).unwrap())
            .unwrap();
        assert_eq!(out.word[0].key, key);
        assert!(out.motion.is_none());
        assert_eq!(
            cycle_list(&permutation(out.n, &out.word)).len(),
            cycle_list(&permutation(state.n, &state.word)).len()
        );
        let inverse = example.orbit_spec(&out, key, true);
        let start = example.cross_point(&out, key).unwrap();
        let id = format!("cross-{key}");
        let mut backward = dragology::DragSession::begin(
            out,
            start,
            move |s| BraidDemo { old: false }.locate(s, &id),
            inverse,
        )
        .unwrap();
        for step in (0..=50).rev() {
            let mut target = state.clone();
            target.motion = Some(BraidMotion::Orbit {
                progress: step as f64 / 50.,
                key,
            });
            let point = example.cross_point(&target, key).unwrap();
            assert!(
                backward
                    .frame(point)
                    .unwrap()
                    .grab_position
                    .unwrap()
                    .distance(point)
                    < 0.2
            );
        }
        let undo = backward
            .finish(example.cross_point(&state, key).unwrap())
            .unwrap();
        assert_eq!(undo.word, state.word);
    }
    #[test]
    fn inserting_and_canceling_inverse_pairs_preview_the_entire_diagram() {
        struct Fixture;
        impl Example for Fixture {
            type State = BraidState;
            fn initial(&self) -> BraidState {
                BraidDemo::preset(3, &[1], false)
            }
            fn scene(&self, s: &BraidState) -> Scene {
                BraidDemo { old: false }.scene(s)
            }
            fn drag(
                &self,
                s: &BraidState,
                id: &str,
                m: Modifiers,
            ) -> Option<DragSpec<'static, BraidState>> {
                BraidDemo { old: false }.drag(s, id, m)
            }
            fn status(&self, s: &BraidState) -> String {
                BraidDemo { old: false }.status(s)
            }
        }
        let ex = BraidDemo { old: false };
        let state = Fixture.initial();
        let initial = ex.scene(&state);
        let start = initial.object("segment-0-2").unwrap().transform
            * Point::new(-geometry::cw(&state) / 2., 0.);
        let cross_start = initial.object("cross-0").unwrap().transform * Point::ORIGIN;
        let mut runtime = crate::runtime::ModelDemo::new(Fixture);
        runtime.set_animated(false);
        runtime.press(start, Modifiers::default(), 1);
        runtime.move_to(start - Vec2::new(0., 14.), Modifiers::default());
        runtime.advance(std::time::Duration::ZERO);
        let mid = runtime.scene().object("cross-0").unwrap().transform * Point::ORIGIN;
        assert!(
            (mid.x - cross_start.x - geometry::cw(&state)).abs() < 0.1,
            "{mid:?} {cross_start:?}"
        );
        runtime.release(start - Vec2::new(0., 28.), Modifiers::default());
        assert!(runtime.status().contains("3 crossings"));
        let apex = runtime.scene().object("segment-0-2").unwrap().transform * Point::ORIGIN;
        let shifted = runtime.scene().object("cross-0").unwrap().transform * Point::ORIGIN;
        runtime.press(apex, Modifiers::default(), 1);
        runtime.move_to(apex + Vec2::new(0., 14.), Modifiers::default());
        runtime.advance(std::time::Duration::ZERO);
        let mid = runtime.scene().object("cross-0").unwrap().transform * Point::ORIGIN;
        assert!(
            (mid.x - shifted.x).abs() < 0.1,
            "right-hand columns must stay fixed while flattening"
        );
        runtime.release(apex + Vec2::new(0., 28.), Modifiers::default());
        assert!(runtime.status().contains("1 crossings"));
        let restored = runtime.scene().object("cross-0").unwrap().transform * Point::ORIGIN;
        assert!(restored.distance(cross_start) < 0.1);
    }
    #[test]
    fn schlegel_projection_has_one_outer_hexagon_and_no_crossing_edges() {
        let all = perms(vec![0, 1, 2, 3]);
        let outer = all
            .iter()
            .filter(|p| projected_permutation(p).to_vec2().hypot() > 0.999999)
            .collect::<Vec<_>>();
        assert_eq!(outer.len(), 6);
        assert!(outer.iter().all(|p| p[3] == 3));
        let mut edges = Vec::new();
        for p in &all {
            for i in 0..3 {
                let mut q = p.clone();
                q.swap(i, i + 1);
                if p < &q {
                    edges.push((p.clone(), q));
                }
            }
        }
        assert_eq!(edges.len(), 36);
        for (i, (a, b)) in edges.iter().enumerate() {
            for (c, d) in edges.iter().skip(i + 1) {
                if [a, b].contains(&c) || [a, b].contains(&d) {
                    continue;
                }
                let (p, q, r, t) = (
                    projected_permutation(a),
                    projected_permutation(b),
                    projected_permutation(c),
                    projected_permutation(d),
                );
                let cross = |a: Point, b: Point, c: Point| (b - a).cross(c - a);
                assert!(
                    !(cross(p, q, r) * cross(p, q, t) < -1e-12
                        && cross(r, t, p) * cross(r, t, q) < -1e-12)
                );
            }
        }
    }
    fn permutation_fixture(generators: &[usize]) -> PermState {
        let mut s = PermutationDemo.initial();
        s.word = generators
            .iter()
            .enumerate()
            .map(|(key, &i)| Letter { key, i, sign: 1 })
            .collect();
        s.next = s.word.len();
        s
    }
    fn release_perm(s: PermState, id: &str, target: Point) -> PermState {
        let start = PermutationDemo.locate(&s, id).unwrap() * Point::ORIGIN;
        let spec = PermutationDemo.drag(&s, id, Modifiers::default()).unwrap();
        let id = id.to_owned();
        dragology::DragSession::begin(s, start, move |s| PermutationDemo.locate(s, &id), spec)
            .unwrap()
            .finish(target)
            .unwrap()
    }
    #[test]
    fn permutation_walk_corner_uses_the_logical_vertex_across_commutation_and_cancellation() {
        let s = permutation_fixture(&[0, 2]);
        let mut target = s.clone();
        target.word.swap(0, 1);
        let p = PermutationDemo.locate(&target, "walk-corner-1").unwrap() * Point::ORIGIN;
        let out = release_perm(s.clone(), "walk-corner-1", p);
        assert_eq!(out.word, target.word);
        assert_eq!(permutation(out.n, &out.word), permutation(s.n, &s.word));
        let s = permutation_fixture(&[0, 0]);
        let target = perm_vertex(&(0..s.n).collect::<Vec<_>>(), &s);
        assert!(release_perm(s, "walk-corner-1", target).word.is_empty());
    }
    #[test]
    fn permutation_pair_insert_and_cancel_are_continuous_and_keep_the_permutation() {
        let s = permutation_fixture(&[0]);
        let id = "insert-0-3";
        let spec = PermutationDemo.drag(&s, id, Modifiers::default()).unwrap();
        let start = PermutationDemo.locate(&s, id).unwrap() * Point::ORIGIN;
        let mut session = dragology::DragSession::begin(
            s.clone(),
            start,
            move |s| PermutationDemo.locate(s, id),
            spec,
        )
        .unwrap();
        let frame = session.frame(start - Vec2::new(0., 12.)).unwrap();
        assert!(matches!(
            frame.preview.as_ref(),
            dragology::Preview::Blend(_)
        ));
        assert!(
            frame
                .grab_position
                .unwrap()
                .distance(start - Vec2::new(0., 12.))
                < 0.01
        );
        let out = session.finish(start - Vec2::new(0., 24.)).unwrap();
        assert_eq!(out.word.len(), 3);
        assert_eq!(permutation(4, &out.word), permutation(4, &s.word));
        let target = PermutationDemo.locate(&s, id).unwrap() * Point::ORIGIN;
        let back = release_perm(out, id, target);
        assert_eq!(back.word, s.word);
    }
    #[test]
    fn source_controls_restore_configured_start_then_reset_to_identity() {
        let example = PermutationDemo;
        let s = example.initial();
        let n3 = example.action(&s, "n", Value::Index(0)).unwrap();
        assert_eq!(permutation(3, &n3.word), vec![1, 2, 0]);
        let reset = example.action(&n3, "source-reset", Value::Trigger).unwrap();
        assert!(reset.word.is_empty());
        let n6 = example.action(&reset, "n", Value::Index(3)).unwrap();
        assert_eq!(n6.n, 6);
        assert!(n6.word.is_empty());
        let braid = BraidDemo { old: false };
        let s = braid.initial();
        let mut edited = s.clone();
        edited.word.clear();
        let closed = braid.action(&edited, "closure", Value::Bool(true)).unwrap();
        assert_eq!(closed.word, s.word);
        let reset = braid
            .action(&closed, "source-reset", Value::Trigger)
            .unwrap();
        assert!(reset.word.is_empty());
        let chosen = braid.action(&reset, "preset", Value::Index(3)).unwrap();
        let chosen_reset = braid
            .action(&chosen, "source-reset", Value::Trigger)
            .unwrap();
        assert_eq!(chosen.word, chosen_reset.word);
    }
    #[test]
    fn permutation_capacity_prevents_all_appending_gestures_at_the_source_limit() {
        let mut s = permutation_fixture(&vec![0; 12]);
        s.adjacent = true;
        for id in ["tile-0", "wire-end-0", "walk"] {
            let p = PermutationDemo.locate(&s, id).unwrap() * Point::ORIGIN;
            assert_eq!(
                release_perm(s.clone(), id, p + Vec2::new(100., 100.)).word,
                s.word
            );
        }
        assert!(PermutationDemo.scene(&s).object("insert-12-3").is_none());
    }
    #[test]
    fn old_braid_has_a_fixed_initial_segment_and_source_crossing_geometry() {
        let e = BraidDemo { old: true };
        let mut s = e.initial();
        s.word.push(Letter {
            key: 0,
            i: 0,
            sign: 1,
        });
        let scene = e.scene(&s);
        assert_eq!((scene.width, scene.height), (250., 400.));
        assert_eq!(
            scene.object("cross-0").unwrap().transform * Point::ORIGIN,
            Point::new(25., 75.)
        );
        assert_eq!(
            scene.object("end-0").unwrap().transform * Point::ORIGIN,
            Point::new(50., 100.)
        );
        assert!(e.drag(&s, "cross-0", Modifiers::default()).is_none());
        assert_eq!(
            e.click(&s, "cross-0", Point::ORIGIN, Modifiers::default(), 1)
                .unwrap()
                .word[0]
                .sign,
            -1
        );
    }
    #[test]
    fn braid_hover_slice_follows_the_pointer_along_the_full_padded_diagram() {
        let e = BraidDemo { old: false };
        let mut s = BraidDemo::preset(3, &[1], false);
        s.slice = true;
        for x in [
            geometry::x0(&s) - 10.,
            geometry::x0(&s) + 8. * geometry::cw(&s),
        ] {
            let hovered = e.hover(&s, Point::new(x, 52.)).unwrap();
            let scene = e.scene(&hovered);
            for value in 0..s.n {
                let dot = scene.object(&format!("moment-point-{value}")).unwrap();
                assert!(((dot.transform * Point::ORIGIN).x - x).abs() < 1e-6);
            }
            // Hovering the straight tail changes the cursor, not the end state.
            if x > geometry::x0(&s) {
                assert_eq!(e.configuration(&hovered), e.configuration(&s));
            }
        }
    }
    #[test]
    fn braid_relations_preserve_strand_permutation_and_pair_overpasses() {
        let s = BraidDemo::preset(3, &[1, 2, 1], false);
        let moved = braid_move(s.n, &s.word, 0, true).unwrap();
        assert_eq!(permutation(3, &s.word), permutation(3, &moved));
        let invariant = |w: &[Letter]| {
            crossings(3, w)
                .into_iter()
                .map(|(a, b, o)| (pair(a, b), o))
                .collect::<BTreeMap<_, _>>()
        };
        assert_eq!(invariant(&s.word), invariant(&moved));
        assert!(braid_move(3, &BraidDemo::preset(3, &[1, -2, 1], false).word, 0, true).is_none());
    }
    #[test]
    fn permutations_round_trip_reduced_words_and_cycles() {
        for p in perms((0..5).collect()) {
            let w = reduced_word(&p);
            assert_eq!(w.len(), inversions(&p));
            assert_eq!(permutation(5, &w), p);
            assert_eq!(from_cycles(5, &cycle_list(&p)), p);
        }
    }
}
