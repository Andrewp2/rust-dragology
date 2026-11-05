//! Cubic geometry translated from Dragology's knot/geometry.ts.
use crate::{Point, Vec2};
pub type Cubic = [Point; 4];
pub fn norm(v: Vec2) -> Vec2 {
    if v.hypot() > 0. {
        v / v.hypot()
    } else {
        Vec2::ZERO
    }
}
pub fn perp(v: Vec2) -> Vec2 {
    Vec2::new(-v.y, v.x)
}
pub fn angle(v: Vec2) -> f64 {
    v.y.atan2(v.x)
}
pub fn polar(a: f64) -> Vec2 {
    Vec2::new(a.cos(), a.sin())
}
pub fn cyc(i: isize, n: usize) -> usize {
    i.rem_euclid(n as isize) as usize
}
pub fn bez(c: Cubic, t: f64) -> Point {
    let m = 1. - t;
    (c[0].to_vec2() * (m * m * m)
        + c[1].to_vec2() * (3. * m * m * t)
        + c[2].to_vec2() * (3. * m * t * t)
        + c[3].to_vec2() * (t * t * t))
        .to_point()
}
pub fn tangent(c: Cubic, t: f64) -> Vec2 {
    let m = 1. - t;
    let d =
        (c[1] - c[0]) * (3. * m * m) + (c[2] - c[1]) * (6. * m * t) + (c[3] - c[2]) * (3. * t * t);
    norm(if d.hypot() > 1e-9 { d } else { c[3] - c[0] })
}
pub fn split(c: Cubic, t: f64) -> (Cubic, Cubic) {
    let a = c[0].lerp(c[1], t);
    let b = c[1].lerp(c[2], t);
    let d = c[2].lerp(c[3], t);
    let e = a.lerp(b, t);
    let f = b.lerp(d, t);
    let m = e.lerp(f, t);
    ([c[0], a, e, m], [m, f, d, c[3]])
}
pub fn sub(c: Cubic, t0: f64, t1: f64) -> Cubic {
    let r = split(c, t0).1;
    if t0 >= 1. {
        r
    } else {
        split(r, (t1 - t0) / (1. - t0)).0
    }
}
pub fn sample(c: Cubic, n: usize) -> Vec<Point> {
    (0..=n).map(|i| bez(c, i as f64 / n as f64)).collect()
}
pub fn length(c: Cubic, n: usize) -> f64 {
    sample(c, n).windows(2).map(|p| p[0].distance(p[1])).sum()
}
pub fn at_length(c: Cubic, l: f64) -> f64 {
    if l <= 0. {
        return 0.;
    }
    let mut acc = 0.;
    let mut prev = c[0];
    for i in 1..=48 {
        let q = bez(c, i as f64 / 48.);
        let seg = prev.distance(q);
        if acc + seg >= l {
            return (i as f64 - 1. + if seg > 0. { (l - acc) / seg } else { 0. }) / 48.;
        }
        acc += seg;
        prev = q;
    }
    1.
}
pub fn closest(c: Cubic, p: Point, lo: f64, hi: f64, n: usize) -> (f64, Point, f64) {
    let mut best = (lo, bez(c, lo), f64::INFINITY);
    for i in 0..=n {
        let t = lo + (hi - lo) * i as f64 / n as f64;
        let q = bez(c, t);
        let d = p.distance(q);
        if d < best.2 {
            best = (t, q, d);
        }
    }
    best
}
pub fn cubic(p: Point, dp: Vec2, q: Point, dq: Vec2, a: f64, b: f64) -> Cubic {
    [p, p + dp * a, q - dq * b, q]
}
pub fn handles(c: Cubic) -> super::model::Edge {
    super::model::Edge {
        a: c[0].distance(c[1]),
        b: c[3].distance(c[2]),
    }
}
pub fn fit(p: Point, dp: Vec2, q: Point, dq: Vec2, pts: &[Point], min: f64) -> super::model::Edge {
    if pts.is_empty() {
        let d = (p.distance(q) / 3.).max(min);
        return super::model::Edge { a: d, b: d };
    }
    let mut cum = vec![0.];
    let mut prev = p;
    for x in pts.iter().chain(std::iter::once(&q)) {
        cum.push(cum.last().unwrap() + prev.distance(*x));
        prev = *x;
    }
    let total = cum.last().copied().unwrap().max(1e-100);
    let mut ts = pts
        .iter()
        .enumerate()
        .map(|(i, _)| cum[i + 1] / total)
        .collect::<Vec<_>>();
    let mut a = total / 3.;
    let mut b = a;
    for _ in 0..4 {
        let (mut a11, mut a12, mut a22, mut r1, mut r2) = (0., 0., 0., 0., 0.);
        let dd = dp.dot(dq);
        for (i, pt) in pts.iter().enumerate() {
            let t = ts[i];
            let m = 1. - t;
            let al = 3. * m * m * t;
            let be = 3. * m * t * t;
            let base = p.to_vec2() * (m * m * m + al) + q.to_vec2() * (be + t * t * t);
            let r = pt.to_vec2() - base;
            a11 += al * al;
            a22 += be * be;
            a12 -= al * be * dd;
            r1 += al * dp.dot(r);
            r2 -= be * dq.dot(r);
        }
        let det = a11 * a22 - a12 * a12;
        if det.abs() > 1e-9 {
            a = (r1 * a22 - r2 * a12) / det;
            b = (a11 * r2 - a12 * r1) / det;
        }
        a = a.max(min).min(4. * total);
        b = b.max(min).min(4. * total);
        let c = cubic(p, dp, q, dq, a, b);
        ts = pts
            .iter()
            .enumerate()
            .map(|(i, pt)| closest(c, *pt, (ts[i] - 0.25).max(0.), (ts[i] + 0.25).min(1.), 24).0)
            .collect();
    }
    super::model::Edge { a, b }
}
pub fn segment_hit(p: Point, p2: Point, q: Point, q2: Point) -> Option<(f64, f64)> {
    let r = p2 - p;
    let s = q2 - q;
    let den = r.cross(s);
    if den.abs() < 1e-12 {
        return None;
    }
    let d = q - p;
    let t = d.cross(s) / den;
    let u = d.cross(r) / den;
    if (0. ..=1.).contains(&t) && (0. ..=1.).contains(&u) {
        Some((t, u))
    } else {
        None
    }
}
pub fn distance_segment(p: Point, a: Point, b: Point) -> f64 {
    let ab = b - a;
    let l2 = ab.hypot2();
    let t = if l2 > 0. {
        ((p - a).dot(ab) / l2).clamp(0., 1.)
    } else {
        0.
    };
    p.distance(a.lerp(b, t))
}
pub fn poly_crossings(p: &[Point]) -> Vec<(f64, f64, Point)> {
    let m = p.len();
    let mut out = Vec::new();
    for a in 0..m {
        for b in a + 2..m {
            if a == 0 && b == m - 1 {
                continue;
            }
            if let Some((t, u)) = segment_hit(p[a], p[(a + 1) % m], p[b], p[(b + 1) % m]) {
                if t < 1. && u < 1. {
                    out.push((a as f64 + t, b as f64 + u, p[a].lerp(p[(a + 1) % m], t)));
                }
            }
        }
    }
    out
}
