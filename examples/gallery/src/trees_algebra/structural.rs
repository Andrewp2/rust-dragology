use super::tree::*;
use crate::*;
use dragology::{BetweenOptions, Interpolation, between, between_with, closest, drop_target, vary};
use std::collections::BTreeMap;

pub struct Outline;
struct OutlineModel;
#[derive(Clone, PartialEq)]
pub struct OutlineState {
    tree: Tree,
    variant: usize,
    floating: bool,
}
fn life() -> Tree {
    fn branch(id: &str, label: &str, children: Vec<Tree>) -> Tree {
        Tree::node(id, label, children)
    }
    fn leaves(names: &[&str]) -> Vec<Tree> {
        names
            .iter()
            .map(|n| {
                Tree::leaf(
                    n.to_lowercase(),
                    match *n {
                        "Cat" => "🐱 Cat",
                        "Lion" => "🦁 Lion",
                        "Tiger" => "🐯 Tiger",
                        "Dog" => "🐕 Dog",
                        "Fox" => "🦊 Fox",
                        "Wolf" => "🐺 Wolf",
                        "Monkey" => "🐵 Monkey",
                        "Gorilla" => "🦍 Gorilla",
                        "Orangutan" => "🦧 Orangutan",
                        "Whale" => "🐋 Whale",
                        "Dolphin" => "🐬 Dolphin",
                        "Eagle" => "🦅 Eagle",
                        "Parrot" => "🦜 Parrot",
                        "Penguin" => "🐧 Penguin",
                        "Owl" => "🦉 Owl",
                        "Turtle" => "🐢 Turtle",
                        "Lizard" => "🦎 Lizard",
                        "Crocodile" => "🐊 Crocodile",
                        "Snake" => "🐍 Snake",
                        "Butterfly" => "🦋 Butterfly",
                        "Bee" => "🐝 Bee",
                        "Ant" => "🐜 Ant",
                        "Ladybug" => "🐞 Ladybug",
                        "Spider" => "🕷️ Spider",
                        "Scorpion" => "🦂 Scorpion",
                        "Octopus" => "🐙 Octopus",
                        "Squid" => "🦑 Squid",
                        "Snail" => "🐌 Snail",
                        _ => *n,
                    },
                )
            })
            .collect()
    }
    branch(
        "root",
        "root",
        vec![branch(
            "animalia",
            "Animalia",
            vec![
                branch(
                    "chordata",
                    "Chordata",
                    vec![
                        branch(
                            "mammalia",
                            "Mammalia",
                            vec![
                                branch(
                                    "carnivora",
                                    "Carnivora",
                                    vec![
                                        branch(
                                            "felidae",
                                            "Felidae",
                                            leaves(&["Cat", "Lion", "Tiger"]),
                                        ),
                                        branch(
                                            "canidae",
                                            "Canidae",
                                            leaves(&["Dog", "Fox", "Wolf"]),
                                        ),
                                    ],
                                ),
                                branch(
                                    "primates",
                                    "Primates",
                                    leaves(&["Monkey", "Gorilla", "Orangutan"]),
                                ),
                                branch("cetacea", "Cetacea", leaves(&["Whale", "Dolphin"])),
                            ],
                        ),
                        branch(
                            "aves",
                            "Aves",
                            leaves(&["Eagle", "Parrot", "Penguin", "Owl"]),
                        ),
                        branch(
                            "reptilia",
                            "Reptilia",
                            leaves(&["Turtle", "Lizard", "Crocodile", "Snake"]),
                        ),
                    ],
                ),
                branch(
                    "arthropoda",
                    "Arthropoda",
                    vec![
                        branch(
                            "insecta",
                            "Insecta",
                            leaves(&["Butterfly", "Bee", "Ant", "Ladybug"]),
                        ),
                        branch("arachnida", "Arachnida", leaves(&["Spider", "Scorpion"])),
                    ],
                ),
                branch(
                    "mollusca",
                    "Mollusca",
                    leaves(&["Octopus", "Squid", "Snail"]),
                ),
            ],
        )],
    )
}
fn outline_initial(v: usize) -> Tree {
    match v {
        0 => Tree::node(
            "root",
            "+",
            vec![Tree::leaf("A", "A"), Tree::leaf("B", "B")],
        ),
        1 => Tree::node(
            "root",
            "root",
            vec![Tree::node(
                "p1",
                "+",
                vec![
                    Tree::node("p2", "+", vec![Tree::leaf("A", "A"), Tree::leaf("B", "B")]),
                    Tree::node("p3", "+", vec![Tree::leaf("C", "C"), Tree::leaf("D", "D")]),
                ],
            )],
        ),
        _ => life(),
    }
}
fn outline_scene(t: &Tree, variant: usize) -> Scene {
    fn draw(scene: &mut Scene, t: &Tree, at: Point, parent: Option<&str>, root: bool) -> f64 {
        let mut object = Object::new(&t.id, Affine::translate(at.to_vec2()), None, vec![]);
        object.parent = parent.map(str::to_owned);
        if !root {
            object.hit = Some(Hit::Rect(Rect::new(0., 0., 100., 25.)));
            object.shapes.push(Shape::Rect {
                bounds: Rect::new(0., 0., 100., 25.),
                radius: 4.,
                fill: [240, 241, 243, 255],
                stroke: None,
            });
            object = object
                .label(&t.label, (8., 12.5), 13., [55, 65, 81, 255])
                .text_baseline(TextBaseline::Middle);
        }
        scene.push(object);
        let indent = if root { 0. } else { 20. };
        let mut y = if root { 0. } else { 25. };
        let mut child_rows = vec![];
        for c in &t.children {
            y += 2.;
            child_rows.push(y);
            y += draw(scene, c, at + Vec2::new(indent, y), Some(&t.id), false);
        }
        if !root && !child_rows.is_empty() {
            let mut line = Object::line(
                format!("connectors-{}", t.id),
                at + Vec2::new(10., 25.),
                at + Vec2::new(10., child_rows.last().unwrap() + 12.5),
                [208, 212, 216, 255],
                1.,
            );
            line.parent = Some(t.id.clone());
            scene.push(line.line_cap(LineCap::Round));
            for (c, cy) in t.children.iter().zip(child_rows) {
                let mut line = Object::line(
                    format!("connector-{}-{}", t.id, c.id),
                    at + Vec2::new(10., cy + 12.5),
                    at + Vec2::new(18., cy + 12.5),
                    [208, 212, 216, 255],
                    1.,
                );
                line.parent = Some(t.id.clone());
                scene.push(line);
            }
        }
        y
    }
    let (w, h) = [(200., 100.), (250., 200.), (350., 1200.)][variant];
    let mut scene = Scene::new(w, h);
    draw(&mut scene, t, Point::new(10., 10.), None, true);
    scene
}
impl Example for OutlineModel {
    type State = OutlineState;
    fn initial(&self) -> Self::State {
        OutlineState {
            tree: outline_initial(1),
            variant: 1,
            floating: true,
        }
    }
    fn scene(&self, s: &Self::State) -> Scene {
        outline_scene(&s.tree, s.variant)
    }
    fn drag(
        &self,
        s: &Self::State,
        id: &str,
        _: Modifiers,
    ) -> Option<DragSpec<'static, Self::State>> {
        let (without, item) = s.tree.remove(id)?;
        let targets = without
            .insertion_targets(&item)
            .into_iter()
            .map(|tree| OutlineState { tree, ..s.clone() })
            .collect::<Vec<_>>();
        Some(if s.floating {
            closest(targets)
                .when_far(
                    OutlineState {
                        tree: without,
                        ..s.clone()
                    },
                    50.,
                )
                .with_floating()
        } else {
            between(targets)
        })
    }
    fn controls(&self, s: &Self::State) -> Vec<Control> {
        vec![
            Control::Choice {
                id: "variant",
                label: "Outline".into(),
                options: vec!["Simple".into(), "Nested".into(), "Tree of life".into()],
                selected: s.variant,
            },
            Control::toggle("floating", "Use floating", s.floating),
        ]
    }
    fn action(&self, s: &Self::State, id: &str, v: Value) -> Option<Self::State> {
        let mut n = s.clone();
        match (id, v) {
            ("variant", Value::Index(i)) if i < 3 => {
                n.variant = i;
                n.tree = outline_initial(i)
            }
            ("floating", Value::Bool(v)) => n.floating = v,
            _ => return None,
        };
        Some(n)
    }
}

