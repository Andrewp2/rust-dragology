//! Gauss-code diagram, invariants, and construction. Source: upstream knot.ts.
use super::geometry::*;
use crate::{Point, Rect, Vec2};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Hint {
    pub a: f64,
    pub b: f64,
    pub e: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub kind: String,
    pub x: f64,
    pub y: f64,
    pub rot: f64,
    pub sign: i32,
    #[serde(default)]
    pub hint: Option<Hint>,
    #[serde(default, rename = "ghostOf")]
    pub ghost_of: Option<String>,
}
impl Node {
    pub fn pos(&self) -> Point {
        Point::new(self.x, self.y)
    }
    pub fn set_pos(&mut self, p: Point) {
        self.x = p.x;
        self.y = p.y;
    }
    pub fn crossing(&self) -> bool {
        self.kind == "x"
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Visit {
    pub n: String,
    pub over: bool,
    pub e: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Edge {
    pub a: f64,
    pub b: f64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Knot {
    pub nodes: BTreeMap<String, Node>,
    pub code: Vec<Visit>,
    pub edges: BTreeMap<String, Edge>,
    #[serde(rename = "nextId")]
    pub next_id: usize,
}
pub fn pass(p: Point, d: Vec2, hint: Option<Hint>, ghost: Option<String>) -> Node {
    Node {
        kind: "p".into(),
        x: p.x,
        y: p.y,
        rot: angle(d),
        sign: 1,
        hint,
        ghost_of: ghost,
    }
}
pub fn crossing(p: Point, over: Vec2, under: Vec2) -> Node {
    Node {
        kind: "x".into(),
        x: p.x,
        y: p.y,
        rot: angle(over),
        sign: if over.cross(under) < 0. { 1 } else { -1 },
        hint: None,
        ghost_of: None,
    }
}
pub fn position(k: &Knot, i: usize) -> Point {
    k.nodes[&k.code[i].n].pos()
}
pub fn direction(k: &Knot, i: usize) -> Vec2 {
    let v = &k.code[i];
    let n = &k.nodes[&v.n];
    polar(
        n.rot
            - if n.crossing() && !v.over {
                n.sign as f64 * std::f64::consts::FRAC_PI_2
            } else {
                0.
            },
    )
}
pub fn cubics(k: &Knot) -> Vec<Cubic> {
    k.code
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let j = (i + 1) % k.code.len();
            let e = k.edges[&v.e];
            cubic(
                position(k, i),
                direction(k, i),
                position(k, j),
                direction(k, j),
                e.a,
                e.b,
            )
        })
        .collect()
}
pub fn edge_index(k: &Knot, e: &str) -> Option<usize> {
    k.code.iter().position(|v| v.e == e)
}
pub fn edge_mid(k: &Knot, e: &str) -> Point {
    bez(cubics(k)[edge_index(k, e).unwrap()], 0.5)
}
fn bounds(ps: &[Point]) -> Rect {
    ps.iter().fold(
        Rect::new(
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ),
        |r, p| Rect::new(r.x0.min(p.x), r.y0.min(p.y), r.x1.max(p.x), r.y1.max(p.y)),
    )
}
fn overlaps(a: Rect, b: Rect) -> bool {
    a.x0 <= b.x1 + 1e-6 && a.x1 + 1e-6 >= b.x0 && a.y0 <= b.y1 + 1e-6 && a.y1 + 1e-6 >= b.y0
}
pub fn stray(k: &Knot, limit: usize) -> usize {
    let n = k.code.len();
    if n == 0 {
        return usize::MAX;
    }
    let pts = cubics(k)
        .into_iter()
        .map(|c| sample(c, 16))
        .collect::<Vec<_>>();
    let eb = pts.iter().map(|p| bounds(p)).collect::<Vec<_>>();
    let sb = pts
        .iter()
        .map(|ps| ps.windows(2).map(bounds).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    let ends = |e: usize, i: usize| -> Vec<&str> {
        let mut out = Vec::new();
        if i == 0 {
            out.push(k.code[e].n.as_str());
        }
        if i == 15 {
            out.push(k.code[(e + 1) % n].n.as_str());
        }
        out
    };
    let mut out = 0;
    for e1 in 0..n {
        for e2 in e1..n {
            if !overlaps(eb[e1], eb[e2]) {
                continue;
            }
            for i in 0..16 {
                for j in if e1 == e2 { i + 2 } else { 0 }..16 {
                    if !overlaps(sb[e1][i], sb[e2][j]) {
                        continue;
                    }
                    if let Some((t, _)) =
                        segment_hit(pts[e1][i], pts[e1][i + 1], pts[e2][j], pts[e2][j + 1])
                    {
                        let x = pts[e1][i].lerp(pts[e1][i + 1], t);
                        let shared = ends(e2, j);
                        let ok = ends(e1, i)
                            .iter()
                            .any(|id| shared.contains(id) && x.distance(k.nodes[*id].pos()) < 1.5);
                        if !ok {
                            out += 1;
                            if out >= limit {
                                return out;
                            }
                        }
                    }
                }
            }
        }
    }
    out
}
pub fn valid(k: &Knot) -> bool {
    stray(k, 1) == 0
}
pub fn new_ids(k: &Knot, kinds: &str) -> (Vec<String>, usize) {
    (
        kinds
            .chars()
            .enumerate()
            .map(|(i, c)| format!("{c}{}", k.next_id + i))
            .collect(),
        k.next_id + kinds.len(),
    )
}
pub fn arc(k: &Knot, i: usize) -> String {
    for s in 0..k.code.len() {
        let v = &k.code[cyc(i as isize - s as isize, k.code.len())];
        if k.nodes[&v.n].crossing() && !v.over {
            return v.n.clone();
        }
    }
    "root".into()
}
#[derive(Clone)]
struct Cross {
    id: String,
    over: usize,
    under: usize,
    sign: i32,
}
fn combo(k: &Knot) -> (Vec<(String, bool)>, Vec<Cross>) {
    let cv = k
        .code
        .iter()
        .filter(|v| k.nodes[&v.n].crossing())
        .map(|v| (v.n.clone(), v.over))
        .collect::<Vec<_>>();
    let mut cs = Vec::<Cross>::new();
    for (i, (id, over)) in cv.iter().enumerate() {
        let ix = cs.iter().position(|c| &c.id == id).unwrap_or_else(|| {
            cs.push(Cross {
                id: id.clone(),
                over: 0,
                under: 0,
                sign: k.nodes[id].sign,
            });
            cs.len() - 1
        });
        if *over {
            cs[ix].over = i;
        } else {
            cs[ix].under = i;
        }
    }
    (cv, cs)
}
fn arc_cv(cv: &[(String, bool)], i: usize) -> String {
    for s in 0..cv.len() {
        let (id, over) = &cv[cyc(i as isize - s as isize, cv.len())];
        if !over {
            return id.clone();
        }
    }
    "root".into()
}
pub type Poly = BTreeMap<i32, i64>;
fn poly_mul(a: &Poly, b: &Poly) -> Poly {
    let mut out = Poly::new();
    for (ea, ca) in a {
        for (eb, cb) in b {
            *out.entry(ea + eb).or_default() += ca * cb;
        }
    }
    out.retain(|_, c| *c != 0);
    out
}
fn poly_add(a: &mut Poly, b: &Poly) {
    for (e, c) in b {
        *a.entry(*e).or_default() += c;
    }
    a.retain(|_, c| *c != 0);
}
fn find(parent: &mut [usize], mut x: usize) -> usize {
    while parent[x] != x {
        parent[x] = parent[parent[x]];
        x = parent[x];
    }
    x
}
fn union(p: &mut [usize], a: usize, b: usize) {
    let a = find(p, a);
    let b = find(p, b);
    p[a] = b;
}
fn jones(cs: &[Cross]) -> Option<Poly> {
    let n = cs.len();
    if n == 0 {
        return Some(Poly::from([(0, 1)]));
    }
    if n > 13 {
        return None;
    }
    let e = 2 * n;
    let pd = cs
        .iter()
        .map(|c| {
            let ui = cyc(c.under as isize - 1, e);
            let oi = cyc(c.over as isize - 1, e);
            if c.sign == 1 {
                [ui, c.over, c.under, oi]
            } else {
                [ui, oi, c.under, c.over]
            }
        })
        .collect::<Vec<_>>();
    let loop_poly = Poly::from([(2, -1), (-2, -1)]);
    let mut bracket = Poly::new();
    for state in 0usize..1 << n {
        let mut p = (0..e).collect::<Vec<_>>();
        let mut na = 0;
        for (i, [a, b, c, d]) in pd.iter().copied().enumerate() {
            if state & (1 << i) != 0 {
                union(&mut p, a, b);
                union(&mut p, c, d);
                na += 1;
            } else {
                union(&mut p, a, d);
                union(&mut p, b, c);
            }
        }
        let loops = (0..e).filter(|i| find(&mut p, *i) == *i).count();
        let mut term = Poly::from([(2 * na - n as i32, 1)]);
        for _ in 1..loops {
            term = poly_mul(&term, &loop_poly);
        }
        poly_add(&mut bracket, &term);
    }
    let w = cs.iter().map(|c| c.sign).sum::<i32>();
    Some(poly_mul(
        &bracket,
        &Poly::from([(-3 * w, if w % 2 == 0 { 1 } else { -1 })]),
    ))
}
pub fn format_jones(p: &Poly) -> String {
    if p.is_empty() {
        return "0".into();
    }
    p.iter()
        .enumerate()
        .map(|(i, (ea, c))| {
            let e = -*ea as f64 / 4.;
            let sign = if *c < 0 {
                "− "
            } else if i == 0 {
                ""
            } else {
                "+ "
            };
            let mag = c.abs();
            let coef = if e == 0. || mag != 1 {
                mag.to_string()
            } else {
                String::new()
            };
            let pow = if e == 0. {
                String::new()
            } else if e == 1. {
                "t".into()
            } else {
                let sup = e
                    .to_string()
                    .chars()
                    .map(|x| match x {
                        '-' => '⁻',
                        '0' => '⁰',
                        '1' => '¹',
                        '2' => '²',
                        '3' => '³',
                        '4' => '⁴',
                        '5' => '⁵',
                        '6' => '⁶',
                        '7' => '⁷',
                        '8' => '⁸',
                        '9' => '⁹',
                        x => x,
                    })
                    .collect::<String>();
                format!("t{sup}")
            };
            format!("{sign}{coef}{pow}")
        })
        .collect::<Vec<_>>()
        .join(" ")
}
#[derive(Clone, Debug)]
pub struct Invariants {
    pub n: usize,
    pub writhe: i32,
    pub tricolorable: bool,
    pub colors: BTreeMap<String, usize>,
    pub jones: Option<Poly>,
}
fn tricolor(cv: &[(String, bool)], cs: &[Cross]) -> (bool, BTreeMap<String, usize>) {
    let mut ids = if cs.is_empty() {
        vec!["root".into()]
    } else {
        cs.iter().map(|c| c.id.clone()).collect::<Vec<String>>()
    };
    ids.sort_by_key(|id| {
        id.chars()
            .filter(char::is_ascii_digit)
            .collect::<String>()
            .parse::<usize>()
            .unwrap_or(0)
    });
    let nv = ids.len();
    let mut rows = cs
        .iter()
        .map(|c| {
            let mut row = vec![0i32; nv];
            for a in [
                arc_cv(cv, c.over),
                arc_cv(cv, cyc(c.under as isize - 1, cv.len())),
                c.id.clone(),
            ] {
                let j = ids.iter().position(|id| *id == a).unwrap();
                row[j] = (row[j] + 1) % 3;
            }
            row
        })
        .collect::<Vec<_>>();
    let mut pivots = Vec::new();
    let mut r = 0;
    for c in 0..nv {
        let Some(pr) = (r..rows.len()).find(|i| rows[*i][c] != 0) else {
            continue;
        };
        rows.swap(r, pr);
        let inv = rows[r][c];
        for x in &mut rows[r] {
            *x = (*x * inv) % 3;
        }
        for i in 0..rows.len() {
            if i != r && rows[i][c] != 0 {
                let f = rows[i][c];
                for j in 0..nv {
                    rows[i][j] = (rows[i][j] - f * rows[r][j]).rem_euclid(3);
                }
            }
        }
        pivots.push(c);
        r += 1;
    }
    let free = (0..nv).filter(|c| !pivots.contains(c)).collect::<Vec<_>>();
    let ok = free.len() >= 2;
    let mut best = None::<Vec<i32>>;
    if ok && free.len() <= 7 {
        for mut a in 0..3usize.pow(free.len() as u32) {
            let mut sol = vec![0; nv];
            for c in &free {
                sol[*c] = (a % 3) as i32;
                a /= 3;
            }
            for (ri, c) in pivots.iter().enumerate() {
                sol[*c] =
                    (-free.iter().map(|fc| rows[ri][*fc] * sol[*fc]).sum::<i32>()).rem_euclid(3);
            }
            if sol.iter().all(|v| *v == sol[0]) {
                continue;
            }
            if best.as_ref().is_none_or(|b| sol < *b) {
                best = Some(sol);
            }
        }
    }
    let colors = ids
        .into_iter()
        .enumerate()
        .map(|(j, id)| (id, best.as_ref().map_or(0, |s| s[j] as usize)))
        .collect();
    (ok, colors)
}
pub fn diagram_key(k: &Knot) -> String {
    let (cv, cs) = combo(k);
    format!(
        "{}|{}",
        cv.iter()
            .map(|(id, o)| format!("{id}{}", if *o { 'o' } else { 'u' }))
            .collect::<Vec<_>>()
            .join(","),
        cs.iter()
            .map(|c| format!("{}{}", c.id, if c.sign > 0 { '+' } else { '-' }))
            .collect::<Vec<_>>()
            .join(",")
    )
}
thread_local! {static INV:std::cell::RefCell<HashMap<String,Invariants>>=std::cell::RefCell::new(HashMap::new());}
pub fn invariants(k: &Knot) -> Invariants {
    let key = diagram_key(k);
    INV.with(|cache| {
        if let Some(v) = cache.borrow().get(&key) {
            return v.clone();
        }
        let (cv, cs) = combo(k);
        let (ok, colors) = tricolor(&cv, &cs);
        let inv = Invariants {
            n: cs.len(),
            writhe: cs.iter().map(|c| c.sign).sum(),
            tricolorable: ok,
            colors,
            jones: jones(&cs),
        };
        let mut cache = cache.borrow_mut();
        if cache.len() > 500 {
            cache.clear();
        }
        cache.insert(key, inv.clone());
        inv
    })
}
pub struct Random(u32);
impl Random {
    pub fn seed(s: &str) -> Self {
        let mut h = 2166136261u32;
        for c in s.encode_utf16() {
            h = (h ^ c as u32).wrapping_mul(16777619);
        }
        Self(h)
    }
    pub fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_mul(1664525).wrapping_add(1013904223);
        self.0 as f64 / 4294967296.
    }
}
pub fn parametric(f: impl Fn(f64) -> (f64, f64, f64)) -> Knot {
    let raw = (0..720)
        .map(|s| f(std::f64::consts::TAU * s as f64 / 720.))
        .collect::<Vec<_>>();
    let ps = raw.iter().map(|p| Point::new(p.0, p.1)).collect::<Vec<_>>();
    let z = |u: f64| {
        let s = u.floor() as usize;
        raw[s % 720].2 + (raw[(s + 1) % 720].2 - raw[s % 720].2) * (u - s as f64)
    };
    from_polyline(&ps, |u, v| z(u) > z(v))
}
pub fn from_polyline(poly: &[Point], over: impl Fn(f64, f64) -> bool) -> Knot {
    let m = poly.len();
    let tan =
        |u: f64| norm(poly[cyc(u.floor() as isize + 2, m)] - poly[cyc(u.floor() as isize - 1, m)]);
    let xs = poly_crossings(poly);
    if xs.is_empty() {
        let h = m / 2;
        return Knot {
            nodes: BTreeMap::from([
                ("n0".into(), pass(poly[0], tan(0.), None, None)),
                ("n1".into(), pass(poly[h], tan(h as f64), None, None)),
            ]),
            code: vec![
                Visit {
                    n: "n0".into(),
                    over: false,
                    e: "e2".into(),
                },
                Visit {
                    n: "n1".into(),
                    over: false,
                    e: "e3".into(),
                },
            ],
            edges: BTreeMap::from([
                (
                    "e2".into(),
                    fit(poly[0], tan(0.), poly[h], tan(h as f64), &poly[1..h], 4.),
                ),
                (
                    "e3".into(),
                    fit(poly[h], tan(h as f64), poly[0], tan(0.), &poly[h + 1..], 4.),
                ),
            ]),
            next_id: 4,
        };
    }
    let mut visits = xs
        .iter()
        .enumerate()
        .flat_map(|(i, (u, v, _))| [(*u, i, *v), (*v, i, *u)])
        .collect::<Vec<_>>();
    visits.sort_by(|a, b| a.0.total_cmp(&b.0));
    let nodes = xs
        .iter()
        .enumerate()
        .map(|(i, (u, v, p))| {
            let o = over(*u, *v);
            (
                format!("n{i}"),
                crossing(
                    *p,
                    tan(if o { *u } else { *v }),
                    tan(if o { *v } else { *u }),
                ),
            )
        })
        .collect();
    let code = visits
        .iter()
        .enumerate()
        .map(|(j, (u, i, v))| Visit {
            n: format!("n{i}"),
            over: over(*u, *v),
            e: format!("e{}", xs.len() + j),
        })
        .collect::<Vec<_>>();
    let mut k = Knot {
        nodes,
        next_id: xs.len() + code.len(),
        code,
        edges: BTreeMap::new(),
    };
    let n = k.code.len();
    for q in 0..n {
        let r = (q + 1) % n;
        let u0 = visits[q].0;
        let mut u1 = visits[r].0;
        if u1 <= u0 {
            u1 += m as f64;
        }
        let mut pts = Vec::new();
        let mut s = (u0 + 0.5).ceil() as usize;
        while (s as f64) < u1 - 0.5 {
            pts.push(poly[s % m]);
            s += 1;
        }
        let edge = fit(
            position(&k, q),
            direction(&k, q),
            position(&k, r),
            direction(&k, r),
            &pts,
            4.,
        );
        k.edges.insert(k.code[q].e.clone(), edge);
    }
    k
}
pub fn preset(i: usize) -> Knot {
    match i {
        0 | 1 => {
            let tangles: HashMap<String, Knot> =
                serde_json::from_str(include_str!("tangles.json")).unwrap();
            tangles[if i == 0 { "A" } else { "B" }].clone()
        }
        2 => parametric(|t| {
            (
                280. + 38. * (t.sin() + 2. * (2. * t).sin()),
                230. + 38. * (t.cos() - 2. * (2. * t).cos()),
                -(3. * t).sin(),
            )
        }),
        3 => parametric(|t| {
            (
                280. + 46. * (2. + (2. * t).cos()) * (3. * t).cos(),
                230. + 46. * (2. + (2. * t).cos()) * (3. * t).sin(),
                (4. * t).sin(),
            )
        }),
        _ => parametric(|t| (280. + 80. * t.cos(), 230. + 80. * t.sin(), 0.)),
    }
}
