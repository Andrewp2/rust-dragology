//! Reidemeister candidates and exact split/merge cleanup, ported from knot.ts.
use super::{geometry::*, model::*};
use crate::{Point, Vec2};
use std::collections::HashSet;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Twist,
    Untwist,
    Push,
    Pull,
    Slide,
}
#[derive(Clone, Debug)]
pub struct Move {
    pub kind: Kind,
    pub from: Knot,
    pub to: Knot,
    pub meets: Option<Point>,
}
fn mv(kind: Kind, from: Knot, to: Knot) -> Move {
    Move {
        kind,
        from,
        to,
        meets: None,
    }
}
fn hint(k: &Knot, e: &str) -> Hint {
    let ed = k.edges[e];
    Hint {
        a: ed.a,
        b: ed.b,
        e: e.into(),
    }
}
fn visit(n: &str, over: bool, e: &str) -> Visit {
    Visit {
        n: n.into(),
        over,
        e: e.into(),
    }
}
fn clamp(h: f64) -> f64 {
    h.max(3.)
}
fn neighbors(k: &Knot, ids: &[String]) -> Vec<String> {
    let mut out = ids.to_vec();
    let n = k.code.len();
    for (q, v) in k.code.iter().enumerate() {
        if !ids.contains(&v.n) {
            continue;
        }
        for j in [cyc(q as isize - 1, n), (q + 1) % n] {
            if !out.contains(&k.code[j].n) {
                out.push(k.code[j].n.clone());
            }
        }
    }
    out
}
fn repair(mut k: Knot, focus: Vec<String>, seed: &str, iterations: usize) -> Option<Knot> {
    let mut count = stray(&k, usize::MAX);
    if count == 0 {
        return Some(k);
    }
    let mut rng = Random::seed(seed);
    let n = k.code.len();
    let edges = k
        .code
        .iter()
        .enumerate()
        .filter(|(i, v)| focus.contains(&v.n) || focus.contains(&k.code[(i + 1) % n].n))
        .map(|(_, v)| v.e.clone())
        .collect::<Vec<_>>();
    for _ in 0..iterations {
        let mut next = k.clone();
        if rng.next() < 0.6 || edges.is_empty() {
            let id = &focus[(rng.next() * focus.len() as f64).floor() as usize];
            let nd = next.nodes.get_mut(id).unwrap();
            nd.x += (rng.next() - 0.5) * 32.;
            nd.y += (rng.next() - 0.5) * 32.;
            nd.rot += (rng.next() - 0.5) * 0.8;
        } else {
            let e = &edges[(rng.next() * edges.len() as f64).floor() as usize];
            let ed = next.edges.get_mut(e).unwrap();
            ed.a = clamp(ed.a * (0.6 + 0.8 * rng.next()));
            ed.b = clamp(ed.b * (0.6 + 0.8 * rng.next()));
        }
        let c = stray(&next, usize::MAX);
        if c < count || (c == count && rng.next() < 0.25) {
            k = next;
            count = c;
            if count == 0 {
                return Some(k);
            }
        }
    }
    None
}
fn first_valid(
    candidates: impl IntoIterator<Item = Knot>,
    near: &[String],
    seed: &str,
) -> Option<Knot> {
    let mut first = None;
    for k in candidates {
        if valid(&k) {
            return Some(k);
        }
        if first.is_none() {
            first = Some(k);
        }
    }
    first.and_then(|k| {
        let focus = neighbors(&k, near);
        repair(k, focus, seed, 300)
    })
}
fn other(k: &Knot, i: usize) -> Option<usize> {
    k.code
        .iter()
        .enumerate()
        .position(|(q, v)| q != i && v.n == k.code[i].n)
}
fn pair(k: &Knot, i: usize) -> Option<(usize, usize, usize)> {
    let j = (i + 1) % k.code.len();
    let a = &k.code[i];
    let b = &k.code[j];
    if a.n == b.n || a.over != b.over || !k.nodes[&a.n].crossing() || !k.nodes[&b.n].crossing() {
        return None;
    }
    Some((j, other(k, i)?, other(k, j)?))
}
fn twist(k: &Knot, i: usize, side: f64, over: bool) -> Option<Move> {
    let e = &k.code[i].e;
    let c = cubics(k)[i];
    let (l, r) = split(c, 0.5);
    let m = l[3];
    let t = tangent(c, 0.5);
    let (ids, next_id) = new_ids(k, "nnnee");
    let (q1, q2, x, el, er) = (&ids[0], &ids[1], &ids[2], &ids[3], &ids[4]);
    let h = hint(k, e);
    let mut from = k.clone();
    from.next_id = next_id;
    from.nodes.insert(
        q1.clone(),
        pass(m, t, Some(h.clone()), over.then(|| x.clone())),
    );
    from.nodes
        .insert(q2.clone(), pass(m, t, Some(h), (!over).then(|| x.clone())));
    from.code[i].e = el.clone();
    from.code
        .splice(i + 1..i + 1, [visit(q1, false, e), visit(q2, false, er)]);
    from.edges.insert(el.clone(), handles(l));
    from.edges.insert(e.clone(), Edge { a: 0., b: 0. });
    from.edges.insert(er.clone(), handles(r));
    let normal = perp(t) * side;
    let d1 = norm(t + normal);
    let d2 = norm(t - normal);
    for size in [58., 44., 74., 34.] {
        let mut to = from.clone();
        to.nodes.remove(q1);
        to.nodes.remove(q2);
        to.nodes.insert(
            x.clone(),
            if over {
                crossing(m, d1, d2)
            } else {
                crossing(m, d2, d1)
            },
        );
        to.code[i + 1] = visit(x, over, e);
        to.code[i + 2] = visit(x, !over, er);
        to.edges.insert(e.clone(), Edge { a: size, b: size });
        if valid(&to) {
            return Some(mv(Kind::Twist, from, to));
        }
    }
    None
}
fn untwist(k: &Knot, i: usize) -> Option<Move> {
    let n = k.code.len();
    let j = (i + 1) % n;
    let v = &k.code[i];
    let w = &k.code[j];
    if v.n != w.n || !k.nodes[&v.n].crossing() {
        return None;
    }
    let cp = k.nodes[&v.n].pos();
    let (ids, next_id) = new_ids(k, "nn");
    let (p1, p2) = (&ids[0], &ids[1]);
    let make = |pos1, dir1, pos2, dir2, loop_edge, other_edge: Option<Edge>| {
        let mut to = k.clone();
        to.next_id = next_id;
        to.nodes.remove(&v.n);
        to.nodes.insert(
            p1.clone(),
            pass(pos1, dir1, None, v.over.then(|| v.n.clone())),
        );
        to.nodes.insert(
            p2.clone(),
            pass(pos2, dir2, None, (!v.over).then(|| v.n.clone())),
        );
        to.code[i].n = p1.clone();
        to.code[i].over = false;
        to.code[j].n = p2.clone();
        to.code[j].over = false;
        to.edges.insert(v.e.clone(), loop_edge);
        if let Some(ed) = other_edge {
            to.edges.insert(w.e.clone(), ed);
        }
        to
    };
    if n == 2 {
        let big = sample(cubics(k)[j], 32);
        let center = (big
            .iter()
            .map(|p| p.to_vec2())
            .fold(Vec2::ZERO, |a, b| a + b)
            / big.len() as f64)
            .to_point();
        let r = (big.iter().map(|p| p.distance(center)).sum::<f64>() / big.len() as f64).max(30.);
        let turn = big
            .windows(2)
            .map(|p| (p[0] - center).cross(p[1] - p[0]))
            .sum::<f64>();
        let sign = if turn >= 0. { 1. } else { -1. };
        let u = norm(cp - center);
        let h = 4. / 3. * r;
        let to = make(
            center + u * r,
            perp(u) * sign,
            center - u * r,
            perp(u) * (-sign),
            Edge { a: h, b: h },
            Some(Edge { a: h, b: h }),
        );
        return valid(&to).then(|| mv(Kind::Untwist, k.clone(), to));
    }
    let t = norm(direction(k, i) + direction(k, j));
    let cs = [8., 5., 12.].map(|h| {
        make(
            cp - t * h,
            t,
            cp + t * h,
            t,
            Edge {
                a: 2. * h / 3.,
                b: 2. * h / 3.,
            },
            None,
        )
    });
    first_valid(cs, &[p1.clone(), p2.clone()], &format!("R1-:{}", v.e))
        .map(|to| mv(Kind::Untwist, k.clone(), to))
}
fn push(k: &Knot, i: usize, j: usize, tf: f64, under: bool) -> Option<Move> {
    let cs = cubics(k);
    let (e_curve, f_curve) = (cs[i], cs[j]);
    let m = bez(e_curve, 0.5);
    let te = tangent(e_curve, 0.5);
    let f = bez(f_curve, tf);
    let td = tangent(f_curve, tf);
    let mut nf = perp(td);
    if nf.dot(f - m) < 0. {
        nf = -nf;
    }
    let parallel = te.dot(td) > 0.;
    let e = &k.code[i].e;
    let fe = &k.code[j].e;
    let (ids, next_id) = new_ids(k, "nnnnnneeee");
    let (pe1, pe2, pf1, pf2, x1, x2, ep, eq, fm, fb) = (
        &ids[0], &ids[1], &ids[2], &ids[3], &ids[4], &ids[5], &ids[6], &ids[7], &ids[8], &ids[9],
    );
    let e1 = sub(e_curve, 0., 0.4);
    let e2 = sub(e_curve, 0.4, 0.6);
    let e3 = sub(e_curve, 0.6, 1.);
    let lf = length(f_curve, 48);
    let l_at = length(sub(f_curve, 0., tf), 32);
    let p = e_curve[0];
    let q = e_curve[3];
    for width in [18., 13., 26.] {
        if l_at - width < 10. || lf - l_at - width < 10. {
            continue;
        }
        let f1 = at_length(f_curve, l_at - width);
        let f2 = at_length(f_curve, l_at + width);
        let fs = [
            sub(f_curve, 0., f1),
            sub(f_curve, f1, f2),
            sub(f_curve, f2, 1.),
        ];
        let first = if parallel { f1 } else { f2 };
        let second = if parallel { f2 } else { f1 };
        let id_at = |t| if t == f1 { x1 } else { x2 };
        let mut from = k.clone();
        from.next_id = next_id;
        let eh = hint(k, e);
        let fh = hint(k, fe);
        from.nodes.insert(
            pe1.clone(),
            pass(
                e1[3],
                tangent(e_curve, 0.4),
                Some(eh.clone()),
                (!under).then(|| id_at(first).clone()),
            ),
        );
        from.nodes.insert(
            pe2.clone(),
            pass(
                e2[3],
                tangent(e_curve, 0.6),
                Some(eh),
                (!under).then(|| id_at(second).clone()),
            ),
        );
        from.nodes.insert(
            pf1.clone(),
            pass(
                fs[0][3],
                tangent(f_curve, f1),
                Some(fh.clone()),
                under.then(|| x1.clone()),
            ),
        );
        from.nodes.insert(
            pf2.clone(),
            pass(
                fs[1][3],
                tangent(f_curve, f2),
                Some(fh),
                under.then(|| x2.clone()),
            ),
        );
        for (id, c) in [
            (ep, e1),
            (e, e2),
            (eq, e3),
            (fe, fs[0]),
            (fm, fs[1]),
            (fb, fs[2]),
        ] {
            from.edges.insert(id.clone(), handles(c));
        }
        from.code[i].e = ep.clone();
        let ev = [visit(pe1, false, e), visit(pe2, false, eq)];
        let fv = [visit(pf1, false, fm), visit(pf2, false, fb)];
        if i > j {
            from.code.splice(i + 1..i + 1, ev);
            from.code.splice(j + 1..j + 1, fv);
        } else {
            from.code.splice(j + 1..j + 1, fv);
            from.code.splice(i + 1..i + 1, ev);
        }
        let outward = |t| {
            let n = perp(tangent(f_curve, t));
            if n.dot(nf) >= 0. { n } else { -n }
        };
        let node_at = |t, ed| {
            let p = bez(f_curve, t);
            let fd = tangent(f_curve, t);
            if under {
                crossing(p, fd, ed)
            } else {
                crossing(p, ed, fd)
            }
        };
        let first_pos = bez(f_curve, first);
        let second_pos = bez(f_curve, second);
        for h in [34., 24., 46.] {
            let mut to = from.clone();
            for id in [pe1, pe2, pf1, pf2] {
                to.nodes.remove(id);
            }
            to.nodes
                .insert(id_at(first).clone(), node_at(first, outward(first)));
            to.nodes
                .insert(id_at(second).clone(), node_at(second, -outward(second)));
            for v in &mut to.code {
                if &v.n == pe1 {
                    v.n = id_at(first).clone();
                    v.over = !under;
                } else if &v.n == pe2 {
                    v.n = id_at(second).clone();
                    v.over = !under;
                } else if &v.n == pf1 {
                    v.n = x1.clone();
                    v.over = under;
                } else if &v.n == pf2 {
                    v.n = x2.clone();
                    v.over = under;
                }
            }
            let dp = p.distance(first_pos);
            let dq = q.distance(second_pos);
            to.edges.insert(
                ep.clone(),
                Edge {
                    a: clamp(k.edges[e].a.min(dp * 0.6)),
                    b: clamp(dp / 3.),
                },
            );
            to.edges.insert(e.clone(), Edge { a: h, b: h });
            to.edges.insert(
                eq.clone(),
                Edge {
                    a: clamp(dq / 3.),
                    b: clamp(k.edges[e].b.min(dq * 0.6)),
                },
            );
            if valid(&to) {
                return Some(Move {
                    kind: Kind::Push,
                    from,
                    to,
                    meets: Some(f),
                });
            }
        }
    }
    None
}
fn pushes(k: &Knot, i: usize, under: bool) -> Vec<Move> {
    let cs = cubics(k);
    let m = bez(cs[i], 0.5);
    let normal = perp(tangent(cs[i], 0.5));
    let samples = cs.iter().map(|c| sample(*c, 16)).collect::<Vec<_>>();
    let mut out = Vec::new();
    for side in [1., -1.] {
        let far = m + normal * (side * 250.);
        let mut hit = None::<(f64, usize, f64)>;
        for (j, s) in samples.iter().enumerate() {
            for q in 0..16 {
                if let Some((t, u)) = segment_hit(m, far, s[q], s[q + 1]) {
                    if t * 250. > 4. && hit.is_none_or(|h| t < h.0) {
                        hit = Some((t, j, (q as f64 + u) / 16.));
                    }
                }
            }
        }
        if let Some((_, j, tf)) = hit {
            if j != i {
                if let Some(m) = push(k, i, j, tf, under) {
                    out.push(m);
                }
            }
        }
    }
    out
}
fn pull(k: &Knot, i: usize) -> Option<Move> {
    let n = k.code.len();
    let (j, ox, oy) = pair(k, i)?;
    if (ox + 1) % n != oy && (oy + 1) % n != ox {
        return None;
    }
    let vx = &k.code[i];
    let vy = &k.code[j];
    let xp = k.nodes[&vx.n].pos();
    let yp = k.nodes[&vy.n].pos();
    let cs = cubics(k);
    let line = norm(yp - xp);
    let bulge = bez(cs[i], 0.5) - xp.lerp(yp, 0.5);
    let mut far = bulge - line * bulge.dot(line);
    if far.hypot() < 1. {
        let d = direction(k, i);
        far = d - line * d.dot(line);
    }
    let home = -norm(far);
    let (ids, next_id) = new_ids(k, "nnnn");
    let (xs, ys, xt, yt) = (&ids[0], &ids[1], &ids[2], &ids[3]);
    let forward = (ox + 1) % n == oy;
    let mut tpts = sample(cs[if forward { ox } else { oy }], 12);
    if !forward {
        tpts.reverse();
    }
    let mut shapes = Vec::new();
    for c in [16., 11., 24.] {
        let off = tpts
            .iter()
            .enumerate()
            .map(|(q, p)| {
                let a = tpts[q.saturating_sub(1)];
                let b = tpts[(q + 1).min(tpts.len() - 1)];
                let mut n = perp(norm(b - a));
                if n.dot(home) < 0. {
                    n = -n;
                }
                *p + n * c
            })
            .collect::<Vec<_>>();
        let d1 = norm(off[1] - off[0]);
        let d2 = norm(off[off.len() - 1] - off[off.len() - 2]);
        let x = off[0];
        let y = off[off.len() - 1];
        shapes.push((x, y, d1, d2, fit(x, d1, y, d2, &off[1..off.len() - 1], 3.)));
    }
    for c in [18., 12., 26.] {
        let x = xp + home * c;
        let y = yp + home * c;
        let d = norm(y - x);
        let h = clamp(x.distance(y) / 3.);
        shapes.push((x, y, d, d, Edge { a: h, b: h }));
    }
    let candidates = shapes.into_iter().map(|(x, y, d1, d2, mid)| {
        let mut to = k.clone();
        to.next_id = next_id;
        to.nodes.remove(&vx.n);
        to.nodes.remove(&vy.n);
        to.nodes
            .insert(xs.clone(), pass(x, d1, None, vx.over.then(|| vx.n.clone())));
        to.nodes
            .insert(ys.clone(), pass(y, d2, None, vx.over.then(|| vy.n.clone())));
        to.nodes.insert(
            xt.clone(),
            pass(xp, direction(k, ox), None, (!vx.over).then(|| vx.n.clone())),
        );
        to.nodes.insert(
            yt.clone(),
            pass(yp, direction(k, oy), None, (!vx.over).then(|| vy.n.clone())),
        );
        for (q, id) in [(i, xs), (j, ys), (ox, xt), (oy, yt)] {
            to.code[q].n = id.clone();
            to.code[q].over = false;
        }
        to.edges.insert(vx.e.clone(), mid);
        to
    });
    first_valid(
        candidates,
        &[xs.clone(), ys.clone()],
        &format!("R2-:{}", vx.e),
    )
    .map(|to| mv(Kind::Pull, k.clone(), to))
}
#[derive(Clone, Copy)]
struct Triangle {
    i: usize,
    oa: usize,
    ob: usize,
    a2: usize,
    b2: usize,
}
fn triangles(k: &Knot, i: usize) -> Vec<Triangle> {
    let n = k.code.len();
    let Some((j, oa, ob)) = pair(k, i) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for da in [-1, 1] {
        for db in [-1, 1] {
            let a2 = cyc(oa as isize + da, n);
            let b2 = cyc(ob as isize + db, n);
            if a2 == b2 || [i, j].contains(&a2) || [i, j].contains(&b2) {
                continue;
            }
            let c = &k.code[a2].n;
            if c == &k.code[b2].n && k.nodes[c].crossing() && c != &k.code[i].n && c != &k.code[j].n
            {
                out.push(Triangle { i, oa, ob, a2, b2 });
            }
        }
    }
    out
}
enum Fix {
    Fit(usize, Vec<Point>),
    Set(usize, Edge),
}
struct Placement {
    a: Node,
    b: Node,
    c: Option<Node>,
    fixes: Vec<Fix>,
}
fn inner(c: Cubic) -> Vec<Point> {
    sample(c, 8)[1..8].to_vec()
}
fn apply_fix(k: &mut Knot, f: Fix) {
    let n = k.code.len();
    let (slot, ed) = match f {
        Fix::Set(slot, ed) => (slot, ed),
        Fix::Fit(slot, pts) => (
            slot,
            fit(
                position(k, slot),
                direction(k, slot),
                position(k, (slot + 1) % n),
                direction(k, (slot + 1) % n),
                &pts,
                3.,
            ),
        ),
    };
    k.edges.insert(k.code[slot].e.clone(), ed);
}
fn slide(k: &Knot, tri: Triangle, crossing_mode: bool) -> Option<Move> {
    let Triangle { i, oa, ob, a2, b2 } = tri;
    let n = k.code.len();
    let j = (i + 1) % n;
    let va = &k.code[i];
    let vb = &k.code[j];
    let a = &k.nodes[&va.n];
    let b = &k.nodes[&vb.n];
    let cid = &k.code[a2].n;
    let c = &k.nodes[cid];
    let cp = c.pos();
    let cs = cubics(k);
    let mut placements = Vec::new();
    if crossing_mode {
        let smid = cs[i];
        let len = length(smid, 24);
        let tc = closest(smid, cp, 0., 1., 64).0.clamp(0.2, 0.8);
        let lc = length(sub(smid, 0., tc), 24);
        let width = (len / 5.).min(12.);
        let tb = at_length(smid, (lc - width).max(0.));
        let ta = at_length(smid, (lc + width).min(len));
        let m = bez(smid, tc);
        let mut normal = perp(tangent(smid, tc));
        if normal.dot(cp - m) > 0. {
            normal = -normal;
        }
        let on_s = |v: usize, o: usize, t| {
            let p = bez(smid, t);
            let sd = tangent(smid, t);
            let rel = angle(direction(k, o)) - angle(direction(k, v));
            let od = polar(angle(sd) + rel);
            if k.code[v].over {
                crossing(p, sd, od)
            } else {
                crossing(p, od, sd)
            }
        };
        for dist in [22., 16., 30.] {
            let mut nd = c.clone();
            nd.set_pos(m + normal * dist);
            let before = cyc(i as isize - 1, n);
            let mut p1 = inner(cs[before]);
            p1.extend(inner(sub(smid, 0., tb)));
            let mut p2 = inner(sub(smid, ta, 1.));
            p2.extend(inner(cs[j]));
            placements.push(Placement {
                a: on_s(i, oa, ta),
                b: on_s(j, ob, tb),
                c: Some(nd),
                fixes: vec![
                    Fix::Fit(before, p1),
                    Fix::Set(i, handles(sub(smid, tb, ta))),
                    Fix::Fit(j, p2),
                ],
            });
        }
    } else {
        let sliding =
            |s_visit: usize, o: usize, c_visit: usize, d: f64| -> Option<(Node, Vec<Fix>)> {
                let forward = (o + 1) % n == c_visit;
                let beyond = cs[if forward {
                    c_visit
                } else {
                    cyc(c_visit as isize - 1, n)
                }];
                let len = length(beyond, 24);
                if len < 14. {
                    return None;
                }
                let dd = d.min(0.8 * len);
                let t = at_length(beyond, if forward { dd } else { len - dd });
                let p = bez(beyond, t);
                let td = tangent(beyond, t);
                let rel = angle(direction(k, s_visit)) - angle(direction(k, o));
                let sd = polar(angle(td) + rel);
                let node = if k.code[s_visit].over {
                    crossing(p, sd, td)
                } else {
                    crossing(p, td, sd)
                };
                let fixes = if forward {
                    let prev = cyc(o as isize - 1, n);
                    let mut ps = inner(cs[prev]);
                    ps.extend(inner(cs[o]));
                    vec![
                        Fix::Fit(prev, ps),
                        Fix::Set(o, handles(sub(beyond, 0., t))),
                        Fix::Set(c_visit, handles(sub(beyond, t, 1.))),
                    ]
                } else {
                    let mut ps = inner(cs[c_visit]);
                    ps.extend(inner(cs[o]));
                    vec![
                        Fix::Set(cyc(c_visit as isize - 1, n), handles(sub(beyond, 0., t))),
                        Fix::Set(c_visit, handles(sub(beyond, t, 1.))),
                        Fix::Fit(o, ps),
                    ]
                };
                Some((node, fixes))
            };
        for f in [1., 0.7, 0.45] {
            let Some((aa, mut fixes)) = sliding(i, oa, a2, f * cp.distance(a.pos())) else {
                continue;
            };
            let Some((bb, ff)) = sliding(j, ob, b2, f * cp.distance(b.pos())) else {
                continue;
            };
            fixes.extend(ff);
            placements.push(Placement {
                a: aa,
                b: bb,
                c: None,
                fixes,
            });
        }
        for f in [1., 0.6, 0.4] {
            let mut aa = a.clone();
            aa.set_pos(cp + (cp - a.pos()) * f);
            let mut bb = b.clone();
            bb.set_pos(cp + (cp - b.pos()) * f);
            placements.push(Placement {
                a: aa,
                b: bb,
                c: None,
                fixes: Vec::new(),
            });
        }
    }
    let moved = [va.n.clone(), vb.n.clone(), cid.clone()];
    let candidates = placements.into_iter().map(|p| {
        let mut to = k.clone();
        for (a, b) in [(i, j), (oa, a2), (ob, b2)] {
            let an = to.code[a].n.clone();
            let ao = to.code[a].over;
            to.code[a].n = to.code[b].n.clone();
            to.code[a].over = to.code[b].over;
            to.code[b].n = an;
            to.code[b].over = ao;
        }
        to.nodes.insert(va.n.clone(), p.a);
        to.nodes.insert(vb.n.clone(), p.b);
        if let Some(c) = p.c {
            to.nodes.insert(cid.clone(), c);
        }
        for q in 0..n {
            let n1 = &to.code[q].n;
            let n2 = &to.code[(q + 1) % n].n;
            if !moved.contains(n1) && !moved.contains(n2) {
                continue;
            }
            let d = to.nodes[n1].pos().distance(to.nodes[n2].pos());
            let ed = to.edges.get_mut(&to.code[q].e).unwrap();
            if moved.contains(n1) {
                ed.a = clamp(d / 3.);
            }
            if moved.contains(n2) {
                ed.b = clamp(d / 3.);
            }
        }
        for f in p.fixes {
            apply_fix(&mut to, f);
        }
        to
    });
    first_valid(candidates, &moved, &format!("R3:{}", va.e))
        .map(|to| mv(Kind::Slide, k.clone(), to))
}
pub fn crossing_moves(k: &Knot, id: &str) -> Vec<Move> {
    (0..k.code.len())
        .flat_map(|i| triangles(k, i))
        .filter(|t| k.code[t.a2].n == id)
        .filter_map(|t| slide(k, t, true))
        .map(conform)
        .collect()
}
pub fn moves(k: &Knot, e: &str, under: bool, only: Option<&[Kind]>) -> Vec<Move> {
    let Some(i) = edge_index(k, e) else {
        return Vec::new();
    };
    let want = |kind| only.is_none_or(|kinds| kinds.contains(&kind));
    let mut out = Vec::new();
    if want(Kind::Twist) {
        out.extend(twist(k, i, 1., !under));
        out.extend(twist(k, i, -1., !under));
    }
    if want(Kind::Untwist) {
        out.extend(untwist(k, i));
    }
    if want(Kind::Push) {
        out.extend(pushes(k, i, under));
    }
    if want(Kind::Pull) {
        out.extend(pull(k, i));
    }
    if want(Kind::Slide) {
        out.extend(
            triangles(k, i)
                .into_iter()
                .filter_map(|t| slide(k, t, false)),
        );
    }
    out.into_iter().map(conform).collect()
}
fn merge_run(k: &Knot, s: usize, count: usize) -> Option<Knot> {
    let n = k.code.len();
    if count == 0 || count >= n {
        return None;
    }
    let end = (s + count + 1) % n;
    let run = (1..=count).map(|q| (s + q) % n).collect::<Vec<_>>();
    let hints = run
        .iter()
        .map(|q| k.nodes[&k.code[*q].n].hint.as_ref())
        .collect::<Vec<_>>();
    let mut keep = k.code[s].e.clone();
    let data = if hints[0].is_some() && hints.iter().all(|h| *h == hints[0]) {
        let h = hints[0].unwrap();
        keep = h.e.clone();
        Edge { a: h.a, b: h.b }
    } else {
        let cs = cubics(k);
        let mut pts = Vec::new();
        for q in 0..=count {
            let ps = sample(cs[(s + q) % n], 8);
            pts.extend_from_slice(&ps[if q == 0 { 1 } else { 0 }..8]);
        }
        fit(
            position(k, s),
            direction(k, s),
            position(k, end),
            direction(k, end),
            &pts,
            3.,
        )
    };
    let mut out = k.clone();
    for q in &run {
        out.nodes.remove(&k.code[*q].n);
        out.edges.remove(&k.code[*q].e);
    }
    out.edges.remove(&k.code[s].e);
    out.edges.insert(keep.clone(), data);
    out.code = k
        .code
        .iter()
        .enumerate()
        .filter(|(q, _)| !run.contains(q))
        .map(|(q, v)| {
            let mut v = v.clone();
            if q == s {
                v.e = keep.clone();
            }
            v
        })
        .collect();
    Some(out)
}
pub fn cleanup(k0: &Knot) -> Knot {
    let mut k = k0.clone();
    let mut failed = HashSet::new();
    for _ in 0..100 {
        let n = k.code.len();
        let n_cross = k.code.iter().filter(|v| k.nodes[&v.n].crossing()).count();
        let mut progressed = false;
        for hinted in [true, false] {
            for s in 0..n {
                if progressed {
                    break;
                }
                let next = (s + 1) % n;
                if k.nodes[&k.code[next].n].crossing() {
                    continue;
                }
                if n_cross > 0 && !k.nodes[&k.code[s].n].crossing() {
                    continue;
                }
                let mut count = 0;
                while count < n - 1 && !k.nodes[&k.code[(s + 1 + count) % n].n].crossing() {
                    let nd = &k.nodes[&k.code[(s + 1 + count) % n].n];
                    if hinted && nd.hint.is_none() {
                        break;
                    }
                    count += 1;
                }
                if n_cross == 0 {
                    count = count.min(n - 2);
                }
                if count == 0 {
                    continue;
                }
                let key = format!("{}:{count}:{hinted}", k.code[s].n);
                if failed.contains(&key) {
                    continue;
                }
                let mut merged = merge_run(&k, s, count);
                if let Some(m) = merged.as_ref() {
                    if !valid(m) {
                        let ends = [k.code[s].n.clone(), k.code[(s + count + 1) % n].n.clone()];
                        let near = neighbors(m, &ends);
                        merged = repair(m.clone(), near, &format!("merge:{key}"), 200);
                    }
                }
                if let Some(m) = merged.filter(valid) {
                    k = m;
                    progressed = true;
                } else {
                    failed.insert(key);
                }
            }
            if progressed {
                break;
            }
        }
        if !progressed {
            break;
        }
    }
    k
}
fn conform(mut m: Move) -> Move {
    let to = &m.to;
    let done = cleanup(to);
    if done == *to {
        return m;
    }
    let mut out = to.clone();
    let kept = to
        .code
        .iter()
        .enumerate()
        .filter(|(_, v)| done.nodes.contains_key(&v.n))
        .map(|(q, _)| q)
        .collect::<Vec<_>>();
    let cs = cubics(&done);
    let n = to.code.len();
    for (r, s) in kept.into_iter().enumerate() {
        let id = &to.code[s].n;
        out.nodes.insert(id.clone(), done.nodes[id].clone());
        let mut run = Vec::new();
        let mut q = (s + 1) % n;
        while !done.nodes.contains_key(&to.code[q].n) {
            run.push(q);
            q = (q + 1) % n;
        }
        let de = &done.code[r].e;
        if run.is_empty() {
            out.edges.insert(to.code[s].e.clone(), done.edges[de]);
            continue;
        }
        let mut prev = 0f64;
        let ts = run
            .iter()
            .map(|q| {
                prev = closest(cs[r], to.nodes[&to.code[*q].n].pos(), 0., 1., 64)
                    .0
                    .max(prev + 0.02)
                    .min(0.98);
                prev
            })
            .collect::<Vec<_>>();
        let h = hint(&done, de);
        for (p, q) in run.iter().enumerate() {
            let nd = &to.nodes[&to.code[*q].n];
            out.nodes.insert(
                to.code[*q].n.clone(),
                pass(
                    bez(cs[r], ts[p]),
                    tangent(cs[r], ts[p]),
                    Some(h.clone()),
                    nd.ghost_of.clone(),
                ),
            );
        }
        let mut bounds = vec![0.];
        bounds.extend(&ts);
        bounds.push(1.);
        for (p, q) in std::iter::once(s).chain(run).enumerate() {
            out.edges.insert(
                to.code[q].e.clone(),
                handles(sub(cs[r], bounds[p], bounds[p + 1])),
            );
        }
    }
    if valid(&out) {
        m.to = out;
    }
    m
}