impl Example for Outline {
    type State = Vec<OutlineState>;
    fn initial(&self) -> Self::State {
        (0..3)
            .map(|variant| OutlineState {
                tree: outline_initial(variant),
                variant,
                floating: true,
            })
            .collect()
    }
    fn scene(&self, s: &Self::State) -> Scene {
        self.scene_dragging(s, None)
    }
    fn scene_dragging(&self, s: &Self::State, id: Option<&str>) -> Scene {
        let mut scene =
            super::panels::compose(s, &["simple", "nested", "tree of life"], id, |s, id| {
                let mut sc = OutlineModel.scene(s);
                raise_group(&mut sc, id);
                sc
            });
        scene.push(
            Object::rect(
                "outline-options-panel",
                Rect::new(366., 0., 556., 74.),
                4.,
                [249, 250, 251, 255],
            )
            .decorative(),
        );
        scene.push(
            Object::text(
                "outline-options-heading",
                (378., 12.),
                "Options",
                12.,
                [55, 65, 81, 255],
            )
            .font_weight(500),
        );
        scene
            .inline_control("floating", Rect::new(378., 36., 544., 62.))
            .font_size(12.);
        scene.width = 556.;
        scene
    }
    fn drag(
        &self,
        s: &Self::State,
        id: &str,
        m: Modifiers,
    ) -> Option<DragSpec<'static, Self::State>> {
        let (i, id) = super::panels::resolve_id(id)?;
        let spec = OutlineModel.drag(s.get(i)?, id, m)?;
        let offset = super::panels::offset(s, i, |s| OutlineModel.scene(s));
        let id = id.to_string();
        Some(super::panels::lift(s.clone(), i, offset, spec, move |s| {
            OutlineModel.locate(s, &id)
        }))
    }
    fn controls(&self, s: &Self::State) -> Vec<Control> {
        vec![Control::toggle("floating", "Use floating", s[0].floating)]
    }
    fn action(&self, s: &Self::State, id: &str, v: Value) -> Option<Self::State> {
        s.iter()
            .map(|s| OutlineModel.action(s, id, v.clone()))
            .collect()
    }
}

