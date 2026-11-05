//! Six synchronized Catalan representations. Rotations preserve node identity
//! and exchange the parent-edge identities, as in the upstream implementation.
use super::*;
use dragology::between;
use std::collections::BTreeMap;
use std::sync::OnceLock;
const EDGE_COLOR: Color = [51, 65, 85, 255];
const NODE_FILLS: [Color; 5] = [
    [253, 230, 138, 255],
    [187, 247, 208, 255],
    [191, 219, 254, 255],
    [251, 207, 232, 255],
    [221, 214, 254, 255],
];
const NODE_STROKES: [Color; 5] = [
    [202, 138, 4, 255],
    [21, 128, 61, 255],
    [29, 78, 216, 255],
    [190, 24, 93, 255],
    [109, 40, 217, 255],
];
const CLASS_COLORS: [Color; 10] = [
    [225, 29, 72, 255],
    [37, 99, 235, 255],
    [22, 163, 74, 255],
    [217, 119, 6, 255],
    [147, 51, 234, 255],
    [8, 145, 178, 255],
    [219, 39, 119, 255],
    [101, 163, 13, 255],
    [124, 58, 237, 255],
    [234, 88, 12, 255],
];
fn class_index(n: usize, i: usize, j: usize) -> usize {
    let (i, j) = (i.min(j), i.max(j));
    (0..n)
        .flat_map(|a| (a + 1..n).map(move |b| (a, b)))
        .position(|p| p == (i, j))
        .unwrap()
}
#[derive(Clone, Debug, PartialEq)]
enum Tree {
    Leaf,
    Node {
        id: usize,
        edge: usize,
        reversed: bool,
        flipped: bool,
        left: Box<Tree>,
        right: Box<Tree>,
    },
}
impl Tree {
    fn node(left: Tree, right: Tree) -> Self {
        Self::Node {
            id: 0,
            edge: 0,
            reversed: false,
            flipped: false,
            left: Box::new(left),
            right: Box::new(right),
        }
    }
    fn leaves(&self) -> usize {
        match self {
            Self::Leaf => 1,
            Self::Node { left, right, .. } => left.leaves() + right.leaves(),
        }
    }
    fn id(&self) -> Option<usize> {
        match self {
            Self::Leaf => None,
            Self::Node { id, .. } => Some(*id),
        }
    }
    fn label(&mut self, c: &mut usize) {
        if let Self::Node {
            id,
            edge,
            left,
            right,
            ..
        } = self
        {
            *id = *c;
            *edge = *c;
            *c += 1;
            left.label(c);
            right.label(c)
        }
    }
    fn steps(&self, out: &mut Vec<(bool, usize)>) {
        if let Self::Node {
            id, left, right, ..
        } = self
        {
            left.steps(out);
            out.push((true, *id));
            right.steps(out);
            out.push((false, *id));
        }
    }
    fn key(&self) -> String {
        let mut steps = vec![];
        self.steps(&mut steps);
        steps
            .iter()
            .map(|(u, _)| if *u { 'U' } else { 'D' })
            .collect()
    }
    #[cfg(test)]
    fn loday(&self, out: &mut Vec<f64>) {
        if let Self::Node { left, right, .. } = self {
            left.loday(out);
            out.push((left.leaves() * right.leaves()) as f64);
            right.loday(out)
        }
    }
}
fn rotate(t: &Tree, target: usize) -> Tree {
    let Tree::Node {
        id,
        edge,
        reversed,
        flipped,
        left,
        right,
    } = t
    else {
        return t.clone();
    };
    if left.id() == Some(target) {
        let Tree::Node {
            id: y,
            edge: ye,
            reversed: yr,
            flipped: yf,
            left: a,
            right: b,
        } = left.as_ref()
        else {
            unreachable!()
        };
        return Tree::Node {
            id: *y,
            edge: *edge,
            reversed: *reversed,
            flipped: *flipped,
            left: a.clone(),
            right: Box::new(Tree::Node {
                id: *id,
                edge: *ye,
                reversed: !*yr,
                flipped: !*yf,
                left: b.clone(),
                right: right.clone(),
            }),
        };
    }
    if right.id() == Some(target) {
        let Tree::Node {
            id: y,
            edge: ye,
            reversed: yr,
            flipped: yf,
            left: b,
            right: c,
        } = right.as_ref()
        else {
            unreachable!()
        };
        return Tree::Node {
            id: *y,
            edge: *edge,
            reversed: *reversed,
            flipped: *flipped,
            left: Box::new(Tree::Node {
                id: *id,
                edge: *ye,
                reversed: !*yr,
                flipped: !*yf,
                left: left.clone(),
                right: b.clone(),
            }),
            right: c.clone(),
        };
    }
    Tree::Node {
        id: *id,
        edge: *edge,
        reversed: *reversed,
        flipped: *flipped,
        left: Box::new(rotate(left, target)),
        right: Box::new(rotate(right, target)),
    }
}
#[derive(Clone)]
struct Info {
    id: usize,
    edge: usize,
    parent: Option<usize>,
    right_child: bool,
    depth: usize,
    lo: usize,
    hi: usize,
    split: usize,
    slot: usize,
    left: Option<usize>,
    right: Option<usize>,
    reversed: bool,
    flipped: bool,
}
fn ordered_info<'a>(tree: &Tree, info: &'a BTreeMap<usize, Info>) -> Vec<&'a Info> {
    fn visit<'a>(t: &Tree, infos: &'a BTreeMap<usize, Info>, out: &mut Vec<&'a Info>) {
        if let Tree::Node {
            id, left, right, ..
        } = t
        {
            visit(left, infos, out);
            visit(right, infos, out);
            out.push(&infos[id]);
        }
    }
    let mut result = Vec::new();
    visit(tree, info, &mut result);
    result
}
fn analyze(t: &Tree) -> BTreeMap<usize, Info> {
    fn visit(
        t: &Tree,
        parent: Option<usize>,
        right_child: bool,
        depth: usize,
        leaf: &mut usize,
        slot: &mut usize,
        out: &mut BTreeMap<usize, Info>,
    ) {
        if let Tree::Node {
            id,
            edge,
            reversed,
            flipped,
            left,
            right,
        } = t
        {
            let lo = *leaf;
            visit(left, Some(*id), false, depth + 1, leaf, slot, out);
            let split = *leaf;
            let io = *slot;
            *slot += 1;
            visit(right, Some(*id), true, depth + 1, leaf, slot, out);
            out.insert(
                *id,
                Info {
                    id: *id,
                    edge: *edge,
                    parent,
                    right_child,
                    depth,
                    lo,
                    hi: *leaf - 1,
                    split,
                    slot: io,
                    left: left.id(),
                    right: right.id(),
                    reversed: *reversed,
                    flipped: *flipped,
                },
            );
        } else {
            *leaf += 1
        }
    }
    let mut result = BTreeMap::new();
    visit(t, None, false, 0, &mut 0, &mut 0, &mut result);
    result
}
fn trees(n: usize) -> Vec<Tree> {
    if n == 0 {
        return vec![Tree::Leaf];
    }
    let mut out = vec![];
    for k in 0..n {
        for l in trees(k) {
            for r in trees(n - 1 - k) {
                let mut t = Tree::node(l.clone(), r);
                t.label(&mut 0);
                out.push(t)
            }
        }
    }
    out
}
#[derive(Clone, Copy, PartialEq)]
enum Side {
    Left,
    Right,
}
#[derive(Clone, PartialEq)]
struct State {
    root: Tree,
    colors: bool,
    bands: bool,
    active: BTreeMap<usize, Side>,
}
fn initial(n: usize) -> State {
    let l = Tree::Leaf;
    let pair = || Tree::node(Tree::Leaf, Tree::Leaf);
    let mut root = match n {
        3 => Tree::node(pair(), pair()),
        5 => Tree::node(Tree::node(l.clone(), pair()), Tree::node(pair(), l)),
        _ => Tree::node(Tree::node(l.clone(), pair()), pair()),
    };
    root.label(&mut 0);
    State {
        root,
        colors: false,
        bands: false,
        active: BTreeMap::new(),
    }
}

