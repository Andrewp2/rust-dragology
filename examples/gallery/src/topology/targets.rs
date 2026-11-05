//! Direct ports of the upstream copying, target and graph demo models.
use super::*;
use dragology::{between, closest, fixed, vary};
use std::collections::BTreeMap;
const BLACK: Color = [0, 0, 0, 255];
const WHITE: Color = [255; 4];
const COPY_COLORS: [Color; 6] = [
    [59, 130, 246, 255],
    [239, 68, 68, 255],
    [34, 197, 94, 255],
    [234, 179, 8, 255],
    [168, 85, 247, 255],
    [236, 72, 153, 255],
];
pub(super) fn entries() -> Vec<Entry> {
    vec![
        Entry::new(
            "drag-to-copy",
            "Drag to copy",
            "Drag to duplicate. Double click to remove.",
            || demo(Dots { alt_only: false }),
        ),
        Entry::new(
            "alt-to-copy",
            "Alt to copy",
            "Hold Alt/Option while dragging to duplicate a dot. You can toggle Alt mid-drag.",
            || demo(Dots { alt_only: true }),
        ),
        Entry::new(
            "graph",
            "Graph",
            "Hold Alt/Option while dragging to make a new connection.",
            || demo(Graph),
        ),
        Entry::new(
            "weird-floating-vs-fixed",
            "Floating and fixed",
            "Fixed and floating candidates alternate; floating candidates leave a half-opacity ghost.",
            || demo(Alternating),
        ),
        Entry::new(
            "snap-to-islands",
            "Snap to islands",
            "Discrete islands in a continuous sea: closest.withFloating().whenFar(vary).",
            || demo(Islands),
        ),
        Entry::new(
            "snap-to-islands-in-out",
            "Island hysteresis",
            "Like snap-to-islands, but with no floating and asymmetric gapIn/gapOut (gapIn=40, gapOut=80).",
            || demo(Hysteresis),
        ),
        Entry::new(
            "magnetic-targets",
            "Magnetic targets",
            "changeGap reweights gaps going into closest, resulting in some funny drop zones.",
            || demo(Magnetic),
        ),
        Entry::new(
            "emerge-from",
            "Emerge from",
            "The second dot emerges from the main dot as the state changes.",
            || demo(Emerge),
        ),
        Entry::new(
            "floating-dynamic-switch",
            "Tethered switch",
            "A floating switch whose tether distance is sqrt(distance) / 4.",
            || demo(Tether),
        ),
    ]
}
#[derive(Clone, PartialEq)]
struct Dot {
    p: Point,
    color: usize,
}
#[derive(Clone, PartialEq)]
struct DotState {
    dots: BTreeMap<usize, Dot>,
    next: usize,
}
struct Dots {
    alt_only: bool,
}
impl Example for Dots {
    type State = DotState;
    fn initial(&self) -> DotState {
        DotState {
            dots: BTreeMap::from([(
                0,
                Dot {
                    p: Point::new(150., 100.),
                    color: 0,
                },
            )]),
            next: 1,
        }
    }
    fn scene(&self, s: &DotState) -> Scene {
        let mut sc = Scene::new(400., 300.);
        for (id, dot) in &s.dots {
            sc.push(
                Object::circle(format!("dot-{id}"), dot.p, 20., COPY_COLORS[dot.color % 6])
                    .clickable(),
            );
        }
        sc
    }
    fn begin_drag(&self, s: &DotState, id: &str, mods: Modifiers) -> Option<(DotState, String)> {
        if self.alt_only && !mods.alt {
            return None;
        }
        let key = id.strip_prefix("dot-")?.parse::<usize>().ok()?;
        let dot = s.dots.get(&key)?.clone();
        let mut next = s.clone();
        let copy = next.next;
        next.next += 1;
        let color = next.dots.len() % COPY_COLORS.len();
        next.dots.insert(copy, Dot { color, ..dot });
        Some((next, format!("dot-{copy}")))
    }
    fn drag(&self, s: &DotState, id: &str, _: Modifiers) -> Option<DragSpec<'static, DotState>> {
        let key = id.strip_prefix("dot-")?.parse::<usize>().ok()?;
        let dot = s.dots.get(&key)?;
        let base = s.clone();
        Some(vary([dot.p.x, dot.p.y], move |[x, y]| {
            let mut next = base.clone();
            next.dots.get_mut(&key).unwrap().p = Point::new(x, y);
            next
        }))
    }
    fn click(
        &self,
        s: &DotState,
        id: &str,
        _: Point,
        _: Modifiers,
        count: usize,
    ) -> Option<DotState> {
        if count < 2 {
            return None;
        }
        let key = id.strip_prefix("dot-")?.parse::<usize>().ok()?;
        let mut t = s.clone();
        t.dots.remove(&key);
        Some(t)
    }
}
#[derive(Clone, PartialEq)]
struct AlternatingState {
    value: u8,
}
struct Alternating;
impl Example for Alternating {
    type State = AlternatingState;
    fn initial(&self) -> Self::State {
        AlternatingState { value: 0 }
    }
    fn scene(&self, s: &Self::State) -> Scene {
        let mut scene = Scene::new(500., 150.);
        let bounds = Rect::new(
            s.value as f64 * 100.,
            0.,
            s.value as f64 * 100. + 100.,
            100.,
        );
        scene.push(Object::rect("switch", bounds, 0., BLACK));
        scene
    }
    fn drag(
        &self,
        _: &Self::State,
        id: &str,
        _: Modifiers,
    ) -> Option<DragSpec<'static, Self::State>> {
        (id == "switch").then(|| {
            closest((0..4).map(|value| {
                let d = fixed(AlternatingState { value });
                if value % 2 == 1 {
                    d.with_floating_options(dragology::FloatingOptions {
                        ghost_opacity: Some(0.5),
                        tether: None,
                    })
                } else {
                    d
                }
            }))
        })
    }
}
#[derive(Clone, PartialEq)]
enum IslandState {
    OnIsland(usize),
    Floating(Point),
}
const ISLANDS: [Point; 2] = [Point::new(100., 150.), Point::new(300., 150.)];
impl IslandState {
    fn p(&self) -> Point {
        match self {
            Self::OnIsland(i) => ISLANDS[*i],
            Self::Floating(p) => *p,
        }
    }
}
struct Islands;
impl Example for Islands {
    type State = IslandState;
    fn initial(&self) -> Self::State {
        IslandState::OnIsland(0)
    }
    fn scene(&self, s: &Self::State) -> Scene {
        let mut scene = Scene::new(400., 300.);
        for (i, p) in ISLANDS.into_iter().enumerate() {
            let radius = if s == &IslandState::OnIsland(i) {
                20.
            } else {
                10.
            };
            scene.push(ring(
                format!("island-{i}"),
                p,
                radius,
                [204, 204, 204, 255],
                2.,
            ));
            scene.push(
                Object::text(
                    format!("island-label-{i}"),
                    p + Vec2::new(0., radius + 16.),
                    if i == 0 { "A" } else { "B" },
                    12.,
                    [170, 170, 170, 255],
                )
                .text_baseline(TextBaseline::Alphabetic)
                .text_anchor(TextAnchor::Middle),
            );
        }
        scene.push(Object::circle("dot", s.p(), 10., [68, 136, 255, 255]));
        scene
    }
    fn drag(
        &self,
        s: &Self::State,
        id: &str,
        _: Modifiers,
    ) -> Option<DragSpec<'static, Self::State>> {
        if id != "dot" {
            return None;
        }
        let p = s.p();
        Some(
            closest([IslandState::OnIsland(0), IslandState::OnIsland(1)])
                .with_floating()
                .when_far(
                    vary([p.x, p.y], |[x, y]| IslandState::Floating(Point::new(x, y))),
                    50.,
                ),
        )
    }
}
#[derive(Clone, PartialEq)]
enum HysteresisState {
    OnIsland,
    Floating(Point),
}
impl HysteresisState {
    fn p(&self) -> Point {
        match self {
            Self::OnIsland => Point::new(100., 100.),
            Self::Floating(p) => *p,
        }
    }
}
struct Hysteresis;
impl Example for Hysteresis {
    type State = HysteresisState;
    fn initial(&self) -> Self::State {
        HysteresisState::OnIsland
    }
    fn scene(&self, s: &Self::State) -> Scene {
        let mut scene = Scene::new(200., 200.);
        scene.push(ring(
            "island",
            Point::new(100., 100.),
            if *s == HysteresisState::OnIsland {
                20.
            } else {
                10.
            },
            [204, 204, 204, 255],
            2.,
        ));
        scene.push(Object::circle("dot", s.p(), 10., [68, 136, 255, 255]));
        scene
    }
    fn drag(
        &self,
        s: &Self::State,
        id: &str,
        _: Modifiers,
    ) -> Option<DragSpec<'static, Self::State>> {
        if id != "dot" {
            return None;
        }
        let p = s.p();
        Some(fixed(HysteresisState::OnIsland).when_far_with(
            vary([p.x, p.y], |[x, y]| {
                HysteresisState::Floating(Point::new(x, y))
            }),
            40.,
            80.,
        ))
    }
}
const MAGNETS: [(Point, f64, Color); 3] = [
    (Point::new(90., 140.), 0.6, [149, 165, 166, 255]),
    (Point::new(210., 55.), 1., [192, 57, 43, 255]),
    (Point::new(350., 140.), 2.5, [41, 128, 185, 255]),
];
#[derive(Clone, PartialEq)]
struct MagneticState {
    current: usize,
}
struct Magnetic;
impl Example for Magnetic {
    type State = MagneticState;
    fn initial(&self) -> Self::State {
        MagneticState { current: 1 }
    }
    fn scene(&self, s: &Self::State) -> Scene {
        let mut sc = Scene::new(420., 300.);
        for (i, (p, strength, color)) in MAGNETS.iter().enumerate() {
            let mut halo =
                Object::circle(format!("target-{i}-halo"), *p, 55. / strength, *color).decorative();
            halo.opacity = 0.06;
            sc.push(halo);
            let mut line = ring(format!("target-{i}-ring"), *p, 55. / strength, *color, 1.);
            line.opacity = 0.35;
            line.stroke_style.dash = vec![4., 3.];
            sc.push(line);
            let mut disc = Object::circle(format!("target-{i}"), *p, 18., *color).decorative();
            disc.opacity = 0.6;
            sc.push(disc);
            sc.push(
                Object::text(
                    format!("target-{i}-label"),
                    *p + Vec2::new(0., 32.),
                    format!("×{strength}"),
                    11.,
                    [85, 85, 85, 255],
                )
                .text_baseline(TextBaseline::Alphabetic)
                .text_anchor(TextAnchor::Middle),
            );
        }
        sc.push(Object::new(
            "puck",
            Affine::translate(MAGNETS[s.current].0.to_vec2()),
            Some(Hit::Circle(Point::ORIGIN, 12.)),
            vec![Shape::Circle {
                center: Point::ORIGIN,
                radius: 12.,
                fill: [51, 51, 51, 255],
                stroke: Some((WHITE, 2.)),
            }],
        ));
        sc
    }
    fn drag(
        &self,
        _: &Self::State,
        id: &str,
        _: Modifiers,
    ) -> Option<DragSpec<'static, Self::State>> {
        (id == "puck").then(|| {
            closest((0..3).map(|current| {
                fixed(MagneticState { current }).change_gap(move |gap| gap * MAGNETS[current].1)
            }))
        })
    }
}
#[derive(Clone, PartialEq)]
struct EmergeState {
    right: bool,
    snap: bool,
}
struct Emerge;
impl Example for Emerge {
    type State = EmergeState;
    fn initial(&self) -> Self::State {
        EmergeState {
            right: false,
            snap: false,
        }
    }
    fn scene(&self, s: &Self::State) -> Scene {
        let mut sc = Scene::new(300., 200.);
        let p = Point::new(if s.right { 200. } else { 100. }, 20.);
        sc.push(Object::new(
            "dot",
            Affine::translate(p.to_vec2()),
            Some(Hit::Circle(Point::ORIGIN, 14.)),
            vec![Shape::Circle {
                center: Point::ORIGIN,
                radius: 14.,
                fill: [124, 179, 240, 255],
                stroke: Some(([74, 144, 217, 255], 2.)),
            }],
        ));
        if s.right {
            let mut o = Object::new(
                "dot2",
                Affine::translate((200., 100.)),
                None,
                vec![Shape::Circle {
                    center: Point::ORIGIN,
                    radius: 14.,
                    fill: [240, 160, 124, 255],
                    stroke: Some(([217, 130, 74, 255], 2.)),
                }],
            );
            o.emerge_from = Some("dot".into());
            sc.push(o);
        }
        sc
    }
    fn drag(
        &self,
        s: &Self::State,
        id: &str,
        _: Modifiers,
    ) -> Option<DragSpec<'static, Self::State>> {
        if id != "dot" {
            return None;
        }
        let d = between([
            EmergeState {
                right: false,
                ..s.clone()
            },
            EmergeState {
                right: true,
                ..s.clone()
            },
        ]);
        Some(if s.snap { d.with_snap_radius(20.) } else { d })
    }
    fn controls(&self, s: &Self::State) -> Vec<Control> {
        vec![Control::toggle("snap", "Snap (20px)", s.snap)]
    }
    fn action(&self, s: &Self::State, id: &str, v: Value) -> Option<Self::State> {
        if let ("snap", Value::Bool(snap)) = (id, v) {
            Some(EmergeState { snap, ..s.clone() })
        } else {
            None
        }
    }
}
#[derive(Clone, PartialEq)]
enum SwitchState {
    Off,
    On,
}
impl SwitchState {
    fn p(&self) -> Point {
        Point::new(if *self == Self::On { 160. } else { 100. }, 100.)
    }
}
struct Tether;
impl Example for Tether {
    type State = SwitchState;
    fn initial(&self) -> Self::State {
        SwitchState::Off
    }
    fn scene(&self, s: &Self::State) -> Scene {
        let mut sc = Scene::new(260., 200.);
        sc.push(
            Object::rect(
                "track",
                Rect::new(70., 70., 190., 130.),
                30.,
                if *s == SwitchState::On {
                    [34, 197, 94, 255]
                } else {
                    [209, 213, 219, 255]
                },
            )
            .decorative(),
        );
        sc.push(Object::new(
            "knob",
            Affine::translate(s.p().to_vec2()),
            Some(Hit::Circle(Point::ORIGIN, 26.)),
            vec![Shape::Circle {
                center: Point::ORIGIN,
                radius: 26.,
                fill: WHITE,
                stroke: Some(([229, 231, 235, 255], 1.)),
            }],
        ));
        sc.objects.last_mut().unwrap().shadow = Some(Shadow {
            offset: Vec2::new(0., 1.),
            blur: 2.,
            color: [0, 0, 0, 38],
        });
        sc
    }
    fn drag(
        &self,
        _: &Self::State,
        id: &str,
        _: Modifiers,
    ) -> Option<DragSpec<'static, Self::State>> {
        if id != "knob" {
            return None;
        }
        Some(
            between([SwitchState::Off, SwitchState::On]).with_floating_options(
                dragology::FloatingOptions {
                    ghost_opacity: None,
                    tether: Some(Box::new(|distance| distance.sqrt() / 4.)),
                },
            ),
        )
    }
}
#[derive(Clone, Debug, PartialEq)]
enum Endpoint {
    Node(usize),
    Free(Point),
}
#[derive(Clone, PartialEq)]
struct Edge {
    from: Endpoint,
    to: Endpoint,
}
#[derive(Clone, PartialEq)]
struct GraphState {
    nodes: BTreeMap<usize, Point>,
    edges: BTreeMap<usize, Edge>,
    next_node: usize,
    next_edge: usize,
}
struct Graph;
fn ep(ep: &Endpoint, s: &GraphState) -> Point {
    match ep {
        Endpoint::Node(n) => s.nodes[n],
        Endpoint::Free(p) => *p,
    }
}
fn edge_geometry(s: &GraphState, id: usize, e: &Edge) -> (Point, Point, Point, Point) {
    let pair = |e: &Edge| match (&e.from, &e.to) {
        (Endpoint::Node(a), Endpoint::Node(b)) => Some((*a.min(b), *a.max(b))),
        _ => None,
    };
    let mut ports: BTreeMap<usize, Vec<(usize, bool, f64)>> = BTreeMap::new();
    for (&key, edge) in &s.edges {
        let group: Vec<_> = s
            .edges
            .iter()
            .filter(|(_, v)| pair(v).is_some() && pair(v) == pair(edge))
            .map(|(k, _)| *k)
            .collect();
        let tie = if group.len() > 1 {
            group.iter().position(|k| *k == key).unwrap_or(0) as f64 * 0.01
        } else {
            0.
        };
        let a = ep(&edge.from, s);
        let b = ep(&edge.to, s);
        if let (Endpoint::Node(x), Endpoint::Node(y)) = (&edge.from, &edge.to) {
            if x == y {
                ports
                    .entry(*x)
                    .or_default()
                    .extend([(key, false, -107. + tie), (key, true, -73. + tie)]);
                continue;
            }
        }
        if let Endpoint::Node(x) = edge.from {
            ports
                .entry(x)
                .or_default()
                .push((key, false, (b - a).atan2().to_degrees() - tie));
        }
        if let Endpoint::Node(x) = edge.to {
            ports
                .entry(x)
                .or_default()
                .push((key, true, (a - b).atan2().to_degrees() + tie));
        }
    }
    let mut angles = BTreeMap::new();
    for ps in ports.values_mut() {
        ps.sort_by(|a, b| a.2.total_cmp(&b.2));
        let mut a: Vec<_> = ps.iter().map(|p| p.2).collect();
        for _ in 0..10 {
            for i in 0..a.len() {
                let next = (i + 1) % a.len();
                let gap = a[next] - a[i] + if next == 0 { 360. } else { 0. };
                if gap < 30. {
                    let push = (30. - gap) / 2.;
                    a[i] -= push;
                    a[next] += push;
                }
            }
        }
        for (p, a) in ps.iter().zip(a) {
            angles.insert((p.0, p.1), a);
        }
    }
    let resolve = |endpoint: &Endpoint, head: bool, other: Point| match endpoint {
        Endpoint::Node(node) => {
            let angle = angles[&(id, head)].to_radians();
            let d = Vec2::new(angle.cos(), angle.sin());
            (s.nodes[node] + d * 25., d)
        }
        Endpoint::Free(p) => {
            let delta = other - *p;
            let dir = if delta.hypot() > 1e-8 {
                delta / delta.hypot()
            } else {
                Vec2::new(1., 0.)
            };
            (*p, dir)
        }
    };
    let (a, ad) = resolve(&e.from, false, ep(&e.to, s));
    let (d, dd) = resolve(&e.to, true, ep(&e.from, s));
    let self_loop = matches!((&e.from,&e.to),(Endpoint::Node(a),Endpoint::Node(b))if a==b);
    let len = if self_loop { 40. } else { a.distance(d) * 0.4 };
    let start = if matches!(e.from, Endpoint::Node(_)) {
        a + ad * 5.
    } else {
        a
    };
    (start, a + ad * len, d + dd * len, d)
}