#[derive(Clone, PartialEq)]
struct BoxNode {
    id: String,
    inside: Vec<BoxNode>,
    below: Vec<BoxNode>,
    position: Point,
    color: Color,
}
impl BoxNode {
    fn new(id: &str, c: usize) -> Self {
        Self {
            id: id.into(),
            inside: vec![],
            below: vec![],
            position: Point::new(100., 20.),
            color: [
                [201, 228, 240, 255],
                [245, 213, 216, 255],
                [212, 237, 207, 255],
                [245, 234, 201, 255],
                [228, 212, 232, 255],
                [245, 220, 201, 255],
                [201, 240, 237, 255],
                [232, 212, 240, 255],
            ][c % 8],
        }
    }
    fn nodes(&self) -> Vec<&Self> {
        let mut n = vec![self];
        for c in self.inside.iter().chain(&self.below) {
            n.extend(c.nodes())
        }
        n
    }
    fn remove(&mut self, id: &str) -> Option<Self> {
        for list in [&mut self.inside, &mut self.below] {
            if let Some(i) = list.iter().position(|c| c.id == id) {
                return Some(list.remove(i));
            }
            for c in list {
                if let Some(found) = c.remove(id) {
                    return Some(found);
                }
            }
        }
        None
    }
    fn find_mut(&mut self, id: &str) -> Option<&mut Self> {
        if self.id == id {
            return Some(self);
        }
        self.inside
            .iter_mut()
            .chain(&mut self.below)
            .find_map(|c| c.find_mut(id))
    }
}
pub struct TwoKinds;
#[derive(Clone, PartialEq)]
pub struct Boxes {
    roots: Vec<BoxNode>,
}
fn box_size(b: &BoxNode) -> (Vec2, Vec2) {
    let sizes: Vec<_> = b.inside.iter().map(box_size).collect();
    let content = if sizes.is_empty() {
        Vec2::new(50., 50.)
    } else {
        Vec2::new(
            34. + sizes.iter().map(|(s, _)| s.x).sum::<f64>()
                + 8. * sizes.len().saturating_sub(1) as f64,
            32. + sizes.iter().map(|(s, _)| s.y).fold(50., f64::max),
        )
    };
    let below: Vec<_> = b.below.iter().map(box_size).collect();
    let all = Vec2::new(
        content.x.max(
            below.iter().map(|(s, _)| s.x).sum::<f64>() + 8. * below.len().saturating_sub(1) as f64,
        ),
        content.y
            + if below.is_empty() {
                0.
            } else {
                24. + below.iter().map(|(s, _)| s.y).fold(0., f64::max)
            },
    );
    (all, content)
}
fn draw_box(scene: &mut Scene, b: &BoxNode, at: Point, parent: Option<&str>) {
    let (all, content) = box_size(b);
    let x = at.x + (all.x - content.x) / 2.;
    let dx = x - at.x;
    let bounds = Rect::new(dx, 0., dx + content.x, content.y);
    let mut o = Object::new(
        &b.id,
        Affine::translate(at.to_vec2()),
        Some(Hit::Rect(bounds)),
        vec![Shape::Rect {
            bounds,
            radius: 6.,
            fill: b.color,
            stroke: None,
        }],
    )
    .label(&b.id, (dx + 4., 13.), 11., [102, 102, 102, 255])
    .text_baseline(TextBaseline::Alphabetic)
    .font_weight(500);
    o.parent = parent.map(str::to_owned);
    if let Shape::Rect { stroke, .. } = &mut o.shapes[0] {
        *stroke = Some(([170, 170, 170, 255], 1.5));
    }
    if !b.inside.is_empty() {
        for row in 0..3 {
            for col in 0..2 {
                o.shapes.push(Shape::Circle {
                    center: Point::new(
                        dx + 8. + col as f64 * 8.,
                        content.y / 2. + (row as f64 - 1.) * 8.,
                    ),
                    radius: 1.5,
                    fill: [51, 51, 51, 90],
                    stroke: None,
                });
            }
        }
    }
    scene.push(o);
    let mut dx = x + 26.;
    for c in &b.inside {
        draw_box(scene, c, Point::new(dx, at.y + 24.), Some(&b.id));
        dx += box_size(c).0.x + 8.;
    }
    let sum = b.below.iter().map(|c| box_size(c).0.x + 8.).sum::<f64>() - 8.;
    let mut dx = at.x + (all.x - sum) / 2.;
    for c in &b.below {
        let end = Point::new(dx + box_size(c).0.x / 2., at.y + content.y + 24.);
        let mut edge = Object::line(
            format!("edge-{}", c.id),
            (x + content.x / 2., at.y + content.y),
            end,
            [170, 170, 170, 255],
            1.5,
        );
        edge.parent = Some(b.id.clone());
        scene.push(edge);
        draw_box(
            scene,
            c,
            Point::new(dx, at.y + content.y + 24.),
            Some(&b.id),
        );
        dx += box_size(c).0.x + 8.;
    }
}
impl Example for TwoKinds {
    type State = Boxes;
    fn initial(&self) -> Boxes {
        let mut root = BoxNode::new("root", 0);
        root.inside = vec![BoxNode::new("A", 4), BoxNode::new("B", 5)];
        let mut left = BoxNode::new("left", 1);
        left.inside = vec![BoxNode::new("C", 6), BoxNode::new("D", 7)];
        let mut right = BoxNode::new("right", 2);
        right.inside = vec![BoxNode::new("E", 4)];
        let mut last = BoxNode::new("right-child", 3);
        last.inside = vec![BoxNode::new("F", 5), BoxNode::new("G", 6)];
        right.below.push(last);
        root.below = vec![left, right];
        Boxes { roots: vec![root] }
    }
    fn scene(&self, s: &Boxes) -> Scene {
        self.scene_dragging(s, None)
    }
    fn scene_dragging(&self, s: &Boxes, id: Option<&str>) -> Scene {
        let mut out = Scene::new(600., 400.);
        for b in &s.roots {
            draw_box(&mut out, b, b.position, None);
        }
        raise_group(&mut out, id);
        out
    }

