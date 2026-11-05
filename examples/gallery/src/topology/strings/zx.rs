//! ZX spiders, phase transport, Hopf cancellation and the snake equation.
use super::*;
use dragology::{closest, fixed};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    Input,
    Output,
    Z,
    X,
    Cup,
    Cap,
}
#[derive(Clone, Debug, PartialEq)]
struct Node {
    kind: Kind,
    phase: u8,
    p: Point,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Port {
    node: String,
    slot: usize,
}
fn port(node: &str, slot: usize) -> Port {
    Port {
        node: node.into(),
        slot,
    }
}
#[derive(Clone, Debug, PartialEq)]
struct Wire {
    a: Port,
    b: Port,
    bow: f64,
    t: f64,
}
#[derive(Clone, Debug, PartialEq)]
struct State {
    puzzle: usize,
    nodes: BTreeMap<String, Node>,
    wires: BTreeMap<usize, Wire>,
    next: usize,
}
fn spider(k: Kind) -> bool {
    matches!(k, Kind::Z | Kind::X)
}
impl State {
    fn new(puzzle: usize) -> Self {
        Self {
            puzzle,
            nodes: BTreeMap::new(),
            wires: BTreeMap::new(),
            next: 0,
        }
    }
    fn add(&mut self, id: &str, k: Kind, phase: u8, x: f64, y: f64) {
        self.nodes.insert(
            id.into(),
            Node {
                kind: k,
                phase,
                p: Point::new(x, y),
            },
        );
    }
    fn wire(&mut self, a: &str, sa: usize, b: &str, sb: usize) {
        let id = self.wires.keys().next_back().map_or(0, |i| i + 1);
        self.wires.insert(
            id,
            Wire {
                a: port(a, sa),
                b: port(b, sb),
                bow: 0.,
                t: 0.5,
            },
        );
    }
    fn internal(&self) -> usize {
        self.nodes
            .values()
            .filter(|n| !matches!(n.kind, Kind::Input | Kind::Output))
            .count()
    }
    fn pos(&self, id: &str) -> Point {
        let n = &self.nodes[id];
        match n.kind {
            Kind::Input => Point::new(n.p.x, 34.),
            Kind::Output => Point::new(n.p.x, 34. + 56. * (self.internal() + 1) as f64),
            _ => n.p,
        }
    }
    fn at(&self, id: &str) -> Vec<usize> {
        self.wires
            .iter()
            .filter(|(_, w)| w.a.node == id || w.b.node == id)
            .map(|(i, _)| *i)
            .collect()
    }
    fn other(&self, wid: usize, id: &str) -> Port {
        let w = &self.wires[&wid];
        if w.a.node == id {
            w.b.clone()
        } else {
            w.a.clone()
        }
    }
    fn neighbors(&self, id: &str) -> Vec<String> {
        let mut seen = BTreeSet::new();
        self.at(id)
            .into_iter()
            .map(|w| self.other(w, id).node)
            .filter(|id| seen.insert(id.clone()))
            .collect()
    }
    fn bare(&self, id: &str) -> bool {
        let n = &self.nodes[id];
        spider(n.kind) && n.phase % 8 == 0 && self.at(id).len() == 2
    }
    fn tidy(mut self, floating: Option<&str>) -> Self {
        let mut ids: Vec<_> = self
            .nodes
            .iter()
            .filter(|(id, n)| {
                !matches!(n.kind, Kind::Input | Kind::Output) && floating != Some(id.as_str())
            })
            .map(|(id, n)| (id.clone(), n.p.y))
            .collect();
        ids.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
        let fy = floating.and_then(|id| self.nodes.get(id)).map(|n| n.p.y);
        for (i, (id, y)) in ids.into_iter().enumerate() {
            self.nodes.get_mut(&id).unwrap().p.y =
                34. + 56. * (i + 1 + usize::from(fy.is_some_and(|f| f < y))) as f64;
        }
        if floating.is_some() {
            return self;
        }
        let mut groups: BTreeMap<(String, String), Vec<usize>> = BTreeMap::new();
        for (&id, w) in &self.wires {
            let key = if w.a.node < w.b.node {
                (w.a.node.clone(), w.b.node.clone())
            } else {
                (w.b.node.clone(), w.a.node.clone())
            };
            groups.entry(key).or_default().push(id);
        }
        for ids in groups.values_mut() {
            ids.sort_by(|a, b| {
                self.wires[a]
                    .bow
                    .total_cmp(&self.wires[b].bow)
                    .then(a.cmp(b))
            });
            if ids
                .windows(2)
                .any(|p| self.wires[&p[1]].bow - self.wires[&p[0]].bow < 8.)
            {
                for (j, id) in ids.iter().enumerate() {
                    self.wires.get_mut(id).unwrap().bow =
                        (j as f64 - (ids.len() - 1) as f64 / 2.) * 28.;
                }
            }
        }
        self
    }
    fn info(&self, p: &Port) -> (Point, f64) {
        let center = self.pos(&p.node);
        match self.nodes[&p.node].kind {
            Kind::Cup => (
                center + Vec2::new(if p.slot == 0 { -14. } else { 14. }, -7.),
                -1.,
            ),
            Kind::Cap => (
                center + Vec2::new(if p.slot == 0 { -14. } else { 14. }, 7.),
                1.,
            ),
            _ => (center, 0.),
        }
    }
    fn curve(&self, w: &Wire) -> [Point; 4] {
        let (a, at) = self.info(&w.a);
        let (b, bt) = self.info(&w.b);
        let dy = b.y - a.y;
        let stiff = (dy.abs() / 2.).max(24.);
        let chord = b - a;
        let perp = if chord.hypot() < 1. {
            Vec2::new(1., 0.)
        } else {
            Vec2::new(-chord.y, chord.x) / chord.hypot()
        };
        [
            a,
            a + Vec2::new(0., if at == 0. { dy / 2. } else { at * stiff }) + perp * w.bow,
            b + Vec2::new(0., if bt == 0. { -dy / 2. } else { bt * stiff }) + perp * w.bow,
            b,
        ]
    }
    fn bead(&self, wid: usize) -> Point {
        let w = &self.wires[&wid];
        bezier(self.curve(w), w.t)
    }
}
fn bezier(p: [Point; 4], t: f64) -> Point {
    let u = 1. - t;
    (p[0].to_vec2() * u.powi(3)
        + p[1].to_vec2() * (3. * u * u * t)
        + p[2].to_vec2() * (3. * u * t * t)
        + p[3].to_vec2() * t.powi(3))
    .to_point()
}
fn fuse(s: &State, from: &str, into: &str) -> State {
    let mut t = s.clone();
    let phase = t.nodes[from].phase;
    t.nodes.get_mut(into).unwrap().phase = (phase + t.nodes[into].phase) % 8;
    for w in t.wires.values_mut() {
        if w.a.node == from {
            w.a = port(into, 0)
        }
        if w.b.node == from {
            w.b = port(into, 0)
        }
    }
    t.wires
        .retain(|_, w| !(w.a.node == into && w.b.node == into));
    t.nodes.remove(from);
    t.tidy(None)
}
fn splice(s: &State, id: &str) -> State {
    let legs = s.at(id);
    if legs.len() != 2 {
        return s.clone();
    }
    let mut t = s.clone();
    let a = s.other(legs[0], id);
    let b = s.other(legs[1], id);
    let w = t.wires.get_mut(&legs[0]).unwrap();
    w.a = a;
    w.b = b;
    t.wires.remove(&legs[1]);
    t.nodes.remove(id);
    t
}
fn remove_identity(s: &State, id: &str) -> State {
    let mut t = splice(s, id);
    t.wires
        .retain(|_, w| !(w.a.node == w.b.node && spider(t.nodes[&w.a.node].kind)));
    t.tidy(None)
}
fn transfer(s: &State, from: &str, to: &str) -> State {
    let mut t = s.clone();
    let p = t.nodes[from].phase;
    t.nodes.get_mut(to).unwrap().phase = (p + t.nodes[to].phase) % 8;
    t.nodes.get_mut(from).unwrap().phase = 0;
    t
}
fn split_legs(s: &State, parent: &str, child: &str) -> State {
    let p = s.pos(parent);
    let axis = s.pos(child) - p;
    if axis.hypot() < 1. {
        return s.clone();
    }
    let mut t = s.clone();
    for (&id, w) in &s.wires {
        for end_a in [true, false] {
            let end = if end_a { &w.a } else { &w.b };
            if end.node != parent && end.node != child {
                continue;
            }
            let other = s.other(id, &end.node);
            if other.node == parent || other.node == child {
                continue;
            }
            let into = if (s.info(&other).0 - p).dot(axis) > 0. {
                child
            } else {
                parent
            };
            if end_a {
                t.wires.get_mut(&id).unwrap().a = port(into, 0)
            } else {
                t.wires.get_mut(&id).unwrap().b = port(into, 0)
            }
        }
    }
    t
}
fn unfuse(s: &State, parent: &str) -> (State, String) {
    let mut next = s.clone();
    let child = format!("s{}", s.next);
    next.next += 1;
    let mut node = s.nodes[parent].clone();
    node.phase = 0;
    next.nodes.insert(child.clone(), node);
    next.wire(parent, 0, &child, 0);
    (next, child)
}

fn twin(s: &State, id: usize) -> Option<usize> {
    let w = &s.wires[&id];
    let a = s.nodes[&w.a.node].kind;
    let b = s.nodes[&w.b.node].kind;
    if !spider(a) || !spider(b) || a == b {
        return None;
    }
    s.wires
        .iter()
        .filter(|(i, o)| {
            **i != id
                && ((o.a.node == w.a.node && o.b.node == w.b.node)
                    || (o.a.node == w.b.node && o.b.node == w.a.node))
        })
        .min_by(|(_, a), (_, b)| (a.bow - w.bow).abs().total_cmp(&(b.bow - w.bow).abs()))
        .map(|(i, _)| *i)
}
fn partner(s: &State, id: &str) -> Option<String> {
    let me = &s.nodes[id];
    for start in s.at(id) {
        let mut prev = start;
        let mut cur = s.other(start, id).node;
        for _ in 0..50 {
            let n = &s.nodes[&cur];
            if matches!(n.kind, Kind::Cup | Kind::Cap) {
                if n.kind != me.kind
                    && ((me.kind == Kind::Cup && n.p.y < me.p.y)
                        || (me.kind == Kind::Cap && n.p.y > me.p.y))
                {
                    return Some(cur);
                }
                break;
            }
            if !spider(n.kind) {
                break;
            }
            let legs = s.at(&cur);
            if legs.len() != 2 {
                break;
            }
            let next = *legs.iter().find(|i| **i != prev).unwrap();
            prev = next;
            cur = s.other(next, &cur).node;
        }
    }
    None
}
fn conditional(s: State, pred: impl Fn(Point) -> bool + 'static) -> DragSpec<'static, State> {
    let s = Rc::new(s);
    DragSpec::custom(move |ctx, p| {
        let mut f = ctx.state_frame(s.clone(), p)?;
        f.gap = if pred(p - ctx.anchor().to_vec2()) {
            0.
        } else {
            f64::INFINITY
        };
        Ok(f)
    })
}
fn spider_spec(s: &State, id: &str, parent: Option<String>) -> DragSpec<'static, State> {
    let mut branches = vec![];
    let g = s.nodes[id].kind;
    let mut targets = s.neighbors(id);
    targets.retain(|other| other != id);
    targets.sort_by_key(|other| s.nodes[other].kind != g);
    for other in targets {
        let og = s.nodes[&other].kind;
        let result = if og == g {
            Some(fuse(s, id, &other))
        } else if s.bare(id) && !matches!(og, Kind::Cup | Kind::Cap) {
            Some(remove_identity(s, id))
        } else {
            None
        };
        if let Some(t) = result {
            let p = s.pos(&other);
            branches.push(conditional(t, move |q| q.distance(p) < 34.));
        }
    }
    let base = s.clone();
    let moving = id.to_string();
    let child = moving.clone();
    let source = parent.clone();
    let constraint_id = moving.clone();
    let layout_id = moving.clone();
    let p = s.pos(id);
    branches.push(
        dragology::vary_constrained(
            [p.x, p.y],
            move |[x, y]| {
                let mut next = base.clone();
                next.nodes.get_mut(&moving).unwrap().p = Point::new(x, y);
                next
            },
            move |s: &State| {
                let x = s.nodes[&constraint_id].p.x;
                [30. - x, x - 290.]
            },
        )
        .during(move |s| {
            let mut next = s.clone().tidy(Some(&layout_id));
            if let Some(ref parent) = source {
                next = split_legs(&next, parent, &layout_id)
            }
            next
        }),
    );
    closest(branches)
        .with_branch_transition(Some(dragology::Transition {
            duration: std::time::Duration::from_millis(150),
            ..Default::default()
        }))
        .on_drop(move |s: &State| {
            if !s.nodes.contains_key(&child) {
                return s.clone();
            }
            if let Some(ref parent) = parent {
                if s.nodes.contains_key(parent) && s.at(&child).len() <= 1 {
                    return fuse(s, &child, parent);
                }
            }
            s.clone().tidy(None)
        })
}
fn graph_key(s: &State) -> String {
    let mut labels: BTreeMap<String, String> = s
        .nodes
        .iter()
        .map(|(id, n)| {
            (
                id.clone(),
                match n.kind {
                    Kind::Input | Kind::Output => format!("B:{id}"),
                    Kind::Z | Kind::X => format!(
                        "S:{}:{}",
                        if n.kind == Kind::Z { "Z" } else { "X" },
                        n.phase % 8
                    ),
                    Kind::Cup => "D:cup".into(),
                    Kind::Cap => "D:cap".into(),
                },
            )
        })
        .collect();
    for _ in 0..4 {
        let mut next = BTreeMap::new();
        for id in s.nodes.keys() {
            let mut neighbors: Vec<_> = s
                .at(id)
                .into_iter()
                .map(|wid| labels[&s.other(wid, id).node].clone())
                .collect();
            neighbors.sort();
            next.insert(
                id.clone(),
                format!("{}({})", labels[id], neighbors.join(",")),
            );
        }
        labels = next;
    }
    let mut nodes: Vec<_> = labels.values().cloned().collect();
    nodes.sort();
    let mut edges: Vec<_> = s
        .wires
        .values()
        .map(|w| {
            let mut pair = [labels[&w.a.node].clone(), labels[&w.b.node].clone()];
            pair.sort();
            pair.join("~")
        })
        .collect();
    edges.sort();
    format!("{}//{}", nodes.join(";"), edges.join(";"))
}
fn matches(s: &State, g: &State) -> bool {
    graph_key(s) == graph_key(g)
}
fn phase_label(p: u8) -> &'static str {
    ["", "π/4", "π/2", "3π/4", "π", "-3π/4", "-π/2", "-π/4"][(p % 8) as usize]
}
fn colors(k: Kind) -> (Color, Color) {
    if k == Kind::Z {
        ([217, 246, 211, 255], [63, 143, 58, 255])
    } else {
        ([246, 200, 200, 255], [176, 58, 58, 255])
    }
}
fn drop_targets(s: &State, id: &str, phase: bool) -> BTreeSet<String> {
    if !s.nodes.contains_key(id) {
        return BTreeSet::new();
    }
    let mine = s.nodes[id].kind;
    s.neighbors(id)
        .into_iter()
        .filter(|other| {
            other != id
                && (s.nodes[other].kind == mine
                    || (!phase
                        && s.bare(id)
                        && !matches!(s.nodes[other].kind, Kind::Cup | Kind::Cap)))
        })
        .collect()
}

