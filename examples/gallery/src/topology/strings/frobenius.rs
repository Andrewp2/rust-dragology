//! Ordered port-graph rewrites for Frobenius algebras. The tangle variant
//! additionally permits commutativity but rejects rewrites making directed cycles.
use super::*;
use dragology::closest;
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Input,
    Output,
    Mult,
    Comult,
    Unit,
    Counit,
}
impl Kind {
    fn arity(self) -> (usize, usize) {
        match self {
            Self::Input => (0, 1),
            Self::Output => (1, 0),
            Self::Mult => (2, 1),
            Self::Comult => (1, 2),
            Self::Unit => (0, 1),
            Self::Counit => (1, 0),
        }
    }
    fn internal(self) -> bool {
        !matches!(self, Self::Input | Self::Output)
    }
}
#[derive(Clone, Debug, PartialEq)]
struct Node {
    kind: Kind,
    p: Point,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Port {
    node: String,
    i: usize,
}
fn port(node: &str, i: usize) -> Port {
    Port {
        node: node.into(),
        i,
    }
}
#[derive(Clone, Debug, PartialEq)]
struct Wire {
    from: Port,
    to: Port,
}
#[derive(Clone, Debug, PartialEq)]
struct State {
    tangle: bool,
    puzzle: usize,
    nodes: BTreeMap<String, Node>,
    wires: BTreeMap<usize, Wire>,
}
impl State {
    fn new(tangle: bool, puzzle: usize) -> Self {
        Self {
            tangle,
            puzzle,
            nodes: BTreeMap::new(),
            wires: BTreeMap::new(),
        }
    }
    fn add(&mut self, id: &str, k: Kind, x: f64, y: f64) {
        self.nodes.insert(
            id.into(),
            Node {
                kind: k,
                p: Point::new(x, y),
            },
        );
    }
    fn wire(&mut self, a: &str, ai: usize, b: &str, bi: usize) {
        let i = self.wires.keys().next_back().map_or(0, |i| i + 1);
        self.wires.insert(
            i,
            Wire {
                from: port(a, ai),
                to: port(b, bi),
            },
        );
    }
    fn kind(&self, id: &str) -> Option<Kind> {
        self.nodes.get(id).map(|n| n.kind)
    }
    fn from(&self, id: &str, i: usize) -> usize {
        *self
            .wires
            .iter()
            .find(|(_, w)| w.from == port(id, i))
            .expect("well-formed output port")
            .0
    }
    fn to(&self, id: &str, i: usize) -> usize {
        *self
            .wires
            .iter()
            .find(|(_, w)| w.to == port(id, i))
            .expect("well-formed input port")
            .0
    }
    fn pos(&self, id: &str) -> Point {
        let n = &self.nodes[id];
        if !self.tangle && n.kind == Kind::Input {
            Point::new(n.p.x, 34.)
        } else if !self.tangle && n.kind == Kind::Output {
            Point::new(
                n.p.x,
                34. + 52. * (self.nodes.values().filter(|n| n.kind.internal()).count() + 1) as f64,
            )
        } else {
            n.p
        }
    }
    fn neighbors(&self, id: &str) -> Vec<String> {
        let mut seen = BTreeSet::new();
        self.wires
            .values()
            .filter_map(|w| {
                if w.from.node == id {
                    Some(w.to.node.clone())
                } else if w.to.node == id {
                    Some(w.from.node.clone())
                } else {
                    None
                }
            })
            .filter(|id| seen.insert(id.clone()))
            .collect()
    }
    fn tidy(mut self, floating: Option<&str>) -> Self {
        if self.tangle {
            return self;
        }
        let mut ids: Vec<_> = self
            .nodes
            .iter()
            .filter(|(id, n)| n.kind.internal() && floating != Some(id.as_str()))
            .map(|(id, n)| (id.clone(), n.p.y))
            .collect();
        ids.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
        let fly = floating.and_then(|id| self.nodes.get(id)).map(|n| n.p.y);
        for (slot, (id, y)) in ids.into_iter().enumerate() {
            self.nodes.get_mut(&id).unwrap().p.y =
                34. + 52. * (slot + 1 + usize::from(fly.is_some_and(|f| f < y))) as f64;
        }
        self
    }
    fn acyclic(&self) -> bool {
        let mut degree: BTreeMap<_, usize> = self.nodes.keys().map(|id| (id.clone(), 0)).collect();
        for w in self.wires.values() {
            *degree.get_mut(&w.to.node).unwrap() += 1
        }
        let mut todo: Vec<_> = degree
            .iter()
            .filter(|(_, d)| **d == 0)
            .map(|(id, _)| id.clone())
            .collect();
        let mut count = 0;
        while let Some(id) = todo.pop() {
            count += 1;
            for w in self.wires.values().filter(|w| w.from.node == id) {
                let d = degree.get_mut(&w.to.node).unwrap();
                *d -= 1;
                if *d == 0 {
                    todo.push(w.to.node.clone())
                }
            }
        }
        count == self.nodes.len()
    }
    #[cfg(test)]
    fn valid(&self) -> bool {
        self.nodes.iter().all(|(id, n)| {
            let (ins, outs) = n.kind.arity();
            (0..ins).all(|i| self.wires.values().filter(|w| w.to == port(id, i)).count() == 1)
                && (0..outs).all(|i| {
                    self.wires
                        .values()
                        .filter(|w| w.from == port(id, i))
                        .count()
                        == 1
                })
        }) && self.wires.values().all(|w| {
            self.nodes.contains_key(&w.from.node)
                && self.nodes.contains_key(&w.to.node)
                && w.from.i < self.nodes[&w.from.node].kind.arity().1
                && w.to.i < self.nodes[&w.to.node].kind.arity().0
        })
    }
}
fn settle(s: &mut State, a: &str, b: &str, upper: Option<&str>) {
    let link = s
        .wires
        .values()
        .find(|w| (w.from.node == a && w.to.node == b) || (w.from.node == b && w.to.node == a))
        .unwrap()
        .clone();
    for id in [a, b] {
        let other: Vec<_> = s
            .neighbors(id)
            .into_iter()
            .filter(|n| n != a && n != b)
            .collect();
        if !other.is_empty() {
            let y = other.iter().map(|n| s.pos(n).y).sum::<f64>() / other.len() as f64;
            s.nodes.get_mut(id).unwrap().p.y = y;
        }
    }
    let ax = s.pos(a).x;
    let bx = s.pos(b).x;
    let mid = (ax + bx) / 2.;
    let half = ((ax - bx).abs() / 2.).max(24.);
    s.nodes.get_mut(&link.from.node).unwrap().p.x = (mid - half).clamp(38., 422.);
    s.nodes.get_mut(&link.to.node).unwrap().p.x = (mid + half).clamp(38., 422.);
    if let Some(u) = upper {
        let l = if u == a { b } else { a };
        if s.pos(u).y > s.pos(l).y - 30. {
            let m = (s.pos(u).y + s.pos(l).y) / 2.;
            s.nodes.get_mut(u).unwrap().p.y = m - 15.;
            s.nodes.get_mut(l).unwrap().p.y = m + 15.;
        }
    }
}
fn unit(s: &State, u: &str, m: &str) -> State {
    let dual = s.kind(u) == Some(Kind::Counit);
    let feed = if dual { s.to(u, 0) } else { s.from(u, 0) };
    let k = if dual {
        s.wires[&feed].from.i
    } else {
        s.wires[&feed].to.i
    };
    let other = if dual {
        s.from(m, 1 - k)
    } else {
        s.to(m, 1 - k)
    };
    let out = if dual { s.to(m, 0) } else { s.from(m, 0) };
    let mut t = s.clone();
    if dual {
        t.wires.get_mut(&out).unwrap().to = s.wires[&other].to.clone()
    } else {
        t.wires.get_mut(&out).unwrap().from = s.wires[&other].from.clone()
    }
    t.wires.remove(&other);
    t.wires.remove(&feed);
    t.nodes.remove(u);
    t.nodes.remove(m);
    t.tidy(None)
}
fn port_end(s: &State, p: &Port, out: bool) -> (Point, Vec2) {
    let center = s.pos(&p.node);
    let kind = s.nodes[&p.node].kind;
    let count = if out { kind.arity().1 } else { kind.arity().0 };
    if !s.tangle {
        let x = if kind.internal() {
            (p.i as f64 - (count as f64 - 1.) / 2.) * 9.
        } else {
            0.
        };
        return (
            center + Vec2::new(x, 0.),
            Vec2::new(0., if out { 1. } else { -1. }),
        );
    }
    if !kind.internal() {
        return (center, Vec2::new(if out { 1. } else { -1. }, 0.));
    }
    let offset = if count == 1 {
        0.
    } else if p.i == 0 {
        -1.
    } else {
        1.
    };
    let angle = if out {
        offset * 35f64.to_radians()
    } else {
        std::f64::consts::PI - offset * 35f64.to_radians()
    };
    let dir = Vec2::new(angle.cos(), angle.sin());
    (center + dir * 7., dir)
}
fn wire_geometry(s: &State, w: &Wire) -> [Point; 4] {
    let (a, ad) = port_end(s, &w.from, true);
    let (b, bd) = port_end(s, &w.to, false);
    let stiff = if s.tangle {
        (a.distance(b) / 3.).max(22.)
    } else {
        ((b.y - a.y).abs() / 2.).max(20.)
    };
    [a, a + ad * stiff, b + bd * stiff, b]
}
fn assoc(s: &State, sat: &str, hub: &str) -> State {
    let dual = s.kind(sat) == Some(Kind::Comult);
    let link = if dual { s.to(sat, 0) } else { s.from(sat, 0) };
    let k = if dual {
        s.wires[&link].from.i
    } else {
        s.wires[&link].to.i
    };
    let ports = |k| {
        if k == 0 {
            [port(sat, 0), port(sat, 1), port(hub, 1)]
        } else {
            [port(hub, 0), port(sat, 0), port(sat, 1)]
        }
    };
    let ids = ports(k).map(|p| {
        if dual {
            s.from(&p.node, p.i)
        } else {
            s.to(&p.node, p.i)
        }
    });
    let xs = ids.map(|id| {
        port_end(
            s,
            if dual {
                &s.wires[&id].to
            } else {
                &s.wires[&id].from
            },
            !dual,
        )
        .0
        .x
    });
    let mut t = s.clone();
    for (id, p) in ids.into_iter().zip(ports(1 - k)) {
        if dual {
            t.wires.get_mut(&id).unwrap().from = p
        } else {
            t.wires.get_mut(&id).unwrap().to = p
        }
    }
    if dual {
        t.wires.get_mut(&link).unwrap().from = port(hub, 1 - k)
    } else {
        t.wires.get_mut(&link).unwrap().to = port(hub, 1 - k)
    }
    if s.tangle {
        settle(&mut t, sat, hub, None)
    } else {
        let x = if k == 0 {
            (xs[1] + xs[2]) / 2.
        } else {
            (xs[0] + xs[1]) / 2.
        };
        t.nodes.get_mut(sat).unwrap().p.x = x.clamp(30., 290.);
        t.nodes.get_mut(hub).unwrap().p.x = if k == 0 {
            (xs[0] + x) / 2.
        } else {
            (x + xs[2]) / 2.
        }
        .clamp(30., 290.);
    }
    t.tidy(None)
}
#[derive(Clone, Copy, Debug, PartialEq)]
enum Form {
    H,
    Left,
    Right,
}
fn form(s: &State, m: &str, c: &str) -> Option<Form> {
    let links: Vec<_> = s
        .wires
        .values()
        .filter(|w| (w.from.node == m && w.to.node == c) || (w.from.node == c && w.to.node == m))
        .collect();
    if links.len() != 1 {
        return None;
    }
    let w = links[0];
    if w.from.node == m {
        Some(Form::H)
    } else if w.from.i == 1 && w.to.i == 0 {
        Some(Form::Left)
    } else if w.from.i == 0 && w.to.i == 1 {
        Some(Form::Right)
    } else {
        None
    }
}
fn frobenius(s: &State, m: &str, c: &str, to: Form) -> State {
    let from = form(s, m, c).unwrap();
    let [a, b, cc, d] = match from {
        Form::H => [s.to(m, 0), s.to(m, 1), s.from(c, 0), s.from(c, 1)],
        Form::Left => [s.to(c, 0), s.to(m, 1), s.from(c, 0), s.from(m, 0)],
        Form::Right => [s.to(m, 0), s.to(c, 0), s.from(m, 0), s.from(c, 1)],
    };
    let link = *s
        .wires
        .iter()
        .find(|(_, w)| (w.from.node == m && w.to.node == c) || (w.from.node == c && w.to.node == m))
        .unwrap()
        .0;
    let [ax, bx, cx, dx] = [
        port_end(s, &s.wires[&a].from, true).0.x,
        port_end(s, &s.wires[&b].from, true).0.x,
        port_end(s, &s.wires[&cc].to, false).0.x,
        port_end(s, &s.wires[&d].to, false).0.x,
    ];
    let (ap, bp, cp, dp, lp) = match to {
        Form::H => (
            port(m, 0),
            port(m, 1),
            port(c, 0),
            port(c, 1),
            Wire {
                from: port(m, 0),
                to: port(c, 0),
            },
        ),
        Form::Left => (
            port(c, 0),
            port(m, 1),
            port(c, 0),
            port(m, 0),
            Wire {
                from: port(c, 1),
                to: port(m, 0),
            },
        ),
        Form::Right => (
            port(m, 0),
            port(c, 0),
            port(m, 0),
            port(c, 1),
            Wire {
                from: port(c, 0),
                to: port(m, 1),
            },
        ),
    };
    let mut t = s.clone();
    t.wires.get_mut(&a).unwrap().to = ap;
    t.wires.get_mut(&b).unwrap().to = bp;
    t.wires.get_mut(&cc).unwrap().from = cp;
    t.wires.get_mut(&d).unwrap().from = dp;
    t.wires.insert(link, lp);
    if s.tangle {
        settle(
            &mut t,
            m,
            c,
            match to {
                Form::H => None,
                Form::Left => Some(c),
                Form::Right => Some(m),
            },
        )
    } else {
        let mid = (s.pos(m).y + s.pos(c).y) / 2.;
        let (mx, cxx) = match to {
            Form::H => {
                let v = (ax + bx + cx + dx) / 4.;
                (v, v)
            }
            Form::Left => ((bx + dx) / 2., (ax + cx) / 2.),
            Form::Right => ((ax + cx) / 2., (bx + dx) / 2.),
        };
        t.nodes.get_mut(m).unwrap().p = Point::new(
            mx.clamp(30., 290.),
            mid + if to == Form::H { -1. } else { 1. },
        );
        t.nodes.get_mut(c).unwrap().p = Point::new(
            cxx.clamp(30., 290.),
            mid + if to == Form::H { 1. } else { -1. },
        );
    }
    t.tidy(None)
}
fn planar_graph_key(s: &State) -> String {
    let mut labels: BTreeMap<String, String> = s
        .nodes
        .iter()
        .map(|(id, n)| {
            (
                id.clone(),
                if !n.kind.internal() {
                    format!("B:{id}")
                } else {
                    format!(
                        "G:{}",
                        match n.kind {
                            Kind::Mult => "mult",
                            Kind::Comult => "comult",
                            Kind::Unit => "unit",
                            Kind::Counit => "counit",
                            _ => unreachable!(),
                        }
                    )
                },
            )
        })
        .collect();
    let edge = |w: &Wire, labels: &BTreeMap<String, String>| {
        format!(
            "{}.{}>{}.{}",
            labels[&w.from.node], w.from.i, labels[&w.to.node], w.to.i
        )
    };
    for _ in 0..4 {
        labels = s
            .nodes
            .keys()
            .map(|id| {
                let mut neighbors: Vec<_> = s
                    .wires
                    .values()
                    .filter(|w| w.from.node == *id || w.to.node == *id)
                    .map(|w| {
                        format!(
                            "{}{}",
                            if w.from.node == *id { "v" } else { "^" },
                            edge(w, &labels)
                        )
                    })
                    .collect();
                neighbors.sort();
                (
                    id.clone(),
                    format!("{}({})", labels[id], neighbors.join(",")),
                )
            })
            .collect();
    }
    let mut nodes: Vec<_> = labels.values().cloned().collect();
    nodes.sort();
    let mut edges: Vec<_> = s.wires.values().map(|w| edge(w, &labels)).collect();
    edges.sort();
    format!("{}//{}", nodes.join(";"), edges.join(";"))
}
fn matches(s: &State, g: &State) -> bool {
    if !s.tangle {
        return planar_graph_key(s) == planar_graph_key(g);
    }
    if s.nodes.len() != g.nodes.len() || s.wires.len() != g.wires.len() {
        return false;
    }
    let mut pairs: BTreeMap<String, String> = BTreeMap::new();
    let mut queue = vec![];
    for (id, n) in &g.nodes {
        if !n.kind.internal() {
            if s.kind(id) != Some(n.kind) {
                return false;
            }
            pairs.insert(id.clone(), id.clone());
            queue.push(id.clone());
        }
    }
    while let Some(gid) = queue.pop() {
        let sid = pairs[&gid].clone();
        for gw in g
            .wires
            .values()
            .filter(|w| w.from.node == gid || w.to.node == gid)
        {
            let outgoing = gw.from.node == gid;
            let (here, other) = if outgoing {
                (&gw.from, &gw.to)
            } else {
                (&gw.to, &gw.from)
            };
            let sw = s.wires.values().find(|w| {
                if outgoing {
                    w.from == port(&sid, here.i)
                } else {
                    w.to == port(&sid, here.i)
                }
            });
            let Some(sw) = sw else { return false };
            let end = if outgoing { &sw.to } else { &sw.from };
            if end.i != other.i || s.kind(&end.node) != g.kind(&other.node) {
                return false;
            }
            if let Some(mapped) = pairs.get(&other.node) {
                if mapped != &end.node {
                    return false;
                }
            } else {
                if pairs.values().any(|v| *v == end.node) {
                    return false;
                }
                pairs.insert(other.node.clone(), end.node.clone());
                queue.push(other.node.clone());
            }
        }
    }
    pairs.len() == g.nodes.len()
}
fn normal(s: &State) -> State {
    let mut t = State::new(true, s.puzzle);
    t.nodes = s
        .nodes
        .iter()
        .filter(|(_, n)| !n.kind.internal())
        .map(|(id, n)| (id.clone(), n.clone()))
        .collect();
    let side = |kind| {
        let mut a: Vec<_> = t
            .nodes
            .iter()
            .filter(|(_, n)| n.kind == kind)
            .map(|(id, n)| (id.clone(), n.p.y))
            .collect();
        a.sort_by(|a, b| a.1.total_cmp(&b.1));
        a.into_iter().map(|a| a.0).collect::<Vec<_>>()
    };
    let ins = side(Kind::Input);
    let outs = side(Kind::Output);
    let genus = s.wires.len() + 1 - s.nodes.len();
    let count = ins.len() + outs.len() - 2 + genus * 2;
    let mut next = 0;
    let add = |t: &mut State, kind, next: &mut usize| {
        let id = format!("n{next}");
        t.add(
            &id,
            kind,
            58. + 344. * (*next as f64 + 0.5) / count.max(1) as f64,
            180.,
        );
        *next += 1;
        id
    };
    let mut cur = port(&ins[0], 0);
    for input in ins.iter().skip(1) {
        let id = add(&mut t, Kind::Mult, &mut next);
        t.wire(&cur.node, cur.i, &id, 0);
        t.wire(input, 0, &id, 1);
        cur = port(&id, 0);
    }
    for _ in 0..genus {
        let d = add(&mut t, Kind::Comult, &mut next);
        let m = add(&mut t, Kind::Mult, &mut next);
        t.wire(&cur.node, cur.i, &d, 0);
        t.wire(&d, 0, &m, 0);
        t.wire(&d, 1, &m, 1);
        cur = port(&m, 0);
    }
    for output in outs.iter().take(outs.len() - 1) {
        let id = add(&mut t, Kind::Comult, &mut next);
        t.wire(&cur.node, cur.i, &id, 0);
        t.wire(&id, 0, output, 0);
        cur = port(&id, 1);
    }
    t.wire(&cur.node, cur.i, outs.last().unwrap(), 0);
    t
}
fn conditional(s: State, predicate: impl Fn(Point) -> bool + 'static) -> DragSpec<'static, State> {
    let s = Rc::new(s);
    DragSpec::custom(move |ctx, p| {
        let mut f = ctx.state_frame(s.clone(), p)?;
        f.gap = if predicate(p - ctx.anchor().to_vec2()) {
            0.
        } else {
            f64::INFINITY
        };
        Ok(f)
    })
}
struct NodeDrag {
    spec: DragSpec<'static, State>,
    targets: BTreeSet<String>,
}

// Keep legal-target feedback and the actual rewrite branches in one calculation.
fn planar_assoc_partner(s: &State, id: &str) -> Option<(String, String)> {
    match s.kind(id)? {
        Kind::Mult => {
            let down = &s.wires[&s.from(id, 0)].to.node;
            if s.kind(down) == Some(Kind::Mult) {
                return Some((id.into(), down.clone()));
            }
            for i in 0..2 {
                let up = &s.wires[&s.to(id, i)].from.node;
                if s.kind(up) == Some(Kind::Mult) {
                    return Some((up.clone(), id.into()));
                }
            }
        }
        Kind::Comult => {
            let up = &s.wires[&s.to(id, 0)].from.node;
            if s.kind(up) == Some(Kind::Comult) {
                return Some((id.into(), up.clone()));
            }
            for i in 0..2 {
                let down = &s.wires[&s.from(id, i)].to.node;
                if s.kind(down) == Some(Kind::Comult) {
                    return Some((down.clone(), id.into()));
                }
            }
        }
        _ => {}
    }
    None
}
fn planar_frob_partner(s: &State, id: &str) -> Option<(String, String)> {
    let kind = s.kind(id)?;
    if !matches!(kind, Kind::Mult | Kind::Comult) {
        return None;
    }
    for w in s
        .wires
        .values()
        .filter(|w| w.from.node == id || w.to.node == id)
    {
        let other = if w.from.node == id {
            &w.to.node
        } else {
            &w.from.node
        };
        let opposite = if kind == Kind::Mult {
            Kind::Comult
        } else {
            Kind::Mult
        };
        if s.kind(other) != Some(opposite) {
            continue;
        }
        let (m, c) = if kind == Kind::Mult {
            (id, other.as_str())
        } else {
            (other.as_str(), id)
        };
        if form(s, m, c).is_some() {
            return Some((m.into(), c.into()));
        }
    }
    None
}
fn node_drag(s: &State, id: &str) -> NodeDrag {
    let mut branches = vec![];
    let p = s.pos(id);
    let g = s.nodes[id].kind;
    let mut xb: (f64, f64) = (
        if s.tangle { 38. } else { 30. },
        if s.tangle { 422. } else { 290. },
    );
    let mut yb = (16., 344.);
    if !s.tangle {
        yb = (
            s.wires
                .values()
                .filter(|w| w.to.node == id)
                .map(|w| s.pos(&w.from.node).y)
                .fold(f64::NEG_INFINITY, f64::max)
                + 26.,
            s.wires
                .values()
                .filter(|w| w.from.node == id)
                .map(|w| s.pos(&w.to.node).y)
                .fold(f64::INFINITY, f64::min)
                - 26.,
        );
    }
    let assoc_partner = if s.tangle {
        None
    } else {
        planar_assoc_partner(s, id)
    };
    let frob_partner = if s.tangle {
        None
    } else {
        planar_frob_partner(s, id)
    };
    for other in s.neighbors(id) {
        let mut target = |priority, spec| branches.push((priority, other.clone(), spec));
        let o = s.pos(&other);
        let og = s.nodes[&other].kind;
        if (g == Kind::Unit && og == Kind::Mult)
            || (g == Kind::Mult && og == Kind::Unit)
            || (g == Kind::Counit && og == Kind::Comult)
            || (g == Kind::Comult && og == Kind::Counit)
        {
            let (u, m) = if matches!(g, Kind::Unit | Kind::Counit) {
                (id, other.as_str())
            } else {
                (other.as_str(), id)
            };
            let port = if s.kind(u) == Some(Kind::Unit) {
                s.wires[&s.from(u, 0)].to.i
            } else {
                s.wires[&s.to(u, 0)].from.i
            };
            target(
                port,
                conditional(unit(s, u, m), move |c| c.distance(o) < 30.),
            );
        }
        if g == og && matches!(g, Kind::Mult | Kind::Comult) {
            let w = s
                .wires
                .values()
                .find(|w| {
                    (w.from.node == id && w.to.node == other)
                        || (w.to.node == id && w.from.node == other)
                })
                .unwrap();
            let (sat, hub, k) = if g == Kind::Mult {
                (&w.from.node, &w.to.node, w.to.i)
            } else {
                (&w.to.node, &w.from.node, w.from.i)
            };
            if !s.tangle && assoc_partner.as_ref() != Some(&(sat.clone(), hub.clone())) {
                continue;
            }
            let result = assoc(s, sat, hub);
            if result.acyclic() {
                if s.tangle {
                    target(2, conditional(result, move |c| c.distance(o) < 24.));
                } else {
                    let side = if k == 0 { -1. } else { 1. };
                    let dir = if sat == id { -side } else { side };
                    target(2, conditional(result, move |c| (c.x - o.x) * dir > 8.));
                    if dir > 0. {
                        xb.0 = xb.0.max(o.x + 2.)
                    } else {
                        xb.1 = xb.1.min(o.x - 2.)
                    }
                }
            }
        }
        if matches!(
            (g, og),
            (Kind::Mult, Kind::Comult) | (Kind::Comult, Kind::Mult)
        ) {
            let (m, c) = if g == Kind::Mult {
                (id, other.as_str())
            } else {
                (other.as_str(), id)
            };
            if !s.tangle && frob_partner.as_ref() != Some(&(m.to_owned(), c.to_owned())) {
                continue;
            }
            if let Some(f) = form(s, m, c) {
                let forms = if f == Form::H {
                    vec![Form::Left, Form::Right]
                } else {
                    vec![Form::H]
                };
                for to in forms {
                    let result = frobenius(s, m, c, to);
                    if !result.acyclic() {
                        continue;
                    }
                    let tangle = s.tangle;
                    let is_c = id == c;
                    let going_up = o.y < p.y;
                    target(
                        3,
                        conditional(result, move |q| {
                            let pass = if tangle {
                                q.distance(o) < 24.
                            } else if going_up {
                                q.y < o.y - 8.
                            } else {
                                q.y > o.y + 8.
                            };
                            let side = if tangle { q.y <= o.y } else { q.x < o.x };
                            pass && (to == Form::H
                                || (side
                                    && ((is_c && to == Form::Left)
                                        || (!is_c && to == Form::Right)))
                                || (!side
                                    && ((is_c && to == Form::Right)
                                        || (!is_c && to == Form::Left))))
                        }),
                    );
                }
            }
        }
    }
    let base = s.clone();
    let moving = id.to_string();
    let constraint_id = moving.clone();
    let layout_id = moving.clone();
    let free = dragology::vary_constrained(
        [p.x, p.y],
        move |[x, y]| {
            let mut next = base.clone();
            next.nodes.get_mut(&moving).unwrap().p = Point::new(x, y);
            next
        },
        move |state: &State| {
            let q = state.nodes[&constraint_id].p;
            [
                if xb.0.is_finite() { xb.0 - q.x } else { 0. },
                if xb.1.is_finite() { q.x - xb.1 } else { 0. },
                if yb.0.is_finite() { yb.0 - q.y } else { 0. },
                if yb.1.is_finite() { q.y - yb.1 } else { 0. },
            ]
        },
    )
    .during(move |s| s.clone().tidy(Some(&layout_id)));
    // Upstream tries unit/counit rules in port order, then associativity,
    // then Frobenius. Several crossing regions can be active at once.
    branches.sort_by_key(|(priority, _, _)| *priority);
    let targets = branches.iter().map(|(_, id, _)| id.clone()).collect();
    let mut specs: Vec<_> = branches.into_iter().map(|(_, _, spec)| spec).collect();
    specs.push(free);
    NodeDrag {
        spec: closest(specs)
            .with_branch_transition(Some(dragology::Transition {
                duration: std::time::Duration::from_millis(150),
                ..Default::default()
            }))
            .on_drop(|s: &State| s.clone().tidy(None)),
        targets,
    }
}
struct Frobenius(bool);
fn target_halo(id: String, p: Point) -> Object {
    use kurbo::{Circle, Shape as _};
    let circle = Circle::new(Point::ORIGIN, 16.).to_path(0.01);
    let border = kurbo::dash(circle.iter(), 0., &[4., 3.]).collect();
    Object::new(
        id,
        Affine::translate(p.to_vec2()),
        None,
        vec![
            Shape::Circle {
                center: Point::ORIGIN,
                radius: 16.,
                fill: [253, 230, 138, 128],
                stroke: None,
            },
            Shape::Path {
                path: border,
                fill: None,
                stroke: Some(([217, 119, 6, 255], 1.5)),
            },
        ],
    )
}

fn render(s: &State, prefix: &str, dragged: Option<&str>) -> Scene {
    let targets = dragged
        .filter(|id| s.nodes.contains_key(*id))
        .map(|id| {
            if s.tangle {
                node_drag(s, id).targets
            } else {
                s.wires
                    .values()
                    .filter_map(|w| {
                        let (a, b) = (&w.from.node, &w.to.node);
                        if matches!(
                            (s.kind(a), s.kind(b)),
                            (Some(Kind::Unit), Some(Kind::Mult))
                                | (Some(Kind::Comult), Some(Kind::Counit))
                        ) {
                            if id == a {
                                Some(b.clone())
                            } else if id == b {
                                Some(a.clone())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect()
            }
        })
        .unwrap_or_default();
    let mut sc = Scene::new(
        if s.tangle { 630. } else { 470. },
        if s.tangle { 360. } else { 540. },
    );
    if s.tangle {
        for (i, x) in [0., 438.].into_iter().enumerate() {
            sc.push(
                Object::rect(
                    format!("{prefix}bank-{i}"),
                    Rect::new(x, 0., x + 22., 360.),
                    0.,
                    [243, 244, 246, 255],
                )
                .decorative(),
            );
        }
        for (i, x) in [22., 438.].into_iter().enumerate() {
            sc.push(Object::line(
                format!("{prefix}bank-edge-{i}"),
                (x, 0.),
                (x, 360.),
                [209, 213, 219, 255],
                1.,
            ));
        }
    } else {
        let out = 34. + 52. * (s.nodes.values().filter(|n| n.kind.internal()).count() + 1) as f64;
        for (i, y) in [34., out].into_iter().enumerate() {
            sc.push(Object::line(
                format!("{prefix}rail-{i}"),
                (0., y),
                (320., y),
                [229, 231, 235, 255],
                1.,
            ));
        }
    }
    for (&wid, w) in &s.wires {
        let [a, b, c, d] = wire_geometry(s, w);
        let mut path = BezPath::new();
        path.move_to(a);
        path.curve_to(b, c, d);
        sc.push(
            Object::path(
                format!("{prefix}wire-{wid}"),
                path,
                if s.tangle {
                    [63, 63, 70, 255]
                } else {
                    [55, 65, 81, 255]
                },
                if s.tangle { 2.5 } else { 2. },
            )
            .line_cap(if s.tangle {
                LineCap::Round
            } else {
                LineCap::Butt
            }),
        );
    }
    for (id, n) in &s.nodes {
        let p = s.pos(id);
        if n.kind.internal() {
            if targets.contains(id) {
                if s.tangle {
                    sc.push(target_halo(format!("{prefix}halo-{id}"), p));
                } else {
                    sc.push(
                        Object::new(
                            format!("{prefix}halo-{id}"),
                            Affine::translate(p.to_vec2()),
                            None,
                            vec![Shape::Circle {
                                center: Point::ORIGIN,
                                radius: 18.,
                                fill: [229, 231, 235, 153],
                                stroke: Some(([55, 65, 81, 255], 1.5)),
                            }],
                        )
                        .dash(vec![4., 3.]),
                    );
                }
            }
            let ink = if s.tangle {
                [63, 63, 70, 255]
            } else {
                [55, 65, 81, 255]
            };
            let node = Object::new(
                format!("{prefix}node-{id}"),
                Affine::translate(p.to_vec2()),
                Some(Hit::Circle(Point::ORIGIN, if s.tangle { 18. } else { 20. })),
                vec![Shape::Circle {
                    center: Point::ORIGIN,
                    radius: 7.,
                    fill: if s.tangle && n.kind == Kind::Mult {
                        [255; 4]
                    } else {
                        ink
                    },
                    stroke: if s.tangle { Some((ink, 2.5)) } else { None },
                }],
            );
            sc.push(if s.tangle { node.clickable() } else { node });
            if !s.tangle {
                sc.push(
                    Object::text(
                        format!("{prefix}label-{id}"),
                        p + Vec2::new(11., -9.),
                        match n.kind {
                            Kind::Mult => "μ",
                            Kind::Comult => "δ",
                            Kind::Unit => "η",
                            Kind::Counit => "ε",
                            _ => "",
                        },
                        11.,
                        [156, 163, 175, 255],
                    )
                    .text_baseline(TextBaseline::Alphabetic)
                    .font_family(FontFamily::Serif)
                    .italic(),
                );
            }
        } else {
            sc.push(
                Object::circle(
                    format!("{prefix}boundary-{id}"),
                    p,
                    if s.tangle { 3. } else { 2.5 },
                    if s.tangle {
                        [63, 63, 70, 255]
                    } else {
                        [55, 65, 81, 255]
                    },
                )
                .decorative(),
            );
        }
    }

    if let Some(id) = dragged {
        let node = format!("{prefix}node-{id}");
        let label = format!("{prefix}label-{id}");
        sc.objects.sort_by_key(|o| o.id == node || o.id == label);
    }
    sc
}
impl Example for Frobenius {
    type State = State;
    fn initial(&self) -> State {
        preset(self.0, 0)
    }
    fn scene(&self, s: &State) -> Scene {
        self.scene_dragging(s, None)
    }
    fn scene_dragging(&self, s: &State, id: Option<&str>) -> Scene {
        let mut sc = render(s, "", id.and_then(|id| id.strip_prefix("node-")));
        if let Some(g) = goal(s) {
            let proved = matches(s, &g);
            let diag = if s.tangle { 460. } else { 320. };
            let goal_width = if s.tangle { 170. } else { 150. };
            let scale = if s.tangle {
                (goal_width - 24.) / diag
            } else {
                0.42
            };
            let height = if s.tangle {
                360. * scale + 40.
            } else {
                (34. + 52. * (g.nodes.values().filter(|n| n.kind.internal()).count() + 1) as f64
                    + 34.)
                    * scale
                    + 44.
            };
            let origin = Point::new(diag + 8., 12.);
            sc.push(Object::new(
                "goal-card",
                Affine::translate(origin.to_vec2()),
                None,
                vec![Shape::Rect {
                    bounds: Rect::new(0., 0., goal_width - 16., height),
                    radius: 8.,
                    fill: if proved {
                        [236, 253, 245, 255]
                    } else {
                        [249, 250, 251, 255]
                    },
                    stroke: Some((
                        if proved {
                            [16, 185, 129, 255]
                        } else {
                            [229, 231, 235, 255]
                        },
                        if proved { 2. } else { 1. },
                    )),
                }],
            ));
            sc.push(
                Object::text(
                    "goal-title",
                    origin + Vec2::new(10., 18.),
                    if s.tangle {
                        let loops = g.wires.len() + 1 - g.nodes.len();
                        format!("GOAL · {loops} loop{}", if loops == 1 { "" } else { "s" })
                    } else {
                        "GOAL".into()
                    },
                    11.,
                    [107, 114, 128, 255],
                )
                .text_baseline(TextBaseline::Alphabetic),
            );
            if proved {
                sc.push(
                    Object::text(
                        "goal-proved",
                        origin + Vec2::new(goal_width - 26., 18.),
                        if s.tangle { "DONE ∎" } else { "PROVED ∎" },
                        11.,
                        [5, 150, 105, 255],
                    )
                    .text_baseline(TextBaseline::Alphabetic)
                    .text_anchor(TextAnchor::End)
                    .font_weight(700),
                );
            }
            let mut goal = render(&g, "goal-", None);
            let x = (goal_width - 16. - diag * scale) / 2.;
            for o in &mut goal.objects {
                o.transform = Affine::translate(origin.to_vec2() + Vec2::new(x, 30.))
                    * Affine::scale(scale)
                    * o.transform;
                o.hit = None;
            }
            sc.objects.extend(goal.objects);
        }
        sc
    }
    fn drag(&self, s: &State, id: &str, _: Modifiers) -> Option<DragSpec<'static, State>> {
        let id = id.strip_prefix("node-")?;
        s.nodes.get(id)?;
        Some(node_drag(s, id).spec)
    }
    fn click(&self, s: &State, id: &str, _: Point, _: Modifiers, _: usize) -> Option<State> {
        if !s.tangle {
            return None;
        }
        let id = id.strip_prefix("node-")?;
        let kind = s.kind(id)?;
        let mut t = s.clone();
        for w in t.wires.values_mut() {
            let p = if kind == Kind::Mult {
                &mut w.to
            } else {
                &mut w.from
            };
            if p.node == id {
                p.i = 1 - p.i
            }
        }
        Some(t)
    }
    fn controls(&self, s: &State) -> Vec<Control> {
        let titles: &[&str] = if s.tangle {
            &["Frobenius", "Cross", "Weave", "Lattice", "Braid"]
        } else {
            &[
                "(1·a)·b = a·b",
                "Reassociate",
                "Frobenius law",
                "Snake",
                "Double snake",
                "Playground",
            ]
        };
        let mut controls: Vec<_> = titles
            .iter()
            .enumerate()
            .map(|(i, t)| {
                Control::action_button(
                    [
                        "puzzle-0", "puzzle-1", "puzzle-2", "puzzle-3", "puzzle-4", "puzzle-5",
                    ][i],
                    *t,
                    true,
                    s.puzzle == i,
                )
            })
            .collect();
        controls.push(Control::button("restart", "reset"));
        controls
    }
    fn action(&self, s: &State, id: &str, value: Value) -> Option<State> {
        if let Some(i) = id
            .strip_prefix("puzzle-")
            .and_then(|v| v.parse::<usize>().ok())
        {
            return Some(preset(s.tangle, i.min(if s.tangle { 4 } else { 5 })));
        }
        match (id, value) {
            ("puzzle", Value::Index(i)) => {
                Some(preset(s.tangle, i.min(if s.tangle { 4 } else { 5 })))
            }
            ("restart", _) => Some(preset(s.tangle, s.puzzle)),
            _ => None,
        }
    }
    fn status(&self, s: &State) -> String {
        if s.tangle {
            [
            "The zigzag form of the Frobenius law. Drop the μ (hollow) onto the δ (solid), or the δ onto the μ, to get δ∘μ.",
            "Two splits whose strands cross over to two merges: one loop. Untangle it into a merge, a handle, and a split.",
            "One wire splits into four strands that cross and merge back: three loops. Pull it out into a chain of three handles.",
            "Nine nodes woven into a lattice, with four loops.",
            "Three strands, split and re-merged with their neighbors twice over: four loops, three in, three out."
        ][s.puzzle].into()
        } else {
            [
            "Drop the unit η onto the μ it feeds (or the μ onto it): both vanish. Try sliding the upper μ across the lower one first, for associativity.",
            "Slide the upper μ sideways across the lower μ (associativity), and the lower δ across the upper δ (coassociativity).",
            "Drag the δ up past the μ, crossing on its left. (Crossing on the right gives the other zigzag; dragging back down restores δ∘μ.)",
            "A cup (δ∘η) and a cap (ε∘μ) make a zigzag. Drag the δ down past the μ (Frobenius), then drop the unit on the μ and the counit on the δ.",
            "Two zigzags in a row. Straighten both.","No goal – poke at the generators."
        ][s.puzzle].into()
        }
    }
}
include!("frobenius_presets.rs");
pub fn entries() -> Vec<Entry> {
    vec![
        Entry::new(
            "frobenius-algebra",
            "Frobenius algebra",
            "⚠️ Just a seed! String diagrams for a (non-commutative) Frobenius algebra, where every drag is an axiom. Drag generators past unrelated ones (interchange), slide a μ across the μ it feeds (associativity; dually for δ), drop a unit on its μ or a counit on its δ (unit laws), and drag the δ of a μ–δ pair past the μ (Frobenius law).",
            || demo(Frobenius(false)),
        ),
        Entry::new(
            "frobenius-tangle",
            "Frobenius tangles",
            "⚠️ Just a seed! A commutative Frobenius algebra, drawn as a tangle of wires running left to right: μ (hollow) merges two wires, δ (solid) splits one. Drag nodes around freely; click a node to swap its two inputs or outputs (commutativity); drop a μ on the μ it feeds to reassociate (likewise δ); drop a μ on a δ it’s wired to, or vice versa, for the Frobenius law (from δ∘μ, come in from above or below to pick which zigzag).",
            || demo(Frobenius(true)),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planar_associativity_precedes_frobenius_when_both_crossings_apply() {
        let state = preset(false, 1);
        let expected = assoc(&state, "d2", "d1");
        let spec = node_drag(&state, "d1").spec;
        let start = state.pos("d1");
        let session = dragology::DragSession::begin(
            state,
            start,
            |s| Some(Affine::translate(s.pos("d1").to_vec2())),
            spec,
        )
        .unwrap();
        // This crosses left of d2 and above l. Upstream genSpec places the
        // associativity branch before either Frobenius branch.
        let result = session.finish((80., 100.)).unwrap();
        assert_eq!(result.wires, expected.wires);
    }

    #[test]
    fn tangle_associativity_precedes_frobenius_for_overlapping_targets() {
        let example = Frobenius(true);
        let state = example
            .click(
                &preset(true, 2),
                "node-m1",
                Point::ORIGIN,
                Modifiers::default(),
                1,
            )
            .unwrap();
        let move_node = |state: State, id: &str, target: Point| {
            let spec = node_drag(&state, id).spec;
            let start = state.pos(id);
            let id = id.to_owned();
            dragology::DragSession::begin(
                state,
                start,
                move |s| Some(Affine::translate(s.pos(&id).to_vec2())),
                spec,
            )
            .unwrap()
            .finish(target)
            .unwrap()
        };
        // Free movement can put two legal targets within the same hit radius.
        let state = move_node(state, "m3", Point::new(190., 100.));
        assert!(form(&state, "m1", "d2").is_some());
        let expected = assoc(&state, "m1", "m3");
        let result = move_node(state, "m1", Point::new(180., 100.));
        assert_eq!(result.wires, expected.wires);
    }
    fn halo_ids(scene: &Scene) -> Vec<&str> {
        scene
            .objects
            .iter()
            .filter(|o| o.id.starts_with("halo-") || o.id.starts_with("goal-halo-"))
            .map(|o| o.id.as_str())
            .collect()
    }

    #[test]
    fn tangle_drag_highlights_legal_target_and_clears_it_on_cancel() {
        let mut demo = crate::runtime::ModelDemo::new(Frobenius(true));
        demo.set_animated(false);
        let initial = demo.scene().clone();
        assert!(halo_ids(&initial).is_empty());
        demo.press(Point::new(160., 120.), Modifiers::default(), 1);
        demo.move_to(Point::new(163., 120.), Modifiers::default());
        demo.advance(std::time::Duration::ZERO);
        assert_eq!(halo_ids(demo.scene()), ["halo-m"]);
        let halo = demo.scene().object("halo-m").unwrap();
        assert_eq!(halo.transform * Point::ORIGIN, Point::new(300., 240.));
        assert!(halo.hit.is_none(), "feedback must not intercept node grabs");
        demo.move_to(Point::new(230., 180.), Modifiers::default());
        demo.advance(std::time::Duration::ZERO);
        assert_eq!(halo_ids(demo.scene()), ["halo-m"]);
        demo.cancel();
        assert_eq!(demo.scene(), &initial);
    }

    #[test]
    fn tangle_highlight_follows_rewrite_preview_and_clears_on_drop() {
        let mut demo = crate::runtime::ModelDemo::new(Frobenius(true));
        demo.set_animated(false);
        demo.press(Point::new(160., 120.), Modifiers::default(), 1);
        // Inside the source's 24px rewrite threshold.
        let near = Point::new(285., 230.);
        demo.move_to(near, Modifiers::default());
        demo.advance(std::time::Duration::ZERO);
        assert_eq!(halo_ids(demo.scene()), ["halo-m"]);
        assert_eq!(
            demo.scene().object("halo-m").unwrap().transform,
            demo.scene().object("node-m").unwrap().transform,
            "the halo must follow the preview, not the original node position",
        );
        let proved = |scene: &Scene| {
            scene
                .object("goal-proved")
                .unwrap()
                .shapes
                .iter()
                .any(|shape| matches!(shape, Shape::Text { text, .. } if text == "DONE ∎"))
        };
        assert!(proved(demo.scene()));
        demo.release(near, Modifiers::default());
        assert!(halo_ids(demo.scene()).is_empty());
        assert!(proved(demo.scene()));
    }

    #[test]
    fn two_directional_rewrites_share_one_target_halo() {
        let example = Frobenius(true);
        let state = frobenius(&example.initial(), "m", "d", Form::H);
        assert_eq!(
            halo_ids(&example.scene_dragging(&state, Some("node-d"))),
            ["halo-m"]
        );
        assert!(halo_ids(&example.scene_dragging(&state, Some("goal-node-d"))).is_empty());
    }

    #[test]
    fn neighboring_handle_nodes_without_a_legal_rewrite_do_not_light_up() {
        let mut state = State::new(true, 0);
        state.add("a", Kind::Input, 22., 180.);
        state.add("d", Kind::Comult, 160., 180.);
        state.add("m", Kind::Mult, 300., 180.);
        state.add("b", Kind::Output, 438., 180.);
        state.wire("a", 0, "d", 0);
        state.wire("d", 0, "m", 0);
        state.wire("d", 1, "m", 1);
        state.wire("m", 0, "b", 0);
        assert!(state.valid());
        for node in ["node-d", "node-m"] {
            assert!(halo_ids(&Frobenius(true).scene_dragging(&state, Some(node))).is_empty());
        }
    }

    // Contract the diagram in the noncommutative Frobenius algebra M₂(Z).
    // Matrix units multiply Eij Ekl = δjk Eil; Δ(Eij)=Σk Eik⊗Ekj.
    // This catches port-order errors that a commutative interpretation misses.
    fn tensor(s: &State) -> BTreeMap<Vec<usize>, usize> {
        let inputs: Vec<_> = s
            .nodes
            .iter()
            .filter(|(_, n)| n.kind == Kind::Input)
            .map(|(id, _)| id.clone())
            .collect();
        let outputs: Vec<_> = s
            .nodes
            .iter()
            .filter(|(_, n)| n.kind == Kind::Output)
            .map(|(id, _)| id.clone())
            .collect();
        let mut order = vec![];
        let mut pending: BTreeSet<_> = s.nodes.keys().cloned().collect();
        while !pending.is_empty() {
            let id = pending
                .iter()
                .find(|id| {
                    s.wires
                        .values()
                        .filter(|w| w.to.node == **id)
                        .all(|w| !pending.contains(&w.from.node))
                })
                .unwrap()
                .clone();
            pending.remove(&id);
            order.push(id);
        }
        let mut result = BTreeMap::new();
        for input in 0..4usize.pow(inputs.len() as u32) {
            let mut v = input;
            let mut assignment = vec![];
            for _ in &inputs {
                assignment.push(v % 4);
                v /= 4;
            }
            let mut terms = vec![BTreeMap::<usize, usize>::new()];
            for id in &order {
                let kind = s.nodes[id].kind;
                let mut next = vec![];
                for term in terms {
                    let vals: Vec<_> = (0..kind.arity().0).map(|i| term[&s.to(id, i)]).collect();
                    let candidates: Vec<Vec<usize>> = match kind {
                        Kind::Input => vec![vec![
                            assignment[inputs.iter().position(|n| n == id).unwrap()],
                        ]],
                        Kind::Output => vec![vec![]],
                        Kind::Unit => vec![vec![0], vec![3]],
                        Kind::Counit => {
                            if vals[0] / 2 == vals[0] % 2 {
                                vec![vec![]]
                            } else {
                                vec![]
                            }
                        }
                        Kind::Mult => {
                            if vals[0] % 2 == vals[1] / 2 {
                                vec![vec![2 * (vals[0] / 2) + vals[1] % 2]]
                            } else {
                                vec![]
                            }
                        }
                        Kind::Comult => (0..2)
                            .map(|k| vec![2 * (vals[0] / 2) + k, 2 * k + vals[0] % 2])
                            .collect(),
                    };
                    for values in candidates {
                        let mut term = term.clone();
                        for (i, value) in values.into_iter().enumerate() {
                            term.insert(s.from(id, i), value);
                        }
                        next.push(term);
                    }
                }
                terms = next;
            }
            for term in terms {
                let mut key = assignment.clone();
                key.extend(outputs.iter().map(|id| term[&s.to(id, 0)]));
                *result.entry(key).or_default() += 1;
            }
        }
        result
    }
    fn rewrites(s: &State) -> Vec<State> {
        let mut out = vec![];
        for w in s.wires.values() {
            let a = &w.from.node;
            let b = &w.to.node;
            match (s.kind(a).unwrap(), s.kind(b).unwrap()) {
                (Kind::Unit, Kind::Mult) => out.push(unit(s, a, b)),
                (Kind::Comult, Kind::Counit) => out.push(unit(s, b, a)),
                (Kind::Mult, Kind::Mult) => out.push(assoc(s, a, b)),
                (Kind::Comult, Kind::Comult) => out.push(assoc(s, b, a)),
                (Kind::Mult, Kind::Comult) | (Kind::Comult, Kind::Mult) => {
                    let (m, c) = if s.kind(a) == Some(Kind::Mult) {
                        (a, b)
                    } else {
                        (b, a)
                    };
                    if let Some(f) = form(s, m, c) {
                        for to in [Form::H, Form::Left, Form::Right] {
                            if f != to {
                                out.push(frobenius(s, m, c, to));
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        out
    }
    #[test]
    fn ordered_frobenius_rewrites_preserve_noncommutative_tensor() {
        for p in 0..6 {
            let s = preset(false, p);
            assert!(s.valid() && s.acyclic());
            let expected = tensor(&s);
            for t in rewrites(&s) {
                assert!(t.valid(), "invalid ports in puzzle {p}");
                if t.acyclic() {
                    assert_eq!(tensor(&t), expected, "ordered port error in puzzle {p}");
                }
            }
        }
    }
    #[test]
    fn tangle_rewrites_preserve_ports_boundaries_and_genus() {
        for p in 0..5 {
            let s = preset(true, p);
            assert!(s.valid() && s.acyclic());
            let goal = normal(&s);
            assert!(goal.valid() && goal.acyclic());
            let euler = s.wires.len() as isize - s.nodes.len() as isize;
            assert_eq!(goal.wires.len() as isize - goal.nodes.len() as isize, euler);
            for t in rewrites(&s) {
                assert!(t.valid());
                assert_eq!(t.wires.len() as isize - t.nodes.len() as isize, euler);
            }
        }
    }
    #[test]
    fn snake_reduction_reaches_identity_with_ordered_ports() {
        let s = preset(false, 3);
        let s = frobenius(&s, "m", "k", Form::H);
        let s = unit(&s, "eta", "m");
        let s = unit(&s, "eps", "k");
        assert!(matches(&s, &goal(&s).unwrap()));
    }
}