    fn drag(&self, s: &Boxes, id: &str, _: Modifiers) -> Option<DragSpec<'static, Boxes>> {
        let mut base = s.clone();
        let item = if let Some(i) = base.roots.iter().position(|b| b.id == id) {
            base.roots.remove(i)
        } else {
            base.roots.iter_mut().find_map(|b| b.remove(id))?
        };
        let nodes: Vec<_> = base
            .roots
            .iter()
            .flat_map(BoxNode::nodes)
            .map(|n| n.id.clone())
            .collect();
        let mut targets = vec![];
        for target in nodes {
            for below in [false, true] {
                let length = base
                    .roots
                    .iter()
                    .flat_map(BoxNode::nodes)
                    .find(|n| n.id == target)
                    .map(|n| if below { n.below.len() } else { n.inside.len() })
                    .unwrap();
                for i in 0..=length {
                    let mut next = base.clone();
                    let n = next
                        .roots
                        .iter_mut()
                        .find_map(|b| b.find_mut(&target))
                        .unwrap();
                    if below {
                        n.below.insert(i, item.clone())
                    } else {
                        n.inside.insert(i, item.clone())
                    }
                    targets.push(next);
                }
            }
        }
        let mut detached = base;
        detached.roots.push(item);
        let index = detached.roots.len() - 1;
        // Source starts the detached top-level box at (0, 0) before vary fits it.
        detached.roots[index].position = Point::ORIGIN;
        let moving = vary([0., 0.], move |[x, y]| {
            let mut t = detached.clone();
            t.roots[index].position = Point::new(x, y);
            t
        });
        Some(if targets.is_empty() {
            moving
        } else {
            closest(targets).with_floating().when_far(moving, 50.)
        })
    }
}