struct ZX;
fn render(s: &State, prefix: &str, dragged: Option<&str>, hovered: Option<&str>) -> Scene {
    use kurbo::Shape as _;
    let mut sc = Scene::new(470., 500.);
    let interactive = prefix.is_empty();
    let target_ids = dragged
        .and_then(|id| {
            id.strip_prefix("spider-")
                .map(|id| (id, false))
                .or_else(|| id.strip_prefix("phase-").map(|id| (id, true)))
        })
        .map(|(id, phase)| drop_targets(s, id, phase))
        .unwrap_or_default();
    for (i, y) in [34., 34. + 56. * (s.internal() + 1) as f64]
        .into_iter()
        .enumerate()
    {
        sc.push(Object::line(
            format!("{prefix}rail-{i}"),
            (0., y),
            (320., y),
            [229, 231, 235, 255],
            1.,
        ));
    }
    for (&wid, w) in &s.wires {
        let [a, b, c, d] = s.curve(w);
        let mut path = BezPath::new();
        path.move_to(a);
        path.curve_to(b, c, d);
        sc.push(Object::path(
            format!("{prefix}wire-{wid}"),
            path,
            [55, 65, 81, 255],
            2.,
        ));
    }
    if interactive {
        for &wid in s.wires.keys() {
            let id = format!("bead-{wid}");
            let mut bead = Object::new(
                id.clone(),
                Affine::translate(s.bead(wid).to_vec2()),
                Some(Hit::Circle(Point::ORIGIN, 10.)),
                vec![Shape::Circle {
                    center: Point::ORIGIN,
                    radius: 4.5,
                    fill: [255; 4],
                    stroke: Some(([55, 65, 81, 255], 1.5)),
                }],
            )
            .hit_when_transparent();
            bead.opacity = if twin(s, wid).is_some()
                || dragged == Some(id.as_str())
                || hovered == Some(id.as_str())
            {
                1.
            } else {
                0.
            };
            sc.push(bead);
        }
    }
    for (id, n) in &s.nodes {
        let p = s.pos(id);
        if target_ids.contains(id) {
            let (fill, stroke) = if spider(n.kind) {
                colors(n.kind)
            } else {
                ([229, 231, 235, 255], [55, 65, 81, 255])
            };
            let mut halo = Object::new(
                format!("{prefix}halo-{id}"),
                Affine::translate(p.to_vec2()),
                None,
                vec![Shape::Circle {
                    center: Point::ORIGIN,
                    radius: 22.,
                    fill: [fill[0], fill[1], fill[2], 89],
                    stroke: Some((stroke, 1.5)),
                }],
            );
            halo.stroke_style.dash = vec![4., 3.];
            sc.push(halo);
        }
        if spider(n.kind) {
            let (fill, stroke) = colors(n.kind);
            sc.push(Object::new(
                format!("{prefix}spider-{id}"),
                Affine::translate(p.to_vec2()),
                if interactive {
                    Some(Hit::Circle(Point::ORIGIN, 26.))
                } else {
                    None
                },
                vec![Shape::Circle {
                    center: Point::ORIGIN,
                    radius: 11.,
                    fill,
                    stroke: Some((stroke, 2.)),
                }],
            ));
            let label = phase_label(n.phase);
            if !label.is_empty() {
                let width = 8. + label.chars().count() as f64 * 6.5;
                let mut pill = Object::new(
                    format!("{prefix}phase-{id}"),
                    Affine::translate(p.to_vec2() + Vec2::new(15., -15.)),
                    if interactive {
                        Some(Hit::Rect(Rect::new(0., 0., width, 16.)))
                    } else {
                        None
                    },
                    vec![
                        Shape::Rect {
                            bounds: Rect::new(0., 0., width, 16.),
                            radius: 8.,
                            fill: [255; 4],
                            stroke: Some((stroke, 1.)),
                        },
                        Shape::Text {
                            origin: Point::new(width / 2., 8.),
                            text: label.into(),
                            size: 11.,
                            color: [17, 24, 39, 255],
                        },
                    ],
                );
                pill.text_style = TextStyle {
                    anchor: TextAnchor::Middle,
                    baseline: TextBaseline::Central,
                    family: FontFamily::Serif,
                    italic: true,
                    ..Default::default()
                };
                sc.push(pill);
            }
        } else if matches!(n.kind, Kind::Cup | Kind::Cap) {
            let cup = n.kind == Kind::Cup;
            let path = kurbo::Arc::new(
                Point::new(0., if cup { -7. } else { 7. }),
                Vec2::new(14., 14.),
                std::f64::consts::PI,
                if cup {
                    -std::f64::consts::PI
                } else {
                    std::f64::consts::PI
                },
                0.,
            )
            .to_path(0.01);
            sc.push(
                Object::new(
                    format!("{prefix}bend-{id}"),
                    Affine::translate(p.to_vec2()),
                    if interactive {
                        Some(Hit::Path(path.clone(), 22.))
                    } else {
                        None
                    },
                    vec![Shape::Path {
                        path,
                        fill: None,
                        stroke: Some(([55, 65, 81, 255], 2.)),
                    }],
                )
                .line_cap(LineCap::Round),
            );
        } else {
            sc.push(
                Object::circle(format!("{prefix}boundary-{id}"), p, 2.5, [55, 65, 81, 255])
                    .decorative(),
            );
        }
    }
    if let Some(id) = dragged {
        if let Some(key) = id.strip_prefix("spider-") {
            let label = format!("phase-{key}");
            sc.objects.sort_by_key(|o| o.id == id || o.id == label);
        } else if id.starts_with("bend-") {
            sc.objects.sort_by_key(|o| o.id == id);
        }
    }
    sc
}