impl Graph {
    fn endpoint_spec(s: &GraphState, edge: usize, head: bool) -> DragSpec<'static, GraphState> {
        let original = s.clone();
        let e = &s.edges[&edge];
        let p = ep(if head { &e.to } else { &e.from }, s);
        let free = vary([p.x, p.y], move |[x, y]| {
            let mut n = original.clone();
            let p = Point::new(x, y);
            if head {
                n.edges.get_mut(&edge).unwrap().to = Endpoint::Free(p);
            } else {
                n.edges.get_mut(&edge).unwrap().from = Endpoint::Free(p);
            }
            n
        });
        let candidates = s
            .nodes
            .iter()
            .map(|(&node, &p)| {
                let mut n = s.clone();
                if head {
                    n.edges.get_mut(&edge).unwrap().to = Endpoint::Node(node);
                } else {
                    n.edges.get_mut(&edge).unwrap().from = Endpoint::Node(node);
                }
                let n = Rc::new(n);
                DragSpec::custom(move |ctx, pointer| {
                    let mut frame = ctx.state_frame(n.clone(), pointer)?;
                    frame.gap = (pointer.distance(p) - 35.).max(0.);
                    Ok(frame)
                })
            })
            .collect::<Vec<_>>();
        closest(candidates)
            .when_far(free, 15.)
            .on_drop(move |s: &GraphState| {
                let mut n = s.clone();
                if n.edges.get(&edge).is_some_and(|e| {
                    matches!(e.from, Endpoint::Free(_)) || matches!(e.to, Endpoint::Free(_))
                }) {
                    n.edges.remove(&edge);
                }
                n
            })
    }
}
impl Example for Graph {
    type State = GraphState;
    fn initial(&self) -> GraphState {
        let nodes = BTreeMap::from([
            (1, Point::new(80., 100.)),
            (2, Point::new(287., 73.)),
            (3, Point::new(274., 200.)),
            (4, Point::new(126., 240.)),
        ]);
        let edges = [
            (1, 2),
            (2, 3),
            (3, 4),
            (4, 1),
            (1, 1),
            (1, 2),
            (2, 1),
            (1, 3),
            (3, 4),
            (4, 3),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, (a, b))| {
            (
                i,
                Edge {
                    from: Endpoint::Node(a),
                    to: Endpoint::Node(b),
                },
            )
        })
        .collect();
        GraphState {
            nodes,
            edges,
            next_node: 5,
            next_edge: 10,
        }
    }
    fn scene(&self, s: &GraphState) -> Scene {
        let mut scene = Scene::new(350., 350.);
        for (&id, e) in &s.edges {
            let (a, b, c, d) = edge_geometry(s, id, e);
            let attachment = if let Endpoint::Node(node) = e.from {
                s.nodes[&node] + (a - s.nodes[&node]).normalize() * 25.
            } else {
                a
            };
            let arrow_len = if e.from == e.to {
                20.
            } else {
                attachment.distance(d).mul_add(0.4, 0.).clamp(8., 20.)
            };
            let dir = (d - c).normalize();
            let mut path = BezPath::new();
            path.move_to(a);
            path.curve_to(b, c - dir * arrow_len, d - dir * arrow_len);
            scene.push(Object::path(format!("edge-{id}"), path, BLACK, 2.));
            let dir = (d - c).normalize();
            let half = arrow_len * (std::f64::consts::PI / 8.).tan();
            let mut arrow = BezPath::new();
            arrow.move_to(Point::ORIGIN);
            arrow.line_to((-arrow_len, half));
            arrow.line_to((-arrow_len, -half));
            arrow.close_path();
            scene.push(Object::new(
                format!("head-{id}"),
                Affine::translate(d.to_vec2()) * Affine::rotate(dir.atan2()),
                Some(Hit::Path(arrow.clone(), 0.)),
                vec![Shape::Path {
                    path: arrow,
                    fill: Some(BLACK),
                    stroke: None,
                }],
            ));
            let mut tail = Object::new(
                format!("tail-{id}"),
                Affine::translate(a.to_vec2()),
                Some(Hit::Circle(Point::ORIGIN, 8.)),
                vec![Shape::Circle {
                    center: Point::ORIGIN,
                    radius: 8.,
                    fill: WHITE,
                    stroke: Some((BLACK, 2.)),
                }],
            )
            .hit_when_transparent();
            tail.opacity = 0.;
            scene.push(tail);
        }
        for (&id, &p) in &s.nodes {
            scene.push(Object::circle(format!("node-{id}"), p, 20., BLACK).cursor(Cursor::Grab));
        }
        let mut path = BezPath::new();
        path.move_to((-10., 0.));
        path.line_to((10., 0.));
        scene.push(
            Object::new(
                "create",
                Affine::translate((35., 35.)),
                Some(Hit::Circle(Point::ORIGIN, 20.)),
                vec![
                    Shape::Circle {
                        center: Point::ORIGIN,
                        radius: 20.,
                        fill: WHITE,
                        stroke: Some((BLACK, 2.)),
                    },
                    Shape::Path {
                        path,
                        fill: None,
                        stroke: Some((BLACK, 2.)),
                    },
                ],
            )
            .cursor(Cursor::Grab),
        );
        // The source gives this stroke its own identity. Morphing one compound
        // path into a single stroke invents a diagonal during +/− transitions.
        let mut vertical = Object::line("create-vertical", (35., 25.), (35., 45.), BLACK, 2.);
        vertical.parent = Some("create".into());
        scene.push(vertical);
        scene
    }
    fn scene_interaction(
        &self,
        s: &GraphState,
        dragged: Option<&str>,
        hovered: Option<&str>,
    ) -> Scene {
        let mut sc = self.scene(s);
        if let Some(hovered) = hovered {
            if hovered.starts_with("tail-") {
                if let Some(o) = sc.objects.iter_mut().find(|o| o.id == hovered) {
                    o.opacity = 1.;
                }
            }
        }
        if dragged.is_some_and(|id| id.starts_with("node-")) {
            sc.objects.retain(|object| object.id != "create-vertical");
        }
        sc
    }
    fn locate(&self, s: &GraphState, id: &str) -> Option<Affine> {
        if let Some(node) = id
            .strip_prefix("node-")
            .and_then(|id| id.parse::<usize>().ok())
        {
            return s.nodes.get(&node).map(|p| Affine::translate(p.to_vec2()));
        }
        if id == "create" {
            return Some(Affine::translate((35., 35.)));
        }
        self.scene(s).object(id).map(|object| object.transform)
    }
    fn begin_drag(
        &self,
        s: &GraphState,
        id: &str,
        mods: Modifiers,
    ) -> Option<(GraphState, String)> {
        if id == "create" {
            let mut next = s.clone();
            let node = next.next_node;
            next.next_node += 1;
            next.nodes.insert(node, Point::ORIGIN);
            // Upstream follows the new node's handler, which itself reacts to Alt.
            if mods.alt {
                let edge = next.next_edge;
                next.next_edge += 1;
                next.edges.insert(
                    edge,
                    Edge {
                        from: Endpoint::Node(node),
                        to: Endpoint::Free(Point::ORIGIN),
                    },
                );
                return Some((next, format!("head-{edge}")));
            }
            return Some((next, format!("node-{node}")));
        }
        let node = id.strip_prefix("node-")?.parse::<usize>().ok()?;
        if !mods.alt {
            return None;
        }
        let mut next = s.clone();
        let edge = next.next_edge;
        next.next_edge += 1;
        next.edges.insert(
            edge,
            Edge {
                from: Endpoint::Node(node),
                to: Endpoint::Free(Point::ORIGIN),
            },
        );
        Some((next, format!("head-{edge}")))
    }
    fn drag(
        &self,
        s: &GraphState,
        id: &str,
        _: Modifiers,
    ) -> Option<DragSpec<'static, GraphState>> {
        if let Some(node) = id
            .strip_prefix("node-")
            .and_then(|x| x.parse::<usize>().ok())
        {
            let p = s.nodes[&node];
            let base = s.clone();
            let free = vary([p.x, p.y], move |[x, y]| {
                let mut n = base.clone();
                n.nodes.insert(node, Point::new(x, y));
                n
            });
            let mut removed = s.clone();
            removed.nodes.remove(&node);
            removed
                .edges
                .retain(|_, e| e.from != Endpoint::Node(node) && e.to != Endpoint::Node(node));
            return Some(closest([
                {
                    let removed = Rc::new(removed);
                    DragSpec::custom(move |ctx, p| {
                        let mut frame = ctx.state_frame(removed.clone(), p)?;
                        frame.gap = (p.distance(Point::new(35., 35.)) - 20.).max(0.);
                        Ok(frame)
                    })
                },
                free,
            ]));
        }
        for (prefix, head) in [("head-", true), ("tail-", false)] {
            if let Some(edge) = id.strip_prefix(prefix).and_then(|x| x.parse().ok()) {
                return Some(Self::endpoint_spec(s, edge, head));
            }
        }
        None
    }
    fn status(&self, s: &GraphState) -> String {
        format!("{} nodes · {} directed edges", s.nodes.len(), s.edges.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dragology::DragSession;
    #[test]
    fn copies_follow_fresh_identity_without_relabeling_original() {
        for (alt_only, alt, count) in [(false, false, 2), (true, false, 1), (true, true, 2)] {
            let demo = Dots { alt_only };
            let original = demo.initial();
            let mods = Modifiers {
                alt,
                ..Default::default()
            };
            let (state, id) = demo
                .begin_drag(&original, "dot-0", mods)
                .unwrap_or((original.clone(), "dot-0".into()));
            let spec = demo.drag(&state, &id, mods).unwrap();
            let copy_id = id.clone();
            let session = DragSession::begin(
                state,
                (150., 100.),
                move |s| Dots { alt_only }.locate(s, &copy_id),
                spec,
            )
            .unwrap();
            let moved = session.finish((225., 185.)).unwrap();
            assert_eq!(moved.dots.len(), count);
            let moving = if count == 2 { 1 } else { 0 };
            assert!(moved.dots[&moving].p.distance(Point::new(225., 185.)) < 0.01);
            if count == 2 {
                assert_eq!(moved.dots[&0].p, original.dots[&0].p);
                assert_eq!(moved.dots[&0].color, 0);
                assert_eq!(moved.dots[&1].color, 1);
            }
        }
    }
    #[test]
    fn graph_alt_drag_connects_and_loose_endpoint_drop_removes_edge() {
        let original = Graph.initial();
        let mods = Modifiers {
            alt: true,
            ..Default::default()
        };
        let (state, id) = Graph.begin_drag(&original, "node-1", mods).unwrap();
        let start = Graph.locate(&state, &id).unwrap() * Point::ORIGIN;
        let spec = Graph.drag(&state, &id, mods).unwrap();
        let key = id.clone();
        let session =
            DragSession::begin(state, start, move |s| Graph.locate(s, &key), spec).unwrap();
        let linked = session.finish(original.nodes[&3]).unwrap();
        assert_eq!(linked.edges.len(), 11);
        assert_eq!(linked.edges[&10].from, Endpoint::Node(1));
        assert_eq!(linked.edges[&10].to, Endpoint::Node(3));
        let p = Graph.locate(&linked, "head-10").unwrap() * Point::ORIGIN;
        let spec = Graph
            .drag(&linked, "head-10", Modifiers::default())
            .unwrap();
        let session = DragSession::begin(linked, p, |s| Graph.locate(s, "head-10"), spec).unwrap();
        let removed = session.finish((600., 600.)).unwrap();
        assert_eq!(removed.edges.len(), 10);
        assert!(!removed.edges.contains_key(&10));
    }
    #[test]
    fn asymmetric_island_thresholds_retain_history() {
        let state = Hysteresis.initial();
        let spec = Hysteresis
            .drag(&state, "dot", Modifiers::default())
            .unwrap();
        let mut session = DragSession::begin(
            state,
            (100., 100.),
            |s| Some(Affine::translate(s.p().to_vec2())),
            spec,
        )
        .unwrap();
        for (x, on) in [(170., true), (190., false), (150., false), (130., true)] {
            assert_eq!(
                matches!(
                    *session.frame((x, 100.)).unwrap().drop_state,
                    HysteresisState::OnIsland
                ),
                on
            );
        }
    }
    #[test]
    fn native_floating_tether_uses_source_distance_function() {
        let state = Tether.initial();
        let spec = Tether.drag(&state, "knob", Modifiers::default()).unwrap();
        let mut session = DragSession::begin(
            state,
            (100., 100.),
            |s| Some(Affine::translate(s.p().to_vec2())),
            spec,
        )
        .unwrap();
        let frame = session.frame((160., 200.)).unwrap();
        // A tether invalidates the cached grab point; the host measures its preview.
        assert_eq!(frame.grab_position, None);
        let mut demo = crate::runtime::ModelDemo::new(Tether);
        demo.set_animated(false);
        demo.press(Point::new(100., 100.), Modifiers::default(), 1);
        demo.move_to(Point::new(160., 200.), Modifiers::default());
        demo.advance(std::time::Duration::ZERO);
        assert!(
            (demo.scene().object("knob").unwrap().transform * Point::ORIGIN)
                .distance(Point::new(160., 102.5))
                < 1e-6
        );
    }
    #[test]
    fn copy_follow_cancellation_restores_original_scene() {
        let mut demo = crate::runtime::ModelDemo::new(Dots { alt_only: false });
        demo.set_animated(false);
        let initial = demo.scene().clone();
        demo.press(Point::new(150., 100.), Modifiers::default(), 1);
        demo.move_to(Point::new(230., 180.), Modifiers::default());
        demo.advance(std::time::Duration::ZERO);
        assert_eq!(
            demo.scene().object("dot-0").unwrap().transform * Point::ORIGIN,
            Point::new(150., 100.)
        );
        assert!(
            (demo.scene().object("dot-1").unwrap().transform * Point::ORIGIN)
                .distance(Point::new(230., 180.))
                < 0.01
        );
        demo.cancel();
        assert_eq!(demo.scene(), &initial);
    }
}
