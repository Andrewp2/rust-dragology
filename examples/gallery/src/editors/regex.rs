use super::*;
use dragology::{closest, fixed};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
#[derive(Clone, PartialEq)]
enum Kind {
    Char(char),
    Concat,
    Alt,
    Star,
}
#[derive(Clone, PartialEq)]
struct Node {
    id: usize,
    kind: Kind,
    children: Vec<Node>,
}
#[derive(Clone, PartialEq)]
pub struct State {
    root: Option<Node>,
    brushes: [usize; 3],
    next: usize,
    trash: Option<Node>,
    trash_star: Option<usize>,
    dfa: bool,
}
pub struct RegexDemo;
fn atom(id: usize, c: char) -> Node {
    Node {
        id,
        kind: Kind::Char(c),
        children: vec![],
    }
}
fn group(id: usize, kind: Kind, children: Vec<Node>) -> Node {
    Node { id, kind, children }
}
fn find(n: &Node, id: usize) -> Option<&Node> {
    if n.id == id {
        Some(n)
    } else {
        n.children.iter().find_map(|c| find(c, id))
    }
}
fn nodes(n: &Node, ids: &mut Vec<usize>) {
    ids.push(n.id);
    for c in &n.children {
        nodes(c, ids)
    }
}
fn replace(n: &Node, id: usize, with: &Node) -> Node {
    if n.id == id {
        return with.clone();
    }
    let mut out = n.clone();
    out.children = n.children.iter().map(|c| replace(c, id, with)).collect();
    out
}
fn remove(n: &Node, id: usize) -> Option<Node> {
    if n.id == id {
        return None;
    }
    let mut out = n.clone();
    out.children = n.children.iter().filter_map(|c| remove(c, id)).collect();
    match out.kind {
        Kind::Char(_) => Some(out),
        _ if out.children.is_empty() => None,
        Kind::Concat | Kind::Alt if out.children.len() == 1 => Some(out.children.remove(0)),
        _ => Some(out),
    }
}
fn placements(root: Option<&Node>, item: &Node, fresh: usize) -> Vec<Node> {
    let Some(root) = root else {
        return vec![item.clone()];
    };
    let mut out = vec![];
    fn visit(
        root: &Node,
        n: &Node,
        parent: Option<&Kind>,
        item: &Node,
        fresh: usize,
        out: &mut Vec<Node>,
    ) {
        if matches!(n.kind, Kind::Concat | Kind::Alt) {
            for i in 0..=n.children.len() {
                let mut target = n.clone();
                target.children.insert(i, item.clone());
                out.push(replace(root, n.id, &target));
            }
        }
        for (kind, id) in [(Kind::Concat, fresh), (Kind::Alt, fresh + 1)] {
            if parent != Some(&kind) {
                for first in [false, true] {
                    let children = if first {
                        vec![item.clone(), n.clone()]
                    } else {
                        vec![n.clone(), item.clone()]
                    };
                    out.push(replace(root, n.id, &group(id, kind.clone(), children)));
                }
            }
        }
        for c in &n.children {
            visit(root, c, Some(&n.kind), item, fresh, out)
        }
    }
    visit(root, root, None, item, fresh, &mut out);
    out
}
fn star_child(root: &Node, id: usize) -> bool {
    (root.kind == Kind::Star && root.children.iter().any(|c| c.id == id))
        || root.children.iter().any(|c| star_child(c, id))
}
fn expression(n: &Node, parent: u8) -> String {
    let (prec, s) = match n.kind {
        Kind::Char(c) => (3, c.to_string()),
        Kind::Concat => (1, n.children.iter().map(|c| expression(c, 1)).collect()),
        Kind::Alt => (
            0,
            n.children
                .iter()
                .map(|c| expression(c, 0))
                .collect::<Vec<_>>()
                .join("|"),
        ),
        Kind::Star => (2, format!("{}*", expression(&n.children[0], 2))),
    };
    if prec < parent { format!("({s})") } else { s }
}
fn size(n: &Node) -> (f64, f64) {
    let sizes: Vec<_> = n.children.iter().map(size).collect();
    match n.kind {
        Kind::Char(_) => (26., 26.),
        Kind::Star => (sizes[0].0 + 29., sizes[0].1 + 10.),
        Kind::Concat => (
            10. + sizes.iter().map(|s| s.0).sum::<f64>()
                + 5. * sizes.len().saturating_sub(1) as f64,
            10. + sizes.iter().map(|s| s.1).fold(0., f64::max),
        ),
        Kind::Alt => (
            27. + sizes.iter().map(|s| s.0).fold(0., f64::max),
            10. + sizes.iter().map(|s| s.1).sum::<f64>()
                + 5. * sizes.len().saturating_sub(1) as f64,
        ),
    }
}
fn centered(
    id: impl Into<String>,
    p: impl Into<Point>,
    text: impl Into<String>,
    size: f64,
    color: Color,
) -> Object {
    Object::text(id, p, text, size, color)
        .text_anchor(TextAnchor::Middle)
        .text_baseline(TextBaseline::Central)
}
fn outline(mut o: Object, color: Color, width: f64) -> Object {
    for s in &mut o.shapes {
        match s {
            Shape::Rect { stroke, .. } | Shape::Circle { stroke, .. } => {
                *stroke = Some((color, width))
            }
            _ => {}
        }
    }
    o
}
fn draw_ast(
    n: &Node,
    x: f64,
    y: f64,
    parent: Option<usize>,
    highlight: &BTreeSet<usize>,
    scene: &mut Scene,
) {
    let (w, h) = size(n);
    let (fill, stroke, radius) = match n.kind {
        Kind::Char(_) => (WHITE, hex_color("94a3b8"), 6.),
        Kind::Concat => ([100, 116, 139, 13], hex_color("cbd5e1"), 8.),
        Kind::Alt => ([167, 139, 250, 26], hex_color("a78bfa"), 8.),
        Kind::Star => ([251, 146, 60, 20], hex_color("fdba74"), 8.),
    };
    let selected = highlight.contains(&n.id);
    let mut obj = outline(
        Object::rect(
            format!("re-{}", n.id),
            Rect::new(x, y, x + w, y + h),
            radius,
            if selected { hex_color("fef3c7") } else { fill },
        ),
        if selected {
            hex_color("f59e0b")
        } else {
            stroke
        },
        1.,
    );
    obj.parent = parent.map(|p| format!("re-{p}"));
    if let Kind::Char(c) = n.kind {
        obj = obj
            .label(c.to_string(), (13., 13.), 16., hex_color("1e293b"))
            .text_anchor(TextAnchor::Middle)
            .text_baseline(TextBaseline::Central)
            .font_family(FontFamily::Mono);
    }
    scene.push(obj);
    if n.kind == Kind::Alt {
        let mut label = centered(
            format!("alt-{}", n.id),
            (x + 11., y + h / 2.),
            "|",
            16.,
            hex_color("7c3aed"),
        )
        .font_weight(700);
        label.parent = Some(format!("re-{}", n.id));
        scene.push(label);
    }
    let mut dx = x + 5.;
    let mut dy = y + 5.;
    for c in &n.children {
        let (cw, ch) = size(c);
        let (cx, cy) = match n.kind {
            Kind::Concat => (dx, y + (h - ch) / 2.),
            Kind::Alt => (x + 22. + (w - 27. - cw) / 2., dy),
            _ => (x + 5., y + 5.),
        };
        draw_ast(c, cx, cy, Some(n.id), highlight, scene);
        dx += cw + 5.;
        dy += ch + 5.;
    }
    if n.kind == Kind::Star {
        let mut badge = star_badge(n.id, x + w - 19., y + 5.);
        badge.parent = Some(format!("re-{}", n.id));
        scene.push(badge);
    }
}
fn star_badge(id: usize, x: f64, y: f64) -> Object {
    outline(
        Object::rect(
            format!("star-{id}"),
            Rect::new(x, y, x + 14., y + 16.),
            4.,
            hex_color("fed7aa"),
        ),
        hex_color("fdba74"),
        1.,
    )
    .label("*", (7., 11.), 17., hex_color("c2410c"))
    .text_anchor(TextAnchor::Middle)
    .text_baseline(TextBaseline::Central)
    .font_weight(700)
}
#[derive(Clone)]
struct EdgeGeometry {
    owner: usize,
    kind: u8,
    bulge: f64,
}
#[derive(Clone)]
struct Nfa {
    start: usize,
    end: usize,
    nodes: BTreeMap<usize, Point>,
    edges: Vec<(usize, usize, Option<char>)>,
    geometry: BTreeMap<(usize, usize), EdgeGeometry>,
    w: f64,
    above: f64,
    below: f64,
}
impl Nfa {
    fn empty() -> Self {
        Self {
            start: usize::MAX,
            end: usize::MAX,
            nodes: BTreeMap::from([(usize::MAX, Point::ORIGIN)]),
            edges: vec![],
            geometry: BTreeMap::new(),
            w: 0.,
            above: 10.,
            below: 10.,
        }
    }
    fn edge(
        &mut self,
        owner: usize,
        a: usize,
        b: usize,
        label: Option<char>,
        kind: u8,
        bulge: f64,
    ) {
        self.edges.push((a, b, label));
        self.geometry
            .insert((a, b), EdgeGeometry { owner, kind, bulge });
    }
    fn append(&mut self, other: Self, dx: f64, dy: f64) {
        self.nodes.extend(
            other
                .nodes
                .into_iter()
                .map(|(id, p)| (id, p + Vec2::new(dx, dy))),
        );
        self.edges.extend(other.edges);
        self.geometry
            .extend(other.geometry.into_iter().map(|(id, mut g)| {
                g.bulge += dy;
                (id, g)
            }));
    }
}
fn thompson(n: &Node) -> Nfa {
    let a = n.id * 2;
    let b = a + 1;
    let mut g = Nfa {
        start: a,
        end: b,
        nodes: BTreeMap::new(),
        edges: vec![],
        geometry: BTreeMap::new(),
        w: 0.,
        above: 10.,
        below: 10.,
    };
    let kids: Vec<_> = n.children.iter().map(thompson).collect();
    match n.kind {
        Kind::Char(c) => {
            g.w = 64.;
            g.edge(n.id, a, b, Some(c), 0, 0.);
        }
        Kind::Concat => {
            g.start = kids[0].start;
            g.end = kids.last().unwrap().end;
            g.above = kids.iter().map(|k| k.above).fold(0., f64::max);
            g.below = kids.iter().map(|k| k.below).fold(0., f64::max);
            for (i, k) in kids.iter().enumerate() {
                let x = g.w;
                g.w += k.w;
                g.append(k.clone(), x, 0.);
                if i + 1 < kids.len() {
                    g.edge(n.id, k.end, kids[i + 1].start, None, 0, 0.);
                    g.w += 40.;
                }
            }
            return g;
        }
        Kind::Alt => {
            let maxw = kids.iter().map(|k| k.w).fold(0., f64::max);
            let total = kids.iter().map(|k| k.above + k.below).sum::<f64>()
                + 16. * kids.len().saturating_sub(1) as f64;
            g.w = 108. + maxw;
            g.above = (total / 2.).max(10.);
            g.below = g.above;
            let mut y = -total / 2.;
            for k in kids {
                let cy = y + k.above;
                let x = 54. + (maxw - k.w) / 2.;
                let (i, o) = (k.start, k.end);
                y += k.above + k.below + 16.;
                g.append(k, x, cy);
                g.edge(n.id, a, i, None, 1, 0.);
                g.edge(n.id, o, b, None, 1, 0.);
            }
        }
        Kind::Star => {
            let k = &kids[0];
            g.w = 92. + k.w;
            g.above = k.above + 30.;
            g.below = k.below + 30.;
            g.append(k.clone(), 46., 0.);
            g.edge(n.id, a, k.start, None, 0, 0.);
            g.edge(n.id, k.end, b, None, 0, 0.);
            g.edge(n.id, k.end, k.start, None, 2, -k.above - 24.);
            g.edge(n.id, a, b, None, 2, k.below + 24.);
        }
    }
    g.nodes.insert(a, Point::ORIGIN);
    g.nodes.insert(b, Point::new(g.w, 0.));
    g
}
fn nfa_name(id: usize) -> String {
    if id == usize::MAX {
        "s-empty".into()
    } else {
        format!("s-n{}-{}", id / 2, if id % 2 == 0 { "in" } else { "out" })
    }
}
fn dfa_name(set: &BTreeSet<usize>) -> String {
    let mut names: Vec<_> = set.iter().map(|i| nfa_name(*i)).collect();
    names.sort();
    format!(
        "dfa-{}",
        names
            .into_iter()
            .map(|s| s
                .trim_start_matches("s-")
                .replace("-in", "i")
                .replace("-out", "o"))
            .collect::<Vec<_>>()
            .join("_")
    )
}
fn dfa_positions(states: &[BTreeSet<usize>], edges: &[(usize, usize, char)]) -> Vec<Point> {
    use dagre::{
        EdgeLabel, LayoutOptions, NodeLabel, RankDir,
        graph::{Graph, GraphOptions},
    };
    let mut graph = Graph::with_options(GraphOptions {
        directed: true,
        multigraph: true,
        compound: true,
    });
    let ids: Vec<_> = states.iter().map(dfa_name).collect();
    for id in &ids {
        graph.set_node(
            id.clone(),
            Some(NodeLabel {
                width: 22.,
                height: 22.,
                ..Default::default()
            }),
        );
    }
    for &(a, b, _) in edges {
        if a != b {
            graph.set_edge(&ids[a], &ids[b], Some(EdgeLabel::default()), None);
        }
    }
    dagre::layout(
        &mut graph,
        Some(LayoutOptions {
            rankdir: RankDir::LR,
            nodesep: 44.,
            ranksep: 72.,
            marginx: 0.,
            marginy: 0.,
            tie_keep_first: true,
            ..Default::default()
        }),
    );
    ids.iter()
        .map(|id| {
            let n = graph.node(id).unwrap();
            Point::new(n.x.unwrap(), n.y.unwrap())
        })
        .collect()
}
fn head(scene: &mut Scene, id: &str, tip: Point, dir: Vec2, color: Color) {
    let side = Vec2::new(-dir.y, dir.x) * (8. * (std::f64::consts::PI / 7.).tan());
    let mut p = BezPath::new();
    p.move_to(tip);
    p.line_to(tip - dir * 8. + side);
    p.line_to(tip - dir * 8. - side);
    p.close_path();
    scene.push(Object::new(
        id,
        Affine::IDENTITY,
        None,
        vec![Shape::Path {
            path: p,
            fill: Some(color),
            stroke: None,
        }],
    ));
}
fn start_arrow(scene: &mut Scene, id: &str, p: Point, r: f64) {
    scene.push(Object::line(
        format!("{id}-line"),
        p - Vec2::new(r + 24., 0.),
        p - Vec2::new(r + 8., 0.),
        hex_color("475569"),
        1.2,
    ));
    head(
        scene,
        id,
        p - Vec2::new(r, 0.),
        Vec2::new(1., 0.),
        hex_color("475569"),
    );
}
fn draw_nfa(scene: &mut Scene, g: &Nfa, origin: Vec2, selected: &BTreeSet<usize>) {
    start_arrow(scene, "nfa-start-arrow", g.nodes[&g.start] + origin, 10.);
    for &(a, b, c) in &g.edges {
        let geo = &g.geometry[&(a, b)];
        let p = g.nodes[&a] + origin;
        let q = g.nodes[&b] + origin;
        let mut path = BezPath::new();
        let (tip, dir, label);
        match geo.kind {
            0 => {
                dir = (q - p).normalize();
                let start = p + dir * 10.;
                tip = q - dir * 10.;
                path.move_to(start);
                path.line_to(tip - dir * 8.);
                label = p.midpoint(q) - Vec2::new(0., if c.is_some() { 9. } else { 7. });
            }
            1 => {
                dir = Vec2::new(1., 0.);
                let start = p + dir * 10.;
                tip = q - dir * 10.;
                let end = tip - dir * 8.;
                let mx = (start.x + end.x) / 2.;
                path.move_to(start);
                path.curve_to(Point::new(mx, start.y), Point::new(mx, end.y), end);
                label = Point::new(mx, (start.y + end.y) / 2. - 6.);
            }
            _ => {
                let bulge = geo.bulge + origin.y;
                let sign = if bulge < p.y { -1. } else { 1. };
                let start = p + Vec2::new(0., sign * 10.);
                tip = q + Vec2::new(0., sign * 10.);
                dir = Vec2::new(0., -sign);
                let end = tip - dir * 8.;
                path.move_to(start);
                path.curve_to(Point::new(p.x, bulge), Point::new(q.x, bulge), end);
                label = Point::new(
                    (p.x + q.x) / 2.,
                    (start.y + 6. * bulge + end.y) / 8. + sign * 7.,
                );
            }
        }
        let hl = selected.contains(&geo.owner);
        let color = hex_color(if hl {
            "f59e0b"
        } else if c.is_some() {
            "1e293b"
        } else {
            "94a3b8"
        });
        let id = format!("edge-{a}-{b}");
        let mut line = Object::path(
            &id,
            path,
            color,
            if hl {
                2.
            } else if c.is_some() {
                1.5
            } else {
                1.
            },
        );
        if c.is_none() {
            line = line.dash(vec![3., 3.]);
        }
        scene.push(line);
        head(scene, &format!("{id}-head"), tip, dir, color);
        scene.push(
            centered(
                format!("{id}-label"),
                label,
                c.map(|c| c.to_string()).unwrap_or_else(|| "ε".into()),
                12.,
                color,
            )
            .font_family(FontFamily::Mono)
            .text_outline(WHITE, 3.),
        );
    }
    for (&id, &p) in &g.nodes {
        let hl = id != usize::MAX && selected.contains(&(id / 2));
        let color = hex_color(if hl { "f59e0b" } else { "475569" });
        scene.push(
            outline(
                Object::circle(
                    format!("nfa-{id}"),
                    p + origin,
                    10.,
                    if hl { hex_color("fef3c7") } else { WHITE },
                ),
                color,
                1.5,
            )
            .decorative(),
        );
        if id == g.end {
            scene.push(
                outline(
                    Object::circle(format!("nfa-inner-{id}"), p + origin, 7., [0; 4]),
                    color,
                    1.5,
                )
                .decorative(),
            );
        }
    }
}
fn draw_dfa(scene: &mut Scene, g: &Nfa, origin: Vec2) {
    let (states, edges) = subsets(g);
    let positions: Vec<_> = dfa_positions(&states, &edges)
        .into_iter()
        .map(|p| p + origin)
        .collect();
    start_arrow(scene, "dfa-start-arrow", positions[0], 11.);
    let mut merged: Vec<(usize, usize, String)> = vec![];
    for (a, b, c) in edges {
        if let Some(e) = merged.iter_mut().find(|e| e.0 == a && e.1 == b) {
            e.2.push(',');
            e.2.push(c);
        } else {
            merged.push((a, b, c.to_string()));
        }
    }
    for (a, b, label) in merged {
        let p = positions[a];
        let q = positions[b];
        let mut path = BezPath::new();
        let (tip, dir, lp);
        if a == b {
            let start = p + Vec2::new(-5., -10.);
            tip = p + Vec2::new(5., -10.);
            dir = Vec2::new(0.4, 1.).normalize();
            path.move_to(start);
            path.curve_to(
                p + Vec2::new(-22., -41.),
                p + Vec2::new(22., -41.),
                tip - dir * 8.,
            );
            lp = p + Vec2::new(0., -41.);
        } else {
            let backward = q.x < p.x;
            let pq = q - p;
            let unit = pq.normalize();
            let normal = Vec2::new(-unit.y, unit.x);
            let ctrl = p.midpoint(q) + normal * if backward { -0.35 * pq.hypot() } else { 0. };
            let start = p + (ctrl - p).normalize() * 11.;
            tip = q + (ctrl - q).normalize() * 11.;
            dir = (tip - ctrl).normalize();
            let end = tip - dir * 8.;
            path.move_to(start);
            path.quad_to(ctrl, end);
            lp = Point::new(
                start.x * 0.25 + ctrl.x * 0.5 + end.x * 0.25,
                start.y * 0.25 + ctrl.y * 0.5 + end.y * 0.25,
            ) + normal * if backward { -8. } else { 8. };
        }
        let id = format!("de-{}-{}", dfa_name(&states[a]), dfa_name(&states[b]));
        let color = hex_color("1e293b");
        scene.push(Object::path(&id, path, color, 1.5));
        head(scene, &format!("{id}-head"), tip, dir, color);
        scene.push(
            centered(format!("{id}-label"), lp, label, 12., color)
                .font_family(FontFamily::Mono)
                .font_weight(700)
                .text_outline(WHITE, 3.),
        );
    }
    for (i, set) in states.iter().enumerate() {
        let id = dfa_name(set);
        scene.push(
            outline(
                Object::circle(&id, positions[i], 11., WHITE),
                hex_color("475569"),
                1.5,
            )
            .decorative(),
        );
        if set.contains(&g.end) {
            scene.push(
                outline(
                    Object::circle(format!("{id}-inner"), positions[i], 8., [0; 4]),
                    hex_color("475569"),
                    1.5,
                )
                .decorative(),
            );
        }
    }
}
fn closure(g: &Nfa, mut set: BTreeSet<usize>) -> BTreeSet<usize> {
    loop {
        let mut added = false;
        for (a, b, c) in &g.edges {
            if c.is_none() && set.contains(a) {
                added |= set.insert(*b);
            }
        }
        if !added {
            return set;
        }
    }
}
fn step(g: &Nfa, set: &BTreeSet<usize>, ch: char) -> BTreeSet<usize> {
    closure(
        g,
        g.edges
            .iter()
            .filter(|(a, _, c)| set.contains(a) && *c == Some(ch))
            .map(|(_, b, _)| *b)
            .collect(),
    )
}
fn accepts(g: &Nfa, word: &str) -> bool {
    let mut set = closure(g, BTreeSet::from([g.start]));
    for c in word.chars() {
        set = step(g, &set, c)
    }
    set.contains(&g.end)
}
fn subsets(g: &Nfa) -> (Vec<BTreeSet<usize>>, Vec<(usize, usize, char)>) {
    let mut states = vec![closure(g, BTreeSet::from([g.start]))];
    let mut queue = VecDeque::from([0]);
    let mut edges = vec![];
    while let Some(i) = queue.pop_front() {
        for c in ['a', 'b'] {
            let next = step(g, &states[i], c);
            if next.is_empty() {
                continue;
            }
            let j = if let Some(j) = states.iter().position(|s| *s == next) {
                j
            } else {
                let j = states.len();
                states.push(next);
                queue.push_back(j);
                j
            };
            edges.push((i, j, c));
        }
    }
    (states, edges)
}
impl Example for RegexDemo {
    type State = State;
    fn initial(&self) -> State {
        State {
            root: Some(group(
                0,
                Kind::Concat,
                vec![
                    group(
                        1,
                        Kind::Star,
                        vec![group(2, Kind::Alt, vec![atom(3, 'a'), atom(4, 'b')])],
                    ),
                    atom(5, 'a'),
                    atom(6, 'b'),
                ],
            )),
            brushes: [10, 11, 12],
            next: 13,
            trash: None,
            trash_star: None,
            dfa: false,
        }
    }
    fn scene(&self, s: &State) -> Scene {
        self.scene_dragging(s, None)
    }
    fn scene_dragging(&self, s: &State, id: Option<&str>) -> Scene {
        let mut scene = Scene::new(820., 820.);
        let mut selected = BTreeSet::new();
        if let Some(id) = id
            .and_then(|s| s.strip_prefix("re-").or_else(|| s.strip_prefix("star-")))
            .and_then(|s| s.parse().ok())
        {
            if let Some(n) = s.root.as_ref().and_then(|r| find(r, id)) {
                let mut ids = vec![];
                nodes(n, &mut ids);
                selected.extend(ids);
            }
        }
        scene.push(
            Object::text(
                "regex",
                (14., 18.),
                s.root
                    .as_ref()
                    .map(|n| expression(n, 0))
                    .unwrap_or_else(|| "ε".into()),
                18.,
                hex_color("1e293b"),
            )
            .text_baseline(TextBaseline::Central)
            .font_family(FontFamily::Mono),
        );
        for (i, id) in s.brushes.iter().enumerate() {
            if i < 2 {
                draw_ast(
                    &atom(*id, if i == 0 { 'a' } else { 'b' }),
                    14.,
                    44. + i as f64 * 38.,
                    None,
                    &selected,
                    &mut scene,
                );
            } else {
                scene.push(star_badge(*id, 20., 125.));
            }
        }
        let trash_active = s.trash.is_some() || s.trash_star.is_some();
        scene.push(
            outline(
                Object::circle(
                    "trash",
                    (27., 172.),
                    15.,
                    hex_color(if trash_active { "fecaca" } else { "f8fafc" }),
                ),
                hex_color(if trash_active { "ef4444" } else { "e2e8f0" }),
                1.,
            )
            .dash(vec![3., 2.])
            .decorative(),
        );
        scene.push(centered(
            "trash-label",
            (27., 172.),
            "🗑",
            14.,
            hex_color("94a3b8"),
        ));
        if let Some(n) = &s.trash {
            let (w, h) = size(n);
            let before = scene.objects.len();
            draw_ast(n, 27. - w / 2., 172. - h / 2., None, &selected, &mut scene);
            for o in &mut scene.objects[before..] {
                o.opacity = 0.;
            }
        }
        if let Some(id) = s.trash_star {
            let mut badge = star_badge(id, 20., 164.);
            badge.opacity = 0.;
            scene.push(badge);
        }
        let ah = if let Some(root) = &s.root {
            draw_ast(root, 72., 44., None, &selected, &mut scene);
            size(root).1
        } else {
            0.
        };
        let g = s.root.as_ref().map(thompson).unwrap_or_else(Nfa::empty);
        let nfa_y = (44. + ah).max(187.) + 28.;
        let origin = Vec2::new(64., nfa_y + 16. + g.above);
        let label = |id: &str, p: Point, text: &str| {
            Object::text(id, p, text, 11., hex_color("64748b"))
                .text_baseline(TextBaseline::Central)
                .font_weight(700)
        };
        scene.push(label("nfa-label", Point::new(14., nfa_y), "THOMPSON NFA"));
        draw_nfa(&mut scene, &g, origin, &selected);
        let match_y = origin.y + g.below + 30.;
        scene.push(label(
            "words-label",
            Point::new(14., match_y),
            "MATCHING STRINGS",
        ));
        for len in 0..=4 {
            let cw = (8. * len as f64 + 8.).max(20.);
            for bits in 0..1usize << len {
                let word: String = (0..len)
                    .rev()
                    .map(|i| if bits & (1 << i) == 0 { 'a' } else { 'b' })
                    .collect();
                let ok = accepts(&g, &word);
                let x = 14. + bits as f64 * (cw + 5.);
                let y = match_y + 16. + len as f64 * 26.;
                scene.push(
                    outline(
                        Object::rect(
                            format!("match-{word}"),
                            Rect::new(x, y, x + cw, y + 20.),
                            5.,
                            hex_color(if ok { "bbf7d0" } else { "f1f5f9" }),
                        ),
                        hex_color(if ok { "22c55e" } else { "e2e8f0" }),
                        1.,
                    )
                    .label(
                        if word.is_empty() { "ε" } else { &word },
                        (cw / 2., 10.),
                        12.,
                        hex_color(if ok { "166534" } else { "cbd5e1" }),
                    )
                    .text_anchor(TextAnchor::Middle)
                    .text_baseline(TextBaseline::Central)
                    .font_family(FontFamily::Mono)
                    .decorative(),
                );
            }
        }
        let dy = match_y + 16. + 124. + 30.;
        let mut toggle = label(
            "dfa-toggle",
            Point::new(14., dy),
            if s.dfa {
                "▾ DFA (SUBSET CONSTRUCTION)"
            } else {
                "▸ DFA (SUBSET CONSTRUCTION)"
            },
        );
        toggle.click_handler = true;
        toggle.hit = Some(Hit::Rect(Rect::new(0., -10., 260., 12.)));
        scene.push(toggle);
        if s.dfa {
            draw_dfa(&mut scene, &g, Vec2::new(64., dy + 56.));
        }
        scene
    }
    fn click(&self, s: &State, id: &str, _: Point, _: Modifiers, _: usize) -> Option<State> {
        if id == "dfa-toggle" {
            Some(State {
                dfa: !s.dfa,
                ..s.clone()
            })
        } else {
            None
        }
    }
    fn drag(&self, s: &State, id: &str, _: Modifiers) -> Option<DragSpec<'static, State>> {
        let star = id.starts_with("star-");
        let n = id
            .strip_prefix(if star { "star-" } else { "re-" })?
            .parse::<usize>()
            .ok()?;
        let brush = s.brushes.iter().position(|x| *x == n);
        if !star && brush.is_none() && s.root.as_ref().is_some_and(|root| root.id == n) {
            return None;
        }
        let mut base = s.clone();
        base.trash = None;
        base.trash_star = None;
        if let Some(i) = brush {
            base.brushes[i] = base.next;
            base.next += 1;
        }
        let mut states = if brush.is_some() {
            vec![]
        } else {
            vec![s.clone()]
        };
        if star {
            let mut old_child = None;
            if brush.is_none() {
                let root = base.root.as_ref()?;
                let old = find(root, n)?;
                if old.kind != Kind::Star {
                    return None;
                }
                old_child = Some(old.children[0].id);
                base.root = Some(replace(root, n, &old.children[0]));
            }
            if let Some(root) = &base.root {
                let mut ids = vec![];
                nodes(root, &mut ids);
                for id in ids {
                    let child = find(root, id).unwrap();
                    if child.kind == Kind::Star || star_child(root, id) || old_child == Some(id) {
                        continue;
                    }
                    let mut target = base.clone();
                    target.root = Some(replace(
                        root,
                        id,
                        &group(n, Kind::Star, vec![child.clone()]),
                    ));
                    states.push(target);
                }
            }
            if brush.is_none() {
                let mut trash = base.clone();
                trash.trash_star = Some(n);
                let clean = base.clone();
                let restore = s.clone();
                let fallback = fixed(base.clone())
                    .with_floating()
                    .on_drop(move |_| restore.clone());
                return Some(
                    closest([
                        closest(states).with_floating(),
                        fixed(trash).with_floating().on_drop(move |_| clean.clone()),
                    ])
                    .when_far(fallback, 50.),
                );
            }
        } else {
            let moving = if let Some(i) = brush {
                atom(n, if i == 0 { 'a' } else { 'b' })
            } else {
                let root = base.root.as_ref()?;
                let moving = find(root, n)?.clone();
                base.root = remove(root, n);
                moving
            };
            for root in placements(base.root.as_ref(), &moving, base.next) {
                let mut t = base.clone();
                t.root = Some(root);
                t.next += 2;
                states.push(t);
            }
            if brush.is_none() {
                let mut trash = base.clone();
                trash.trash = Some(moving);
                let clean = base.clone();
                let restore = s.clone();
                let fallback = fixed(base.clone())
                    .with_floating()
                    .on_drop(move |_| restore.clone());
                return Some(
                    closest([
                        closest(states).with_floating(),
                        fixed(trash).with_floating().on_drop(move |_| clean.clone()),
                    ])
                    .when_far(fallback, 50.),
                );
            }
        }
        Some(
            closest(states)
                .with_floating()
                .when_far(fixed(s.clone()).with_floating(), 50.),
        )
    }
    fn status(&self, s: &State) -> String {
        s.root
            .as_ref()
            .map(|n| {
                format!(
                    "{} · drag subexpressions, palette letters, or star badges",
                    expression(n, 0)
                )
            })
            .unwrap_or_else(|| "Drag a letter into the empty expression".into())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn upstream_nfa_and_dagre_layout_fixtures() {
        fn node(v: &serde_json::Value) -> Node {
            Node {
                id: v["id"]
                    .as_str()
                    .unwrap()
                    .trim_start_matches('n')
                    .parse()
                    .unwrap(),
                kind: match v["kind"].as_str().unwrap() {
                    "char" => Kind::Char(v["char"].as_str().unwrap().chars().next().unwrap()),
                    "concat" => Kind::Concat,
                    "alt" => Kind::Alt,
                    _ => Kind::Star,
                },
                children: v["children"].as_array().unwrap().iter().map(node).collect(),
            }
        }
        let fixtures: serde_json::Value =
            serde_json::from_str(include_str!("regex/upstream-fixtures.json")).unwrap();
        for (case, f) in fixtures.as_array().unwrap().iter().enumerate() {
            let g = if f["root"].is_null() {
                Nfa::empty()
            } else {
                thompson(&node(&f["root"]))
            };
            for (name, value) in [("w", g.w), ("above", g.above), ("below", g.below)] {
                assert_eq!(
                    value,
                    f["nfa"][name].as_f64().unwrap(),
                    "case {case} {name}"
                );
            }
            for (id, p) in &g.nodes {
                let expected = f["nfa"]["states"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|s| s["id"] == nfa_name(*id))
                    .unwrap();
                assert_eq!(
                    (p.x, p.y),
                    (
                        expected["x"].as_f64().unwrap(),
                        expected["y"].as_f64().unwrap()
                    ),
                    "case {case}"
                );
            }
            let (states, edges) = subsets(&g);
            let positions = dfa_positions(&states, &edges);
            assert_eq!(
                states.len(),
                f["dfa"]["states"].as_array().unwrap().len(),
                "case {case}"
            );
            for (set, p) in states.iter().zip(positions) {
                let id = dfa_name(set);
                let expected = f["dfa"]["states"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|s| s["id"] == id)
                    .unwrap();
                assert!(
                    (p.x - expected["x"].as_f64().unwrap()).abs() < 1e-8
                        && (p.y - expected["y"].as_f64().unwrap()).abs() < 1e-8,
                    "case {case} {id}: {p:?} vs {expected}"
                );
            }
        }
    }
    #[test]
    fn palette_fallback_follows_pointer_and_cancels_insertion() {
        let mut app = demo(RegexDemo);
        app.set_animated(false);
        let original = app.status();
        let anchor = Point::new(13., 13.);
        let start = app.scene().object("re-10").unwrap().transform * anchor;
        let far = Point::new(780., 780.);
        app.press(start, Modifiers::default(), 1);
        app.move_to(far, Modifiers::default());
        app.advance(std::time::Duration::ZERO);
        let floating = app
            .scene()
            .object("re-10")
            .expect("palette brush remains visible");
        assert!((floating.transform * anchor).distance(far) < 1e-6);
        app.release(far, Modifiers::default());
        assert_eq!(app.status(), original);
    }
    #[test]
    fn floating_star_badge_stays_with_its_subtree() {
        let mut app = demo(RegexDemo);
        app.set_animated(false);
        let origin = app.scene().object("re-1").unwrap().transform * Point::ORIGIN;
        let badge = app.scene().object("star-1").unwrap().transform * Point::ORIGIN;
        app.press(origin + Vec2::new(1., 1.), Modifiers::default(), 1);
        app.move_to(Point::new(780., 780.), Modifiers::default());
        app.advance(std::time::Duration::ZERO);
        let moved = app.scene().object("re-1").unwrap().transform * Point::ORIGIN;
        let moved_badge = app.scene().object("star-1").unwrap().transform * Point::ORIGIN;
        assert!((moved_badge - moved - (badge - origin)).hypot() < 1e-6);
        assert!(moved.distance(origin) > 500.);
        let state = RegexDemo.initial();
        assert!(
            RegexDemo
                .drag(&state, "re-0", Modifiers::default())
                .is_none()
        );
    }
    #[test]
    fn thompson_and_subset_recognize_suffix() {
        let s = RegexDemo.initial();
        let g = thompson(s.root.as_ref().unwrap());
        let (ds, de) = subsets(&g);
        for n in 0..8 {
            for b in 0..1 << n {
                let w = (0..n)
                    .map(|i| if b & (1 << i) == 0 { 'a' } else { 'b' })
                    .collect::<String>();
                assert_eq!(accepts(&g, &w), w.ends_with("ab"));
                let mut state = 0;
                for c in w.chars() {
                    state = de
                        .iter()
                        .find(|(a, _, ch)| *a == state && *ch == c)
                        .unwrap()
                        .1;
                }
                assert_eq!(ds[state].contains(&g.end), accepts(&g, &w));
            }
        }
    }
    #[test]
    fn deletion_normalizes_and_placement_preserves_ids() {
        let s = RegexDemo.initial();
        let root = s.root.unwrap();
        let without = remove(&root, 3).unwrap();
        assert_eq!(expression(&without, 0), "b*ab");
        for option in placements(Some(&without), &atom(3, 'a'), 20) {
            let mut ids = vec![];
            nodes(&option, &mut ids);
            assert_eq!(ids.iter().collect::<BTreeSet<_>>().len(), ids.len());
            assert!(find(&option, 3).is_some());
        }
    }
}