impl Example for ZX {
    type State = State;
    fn initial(&self) -> State {
        preset(0, false)
    }
    fn scene(&self, s: &State) -> Scene {
        self.scene_interaction(s, None, None)
    }
    fn scene_dragging(&self, s: &State, id: Option<&str>) -> Scene {
        self.scene_interaction(s, id, None)
    }
    fn scene_interaction(&self, s: &State, dragged: Option<&str>, hovered: Option<&str>) -> Scene {
        let mut sc = render(s, "", dragged, hovered);
        if s.puzzle != 3 {
            let g = preset(s.puzzle, true);
            let proved = matches(s, &g);
            let height = (34. + 56. * (g.internal() + 1) as f64 + 34.) * 0.42 + 44.;
            let origin = Point::new(328., 12.);
            sc.push(Object::new(
                "goal-frame",
                Affine::translate(origin.to_vec2()),
                None,
                vec![Shape::Rect {
                    bounds: Rect::new(0., 0., 134., height),
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
                    "GOAL",
                    11.,
                    [107, 114, 128, 255],
                )
                .text_baseline(TextBaseline::Alphabetic),
            );
            if proved {
                sc.push(
                    Object::text(
                        "goal-proved",
                        origin + Vec2::new(124., 18.),
                        "PROVED ∎",
                        11.,
                        [5, 150, 105, 255],
                    )
                    .text_anchor(TextAnchor::End)
                    .text_baseline(TextBaseline::Alphabetic)
                    .font_weight(700),
                );
            }
            let mut scene = render(&g, "goal-", None, None);
            for o in &mut scene.objects {
                o.transform =
                    Affine::translate(origin.to_vec2() + Vec2::new((134. - 320. * 0.42) / 2., 30.))
                        * Affine::scale(0.42)
                        * o.transform;
            }
            sc.objects.extend(scene.objects);
        }
        sc
    }
    fn begin_drag(&self, s: &State, id: &str, mods: Modifiers) -> Option<(State, String)> {
        if !mods.alt {
            return None;
        }
        let parent = id.strip_prefix("spider-")?;
        s.nodes.get(parent)?;
        let (next, child) = unfuse(s, parent);
        Some((next, format!("spider-{child}")))
    }
    fn locate(&self, s: &State, id: &str) -> Option<Affine> {
        if let Some(id) = id
            .strip_prefix("spider-")
            .or_else(|| id.strip_prefix("bend-"))
        {
            s.nodes.get(id)?;
            Some(Affine::translate(s.pos(id).to_vec2()))
        } else if let Some(id) = id.strip_prefix("phase-") {
            s.nodes.get(id)?;
            Some(Affine::translate(
                s.pos(id).to_vec2() + Vec2::new(15., -15.),
            ))
        } else if let Some(id) = id.strip_prefix("bead-") {
            let wid = id.parse().ok()?;
            s.wires.get(&wid)?;
            Some(Affine::translate(s.bead(wid).to_vec2()))
        } else {
            None
        }
    }
    fn drag(&self, s: &State, id: &str, m: Modifiers) -> Option<DragSpec<'static, State>> {
        if let Some(id) = id.strip_prefix("spider-") {
            let parent =
                if m.alt && id == format!("s{}", s.next.saturating_sub(1)) && s.at(id).len() == 1 {
                    s.neighbors(id).into_iter().next()
                } else {
                    None
                };
            return Some(spider_spec(s, id, parent));
        }
        if let Some(id) = id.strip_prefix("phase-") {
            let mut a = vec![];
            for target in drop_targets(s, id, true) {
                let p = s.pos(&target);
                let next = Rc::new(transfer(s, id, &target));
                a.push(DragSpec::custom(move |ctx, pointer| {
                    let mut frame = ctx.state_frame(next.clone(), pointer)?;
                    frame.gap = (pointer.distance(p) - 26.).max(0.);
                    Ok(frame)
                }));
            }
            return Some(
                closest(a)
                    .when_far(fixed(s.clone()), 0.)
                    .with_floating()
                    .with_branch_transition(Some(dragology::Transition {
                        duration: std::time::Duration::from_millis(150),
                        ..Default::default()
                    })),
            );
        }
        if let Some(id) = id.strip_prefix("bend-") {
            let id = id.to_string();
            let me = &s.nodes[&id];
            let partner = partner(s, &id);
            let p = partner.as_ref().map(|id| s.pos(id));
            let cup = me.kind == Kind::Cup;
            let mut a = vec![];
            if let (Some(partner), Some(p)) = (partner, p) {
                let t = splice(&splice(s, &id), &partner).tidy(None);
                a.push(conditional(t, move |q| {
                    if cup {
                        q.y < p.y + 14.
                    } else {
                        q.y > p.y - 14.
                    }
                }));
            }
            let base = s.clone();
            let moving = id.clone();
            let constrained = id.clone();
            let layout = id.clone();
            a.push(
                dragology::vary_constrained(
                    [me.p.x, me.p.y],
                    move |[x, y]| {
                        let mut t = base.clone();
                        t.nodes.get_mut(&moving).unwrap().p = Point::new(x, y);
                        t
                    },
                    move |t: &State| {
                        let q = t.pos(&constrained);
                        [
                            44. - q.x,
                            q.x - 276.,
                            p.map_or(0., |p| {
                                if cup {
                                    p.y + 30. - q.y
                                } else {
                                    q.y - p.y + 30.
                                }
                            }),
                        ]
                    },
                )
                .during(move |t| t.clone().tidy(Some(&layout))),
            );
            return Some(
                closest(a)
                    .with_branch_transition(Some(dragology::Transition {
                        duration: std::time::Duration::from_millis(200),
                        ..Default::default()
                    }))
                    .on_drop(|s: &State| s.clone().tidy(None)),
            );
        }
        if let Some(id) = id.strip_prefix("bead-") {
            let wid: usize = id.parse().ok()?;
            let w = s.wires.get(&wid)?;
            let twin = twin(s, wid);
            let tb = twin.map(|i| s.wires[&i].bow);
            let right = tb.is_some_and(|b| w.bow >= b);
            let mut a = vec![];
            if let Some(twin) = twin {
                let [origin, _, _, end] = s.curve(w);
                let chord = end - origin;
                let perp = if chord.hypot() < 1. {
                    Vec2::new(1., 0.)
                } else {
                    Vec2::new(-chord.y, chord.x) / chord.hypot()
                };
                let mine = (s.bead(wid) - origin).dot(perp);
                let other = (s.bead(twin) - origin).dot(perp);
                let side = if mine >= other { 1. } else { -1. };
                let mut t = s.clone();
                t.wires.remove(&wid);
                t.wires.remove(&twin);
                t = t.tidy(None);
                a.push(conditional(t, move |p| {
                    let v = p - origin;
                    let along = v.dot(chord) / chord.hypot2().max(1.);
                    (other - v.dot(perp)) * side > 12. && along > -0.05 && along < 1.05
                }));
            }
            let base = s.clone();
            a.push(dragology::vary_constrained(
                [w.bow, w.t],
                move |[bow, t]| {
                    let mut s = base.clone();
                    let wire = s.wires.get_mut(&wid).unwrap();
                    wire.bow = bow;
                    wire.t = t;
                    s
                },
                move |s: &State| {
                    let w = &s.wires[&wid];
                    [
                        0.12 - w.t,
                        w.t - 0.88,
                        -70. - w.bow,
                        w.bow - 70.,
                        tb.map_or(0., |b| {
                            if right {
                                b + 10. - w.bow
                            } else {
                                w.bow - (b - 10.)
                            }
                        }),
                    ]
                },
            ));
            return Some(
                closest(a).with_branch_transition(Some(dragology::Transition {
                    duration: std::time::Duration::from_millis(200),
                    ..Default::default()
                })),
            );
        }
        None
    }
    fn controls(&self, s: &State) -> Vec<Control> {
        let mut controls: Vec<_> = [
            "CNOT · CNOT = 1",
            "Phase through CNOT",
            "Spider on a snake",
            "Playground",
        ]
        .iter()
        .enumerate()
        .map(|(i, t)| {
            Control::action_button(
                ["puzzle-0", "puzzle-1", "puzzle-2", "puzzle-3"][i],
                *t,
                true,
                s.puzzle == i,
            )
        })
        .collect();
        controls.push(Control::button("restart", "reset"));
        controls
    }
    fn action(&self, s: &State, id: &str, v: Value) -> Option<State> {
        if let Some(i) = id
            .strip_prefix("puzzle-")
            .and_then(|v| v.parse::<usize>().ok())
        {
            return Some(preset(i.min(3), false));
        }
        match (id, v) {
            ("puzzle", Value::Index(i)) => Some(preset(i.min(3), false)),
            ("restart", _) => Some(preset(s.puzzle, false)),
            _ => None,
        }
    }
    fn status(&self, s: &State) -> String {
        ["Two CNOTs in a row cancel. Fuse the Zs, fuse the Xs, then grab the bead on one of the twin wires and pull it across the other wire (Hopf: they cancel). Finally drop each bare spider on a neighbor.",
        "A Z-phase slides through a CNOT's control. Fuse it into the control, Alt-drag a fresh spider out toward the bottom-left (legs follow the side you pull toward, so only the output goes with it), then drag the phase label down onto it.",
        "Two π/4 spiders sit on a zigzag wire. Drag the cup up past the cap to yank it straight, then fuse.","No goal – three independent strands to poke at."][s.puzzle].into()
    }
}
fn preset(i: usize, goal: bool) -> State {
    let mut s = State::new(i);
    if i == 0 || i == 1 {
        for (id, kind, x) in [
            ("in1", Kind::Input, 100.),
            ("in2", Kind::Input, 220.),
            ("out1", Kind::Output, 100.),
            ("out2", Kind::Output, 220.),
        ] {
            s.add(id, kind, 0, x, 0.);
        }
    }
    match (i, goal) {
        (0, false) => {
            for (id, k, x, y) in [
                ("z1", Kind::Z, 100., 1.),
                ("x1", Kind::X, 220., 2.),
                ("z2", Kind::Z, 100., 3.),
                ("x2", Kind::X, 220., 4.),
            ] {
                s.add(id, k, 0, x, y);
            }
            for (a, b) in [
                ("in1", "z1"),
                ("z1", "z2"),
                ("z2", "out1"),
                ("in2", "x1"),
                ("x1", "x2"),
                ("x2", "out2"),
                ("z1", "x1"),
                ("z2", "x2"),
            ] {
                s.wire(a, 0, b, 0);
            }
        }
        (0, true) => {
            s.wire("in1", 0, "out1", 0);
            s.wire("in2", 0, "out2", 0);
        }
        (1, g) => {
            s.add("p", Kind::Z, 2, 100., if g { 3. } else { 1. });
            s.add("z", Kind::Z, 0, 100., if g { 1. } else { 2. });
            s.add("x", Kind::X, 0, 220., if g { 2. } else { 3. });
            for (a, b) in if g {
                [
                    ("in1", "z"),
                    ("z", "p"),
                    ("p", "out1"),
                    ("in2", "x"),
                    ("x", "out2"),
                    ("z", "x"),
                ]
            } else {
                [
                    ("in1", "p"),
                    ("p", "z"),
                    ("z", "out1"),
                    ("in2", "x"),
                    ("x", "out2"),
                    ("z", "x"),
                ]
            } {
                s.wire(a, 0, b, 0);
            }
        }
        (2, g) => {
            s.add("in1", Kind::Input, 0, if g { 160. } else { 100. }, 0.);
            s.add("out1", Kind::Output, 0, if g { 160. } else { 240. }, 0.);
            if g {
                s.add("z", Kind::Z, 2, 160., 1.);
                s.wire("in1", 0, "z", 0);
                s.wire("z", 0, "out1", 0);
            } else {
                s.add("cap", Kind::Cap, 0, 200., 1.);
                s.add("za", Kind::Z, 1, 150., 2.);
                s.add("cup", Kind::Cup, 0, 130., 3.);
                s.add("zb", Kind::Z, 1, 240., 4.);
                for (a, sa, b, sb) in [
                    ("in1", 0, "cup", 0),
                    ("cup", 1, "za", 0),
                    ("za", 0, "cap", 0),
                    ("cap", 1, "zb", 0),
                    ("zb", 0, "out1", 0),
                ] {
                    s.wire(a, sa, b, sb);
                }
            }
        }
        _ => {
            for (id, k, x) in [
                ("in1", Kind::Input, 60.),
                ("in2", Kind::Input, 150.),
                ("in3", Kind::Input, 230.),
                ("out1", Kind::Output, 60.),
                ("out2", Kind::Output, 150.),
                ("out3", Kind::Output, 290.),
            ] {
                s.add(id, k, 0, x, 0.);
            }
            s.add("za", Kind::Z, 2, 60., 1.);
            s.add("xb", Kind::X, 1, 150., 2.);
            s.add("zc", Kind::Z, 1, 60., 3.);
            s.add("cap", Kind::Cap, 0, 276., 4.);
            s.add("cup", Kind::Cup, 0, 244., 5.);
            for (a, sa, b, sb) in [
                ("in1", 0, "za", 0),
                ("za", 0, "zc", 0),
                ("zc", 0, "out1", 0),
                ("in2", 0, "xb", 0),
                ("xb", 0, "out2", 0),
                ("in3", 0, "cup", 0),
                ("cup", 1, "cap", 0),
                ("cap", 1, "out3", 0),
            ] {
                s.wire(a, sa, b, sb);
            }
        }
    }
    s.tidy(None)
}
pub fn entries() -> Vec<Entry> {
    vec![Entry::new(
        "zx-string-diagram",
        "ZX string diagrams",
        "⚠️ Just a seed! A ZX-calculus string diagram editor where every drag is a rewrite rule. Drag spiders past each other (interchange), drop one on an adjacent same-colored spider (fusion; a bare phase-0 spider drops on any neighbor: identity), Alt-drag out of a spider (unfusion), drag a phase label onto a same-colored neighbor, drag a wire’s bead sideways (and through a twin wire: Hopf), and drag a cup past its cap (snake).",
        || demo(ZX),
    )]
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn alt_unfusion_preserves_parent_identity_phase_and_cancel_snapshot() {
        let mut state = preset(1, false);
        state = fuse(&state, "p", "z");
        let before = state.nodes["z"].clone();
        let (next, id) = ZX
            .begin_drag(
                &state,
                "spider-z",
                Modifiers {
                    alt: true,
                    ..Default::default()
                },
            )
            .unwrap();
        let child = id.strip_prefix("spider-").unwrap();
        assert_ne!(child, "z");
        assert_eq!(next.nodes["z"], before);
        assert_eq!(next.nodes[child].phase, 0);
        assert_eq!(next.at(child).len(), 1);
        let mut demo = crate::runtime::ModelDemo::new(ZX);
        demo.set_animated(false);
        let initial = demo.scene().clone();
        demo.press(
            Point::new(100., 90.),
            Modifiers {
                alt: true,
                ..Default::default()
            },
            1,
        );
        demo.move_to(
            Point::new(50., 160.),
            Modifiers {
                alt: true,
                ..Default::default()
            },
        );
        demo.advance(std::time::Duration::ZERO);
        demo.cancel();
        assert_eq!(demo.scene(), &initial);
    }
    #[test]
    fn ordinary_wire_bead_is_invisible_but_hoverable() {
        let state = preset(0, false);
        let scene = ZX.scene(&state);
        let bead = scene.object("bead-0").unwrap();
        assert_eq!(bead.opacity, 0.);
        let mut demo = crate::runtime::ModelDemo::new(ZX);
        demo.set_animated(false);
        demo.move_to(state.bead(0), Modifiers::default());
        demo.advance(std::time::Duration::ZERO);
        assert_eq!(demo.scene().object("bead-0").unwrap().opacity, 1.);
        assert_eq!(phase_label(5), "-3π/4");
        assert_eq!(phase_label(6), "-π/2");
        assert_eq!(phase_label(7), "-π/4");
    }
    #[test]
    fn cnot_squared_reduces_to_two_identity_wires() {
        let s = preset(0, false);
        let mut s = fuse(&fuse(&s, "z1", "z2"), "x1", "x2");
        let wid = *s
            .wires
            .iter()
            .find(|(_, w)| w.a.node == "z2" && w.b.node == "x2")
            .unwrap()
            .0;
        let twin = twin(&s, wid).unwrap();
        s.wires.remove(&wid);
        s.wires.remove(&twin);
        assert!(s.bare("z2") && s.bare("x2"));
        s = remove_identity(&remove_identity(&s, "z2"), "x2");
        assert!(matches(&s, &preset(0, true)));
    }
    #[test]
    fn snake_yank_preserves_threaded_phases() {
        let s = preset(2, false);
        assert_eq!(partner(&s, "cup"), Some("cap".into()));
        let s = splice(&splice(&s, "cup"), "cap").tidy(None);
        let s = fuse(&s, "za", "zb");
        assert!(matches(&s, &preset(2, true)));
    }
    #[test]
    fn phase_transport_and_unfusion_reach_the_source_goal() {
        let s = fuse(&preset(1, false), "p", "z");
        let (mut s, child) = unfuse(&s, "z");
        // Pull down-left far enough that the control leg remains with its parent.
        s.nodes.get_mut(&child).unwrap().p = Point::new(30., 140.);
        s = s.tidy(Some(&child));
        s = split_legs(&s, "z", &child);
        s = transfer(&s, "z", &child);
        assert!(matches(&s, &preset(1, true)));
    }
}