fn reverse_edge(t: &mut Tree, id: usize) {
    if let Tree::Node {
        id: n,
        reversed,
        left,
        right,
        ..
    } = t
    {
        if *n == id {
            *reversed = !*reversed
        } else {
            reverse_edge(left, id);
            reverse_edge(right, id)
        }
    }
}
fn flip_variants(s: &State, target: usize) -> Option<(State, [State; 2])> {
    let info = analyze(&s.root);
    let child = info.get(&target)?;
    let parent = child.parent?;
    let mut from = s.clone();
    from.active.insert(
        parent,
        if child.right_child {
            Side::Right
        } else {
            Side::Left
        },
    );
    from.active.remove(&target);
    let mut to = s.clone();
    to.root = rotate(&s.root, target);
    to.active.insert(
        target,
        if child.right_child {
            Side::Left
        } else {
            Side::Right
        },
    );
    to.active.remove(&parent);
    let mut other = to.clone();
    reverse_edge(&mut other.root, parent);
    Some((from, [to, other]))
}
fn nondegenerate_between(from: State, to: State) -> DragSpec<'static, State> {
    let mut spec = between([from.clone(), to.clone()]);
    let from = Rc::new(from);
    DragSpec::custom(move |ctx, p| {
        let a = ctx.grab_in(from.as_ref())?;
        let b = ctx.grab_in(&to)?;
        if a.zip(b).is_some_and(|(a, b)| a.distance(b) < 1e-7) {
            ctx.state_frame(from.clone(), p)
        } else {
            spec.evaluate(ctx, p)
        }
    })
}
fn line_handle(id: String, a: Point, b: Point) -> Object {
    let v = b - a;
    let len = v.hypot();
    let mut path = BezPath::new();
    path.move_to((0., 0.));
    path.line_to((1., 0.));
    Object::new(
        id,
        Affine::new([v.x, v.y, -v.y, v.x, a.x, a.y]),
        Some(Hit::Path(path.clone(), 16. / len)),
        vec![Shape::Path {
            path,
            fill: None,
            stroke: Some((EDGE_COLOR, 3.5)),
        }],
    )
    .non_scaling_stroke()
    .line_cap(LineCap::Round)
}
struct Lattice {
    keys: Vec<String>,
    trees: Vec<Tree>,
    points: Vec<Point>,
    edges: Vec<(usize, usize, usize)>,
}
fn lattice(n: usize) -> &'static Lattice {
    static CACHE: OnceLock<Vec<Lattice>> = OnceLock::new();
    &CACHE.get_or_init(|| (3..=5).map(make_lattice).collect())[n - 3]
}
fn make_lattice(n: usize) -> Lattice {
    let trees = trees(n);
    let keys: Vec<_> = trees.iter().map(Tree::key).collect();
    let mut edges = vec![];
    for (i, t) in trees.iter().enumerate() {
        let info = analyze(t);
        for a in ordered_info(t, &info)
            .into_iter()
            .filter(|a| a.parent.is_some() && !a.right_child)
        {
            let parent = &info[&a.parent.unwrap()];
            let j = keys
                .iter()
                .position(|k| *k == rotate(t, a.id).key())
                .unwrap();
            edges.push((i, j, class_index(n, a.slot, parent.slot)));
        }
    }
    // getLattice is deterministic and supports exactly these three sizes.
    // These coordinates are its cached results at upstream b145bdd56530f7cf.
    // Native libm and V8 differ at strict hill-climb comparisons, so executing
    // the numeric search again can change the selected nudges by several pixels.
    // Preserve the source result, including its height - projectedY orientation.
    let layouts: serde_json::Value =
        serde_json::from_str(include_str!("catalan_layouts.json")).expect("pinned source layouts");
    let layout = layouts
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["n"].as_u64() == Some(n as u64))
        .unwrap();
    let points = keys
        .iter()
        .map(|key| {
            let v = layout["vertices"]
                .as_array()
                .unwrap()
                .iter()
                .find(|v| v["key"].as_str() == Some(key))
                .unwrap();
            Point::new(v["x"].as_f64().unwrap(), v["y"].as_f64().unwrap())
        })
        .collect();
    Lattice {
        keys,
        trees,
        points,
        edges,
    }
}
fn vertex(k: usize, m: usize) -> Point {
    let a = std::f64::consts::TAU * k as f64 / m as f64
        + std::f64::consts::FRAC_PI_2
        + std::f64::consts::PI / m as f64;
    Point::new(125. + 95. * a.cos(), 150. + 95. * a.sin())
}
fn handle_path(id: String, p: Point, mut path: BezPath, color: Color, width: f64) -> Object {
    path.apply_affine(Affine::translate(-p.to_vec2()));
    Object::new(
        id,
        Affine::translate(p.to_vec2()),
        Some(Hit::Path(path.clone(), 12.)),
        vec![Shape::Path {
            path,
            fill: None,
            stroke: Some((color, width)),
        }],
    )
}
struct Catalan;
impl Example for Catalan {
    type State = State;
    fn initial(&self) -> State {
        initial(4)
    }
    fn scene(&self, s: &State) -> Scene {
        let n = s.root.leaves() - 1;
        let info = analyze(&s.root);
        let lat = lattice(n);
        let mut sc = Scene::new(700., 380. + (n * (n - 1)) as f64 * 23.);
        for (id, p, label) in [
            ("poly", Point::new(20., 30.), "TRIANGULATION"),
            ("tree", Point::new(260., 30.), "BINARY TREE"),
            ("dyck", Point::new(500., 30.), "DYCK PATH"),
            ("pair", Point::new(510., 215.), "PAIRING"),
            ("paren", Point::new(250., 246.), "PARENTHESIZATION"),
            ("lat", Point::new(20., 320.), "TAMARI LATTICE"),
        ] {
            sc.push(
                Object::text(format!("heading-{id}"), p, label, 11., [148, 163, 184, 255])
                    .text_baseline(TextBaseline::Alphabetic)
                    .font_weight(600)
                    .letter_spacing(1.),
            );
        }
        for a in ordered_info(&s.root, &info) {
            let (u, v, apex, reversed) = match s.active.get(&a.id) {
                Some(Side::Left) if a.left.is_some() => {
                    (a.lo, a.split, a.hi + 1, info[&a.left.unwrap()].reversed)
                }
                Some(Side::Right) if a.right.is_some() => {
                    (a.split, a.hi + 1, a.lo, info[&a.right.unwrap()].reversed)
                }
                _ => (a.lo, a.hi + 1, a.split, a.reversed),
            };
            let (p, q, r) = (
                vertex(if reversed { v } else { u }, n + 2),
                vertex(if reversed { u } else { v }, n + 2),
                vertex(apex, n + 2),
            );
            let mid = Point::new((p.x + q.x + r.x) / 3., (p.y + q.y + r.y) / 3.);
            let mut path = BezPath::new();
            for (i, v) in [p, q, r].iter().enumerate() {
                if i == 0 {
                    path.move_to(*v - mid.to_vec2())
                } else {
                    path.line_to(*v - mid.to_vec2())
                }
            }
            path.close_path();
            let fill = NODE_FILLS[a.id % 5];
            sc.push(Object::new(
                format!("tri-{}", a.id),
                Affine::translate(mid.to_vec2()),
                None,
                vec![Shape::Path {
                    path,
                    fill: Some(fill),
                    stroke: None,
                }],
            ));
        }
        for k in 0..n + 2 {
            sc.push(Object::line(
                format!("boundary-{k}"),
                vertex(k, n + 2),
                vertex((k + 1) % (n + 2), n + 2),
                EDGE_COLOR,
                1.5,
            ));
        }
        sc.push(Object::line(
            "root-edge",
            vertex(0, n + 2),
            vertex(n + 1, n + 2),
            EDGE_COLOR,
            4.,
        ));
        for a in ordered_info(&s.root, &info) {
            if a.parent.is_some() {
                let p = vertex(a.lo, n + 2);
                let q = vertex(a.hi + 1, n + 2);
                let (a0, b0) = if a.reversed { (q, p) } else { (p, q) };
                sc.push(line_handle(format!("diag-{}", a.edge), a0, b0));
            }
        }
        let tree_p = |a: &Info| {
            Point::new(
                270. + (a.slot as f64 + 0.5) * 200. / n as f64,
                60. + a.depth as f64 * 30.,
            )
        };
        for a in ordered_info(&s.root, &info) {
            let p = tree_p(a);
            if let Some(parent) = a.parent {
                let q = tree_p(&info[&parent]);
                let mut path = BezPath::new();
                if a.flipped {
                    path.move_to(q);
                    path.line_to(p)
                } else {
                    path.move_to(p);
                    path.line_to(q)
                }
                sc.push(
                    handle_path(
                        format!("tree-edge-{}", a.edge),
                        p.lerp(q, 0.5),
                        path,
                        EDGE_COLOR,
                        3.,
                    )
                    .decorative()
                    .line_cap(LineCap::Round),
                );
            }
        }
        for a in ordered_info(&s.root, &info) {
            for (child, leaf) in [(a.left, a.lo), (a.right, a.split)] {
                if child.is_none() {
                    sc.push(Object::line(
                        format!("leaf-edge-{leaf}"),
                        tree_p(a),
                        Point::new(270. + leaf as f64 * 200. / n as f64, 60. + n as f64 * 30.),
                        [148, 163, 184, 255],
                        1.5,
                    ));
                }
            }
        }
        for i in 0..=n {
            sc.push(
                Object::text(
                    format!("tree-leaf-{i}"),
                    Point::new(270. + i as f64 * 200. / n as f64, 60. + n as f64 * 30.),
                    ((b'a' + i as u8) as char).to_string(),
                    14.,
                    [71, 85, 105, 255],
                )
                .text_anchor(TextAnchor::Middle)
                .text_baseline(TextBaseline::Middle)
                .italic(),
            );
            let p = vertex(i, n + 2).lerp(vertex(i + 1, n + 2), 0.5);
            let d = p - Point::new(125., 150.);
            sc.push(
                Object::text(
                    format!("side-letter-{i}"),
                    p + d / d.hypot() * 16.,
                    ((b'a' + i as u8) as char).to_string(),
                    13.,
                    [71, 85, 105, 255],
                )
                .text_anchor(TextAnchor::Middle)
                .text_baseline(TextBaseline::Middle)
                .italic(),
            );
        }
        for a in ordered_info(&s.root, &info) {
            let p = tree_p(a);
            let node = Object::new(
                format!("tree-{}", a.id),
                Affine::translate(p.to_vec2()),
                a.parent.map(|_| Hit::Circle(Point::ORIGIN, 9.)),
                vec![Shape::Circle {
                    center: Point::ORIGIN,
                    radius: 9.,
                    fill: NODE_FILLS[a.id % 5],
                    stroke: Some((NODE_STROKES[a.id % 5], 2.)),
                }],
            );
            sc.push(node);
        }
        let mut steps = vec![];
        s.root.steps(&mut steps);
        let dx = 170. / (2 * n) as f64;
        for i in 0..=2 * n {
            let x = 510. + i as f64 * dx;
            sc.push(Object::line(
                format!("dyck-v-{i}"),
                (x, 180. - n as f64 * dx),
                (x, 180.),
                [226, 232, 240, 255],
                1.,
            ));
        }
        for i in 0..=n {
            let y = 180. - i as f64 * dx;
            sc.push(Object::line(
                format!("dyck-h-{i}"),
                (510., y),
                (680., y),
                [226, 232, 240, 255],
                1.,
            ));
        }
        sc.push(Object::line(
            "dyck-baseline",
            (510., 180.),
            (680., 180.),
            EDGE_COLOR,
            2.,
        ));
        let mut y = 0.;
        let mut ends: BTreeMap<usize, (usize, f64, usize)> = BTreeMap::new();
        for (i, (up, id)) in steps.iter().enumerate() {
            if *up {
                ends.insert(*id, (i, y, i));
                y += 1.
            } else {
                ends.get_mut(id).unwrap().2 = i;
                y -= 1.;
            }
        }
        for (&id, &(start, y, end)) in &ends {
            let a = Point::new(510. + start as f64 * dx, 180. - y * dx);
            let b = Point::new(a.x + dx, a.y - dx);
            let c = Point::new(510. + end as f64 * dx, 180. - (y + 1.) * dx);
            let d = Point::new(c.x + dx, c.y + dx);
            let center = a;
            let height = y * dx;
            let width = d.x - a.x;
            let points = if s.bands {
                [
                    Point::ORIGIN,
                    Point::new(width, 0.),
                    Point::new(width - dx, -dx),
                    Point::new(dx, -dx),
                ]
            } else {
                [
                    Point::ORIGIN,
                    Point::new(dx, -dx),
                    Point::new(height + 2. * dx, height),
                    Point::new(height, height),
                ]
            };
            let mut path = BezPath::new();
            for (i, p) in points.into_iter().enumerate() {
                if i == 0 {
                    path.move_to(p)
                } else {
                    path.line_to(p)
                }
            }
            path.close_path();
            let mut stroke = BezPath::new();
            stroke.move_to(a - center.to_vec2());
            stroke.line_to(b - center.to_vec2());
            stroke.move_to(c - center.to_vec2());
            stroke.line_to(d - center.to_vec2());
            let fill = NODE_FILLS[id % 5];
            sc.push(
                Object::new(
                    format!("hump-{id}"),
                    Affine::translate(center.to_vec2()),
                    Some(Hit::Path(path.clone(), 6.)),
                    vec![
                        Shape::Path {
                            path,
                            fill: Some(fill),
                            stroke: None,
                        },
                        Shape::Path {
                            path: stroke,
                            fill: None,
                            stroke: Some(([71, 85, 105, 255], 2.5)),
                        },
                    ],
                )
                .line_cap(LineCap::Round),
            );
        }
        let mut arcs: Vec<_> = ends.iter().collect();
        arcs.sort_by(|a, b| a.1.1.total_cmp(&b.1.1));
        for (&id, &(start, _, end)) in arcs {
            let radius = (end - start) as f64 * dx / 2.;
            let center = Point::new(520. + (start as f64 + 0.5) * dx + radius, 305.);
            let mut path = BezPath::from_svg("M -1 0 A 1 1 0 0 1 1 0 Z").unwrap();
            path.close_path();
            sc.push(
                Object::new(
                    format!("arc-{id}"),
                    Affine::translate(center.to_vec2()) * Affine::scale(radius),
                    Some(Hit::Path(path.clone(), 0.)),
                    vec![Shape::Path {
                        path,
                        fill: Some(NODE_FILLS[id % 5]),
                        stroke: Some((NODE_STROKES[id % 5], 2.)),
                    }],
                )
                .non_scaling_stroke(),
            );
        }
        for i in 0..2 * n {
            sc.push(
                Object::circle(
                    format!("pair-point-{i}"),
                    (520. + (i as f64 + 0.5) * dx, 305.),
                    2.5,
                    EDGE_COLOR,
                )
                .decorative(),
            );
        }
        // Parenthesis objects use the edge identities rather than node identities.
        fn tokens(t: &Tree, root: bool, leaf: &mut usize, out: &mut Vec<(String, Option<usize>)>) {
            match t {
                Tree::Leaf => {
                    out.push((((b'a' + *leaf as u8) as char).to_string(), None));
                    *leaf += 1;
                }
                Tree::Node {
                    edge, left, right, ..
                } => {
                    if !root {
                        out.push(("(".into(), Some(*edge)))
                    }
                    tokens(left, false, leaf, out);
                    tokens(right, false, leaf, out);
                    if !root {
                        out.push((")".into(), Some(*edge)))
                    }
                }
            }
        }
        let mut tok = vec![];
        tokens(&s.root, true, &mut 0, &mut tok);
        for (i, (label, edge)) in tok.iter().enumerate() {
            let p = Point::new(265. + i as f64 * 18., 270.);
            if let Some(edge) = edge {
                sc.push(
                    Object::rect(
                        format!(
                            "paren-{}-{edge}",
                            if label == "(" { "open" } else { "close" }
                        ),
                        Rect::from_origin_size(p - Vec2::new(9., 14.), (18., 28.)),
                        2.,
                        [0; 4],
                    )
                    .label(label, (9., 14.), 20., EDGE_COLOR)
                    .text_anchor(TextAnchor::Middle)
                    .text_baseline(TextBaseline::Middle)
                    .font_weight(600),
                );
            } else {
                sc.push(
                    Object::text(format!("letter-{i}"), p, label, 18., EDGE_COLOR)
                        .text_anchor(TextAnchor::Middle)
                        .text_baseline(TextBaseline::Middle)
                        .italic(),
                );
            }
        }
        for &(a, b, class) in &lat.edges {
            sc.push(Object::line(
                format!("lattice-edge-{a}-{b}"),
                lat.points[a],
                lat.points[b],
                if s.colors {
                    {
                        let mut c = CLASS_COLORS[class % 10];
                        c[3] = 115;
                        c
                    }
                } else {
                    [203, 213, 225, 255]
                },
                1.5,
            ));
        }
        if s.colors {
            let mut row = 0;
            for i in 0..n {
                for j in i + 1..n {
                    let p = Point::new(610., 350. + 18. * row as f64);
                    let color = CLASS_COLORS[class_index(n, i, j) % 10];
                    sc.push(Object::line(
                        format!("legend-line-{i}-{j}"),
                        p,
                        p + Vec2::new(18., 0.),
                        color,
                        3.,
                    ));
                    sc.push(
                        Object::text(
                            format!("legend-text-{i}-{j}"),
                            p + Vec2::new(24., 0.),
                            format!(
                                "{}·{}, {}·{}",
                                (b'a' + i as u8) as char,
                                (b'b' + i as u8) as char,
                                (b'a' + j as u8) as char,
                                (b'b' + j as u8) as char
                            ),
                            11.,
                            [100, 116, 139, 255],
                        )
                        .text_baseline(TextBaseline::Middle),
                    );
                    row += 1;
                }
            }
        }
        let radius = if n <= 4 { 14. } else { 11. };
        for (i, p) in lat.points.iter().enumerate() {
            let at = |k: usize| {
                let angle = std::f64::consts::TAU * k as f64 / (n + 2) as f64
                    + std::f64::consts::FRAC_PI_2
                    + std::f64::consts::PI / (n + 2) as f64;
                Point::new(radius * angle.cos(), radius * angle.sin())
            };
            let mut path = BezPath::new();
            for k in 0..n + 2 {
                if k == 0 {
                    path.move_to(at(k))
                } else {
                    path.line_to(at(k))
                }
            }
            path.close_path();
            let mut diagonals = BezPath::new();
            for a in analyze(&lat.trees[i]).values() {
                if a.parent.is_some() {
                    diagonals.move_to(at(a.lo));
                    diagonals.line_to(at(a.hi + 1));
                }
            }
            sc.push(Object::new(
                format!("vertex-{i}"),
                Affine::translate(p.to_vec2()),
                None,
                vec![
                    Shape::Path {
                        path,
                        fill: Some([255; 4]),
                        stroke: Some(([148, 163, 184, 255], 1.)),
                    },
                    Shape::Path {
                        path: {
                            let mut p = BezPath::new();
                            p.move_to(at(0));
                            p.line_to(at(n + 1));
                            p
                        },
                        fill: None,
                        stroke: Some(([100, 116, 139, 255], 2.)),
                    },
                    Shape::Path {
                        path: diagonals,
                        fill: None,
                        stroke: Some(([100, 116, 139, 255], 1.2)),
                    },
                ],
            ));
        }
        if let Some(i) = lat.keys.iter().position(|k| *k == s.root.key()) {
            sc.push(Object::new(
                "walk",
                Affine::translate(lat.points[i].to_vec2()),
                Some(Hit::Circle(Point::ORIGIN, radius + 5.)),
                vec![Shape::Circle {
                    center: Point::ORIGIN,
                    radius: radius + 5.,
                    fill: [51, 65, 85, 16],
                    stroke: Some(([15, 23, 42, 255], 2.5)),
                }],
            ));
        }

        sc
    }
    fn scene_dragging(&self, s: &State, id: Option<&str>) -> Scene {
        let mut scene = self.scene(s);
        if let Some(id) = id.filter(|id| id.starts_with("tree-")) {
            scene.objects.sort_by_key(|o| o.id == id);
        }
        scene
    }
    fn drag(&self, s: &State, id: &str, _: Modifiers) -> Option<DragSpec<'static, State>> {
        let infos = analyze(&s.root);
        let mut targets = vec![];
        if id == "walk" {
            targets.extend(infos.values().filter(|a| a.parent.is_some()).map(|a| a.id));
        } else if let Some(rest) = id
            .strip_prefix("diag-")
            .or_else(|| id.strip_prefix("tree-edge-"))
            .or_else(|| id.strip_prefix("paren-open-"))
            .or_else(|| id.strip_prefix("paren-close-"))
        {
            let edge: usize = rest.parse().ok()?;
            targets.push(infos.values().find(|a| a.edge == edge)?.id);
        } else {
            let (prefix, rest) = id.rsplit_once('-')?;
            let nid = rest.parse().ok()?;
            let a = infos.get(&nid)?;
            if prefix == "hump" || prefix == "arc" {
                if a.right_child {
                    targets.push(nid)
                }
                if let Some(left) = a.left {
                    targets.push(left)
                }
            } else {
                if a.parent.is_some() {
                    targets.push(nid)
                }
            }
        }
        let mut specs = vec![];
        for target in targets {
            let Some((from, variants)) = flip_variants(s, target) else {
                continue;
            };
            let count = if id.starts_with("diag-") { 2 } else { 1 };
            for to in variants.into_iter().take(count) {
                let Some(a) = self.locate(&from, id) else {
                    continue;
                };
                let Some(b) = self.locate(&to, id) else {
                    continue;
                };
                if a != b {
                    specs.push(nondegenerate_between(from.clone(), to));
                }
            }
        }
        if specs.is_empty() {
            None
        } else {
            let spec = dragology::closest_with(
                specs,
                dragology::ClosestOptions {
                    lock_past: if id == "walk" { Some(20.) } else { None },
                    ..Default::default()
                },
            );
            Some(if id.starts_with("diag-") || id.starts_with("paren-") {
                spec
            } else {
                spec.with_snap_radius_options(
                    1.,
                    dragology::SnapOptions {
                        chain: true,
                        ..Default::default()
                    },
                )
            })
        }
    }
    fn controls(&self, s: &State) -> Vec<Control> {
        vec![
            Control::Choice {
                id: "n",
                label: "n".into(),
                options: vec!["3".into(), "4".into(), "5".into()],
                selected: s.root.leaves() - 4,
            },
            Control::toggle("colors", "Color lattice edges", s.colors),
            Control::Choice {
                id: "tiles",
                label: "Dyck tiles".into(),
                options: vec!["diagonal strips".into(), "horizontal bands".into()],
                selected: usize::from(s.bands),
            },
        ]
    }
    fn action(&self, s: &State, id: &str, v: Value) -> Option<State> {
        let mut t = s.clone();
        match (id, v) {
            ("n", Value::Index(n)) => {
                t = initial(n.min(2) + 3);
                t.colors = s.colors;
                t.bands = s.bands
            }
            ("colors", Value::Bool(b)) => t.colors = b,
            ("tiles", Value::Index(i)) => t.bands = i == 1,
            _ => return None,
        }
        Some(t)
    }
    fn status(&self, s: &State) -> String {
        format!(
            "{} internal nodes · {} triangulations · {}",
            s.root.leaves() - 1,
            lattice(s.root.leaves() - 1).keys.len(),
            s.root.key()
        )
    }
}
pub fn entries() -> Vec<Entry> {
    vec![Entry::new(
        "catalan-explorer",
        "Catalan explorer",
        "Six views of one tree: rotate, flip diagonals, slide whole Dyck excursions and walk the Tamari lattice.",
        || demo(Catalan),
    )]
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rotations_preserve_node_and_edge_identities_and_catalan_count() {
        for (n, count) in [(3, 5), (4, 14), (5, 42)] {
            let all = trees(n);
            assert_eq!(all.len(), count);
            for tree in all {
                let before = analyze(&tree);
                let mut coords = vec![];
                tree.loday(&mut coords);
                assert_eq!(coords.iter().sum::<f64>(), (n * (n + 1) / 2) as f64);
                for a in before.values().filter(|a| a.parent.is_some()) {
                    let rotated = rotate(&tree, a.id);
                    let after = analyze(&rotated);
                    assert_eq!(
                        before.keys().collect::<Vec<_>>(),
                        after.keys().collect::<Vec<_>>()
                    );
                    let mut edges: Vec<_> = after.values().map(|a| a.edge).collect();
                    edges.sort();
                    assert_eq!(edges, (0..n).collect::<Vec<_>>());
                    assert_eq!(after[&a.parent.unwrap()].edge, a.edge);
                    assert_eq!(rotate(&rotated, a.parent.unwrap()).key(), tree.key());
                }
            }
        }
    }
    #[test]
    fn diagonal_grabs_follow_their_fraction_and_never_fail_at_symmetric_midpoints() {
        for root in trees(4) {
            let state = State {
                root,
                colors: false,
                bands: false,
                active: BTreeMap::new(),
            };
            for info in analyze(&state.root).values().filter(|i| i.parent.is_some()) {
                let id = format!("diag-{}", info.edge);
                let transform = Catalan.locate(&state, &id).unwrap();
                for t in [0., 0.25, 0.5, 0.75, 1.] {
                    let start = transform * Point::new(t, 0.);
                    let spec = Catalan.drag(&state, &id, Modifiers::default()).unwrap();
                    let element = id.clone();
                    let mut session = dragology::DragSession::begin(
                        state.clone(),
                        start,
                        move |s| Catalan.locate(s, &element),
                        spec,
                    )
                    .unwrap();
                    assert!(session.frame(start + Vec2::new(15., -20.)).is_ok());
                }
            }
        }
    }
    #[test]
    fn lattice_edges_have_the_source_loday_direction_classes() {
        for n in 3..=5 {
            let lat = lattice(n);
            assert_eq!(lat.keys.len(), [5, 14, 42][n - 3]);
            for &(a, b, class) in &lat.edges {
                let mut ca = vec![];
                let mut cb = vec![];
                lat.trees[a].loday(&mut ca);
                lat.trees[b].loday(&mut cb);
                let changed: Vec<_> = (0..n).filter(|&i| ca[i] != cb[i]).collect();
                assert_eq!(changed.len(), 2);
                assert_eq!(ca.iter().sum::<f64>(), cb.iter().sum::<f64>());
                assert_eq!(class, class_index(n, changed[0], changed[1]));
            }
        }
    }
    #[test]
    fn dyck_words_are_balanced_and_rotations_move_whole_excursions() {
        for tree in trees(5) {
            let mut depth = 0;
            for c in tree.key().chars() {
                depth += if c == 'U' { 1 } else { -1 };
                assert!(depth >= 0)
            }
            assert_eq!(depth, 0);
        }
    }
}