pub struct Twisted;
struct TwistedModel;
#[derive(Clone, PartialEq)]
pub struct Morph {
    map: Vec<usize>,
    one: bool,
    traditional: bool,
    fixed: bool,
    quality: usize,
    natural: bool,
    highlighted: Option<usize>,
}
fn parent(i: usize) -> usize {
    (i - 1) / 2
}
fn ancestor(a: usize, mut b: usize) -> bool {
    loop {
        if a == b {
            return true;
        }
        if b == 0 {
            return false;
        }
        b = parent(b)
    }
}
fn distance(mut a: usize, mut b: usize) -> usize {
    let mut n = 0;
    while a != b {
        if a > b {
            a = parent(a)
        } else {
            b = parent(b)
        }
        n += 1
    }
    n
}
fn preorder(n: usize) -> Vec<usize> {
    fn visit(i: usize, n: usize, out: &mut Vec<usize>) {
        if i < n {
            out.push(i);
            visit(i * 2 + 1, n, out);
            visit(i * 2 + 2, n, out);
        }
    }
    let mut out = vec![];
    visit(0, n, &mut out);
    out
}
fn mappings(n: usize) -> Vec<Vec<usize>> {
    fn visit(order: &[usize], index: usize, m: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
        if index == order.len() {
            out.push(m.clone());
            return;
        }
        let node = order[index];
        for &target in order {
            if node == 0 || ancestor(m[parent(node)], target) {
                m[node] = target;
                visit(order, index + 1, m, out);
            }
        }
    }
    let mut out = vec![];
    visit(&preorder(n), 0, &mut vec![0; n], &mut out);
    out
}
#[derive(Default)]
struct MorphLayout {
    fg: BTreeMap<usize, Point>,
    bg: BTreeMap<usize, (Point, f64)>,
    bridge: BTreeMap<usize, Point>,
    width: f64,
    height: f64,
    root_x: f64,
}
impl MorphLayout {
    fn merge(&mut self, other: MorphLayout, delta: Vec2) {
        for (i, p) in other.fg {
            self.fg.insert(i, p + delta);
        }
        for (i, p) in other.bridge {
            self.bridge.insert(i, p + delta);
        }
        for (i, (p, r)) in other.bg {
            self.bg.insert(i, (p + delta, r));
        }
    }
}
fn domain_children(i: usize, n: usize) -> impl Iterator<Item = usize> {
    (i * 2 + 1..=i * 2 + 2).filter(move |j| *j < n)
}
fn fg_layout(s: &Morph, node: usize, bg: usize) -> (MorphLayout, Vec<usize>) {
    let mut out = MorphLayout::default();
    let mut below = vec![];
    let mut child_x = 0.;
    let mut child_h: f64 = 0.;
    let mut children = vec![];
    for (j, c) in domain_children(node, s.map.len()).enumerate() {
        if j > 0 {
            child_x += 20.;
        }
        if s.map[c] == bg {
            let (r, rest) = fg_layout(s, c, bg);
            below.extend(rest);
            child_h = child_h.max(r.height);
            let w = r.width;
            children.push((child_x, r));
            child_x += w;
        } else {
            below.push(c);
            out.bridge.insert(c, Point::new(child_x, 0.));
        }
    }
    let offset = (40. - child_x).max(0.) / 2.;
    for p in out.bridge.values_mut() {
        *p += Vec2::new(offset, 60.);
    }
    out.width = child_x.max(40.);
    out.height = 40. + if child_h > 0. { 20. + child_h } else { 0. };
    out.fg.insert(node, Point::new(out.width / 2., 20.));
    for (x, r) in children {
        out.merge(r, Vec2::new(offset + x, 60.));
    }
    (out, below)
}
fn bg_layout(s: &Morph, bg: usize, roots: Vec<usize>) -> MorphLayout {
    let (here, mut below): (Vec<_>, Vec<_>) = roots.into_iter().partition(|i| s.map[*i] == bg);
    let mut contents = MorphLayout::default();
    let mut x = 10.;
    let mut max_x: f64 = 20.;
    let mut max_y: f64 = 20.;
    for node in here {
        let (r, rest) = fg_layout(s, node, bg);
        below.extend(rest);
        max_y = max_y.max(10. + r.height);
        let w = r.width;
        contents.merge(r, Vec2::new(x, 10.));
        x += w + 20.;
        max_x = max_x.max(x - 20.);
    }
    max_x += 10.;
    max_y += 10.;
    let radius = (max_x / 2.).hypot(max_y / 2.);
    let diameter = radius * 2.;
    let mut circle = MorphLayout::default();
    circle.merge(
        contents,
        Vec2::new(radius - max_x / 2., radius - max_y / 2.),
    );
    circle.bg.insert(bg, (Point::new(radius, radius), radius));
    let children: Vec<_> = domain_children(bg, s.map.len())
        .map(|c| bg_layout(s, c, below.clone()))
        .collect();
    if children.is_empty() {
        circle.width = diameter;
        circle.height = diameter;
        circle.root_x = radius;
        return circle;
    }
    let cw = children.iter().map(|c| c.width).sum::<f64>()
        + 40. * children.len().saturating_sub(1) as f64;
    let width = diameter.max(cw);
    let co = (width - diameter) / 2.;
    let mut out = MorphLayout {
        width,
        height: diameter,
        root_x: co + radius,
        ..Default::default()
    };
    out.merge(circle, Vec2::new(co, 0.));
    let mut cx = (width - cw) / 2.;
    for c in children {
        let w = c.width;
        out.height = out.height.max(diameter + 40. + c.height);
        out.merge(c, Vec2::new(cx, diameter + 40.));
        cx += w + 40.;
    }
    out
}
fn morph_layout(s: &Morph) -> MorphLayout {
    let layout = bg_layout(s, 0, vec![0]);
    let width = if s.map.len() == 3 {
        300.
    } else if s.traditional {
        600.
    } else {
        400.
    };
    let dx = if s.fixed {
        width / 2. - layout.root_x
    } else {
        0.
    };
    let mut out = MorphLayout {
        width,
        height: layout.height,
        root_x: layout.root_x + dx,
        ..Default::default()
    };
    out.merge(layout, Vec2::new(dx, 0.));
    out
}
fn cardinal(a: Point, m: Point, b: Point) -> BezPath {
    let mut p = BezPath::new();
    p.move_to(a);
    p.curve_to(a, m - (b - a) / 6., m);
    p.curve_to(m + (b - a) / 6., b, b);
    p
}
fn towards(a: Point, b: Point, d: f64) -> Point {
    a + (b - a).normalize() * d
}
impl Example for TwistedModel {
    type State = Morph;
    fn initial(&self) -> Morph {
        Morph {
            map: vec![0; 7],
            one: false,
            traditional: false,
            fixed: false,
            quality: 0,
            natural: true,
            highlighted: None,
        }
    }
    fn scene(&self, s: &Morph) -> Scene {
        self.scene_dragging(s, None)
    }
    fn scene_dragging(&self, s: &Morph, dragged: Option<&str>) -> Scene {
        let layout = morph_layout(s);
        let small = s.map.len() == 3;
        let mut out = Scene::new(
            if small {
                300.
            } else if s.traditional {
                600.
            } else {
                400.
            },
            match (small, s.traditional) {
                (true, false) => 400.,
                (true, true) => 700.,
                (false, false) => 600.,
                (false, true) => 1100.,
            },
        );
        for i in 1..s.map.len() {
            out.push(Object::line(
                format!("bg-edge-{i}"),
                layout.bg[&parent(i)].0,
                layout.bg[&i].0,
                [211, 211, 211, 255],
                12.,
            ));
        }
        for (&i, &(p, r)) in &layout.bg {
            out.push(
                Object::circle(
                    format!("bg-{i}"),
                    p,
                    r,
                    if s.highlighted == Some(i) {
                        [147, 197, 253, 255]
                    } else {
                        [211, 211, 211, 255]
                    },
                )
                .decorative(),
            );
        }
        for i in 1..s.map.len() {
            let a = layout.fg[&parent(i)];
            let b = layout.fg[&i];
            out.push(Object::path(
                format!("edge-{i}"),
                cardinal(a, layout.bridge.get(&i).copied().unwrap_or(a), b),
                [0, 0, 0, 255],
                2.,
            ));
        }
        for (&i, &p) in &layout.fg {
            let id = format!("n{i}");
            out.push(Object::circle(
                &id,
                p,
                20.,
                if s.quality != 0 && dragged == Some(id.as_str()) {
                    [59, 130, 246, 255]
                } else {
                    [0, 0, 0, 255]
                },
            ));
        }
        if s.traditional {
            let all = Morph {
                map: vec![0; s.map.len()],
                ..s.clone()
            };
            let (tree, _) = fg_layout(&all, 0, 0);
            let y = if small { 300. } else { 500. };
            let right = Vec2::new(tree.width + 40., y);
            let left = Vec2::new(0., y);
            for (name, offset, fill, edgew) in [
                ("domain", left, [0, 0, 0, 255], 2.),
                ("codomain", right, [211, 211, 211, 255], 12.),
            ] {
                for i in 1..s.map.len() {
                    out.push(Object::line(
                        format!("{name}-edge-{i}"),
                        tree.fg[&parent(i)] + offset,
                        tree.fg[&i] + offset,
                        fill,
                        edgew,
                    ));
                }
                for (&i, &p) in &tree.fg {
                    out.push(
                        Object::circle(format!("{name}-{i}"), p + offset, 20., fill).decorative(),
                    );
                }
            }
            for i in 0..s.map.len() {
                let from = tree.fg[&i] + left;
                let to = tree.fg[&s.map[i]] + right;
                let mid = from.lerp(to, 0.5) + Vec2::new(0., -from.distance(to) / 6.);
                let start = towards(from, mid, 20.);
                let tip = towards(to, mid, 20.);
                let mut path = BezPath::new();
                path.move_to(start);
                path.quad_to(mid, towards(tip, mid, 5.));
                out.push(Object::path(
                    format!("mapping-curve-{i}"),
                    path,
                    [66, 135, 245, 255],
                    2.,
                ));
                let direction = (to - mid).normalize();
                let length = 15.;
                let normal = Vec2::new(-direction.y, direction.x);
                let base = Point::ORIGIN - direction * length;
                let width = length * (std::f64::consts::PI / 10.).tan();
                let mut triangle = BezPath::new();
                triangle.move_to(Point::ORIGIN);
                triangle.line_to(base + normal * width);
                triangle.line_to(base - normal * width);
                triangle.close_path();
                out.push(Object::new(
                    format!("mapping-{i}"),
                    Affine::translate(tip.to_vec2()),
                    Some(Hit::Path(triangle.clone(), 0.)),
                    vec![Shape::Path {
                        path: triangle,
                        fill: Some([66, 135, 245, 255]),
                        stroke: None,
                    }],
                ));
            }
        }
        for o in &mut out.objects {
            o.interpolate = s.quality != 2;
        }
        out
    }
    fn drag(&self, s: &Morph, id: &str, _: Modifiers) -> Option<DragSpec<'static, Morph>> {
        let i = id
            .strip_prefix('n')
            .or_else(|| id.strip_prefix("mapping-"))?
            .parse::<usize>()
            .ok()?;
        if i >= s.map.len() {
            return None;
        }
        let mut best: BTreeMap<usize, (usize, Vec<usize>)> = BTreeMap::new();
        for m in mappings(s.map.len()) {
            if s.one
                && m.iter()
                    .zip(&s.map)
                    .enumerate()
                    .any(|(j, (a, b))| i != j && a != b)
            {
                continue;
            }
            let cost = m.iter().zip(&s.map).map(|(&a, &b)| distance(a, b)).sum();
            if best.get(&m[i]).is_none_or(|(old, _)| cost < *old) {
                best.insert(m[i], (cost, m));
            }
        }
        if s.quality == 0 {
            let states: Vec<_> = preorder(s.map.len())
                .into_iter()
                .filter_map(|id| best.remove(&id))
                .map(|(_, map)| Morph {
                    map,
                    highlighted: None,
                    ..s.clone()
                })
                .collect();
            Some(
                between_with(
                    states,
                    BetweenOptions {
                        interpolation: if s.natural {
                            Interpolation::NaturalNeighbor
                        } else {
                            Interpolation::Delaunay
                        },
                        ..Default::default()
                    },
                )
                .with_snap_radius_options(
                    20.,
                    dragology::SnapOptions {
                        transition: Some(dragology::Transition::default()),
                        chain: false,
                    },
                ),
            )
        } else {
            let layout = morph_layout(s);
            let mut targets = vec![];
            for target in preorder(s.map.len()) {
                let Some((_, map)) = best.remove(&target) else {
                    continue;
                };
                let (p, r) = layout.bg[&target];
                let mut preview = s.clone();
                preview.highlighted = Some(target);
                let result = Morph {
                    map,
                    highlighted: None,
                    ..s.clone()
                };
                targets.push(
                    drop_target(preview, Rect::new(p.x - r, p.y - r, p.x + r, p.y + r))
                        .on_drop(move |_| result.clone()),
                );
            }
            Some(
                closest(targets)
                    .when_far(s.clone(), 50.)
                    .with_drop_transition((s.quality == 1).then_some(dragology::Transition {
                        duration: std::time::Duration::from_millis(300),
                        ..Default::default()
                    })),
            )
        }
    }

    fn controls(&self, s: &Morph) -> Vec<Control> {
        vec![
            Control::Choice {
                id: "size",
                label: "Mapping".into(),
                options: vec!["3 → 3".into(), "7 → 7".into()],
                selected: usize::from(s.map.len() == 7),
            },
            Control::toggle("one", "Only drag one node at a time", s.one),
            Control::toggle("trad", "Show traditional representation", s.traditional),
            Control::toggle("fixed", "Fix root node X position", s.fixed),
            Control::Choice {
                id: "quality",
                label: "Drag quality".into(),
                options: vec!["good".into(), "bad".into(), "worse".into()],
                selected: s.quality,
            },
            Control::Choice {
                id: "interpolation",
                label: "Interpolation".into(),
                options: vec!["natural-neighbor".into(), "delaunay".into()],
                selected: usize::from(!s.natural),
            },
        ]
    }
    fn action(&self, s: &Morph, id: &str, v: Value) -> Option<Morph> {
        let mut n = s.clone();
        match (id, v) {
            ("size", Value::Index(i)) => n.map = vec![0; if i == 0 { 3 } else { 7 }],
            ("one", Value::Bool(v)) => n.one = v,
            ("trad", Value::Bool(v)) => n.traditional = v,
            ("fixed", Value::Bool(v)) => n.fixed = v,
            ("quality", Value::Index(v)) => n.quality = v,
            ("interpolation", Value::Index(i)) => n.natural = i == 0,
            _ => return None,
        }
        Some(n)
    }
}

