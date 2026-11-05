//! Source-backed path-length tracks and strand pieces for the braid demo.
//! The piece structure is shared by both ends of a rewrite, including collapsed
//! columns. This lets the renderer interpolate the whole predicted diagram.
use super::*;
#[derive(Clone)]
enum Segment {
    Line {
        a: Point,
        b: Point,
        len: f64,
    },
    Arc {
        center: Point,
        r: f64,
        a0: f64,
        a1: f64,
        len: f64,
    },
}
impl Segment {
    fn len(&self) -> f64 {
        match self {
            Self::Line { len, .. } | Self::Arc { len, .. } => *len,
        }
    }
    fn at(&self, d: f64) -> Point {
        let t = if self.len() > 0. { d / self.len() } else { 0. };
        match self {
            Self::Line { a, b, .. } => a.lerp(*b, t),
            Self::Arc {
                center, r, a0, a1, ..
            } => {
                let a = a0 + (a1 - a0) * t;
                *center + Vec2::new(a.cos() * r, a.sin() * r)
            }
        }
    }
}
#[derive(Clone)]
pub(super) struct Track {
    segments: Vec<Segment>,
    start: Point,
    pub length: f64,
}
impl Track {
    fn new(start: Point) -> Self {
        Self {
            segments: Vec::new(),
            start,
            length: 0.,
        }
    }
    fn end(&self) -> Point {
        self.segments.last().map_or(self.start, |s| s.at(s.len()))
    }
    fn line(mut self, b: Point) -> Self {
        let a = self.end();
        let len = a.distance(b);
        self.segments.push(Segment::Line { a, b, len });
        self.length += len;
        self
    }
    fn arc(mut self, center: Point, r: f64, a0: f64, a1: f64) -> Self {
        let len = r * (a1 - a0).abs();
        self.segments.push(Segment::Arc {
            center,
            r,
            a0,
            a1,
            len,
        });
        self.length += len;
        self
    }
    pub fn at(&self, d: f64) -> Point {
        let mut rest = d.clamp(0., self.length);
        for s in &self.segments {
            if rest <= s.len() {
                return s.at(rest);
            }
            rest -= s.len();
        }
        self.end()
    }
    fn points(&self, d0: f64, d1: f64) -> Vec<Point> {
        let mut pts = vec![self.at(d0)];
        let mut start = 0.;
        for s in &self.segments {
            let lo = d0.max(start);
            let hi = d1.min(start + s.len());
            if hi > lo && matches!(s, Segment::Arc { .. }) {
                let steps = ((hi - lo) / 4.).ceil() as usize;
                for k in 1..steps {
                    pts.push(s.at(lo - start + (hi - lo) * k as f64 / steps as f64));
                }
            }
            if start + s.len() > lo && start + s.len() < d1 {
                pts.push(s.at(s.len()));
            }
            start += s.len();
        }
        pts.push(self.at(d1));
        pts
    }
}
pub(super) fn x0(s: &BraidState) -> f64 {
    if s.closure { 82. } else { 48. }
}
pub(super) fn cw(s: &BraidState) -> f64 {
    let columns = if s.closure { 14 } else { 12.max(2 * s.n + 4) };
    (680. - x0(s) - if s.closure { 90. } else { 44. } - 100. - 16.) / columns as f64
}
pub(super) fn row_y(s: &BraidState, row: f64) -> f64 {
    let row = if let Some(BraidMotion::Grow { progress, .. }) = s.motion {
        let before = s.n as f64 - 2.;
        row.min(before) + (row - before).max(0.) * progress
    } else {
        row
    };
    52. + 28. * row
}
pub(super) fn bottom_y(s: &BraidState) -> f64 {
    row_y(s, s.n as f64 - 1.) + 18.
}
fn loop_track(
    d: &BraidDemo,
    s: &BraidState,
    row: f64,
    start: f64,
    end: f64,
    end_row: f64,
) -> Track {
    let grow = if let Some(BraidMotion::Grow { progress, .. }) = s.motion {
        progress
    } else {
        1.
    };
    let l = s.n as f64 - 1. - row;
    let off = if l <= 1. { l * grow } else { l - 1. + grow } * 12.;
    let y = row_y(s, row);
    let ye = row_y(s, end_row);
    let xr = x0(s) + d.capacity(s) as f64 * cw(s) + 14. + off;
    let xl = x0(s) - 14. - off;
    let yb = bottom_y(s) + off;
    let rb = 8. + off;
    let half = std::f64::consts::FRAC_PI_2;
    Track::new(Point::new(start, y))
        .line(Point::new(xr - 8., y))
        .arc(Point::new(xr - 8., y + 8.), 8., -half, 0.)
        .line(Point::new(xr, yb - rb))
        .arc(Point::new(xr - rb, yb - rb), rb, 0., half)
        .line(Point::new(xl + rb, yb))
        .arc(Point::new(xl + rb, yb - rb), rb, half, std::f64::consts::PI)
        .line(Point::new(xl, ye + 8.))
        .arc(
            Point::new(xl + 8., ye + 8.),
            8.,
            std::f64::consts::PI,
            3. * half,
        )
        .line(Point::new(end, ye))
}
pub(super) struct Orbit {
    pub into: bool,
    pub j: usize,
    pub i: usize,
    pub shift: f64,
    tracks: [Track; 2],
    distances: [f64; 2],
    pub center: Point,
    seam: f64,
    width: f64,
}
impl Orbit {
    fn crossing_points(&self, k: usize) -> Vec<Point> {
        (0..=16)
            .map(|q| {
                let t = q as f64 / 16.;
                let f = 1.5 * t * (1. - t) + t * t * t;
                let u = 3. * t * t - 2. * t * t * t;
                self.tracks[k].at(self.distances[k] + f * self.width).lerp(
                    self.tracks[1 - k].at(self.distances[1 - k] + f * self.width),
                    u,
                )
            })
            .collect()
    }
}
pub(super) fn theta_at(f: f64) -> f64 {
    let (mut lo, mut hi) = (0., 1.);
    for _ in 0..30 {
        let t = (lo + hi) * 0.5;
        if 1.5 * t * (1. - t) + t * t * t < f {
            lo = t;
        } else {
            hi = t;
        }
    }
    let t = (lo + hi) * 0.5;
    let q = 3. * t * t - 2. * t * t * t;
    (1. - 2. * q).clamp(-1., 1.).acos()
}
pub(super) fn end_fraction(d: &BraidDemo, s: &BraidState) -> Option<f64> {
    let o = orbit(d, s)?;
    if o.into {
        None
    } else {
        Some(((o.seam - (o.distances[0] + o.distances[1]) * 0.5) / cw(s)).clamp(0., 1.))
    }
}
pub(super) fn displayed(s: &BraidState) -> BraidState {
    let mut state = s.clone();
    if let Some((start, letters)) = state.closed.take() {
        state.word.splice(start..start, letters);
        state.motion = Some(BraidMotion::Pair {
            start,
            width: 0.,
            amplitude: 0.,
        });
    }
    if let Some(BraidMotion::Orbit { progress, .. }) = s.motion {
        if progress >= 0.5 {
            rotate_last(&mut state);
        }
    }
    state
}
pub(super) fn rotate_last(s: &mut BraidState) {
    if let Some(letter) = s.word.pop() {
        if let Some(p) = s.palette.as_mut() {
            p.swap(letter.i, letter.i + 1);
        }
        s.word.insert(0, letter);
    }
}
pub(super) fn orbit(d: &BraidDemo, s: &BraidState) -> Option<Orbit> {
    let BraidMotion::Orbit { progress, .. } = s.motion.as_ref()? else {
        return None;
    };
    let into = *progress >= 0.5;
    let j = if into { 0 } else { s.word.len() - 1 };
    let i = s.word[j].i;
    let rest = (s.word.len() - 1) as f64 * cw(s);
    let x = x0(s);
    let lengths = [i, i + 1].map(|r| loop_track(d, s, r as f64, x + rest, x, r as f64).length);
    let mid = (lengths[0] + lengths[1]) * 0.5;
    let shift = if into {
        (cw(s) - (1. - progress) * (mid - cw(s))).clamp(0., cw(s))
    } else {
        0.
    };
    let tracks =
        [i, i + 1].map(|r| loop_track(d, s, r as f64, x + shift + rest, x + shift, r as f64));
    let distances = lengths.map(|len| progress * (len - cw(s)));
    let center = tracks[0]
        .at(distances[0] + cw(s) / 2.)
        .lerp(tracks[1].at(distances[1] + cw(s) / 2.), 0.5);
    Some(Orbit {
        width: cw(s),
        into,
        j,
        i,
        shift,
        tracks,
        distances,
        center,
        seam: d.capacity(s) as f64 * cw(s) - shift - rest,
    })
}
fn pair_gap(s: &BraidState, j: usize) -> bool {
    matches!(s.motion,Some(BraidMotion::Pair{start,amplitude,..})if (j==start||j==start+1)&&amplitude==0.)
}
pub(super) fn col_width(s: &BraidState, j: usize, orbit: Option<&Orbit>) -> f64 {
    if orbit.is_some_and(|o| o.into && j == 0) {
        return orbit.unwrap().shift;
    }
    match s.motion {
        Some(BraidMotion::Turn { progress }) if j + 1 == s.word.len() => cw(s) * progress,
        Some(BraidMotion::Grow { at_end, progress })
            if j == if at_end { s.word.len() - 1 } else { 0 } =>
        {
            cw(s) * progress
        }
        Some(BraidMotion::Pair { start, width, .. }) if j == start || j == start + 1 => {
            cw(s) * width
        }
        _ => cw(s),
    }
}
pub(super) fn col_x(s: &BraidState, j: usize, o: Option<&Orbit>) -> f64 {
    x0(s) + (0..j).map(|k| col_width(s, k, o)).sum::<f64>()
}
fn next_row(s: &BraidState, j: usize, r: f64) -> f64 {
    let c = &s.word[j];
    if pair_gap(s, j) {
        return r;
    }
    let target = if r == c.i as f64 {
        r + 1.
    } else if r == c.i as f64 + 1. {
        r - 1.
    } else {
        r
    };
    if let Some(BraidMotion::Turn { progress }) = s.motion {
        if j + 1 == s.word.len() {
            return r + (target - r) * progress;
        }
    }
    target
}
fn polyline(path: &mut BezPath, points: impl IntoIterator<Item = Point>) {
    for p in points {
        path.line_to(p);
    }
}
fn started(p: Point) -> BezPath {
    let mut path = BezPath::new();
    path.move_to(p);
    path
}
fn color_palette(s: &BraidState) -> Vec<usize> {
    if !s.closure {
        return (0..s.n).collect();
    }
    let word = if matches!(s.motion, Some(BraidMotion::Turn { progress }) if progress <= 0.5) {
        &s.word[..s.word.len() - 1]
    } else {
        &s.word
    };
    let perm = permutation(s.n, word);
    let mut colors = vec![usize::MAX; s.n];
    let mut used = BTreeSet::new();
    for v in 0..s.n {
        if colors[v] != usize::MAX {
            continue;
        }
        let hint = s.palette.as_ref().and_then(|p| p.get(v)).copied();
        let color = hint
            .filter(|c| !used.contains(c))
            .unwrap_or_else(|| (0..8).find(|c| !used.contains(c)).unwrap_or(0));
        used.insert(color);
        let mut u = v;
        while colors[u] == usize::MAX {
            colors[u] = color;
            u = perm.iter().position(|x| *x == u).unwrap();
        }
    }
    colors
}
pub(super) fn palette(s: &BraidState) -> Vec<usize> {
    color_palette(s)
}
struct Piece {
    strand: usize,
    key: String,
    path: BezPath,
    z: usize,
}
pub(super) fn drawing(d: &BraidDemo, raw: &BraidState) -> Scene {
    let state = displayed(raw);
    let s = &state;
    let columns = d.capacity(s);
    let full = s
        .word
        .iter()
        .enumerate()
        .filter(|(j, _)| !pair_gap(s, *j))
        .count()
        - usize::from(matches!(s.motion,Some(BraidMotion::Turn{progress}) if progress<=0.5))
        >= columns - 1;
    let o = orbit(d, s);
    let colors = color_palette(s);
    let mut scene = Scene::new(
        680.,
        if s.closure {
            286.
        } else {
            108. + s.n as f64 * 28.
        },
    );
    let left = x0(s);
    let right = left + columns as f64 * cw(s);
    let mut arr = (0..s.n).collect::<Vec<_>>();
    let cross = s
        .word
        .iter()
        .enumerate()
        .map(|(j, l)| {
            let a = arr[l.i];
            let b = arr[l.i + 1];
            let sign = if pair_gap(s, j) {
                if let Some(BraidMotion::Pair { start, .. }) = s.motion {
                    s.word[start].sign
                } else {
                    l.sign
                }
            } else {
                l.sign
            };
            if !pair_gap(s, j) {
                arr.swap(l.i, l.i + 1);
            }
            (a, b, if sign > 0 { a } else { b })
        })
        .collect::<Vec<_>>();
    let rank = if let Some(start) = s.by_column {
        let mut rank = BTreeMap::new();
        for (_, _, over) in cross.iter().skip(start).take(3) {
            *rank.entry(*over).or_insert(0usize) += 1;
        }
        rank
    } else {
        BTreeMap::new()
    };
    let mut pieces = Vec::new();
    let mut ends = Vec::new();
    for v in 0..s.n {
        let mut row = v as f64;
        let mut from = started(Point::new(
            if s.closure { left } else { left - 14. },
            row_y(s, row),
        ));
        let mut on_track = None::<usize>;
        let mut track_from = 0.;
        for (j, c) in s.word.iter().enumerate() {
            if s.by_column.is_none() && row != c.i as f64 && row != c.i as f64 + 1. {
                continue;
            }
            let next = next_row(s, j, row);
            let z = if s.by_column.is_some_and(|a| (a..a + 3).contains(&j)) {
                rank.get(&v).copied().unwrap_or(0)
            } else {
                usize::from(cross[j].2 == v)
            };
            let key = if s.by_column.is_some() {
                format!("column-{j}")
            } else {
                format!("letter-{}", c.key)
            };
            if let Some(o) = o.as_ref().filter(|o| o.j == j) {
                let k = usize::from(row != o.i as f64);
                let other = &o.tracks[1 - k];
                let pts = o.crossing_points(k);
                let after = o.distances[1 - k] + cw(s);
                if o.into {
                    let mut path = started(pts[0]);
                    polyline(&mut path, pts.iter().copied().skip(1));
                    pieces.push(Piece {
                        strand: v,
                        key,
                        path,
                        z,
                    });
                    from = started(*pts.last().unwrap());
                    polyline(&mut from, other.points(after, other.length));
                } else {
                    polyline(&mut from, o.tracks[k].points(0., o.distances[k]));
                    polyline(&mut from, pts.iter().copied());
                    pieces.push(Piece {
                        strand: v,
                        key,
                        path: from,
                        z,
                    });
                    from = started(*pts.last().unwrap());
                    on_track = Some(1 - k);
                    track_from = after;
                }
                row = next;
                continue;
            }
            let x = col_x(s, j, o.as_ref());
            let w = col_width(s, j, o.as_ref());
            let y0 = row_y(s, row);
            let y1 = row_y(s, next);
            from.line_to((x, y0));
            from.curve_to((x + w / 2., y0), (x + w / 2., y1), (x + w, y1));
            pieces.push(Piece {
                strand: v,
                key,
                path: from,
                z,
            });
            from = started(Point::new(x + w, y1));
            row = next;
        }
        if let Some(k) = on_track {
            let orbit = o.as_ref().unwrap();
            polyline(
                &mut from,
                orbit.tracks[k].points(track_from, track_from.max(orbit.seam)),
            );
        } else {
            from.line_to((if s.closure { right } else { right + 14. }, row_y(s, row)));
        }
        pieces.push(Piece {
            strand: v,
            key: "free".into(),
            path: from,
            z: 0,
        });
        ends.push((v, row));
    }
    if s.closure {
        for (v, row) in &ends {
            let k = o.as_ref().map(|o| *row - o.i as f64);
            let pts = if let (Some(o), Some(k)) = (o.as_ref(), k.filter(|k| *k == 0. || *k == 1.)) {
                let k = k as usize;
                let tr = &o.tracks[k];
                let at = o.distances[k];
                if o.into {
                    tr.points(o.seam, o.seam.max(at))
                } else {
                    tr.points(o.seam.max(at + cw(s)), tr.length)
                }
            } else {
                let tr = loop_track(d, s, *row, right, left, row.round());
                tr.points(0., tr.length)
            };
            let mut path = started(pts[0]);
            polyline(&mut path, pts.into_iter().skip(1));
            for halo in [true, false] {
                scene.push(
                    Object::path(
                        format!("loop-{}{}", *row as usize, if halo { "-halo" } else { "" }),
                        path.clone(),
                        if halo {
                            [255; 4]
                        } else {
                            STRAND_STROKES[colors[*v] % 6]
                        },
                        if halo { 9. } else { 3. },
                    )
                    .line_cap(if halo {
                        LineCap::Butt
                    } else {
                        LineCap::Round
                    }),
                );
            }
        }
    }
    // Halos at each level are drawn before every stroke at that level.
    for z in 0..=2 {
        for halo in [true, false] {
            for p in pieces.iter().filter(|p| p.z == z) {
                let mut object = Object::path(
                    format!(
                        "wire-{}-{}{}",
                        p.strand,
                        p.key,
                        if halo { "-halo" } else { "" }
                    ),
                    p.path.clone(),
                    if halo {
                        [255; 4]
                    } else {
                        STRAND_STROKES[colors[p.strand] % 6]
                    },
                    if halo { 9. } else { 3. },
                )
                .line_cap(if halo { LineCap::Butt } else { LineCap::Round });
                if let Some(BraidMotion::Grow { at_end, progress }) = s.motion {
                    let hidden = if at_end { s.n - 1 } else { s.n - 2 };
                    if p.strand == hidden {
                        object.opacity = progress;
                    }
                }
                scene.push(object);
            }
        }
    }
    for (j, l) in s.word.iter().enumerate() {
        let p = if let Some(o) = o.as_ref().filter(|o| o.j == j) {
            o.center
        } else {
            Point::new(
                col_x(s, j, o.as_ref()) + col_width(s, j, o.as_ref()) / 2.,
                row_y(s, l.i as f64 + 0.5),
            )
        };
        let mut handle = Object::circle(
            format!("cross-{}", l.key),
            p,
            cw(s).min(28.) / 2. - 1.,
            [0; 4],
        );
        if pair_gap(s, j) {
            handle.opacity = 0.;
        }
        scene.push(handle);
        if !pair_gap(s, j) {
            let inv = l.sign < 0;
            let y = if s.closure {
                18.
            } else {
                row_y(s, s.n as f64 - 1.) + 34.
            };
            let x = if let Some(o) = o.as_ref().filter(|o| o.j == j) {
                o.center.x
            } else {
                col_x(s, j, o.as_ref()) + col_width(s, j, o.as_ref()) / 2.
            };
            let alpha = if let Some(o) = o.as_ref().filter(|o| o.j == j) {
                (1. - (left - o.center.x)
                    .max(o.center.x - right)
                    .max((o.center.y - row_y(s, l.i as f64 + 0.5)).abs())
                    / 16.)
                    .clamp(0., 1.)
            } else if let Some(BraidMotion::Turn { progress }) = s.motion {
                if j + 1 == s.word.len() { progress } else { 1. }
            } else {
                1.
            };
            let origin = x - if inv { 9. } else { 6. };
            for (suffix, p, label, size, centered) in [
                ("sigma", Point::new(origin, y), "σ".to_owned(), 17., true),
                (
                    "index",
                    Point::new(origin + 6., y + 6.),
                    (l.i + 1).to_string(),
                    11.,
                    false,
                ),
                (
                    "inverse",
                    Point::new(origin + 6., y - 7.),
                    "−1".to_owned(),
                    11.,
                    false,
                ),
            ] {
                let mut glyph = Object::text(
                    format!("word-{}-{suffix}", l.key),
                    p,
                    label,
                    size,
                    [31, 41, 55, 255],
                )
                .text_baseline(TextBaseline::Central)
                .font_family(FontFamily::Serif);
                if centered {
                    glyph = glyph.text_anchor(TextAnchor::Middle).italic();
                }
                glyph.opacity = if suffix == "inverse" && !inv {
                    0.
                } else {
                    alpha
                };
                scene.push(glyph);
            }
        }
    }
    // One stable identity per strand/column, shared by a straight bit and the
    // apex it becomes when an inverse pair is inserted there.
    for v in 0..s.n {
        let mut row = v as f64;
        let mut j = 0;
        while j <= s.word.len() {
            let letter = s.word.get(j);
            let next = letter.map_or(row, |_| next_row(s, j, row));
            let pair = letter
                .zip(s.word.get(j + 1))
                .is_some_and(|(a, b)| a.i == b.i && a.sign != b.sign);
            let gap = pair_gap(s, j);
            let apex = pair && (next != row || gap);
            let braid_side = j > 0
                && braid_move(s.n, &s.word, j - 1, true).is_some()
                && s.word
                    .get(j)
                    .is_some_and(|l| row != l.i as f64 && row != l.i as f64 + 1.)
                && (row == s.word[j - 1].i as f64 || row == s.word[j - 1].i as f64 + 1.);
            let logical_len = s
                .word
                .iter()
                .enumerate()
                .filter(|(k, _)| !pair_gap(s, *k))
                .count();
            let straight = next == row && !gap && (logical_len + 2 < columns || braid_side);
            let x = col_x(s, j, o.as_ref()) + cw(s);
            if apex || straight {
                let y = row_y(s, if apex && !gap { next } else { row });
                let mut handle = Object::new(
                    format!("segment-{j}-{v}"),
                    Affine::translate((x, y)),
                    Some(Hit::Rect(if apex {
                        Rect::new(-9.8, -9.8, 9.8, 9.8)
                    } else {
                        Rect::new(-cw(s) * 0.9, -8.4, -cw(s) * 0.1, 8.4)
                    })),
                    vec![Shape::Rect {
                        bounds: if apex {
                            Rect::new(-9.8, -9.8, 9.8, 9.8)
                        } else {
                            Rect::new(-cw(s) * 0.9, -8.4, -cw(s) * 0.1, 8.4)
                        },
                        radius: 4.,
                        fill: [0; 4],
                        stroke: None,
                    }],
                );
                if gap {
                    handle.hit = None;
                }
                scene.push(handle);
            }
            row = next;
            if gap {
                j += 2;
            } else {
                j += 1;
            }
        }
    }
    if !s.closure {
        for v in 0..s.n {
            scene.push(
                Object::text(
                    format!("start-{v}"),
                    (left - 22., row_y(s, v as f64)),
                    (v + 1).to_string(),
                    11.,
                    [107, 114, 128, 255],
                )
                .text_anchor(TextAnchor::Middle)
                .text_baseline(TextBaseline::Central),
            );
        }
        for (v, row) in &ends {
            scene.push(
                Object::text(
                    format!("end-{v}"),
                    (right + 22., row_y(s, *row)),
                    (v + 1).to_string(),
                    11.,
                    STRAND_STROKES[*v % 6],
                )
                .text_anchor(TextAnchor::Middle)
                .text_baseline(TextBaseline::Central)
                .font_weight(600),
            );
        }
    }
    if s.closure {
        let mut border = Object::rect(
            "braid-box",
            Rect::new(
                left,
                row_y(s, -0.5),
                right,
                row_y(s, s.n as f64 - 1.) + 0.4 * 28.,
            ),
            4.,
            [0; 4],
        )
        .decorative();
        if let Shape::Rect { stroke, .. } = &mut border.shapes[0] {
            *stroke = Some(([203, 213, 225, 255], 1.));
        }
        scene.objects.insert(0, border);
        for (id, x) in [("curl-left", left), ("curl-right", right)] {
            let (a, b) = if id == "curl-right" {
                (-cw(s) * 0.7, 12.)
            } else {
                (-12., 4.)
            };
            scene.push(Object::new(
                id,
                Affine::translate((x, row_y(s, s.n as f64 - 1.))),
                Some(Hit::Rect(Rect::new(a, -8.4, b, 8.4))),
                vec![Shape::Rect {
                    bounds: Rect::new(a, -8.4, b, 8.4),
                    radius: 4.,
                    fill: [0; 4],
                    stroke: None,
                }],
            ));
        }
        let half = (right - left) / 2. + 6.;
        scene.push(Object::new(
            "destabilize",
            Affine::translate(((left + right) / 2., bottom_y(s))),
            Some(Hit::Rect(Rect::new(-half, -4.8, half, 4.8))),
            vec![Shape::Rect {
                bounds: Rect::new(-half, -4.8, half, 4.8),
                radius: 4.,
                fill: [0; 4],
                stroke: None,
            }],
        ));
    }
    let word_y = if s.closure {
        18.
    } else {
        row_y(s, s.n as f64 - 1.) + 34.
    };
    scene.push(
        Object::text(
            "word-beta",
            (left - 30., word_y),
            "β",
            17.,
            [31, 41, 55, 255],
        )
        .text_anchor(TextAnchor::Middle)
        .text_baseline(TextBaseline::Central)
        .font_family(FontFamily::Serif)
        .italic(),
    );
    if s.word.iter().enumerate().all(|(j, _)| pair_gap(s, j))
        && !matches!(s.motion, Some(BraidMotion::Turn { .. }))
    {
        scene.push(
            Object::text(
                "word-empty",
                (left + cw(s) / 2., word_y),
                "e",
                17.,
                [156, 163, 175, 255],
            )
            .text_anchor(TextAnchor::Middle)
            .text_baseline(TextBaseline::Central)
            .font_family(FontFamily::Serif)
            .italic(),
        );
    }
    let mut plane = BezPath::new();
    for (i, (row, depth)) in [
        (-0.75, -DEPTH),
        (s.n as f64 - 0.25, -DEPTH),
        (s.n as f64 - 0.25, DEPTH),
        (-0.75, DEPTH),
    ]
    .into_iter()
    .enumerate()
    {
        let p = d.project_slice(s, row, depth).0;
        if i == 0 {
            plane.move_to(p);
        } else {
            plane.line_to(p);
        }
    }
    plane.close_path();
    scene.push(Object::new(
        "slice-plane",
        Affine::IDENTITY,
        None,
        vec![Shape::Path {
            path: plane,
            fill: Some(if !full {
                [248, 250, 252, 255]
            } else {
                [255, 247, 237, 255]
            }),
            stroke: Some(if !full {
                ([203, 213, 225, 255], 1.)
            } else {
                ([251, 146, 60, 255], 1.5)
            }),
        }],
    ));
    for r in 0..s.n {
        scene.push(Object::line(
            format!("slice-grid-{r}"),
            d.project_slice(s, r as f64, DEPTH).0,
            d.project_slice(s, r as f64, -DEPTH).0,
            [226, 232, 240, 255],
            1.,
        ));
    }
    let a = d.project_slice(s, -0.75, 0.).0;
    let b = d.project_slice(s, s.n as f64 - 0.25, 0.).0;
    let mut dashed = BezPath::new();
    let mut y = a.y;
    while y < b.y {
        dashed.move_to((a.x, y));
        dashed.line_to((a.x, (y + 3.).min(b.y)));
        y += 6.;
    }
    scene.push(Object::path(
        "slice-center-line",
        dashed,
        [203, 213, 225, 255],
        1.,
    ));
    if full {
        let p = d.project_slice(s, s.n as f64 - 0.25, DEPTH).0;
        scene.push(
            Object::text(
                "braid-full",
                (614., p.y + 16.),
                "Braid is full",
                11.,
                [194, 65, 12, 255],
            )
            .text_anchor(TextAnchor::Middle)
            .text_baseline(TextBaseline::Alphabetic)
            .font_weight(600),
        );
    }
    let mut points = d.configuration(s);
    points.sort_by(|a, b| a.2.total_cmp(&b.2));
    for (v, row, depth) in points {
        let (p, scale) = d.project_slice(s, row, depth);
        let mut point = Object::circle(format!("slice-{v}"), p, 11. * scale, [0; 4]);
        point.shapes.push(Shape::Circle {
            center: Point::ORIGIN,
            radius: 6.5 * scale,
            fill: STRAND_STROKES[colors[v] % 6],
            stroke: Some(([255; 4], 1.5)),
        });
        scene.push(point);
    }
    if s.slice {
        if let Some(t) = s.hover {
            scene.push(
                Object::line(
                    "moment",
                    (left + t * cw(s), row_y(s, -0.7)),
                    (left + t * cw(s), row_y(s, s.n as f64 - 0.3)),
                    [148, 163, 184, 255],
                    1.,
                )
                .dash(vec![3., 3.]),
            );
            for (v, row, _) in d.configuration(s) {
                let mut dot = Object::circle(
                    format!("moment-point-{v}"),
                    (left + t * cw(s), row_y(s, row)),
                    4.,
                    STRAND_STROKES[colors[v] % 6],
                )
                .decorative();
                if let Shape::Circle { stroke, .. } = &mut dot.shapes[0] {
                    *stroke = Some(([255; 4], 1.5));
                }
                scene.push(dot);
            }
        }
    }
    scene
}