impl Example for Twisted {
    type State = Vec<Morph>;
    fn initial(&self) -> Self::State {
        let mut small = TwistedModel.initial();
        small.map = vec![0; 3];
        vec![small, TwistedModel.initial()]
    }
    fn scene(&self, s: &Self::State) -> Scene {
        self.scene_dragging(s, None)
    }
    fn scene_dragging(&self, s: &Self::State, id: Option<&str>) -> Scene {
        let mut scene = super::panels::compose(s, &["3→3", "7→7"], id, |s, id| {
            TwistedModel.scene_dragging(s, id)
        });
        let x = scene.width + 16.;
        scene.push(
            Object::rect(
                "twisted-options-panel",
                Rect::new(x, 0., x + 300., 224.),
                4.,
                [249, 250, 251, 255],
            )
            .decorative(),
        );
        scene.push(
            Object::text(
                "twisted-options-heading",
                (x + 12., 12.),
                "Options",
                12.,
                [55, 65, 81, 255],
            )
            .font_weight(500),
        );
        for (i, id) in ["one", "trad", "fixed", "quality", "interpolation"]
            .iter()
            .enumerate()
        {
            let y = 36. + i as f64 * 36.;
            let control = scene.inline_control(id, Rect::new(x + 12., y, x + 288., y + 28.));
            control.font_size(12.);
            if i >= 3 {
                control.select();
            }
        }
        scene.width = x + 300.;
        scene
    }
    fn drag(
        &self,
        s: &Self::State,
        id: &str,
        m: Modifiers,
    ) -> Option<DragSpec<'static, Self::State>> {
        let (i, id) = super::panels::resolve_id(id)?;
        let spec = TwistedModel.drag(s.get(i)?, id, m)?;
        let offset = super::panels::offset(s, i, |s| TwistedModel.scene(s));
        let id = id.to_string();
        Some(super::panels::lift(s.clone(), i, offset, spec, move |s| {
            TwistedModel.locate(s, &id)
        }))
    }
    fn controls(&self, s: &Self::State) -> Vec<Control> {
        TwistedModel
            .controls(&s[0])
            .into_iter()
            .filter(|c| !matches!(c, Control::Choice { id: "size", .. }))
            .collect()
    }
    fn action(&self, s: &Self::State, id: &str, v: Value) -> Option<Self::State> {
        s.iter()
            .map(|s| TwistedModel.action(s, id, v.clone()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nested_box_gestures_preserve_the_forest_and_detach_at_the_grabbed_origin() {
        let state = TwoKinds.initial();
        let original: std::collections::BTreeSet<_> = state
            .roots
            .iter()
            .flat_map(BoxNode::nodes)
            .map(|n| n.id.clone())
            .collect();
        for id in &original {
            let source = state
                .roots
                .iter()
                .flat_map(BoxNode::nodes)
                .find(|n| n.id == *id)
                .unwrap();
            let subtree: std::collections::BTreeSet<_> =
                source.nodes().iter().map(|n| n.id.clone()).collect();
            let grab = Point::new(4., 13.);
            let press = TwoKinds.locate(&state, id).unwrap() * grab;
            let key = id.clone();
            let spec = TwoKinds.drag(&state, id, Modifiers::default()).unwrap();
            let mut session = dragology::DragSession::begin(
                state.clone(),
                press,
                move |s| TwoKinds.locate(s, &key),
                spec,
            )
            .unwrap();
            for p in [
                Point::new(150., 100.),
                Point::new(300., 150.),
                Point::new(500., 300.),
                Point::new(850., 650.),
            ] {
                let frame = session.frame(p).unwrap();
                let nodes: Vec<_> = frame
                    .drop_state
                    .roots
                    .iter()
                    .flat_map(BoxNode::nodes)
                    .collect();
                let ids: std::collections::BTreeSet<_> =
                    nodes.iter().map(|n| n.id.clone()).collect();
                assert_eq!(ids, original);
                assert_eq!(ids.len(), nodes.len());
                let held = nodes.iter().find(|n| n.id == *id).unwrap();
                let held_ids: std::collections::BTreeSet<_> =
                    held.nodes().iter().map(|n| n.id.clone()).collect();
                assert_eq!(held_ids, subtree);
            }
            let drop = Point::new(850., 650.);
            let detached = session.finish(drop).unwrap();
            assert!(detached.roots.iter().any(|n| n.id == *id));
            let gap = (TwoKinds.locate(&detached, id).unwrap() * grab).distance(drop);
            assert!(gap < 0.5, "{id}: vary fit missed the pointer by {gap}");
        }
    }
    #[test]
    fn outline_panels_edit_independently_through_the_shared_drag_engine() {
        let state = Outline.initial();
        let id = "panel0/A".to_string();
        let press = Outline.locate(&state, &id).unwrap() * Point::new(8., 12.5);
        let mut expected = state.clone();
        expected[0].tree.children.swap(0, 1);
        let target = Outline.locate(&expected, &id).unwrap() * Point::new(8., 12.5);
        let spec = Outline.drag(&state, &id, Modifiers::default()).unwrap();
        let result = dragology::DragSession::begin(
            state.clone(),
            press,
            move |s| Outline.locate(s, &id),
            spec,
        )
        .unwrap()
        .finish(target)
        .unwrap();
        assert_eq!(result[0].tree.children[1].id, "A");
        assert!(
            result[1..] == state[1..],
            "another canvas changed while dragging the simple outline"
        );
        assert!(Outline.scene(&result).height > 1500.);
    }
    #[test]
    fn morphs_preserve_the_ancestor_order() {
        for n in [3, 7] {
            for m in mappings(n) {
                for i in 1..n {
                    assert!(ancestor(m[parent(i)], m[i]));
                }
            }
        }
    }
    #[test]
    fn morph_layout_places_every_node_in_its_image_circle() {
        for n in [3, 7] {
            for map in mappings(n) {
                let s = Morph {
                    map,
                    ..TwistedModel.initial()
                };
                let l = morph_layout(&s);
                assert_eq!(l.fg.len(), n);
                assert_eq!(l.bg.len(), n);
                for (i, &p) in &l.fg {
                    let (c, r) = l.bg[&s.map[*i]];
                    assert!(p.distance(c) + 20. <= r + 1e-8);
                }
            }
        }
    }
    #[test]
    fn moving_outline_subtrees_never_duplicates_nodes_or_creates_cycles() {
        let tree = outline_initial(1);
        let (without, node) = tree.remove("p2").unwrap();
        for candidate in without.insertion_targets(&node) {
            let ids: std::collections::BTreeSet<_> =
                candidate.nodes().iter().map(|n| n.id.clone()).collect();
            assert_eq!(ids.len(), tree.nodes().len());
        }
    }
}
