//! Native port of Dragology's MIT-licensed rotations/rebalancing playground.
//! Source: declarative-dragging/dragology b145bdd56530f7cfa9a61bf6a7170459d2f140e1.
use crate::*;
use dragology::{FloatingOptions, SnapOptions};
use dragology::{between, closest, drop_target, fixed};
use std::collections::BTreeMap;
mod bst;
mod two_three;

pub fn entry() -> Entry {
    Entry::new(
        "rotations-rebalancing",
        "Rotations & rebalancing",
        "",
        balancing_page,
    )
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Splay,
    Avl,
    RedBlack,
    TwoThree,
    Heap,
}
const KEYS: [i32; 10] = [5, 2, 8, 1, 9, 3, 7, 4, 6, 10];
const TT_KEYS: [i32; 12] = [50, 20, 80, 10, 30, 60, 90, 40, 70, 25, 55, 85];
#[derive(Clone, Debug, PartialEq)]
struct State {
    mode: Mode,
    root: bst::Link,
    deck: Vec<i32>,
    tree: Option<two_three::Node>,
    next_id: usize,
    heap: Vec<i32>,
    out: Vec<i32>,
    ghost_edges: Vec<(i32, i32)>,
    last_step: String,
}
fn balancing_page() -> Box<dyn Demo> {
    Box::new(CompositeDemo::new(vec![
        Panel {
            id: "splay",
            title: "Splay tree",
            description: "Drag any non-root node upward. Each splay step (zig, zig-zig, zig-zag) is a d.between from the current tree to the tree one step later, with a chained snap radius, so a single drag to the root plays out as a sequence of rotations. Nodes are laid out by in-order rank, so rotations only move nodes vertically.",
            demo: demo(ModePanel(Mode::Splay)),
        },
        Panel {
            id: "balanced",
            title: "AVL / red-black tree",
            description: "Drag a key from the deck into the tree. The drop preview is the tree after insertion and rebalancing — rotations (and, for red-black, recolorings) included. Move the key away to cancel.",
            demo: demo(ModePanel(Mode::Avl)),
        },
        Panel {
            id: "two-three",
            title: "2-3 tree",
            description: "Drag a key into a full leaf and watch the split propagate upward: the middle key is pushed into the parent, which may split in turn. Nodes created by splits use dragologyEmergeFrom to grow out of the node they split from.",
            demo: demo(ModePanel(Mode::TwoThree)),
        },
        Panel {
            id: "heap",
            title: "Binary heap",
            description: "Drag a value from the deck to append it to the heap, then drag it up past its parent to sift it up (each swap is a chained d.between). Drag a node down past its larger child to sift down. Drag the root into the tray to extract the max. The array view below stays in sync because it is the same state rendered twice.",
            demo: demo(ModePanel(Mode::Heap)),
        },
    ]))
}
struct ModePanel(Mode);
impl Example for ModePanel {
    type State = State;
    fn initial(&self) -> State {
        initial(self.0)
    }
    fn scene(&self, s: &State) -> Scene {
        Balancing.scene(s)
    }
    fn scene_dragging(&self, s: &State, id: Option<&str>) -> Scene {
        Balancing.scene_dragging(s, id)
    }
    fn drag(&self, s: &State, id: &str, m: Modifiers) -> Option<DragSpec<'static, State>> {
        Balancing.drag(s, id, m)
    }
    fn click(&self, s: &State, id: &str, p: Point, m: Modifiers, c: usize) -> Option<State> {
        Balancing.click(s, id, p, m, c)
    }
    fn controls(&self, s: &State) -> Vec<Control> {
        Balancing.controls(s)
    }
    fn action(&self, s: &State, id: &str, v: Value) -> Option<State> {
        Balancing.action(s, id, v)
    }
    fn action_transition(&self, s: &State, id: &str) -> Option<dragology::Transition> {
        Balancing.action_transition(s, id)
    }
}
struct Balancing;
fn initial(mode: Mode) -> State {
    let mut s = State {
        mode,
        root: None,
        deck: vec![],
        tree: None,
        next_id: 0,
        heap: vec![],
        out: vec![],
        ghost_edges: vec![],
        last_step: String::new(),
    };
    match mode {
        Mode::Splay => {
            for key in [6, 3, 9, 1, 4, 8, 10, 2, 5, 7] {
                s.root = bst::plain(s.root, key)
            }
        }
        Mode::Avl | Mode::RedBlack => s.deck = KEYS.to_vec(),
        Mode::TwoThree => s.deck = TT_KEYS.to_vec(),
        Mode::Heap => {
            s.heap = vec![88, 50, 71, 33, 12, 60];
            s.deck = vec![14, 42, 7, 91, 23, 65];
        }
    }
    s
}
const fn rgb(value: u32) -> Color {
    [(value >> 16) as u8, (value >> 8) as u8, value as u8, 255]
}
fn key_object(key: i32, pos: Point, fill: Color, stroke: Color, text: Color) -> Object {
    let mut object = Object::circle(format!("node-{key}"), pos, 15., fill)
        .label(key.to_string(), Point::ORIGIN, 13., text)
        .text_anchor(TextAnchor::Middle)
        .text_baseline(TextBaseline::Central)
        .font_weight(600);
    if let Shape::Circle { stroke: line, .. } = &mut object.shapes[0] {
        *line = Some((stroke, 2.))
    }
    object
}
fn box_object(
    id: impl Into<String>,
    bounds: Rect,
    radius: f64,
    fill: Color,
    stroke: Color,
) -> Object {
    let mut object = Object::rect(id, bounds, radius, fill);
    if let Shape::Rect { stroke: line, .. } = &mut object.shapes[0] {
        *line = Some((stroke, 1.))
    }
    object
}
fn reset_button(scene: &mut Scene) {
    scene.push(
        box_object(
            "reset",
            Rect::from_origin_size((scene.width - 60., 6.), (46.5, 20.)),
            4.,
            rgb(0xf1f5f9),
            rgb(0xcbd5e1),
        )
        .label("reset", (7., 10.), 11., rgb(0x475569))
        .text_baseline(TextBaseline::Central)
        .clickable(),
    );
}
fn caption(scene: &mut Scene, x: f64, y: f64, text: &str) {
    scene.push(
        Object::text("caption", (x, y), text, 12., rgb(0x64748b))
            .text_baseline(TextBaseline::Alphabetic)
            .italic(),
    );
}

fn key_cell(key: i32, pos: Point) -> Object {
    Object::new(
        format!("node-{key}"),
        Affine::translate(pos.to_vec2()),
        Some(Hit::Rect(Rect::new(-15., -15., 15., 15.))),
        vec![Shape::Rect {
            bounds: Rect::new(-15., -15., 15., 15.),
            radius: 4.,
            fill: rgb(0xffffff),
            stroke: Some((rgb(0x94a3b8), 1.5)),
        }],
    )
    .label(key.to_string(), Point::ORIGIN, 12., rgb(0x111111))
    .text_anchor(TextAnchor::Middle)
    .text_baseline(TextBaseline::Central)
    .font_weight(600)
}
fn draw_deck(scene: &mut Scene, s: &State) {
    let (x, dx, left, width, label) = match s.mode {
        Mode::TwoThree => (80., 34., 58., 418., "deck →"),
        Mode::Heap => (76., 38., 54., 234., "insert →"),
        _ => (90., 40., 65., 410., "deck →"),
    };
    scene.push(
        box_object(
            "deck",
            Rect::from_origin_size((left, 14.), (width, 44.)),
            8.,
            rgb(0xf8fafc),
            rgb(0xe2e8f0),
        )
        .decorative(),
    );
    scene.push(
        Object::text("deck-label", (12., 40.), label, 12., rgb(0x64748b))
            .text_baseline(TextBaseline::Alphabetic)
            .italic(),
    );
    for (i, &key) in s.deck.iter().enumerate() {
        let pos = Point::new(x + i as f64 * dx, 36.);
        scene.push(if s.mode == Mode::TwoThree {
            key_cell(key, pos)
        } else {
            key_object(key, pos, rgb(0xffffff), rgb(0x94a3b8), rgb(0x111111))
        });
    }
}
fn real_edges(root: &bst::Link) -> Vec<(i32, i32)> {
    let mut e = Vec::new();
    bst::edges(root, &mut e);
    e
}
fn draw_edges(
    scene: &mut Scene,
    edges: &[(i32, i32)],
    ghosts: &[(i32, i32)],
    positions: &BTreeMap<i32, Point>,
) {
    for &(p, c) in edges
        .iter()
        .chain(ghosts.iter().filter(|e| !edges.contains(e)))
    {
        if let (Some(a), Some(b)) = (positions.get(&p), positions.get(&c)) {
            let mut o = Object::line(format!("edge-{p}-{c}"), *a, *b, rgb(0x94a3b8), 2.);
            if !edges.contains(&(p, c)) {
                o.opacity = 0.;
            }
            scene.push(o);
        }
    }
}
fn bst_scene(s: &State) -> Scene {
    let splay = s.mode == Mode::Splay;
    let mut scene = Scene::new(520., if splay { 320. } else { 340. });
    reset_button(&mut scene);
    let keys = (1..=10).collect::<Vec<_>>();
    let mut positions = bst::positions(&s.root, &keys, if splay { 40. } else { 110. });
    if splay {
        for p in positions.values_mut() {
            p.y = 40. + (p.y - 40.) * 50. / 52.;
        }
    } else {
        draw_deck(&mut scene, s);
        for (i, &key) in s.deck.iter().enumerate() {
            positions.insert(key, Point::new(90. + i as f64 * 40., 36.));
        }
    }
    draw_edges(&mut scene, &real_edges(&s.root), &s.ghost_edges, &positions);
    let mut nodes = Vec::new();
    bst::inorder(&s.root, &mut nodes);
    for n in nodes {
        let pos = positions[&n.key];
        let (fill, stroke, text) = if s.mode == Mode::RedBlack {
            if n.red {
                (rgb(0xef4444), rgb(0xb91c1c), rgb(0xffffff))
            } else {
                (rgb(0x1f2937), rgb(0x111827), rgb(0xffffff))
            }
        } else if splay && s.root.as_ref().is_some_and(|r| r.key == n.key) {
            (rgb(0xfde68a), rgb(0xd97706), rgb(0x111111))
        } else {
            (rgb(0xdbeafe), rgb(0x3b82f6), rgb(0x111111))
        };
        scene.push(key_object(n.key, pos, fill, stroke, text));
        if s.mode == Mode::Avl {
            let bf = bst::balance(n);
            scene.push(
                Object::text(
                    format!("balance-{}", n.key),
                    pos + Vec2::new(17., -13.),
                    if bf > 0 {
                        format!("+{bf}")
                    } else {
                        bf.to_string()
                    },
                    9.,
                    rgb(0x64748b),
                )
                .font_family(FontFamily::Mono)
                .text_baseline(TextBaseline::Alphabetic),
            );
        }
    }
    if splay {
        caption(
            &mut scene,
            25.,
            308.,
            &if s.last_step.is_empty() {
                "drag a node up toward the root".into()
            } else {
                format!("last step: {}", s.last_step)
            },
        );
    } else {
        caption(
            &mut scene,
            25.,
            328.,
            if s.root.is_none() {
                "drag a key from the deck into the tree"
            } else if s.mode == Mode::Avl {
                "small numbers are balance factors (right height − left height)"
            } else {
                "preview shows recolorings and rotations of the insert fixup"
            },
        );
    }
    scene
}
fn tt_scene(s: &State) -> Scene {
    let mut scene = Scene::new(520., 340.);
    reset_button(&mut scene);
    draw_deck(&mut scene, s);
    if let Some(root) = &s.tree {
        let mut positions = BTreeMap::new();
        two_three::layout(root, Point::new(260., 110.), &mut positions);
        let mut nodes = Vec::new();
        two_three::nodes(root, &mut nodes);
        for n in &nodes {
            let p = positions[&n.id];
            for c in &n.children {
                let q = positions[&c.id];
                let mut edge = Object::line(
                    format!("tt-edge-{}-{}", n.id, c.id),
                    p + Vec2::new(0., 15.),
                    q - Vec2::new(0., 15.),
                    rgb(0x94a3b8),
                    2.,
                );
                edge.emerge_from = c.emerge.or(n.emerge).map(|id| format!("tt-node-{id}"));
                scene.push(edge);
            }
        }
        for n in nodes {
            let p = positions[&n.id];
            let w = n.keys.len() as f64 * 30.;
            let mut o = Object::new(
                format!("tt-node-{}", n.id),
                Affine::translate(p.to_vec2()),
                None,
                vec![Shape::Rect {
                    bounds: Rect::from_center_size(Point::ORIGIN, (w + 6., 36.)),
                    radius: 7.,
                    fill: [224, 231, 255, 255],
                    stroke: Some(([99, 102, 241, 255], 1.5)),
                }],
            );
            o.emerge_mode = EmergeMode::Clone;
            // Upstream wraps the container rect so a split retains its own size.
            o.emerge_bounds = false;
            o.emerge_from = n.emerge.map(|id| format!("tt-node-{id}"));
            scene.push(o);
            for (i, &key) in n.keys.iter().enumerate() {
                scene.push(key_cell(
                    key,
                    p + Vec2::new(-w / 2. + 15. + i as f64 * 30., 0.),
                ));
            }
        }
    }
    caption(
        &mut scene,
        12.,
        328.,
        if s.tree.is_some() {
            "a node with 3 keys splits; its middle key is pushed up"
        } else {
            "drag a key from the deck into the tree"
        },
    );
    scene
}
fn heap_position(i: usize) -> Point {
    let level = (i + 1).ilog2();
    let count = 1usize << level;
    let slot = i - (count - 1);
    Point::new(
        20. + 440. * (slot as f64 + 0.5) / count as f64,
        100. + f64::from(level) * 52.,
    )
}
fn heap_scene(s: &State) -> Scene {
    let mut scene = Scene::new(560., 340.);
    reset_button(&mut scene);
    draw_deck(&mut scene, s);
    scene.push(
        box_object(
            "out-tray",
            Rect::new(330., 14., 498., 58.),
            8.,
            rgb(0xfff7ed),
            rgb(0xfdba74),
        )
        .dash(vec![4., 3.])
        .decorative(),
    );
    scene.push(
        Object::text(
            "out-label",
            (492., 36.),
            "← extract max",
            12.,
            rgb(0xc2410c),
        )
        .text_anchor(TextAnchor::End)
        .text_baseline(TextBaseline::Central)
        .italic(),
    );
    for (i, &key) in s.out.iter().enumerate() {
        scene.push(key_object(
            key,
            Point::new(350. + i as f64 * 30., 36.),
            rgb(0xfed7aa),
            rgb(0xea580c),
            rgb(0x111111),
        ));
    }
    for i in 1..s.heap.len() {
        let p = (i - 1) / 2;
        scene.push(Object::line(
            format!("edge-{}-{}", s.heap[p], s.heap[i]),
            heap_position(p),
            heap_position(i),
            rgb(0x94a3b8),
            2.,
        ));
    }
    for (i, &key) in s.heap.iter().enumerate() {
        scene.push(key_object(
            key,
            heap_position(i),
            if i == 0 { rgb(0xfde68a) } else { rgb(0xdcfce7) },
            if i == 0 { rgb(0xd97706) } else { rgb(0x16a34a) },
            rgb(0x111111),
        ));
    }
    for i in 0..15 {
        scene.push(
            box_object(
                format!("slot-{i}"),
                Rect::from_center_size((24. + i as f64 * 28., 292.), (28., 28.)),
                0.,
                if i < s.heap.len() {
                    rgb(0xffffff)
                } else {
                    rgb(0xf8fafc)
                },
                rgb(0xcbd5e1),
            )
            .decorative(),
        );
        scene.push(
            Object::text(
                format!("index-{i}"),
                (24. + i as f64 * 28., 316.),
                i.to_string(),
                9.,
                rgb(0x94a3b8),
            )
            .text_anchor(TextAnchor::Middle)
            .text_baseline(TextBaseline::Alphabetic)
            .font_family(FontFamily::Mono),
        );
    }
    for (i, &key) in s.heap.iter().enumerate() {
        scene.push(
            Object::text(
                format!("cell-{key}"),
                (24. + i as f64 * 28., 292.),
                key.to_string(),
                12.,
                rgb(0x111111),
            )
            .text_anchor(TextAnchor::Middle)
            .text_baseline(TextBaseline::Central)
            .font_weight(500),
        );
    }
    scene
}
fn inserted(s: &State, key: i32) -> State {
    let mut n = s.clone();
    n.deck.retain(|k| *k != key);
    match s.mode {
        Mode::Avl => n.root = bst::avl(n.root, key),
        Mode::RedBlack => n.root = bst::red_black(n.root, key),
        Mode::TwoThree => n.tree = Some(two_three::insert(n.tree, key, &mut n.next_id)),
        Mode::Heap => {
            n.heap.push(key);
        }
        Mode::Splay => {}
    }
    n
}
fn heap_swaps(s: &State, i: usize) -> Vec<State> {
    let mut out = Vec::new();
    let me = s.heap[i];
    let mut neighbors = Vec::new();
    if i > 0 {
        let p = (i - 1) / 2;
        if me > s.heap[p] {
            neighbors.push(p)
        }
    }
    let l = 2 * i + 1;
    let r = l + 1;
    if l < s.heap.len() {
        let bigger = if r < s.heap.len() && s.heap[r] > s.heap[l] {
            r
        } else {
            l
        };
        if s.heap[bigger] > me {
            neighbors.push(bigger)
        }
    }
    for j in neighbors {
        let mut n = s.clone();
        n.heap.swap(i, j);
        out.push(n)
    }
    out
}
fn extract(s: &State) -> State {
    let mut n = s.clone();
    let root = n.heap.remove(0);
    if let Some(last) = n.heap.pop() {
        n.heap.insert(0, last);
    }
    n.out.push(root);
    n
}
fn clear_ghosts(s: &State) -> State {
    let mut n = s.clone();
    n.ghost_edges.clear();
    n
}
fn wiggle(s: &State) -> DragSpec<'static, State> {
    fixed(s.clone()).with_floating_options(FloatingOptions {
        tether: Some(Box::new(|distance| distance / 4.)),
        ..Default::default()
    })
}
impl Example for Balancing {
    type State = State;
    fn initial(&self) -> State {
        initial(Mode::Splay)
    }
    fn scene(&self, s: &State) -> Scene {
        match s.mode {
            Mode::TwoThree => tt_scene(s),
            Mode::Heap => heap_scene(s),
            _ => bst_scene(s),
        }
    }
    fn scene_dragging(&self, s: &State, id: Option<&str>) -> Scene {
        let mut scene = self.scene(s);
        if let Some(id) = id {
            if let Some(i) = scene.objects.iter().position(|o| o.id == id) {
                let mut object = scene.objects.remove(i);
                if s.mode != Mode::TwoThree {
                    if let Some(Shape::Circle { stroke, .. }) = object.shapes.first_mut() {
                        *stroke = Some((rgb(0xf59e0b), 3.));
                    }
                } else if s.deck.iter().any(|k| id == format!("node-{k}")) {
                    if let Some(Shape::Rect { stroke, fill, .. }) = object.shapes.first_mut() {
                        *stroke = Some((rgb(0xf59e0b), 2.5));
                        *fill = rgb(0xfef3c7);
                    }
                }
                scene.push(object);
            }
            if s.mode == Mode::Heap {
                if let Some(key) = id.strip_prefix("node-") {
                    if let Some(cell) = scene
                        .objects
                        .iter_mut()
                        .find(|o| o.id == format!("cell-{key}"))
                    {
                        cell.text_style.weight = 800;
                        for shape in &mut cell.shapes {
                            if let Shape::Text { color, .. } = shape {
                                *color = rgb(0xd97706)
                            }
                        }
                    }
                }
            }
        }
        scene
    }
    fn click(&self, s: &State, id: &str, _: Point, _: Modifiers, _: usize) -> Option<State> {
        (id == "reset").then(|| initial(s.mode))
    }
    fn drag(&self, s: &State, id: &str, _: Modifiers) -> Option<DragSpec<'static, State>> {
        let key: i32 = id.strip_prefix("node-")?.parse().ok()?;
        if s.deck.contains(&key) {
            if s.mode == Mode::Heap && s.heap.len() >= 15 {
                return None;
            }
            let mut target = inserted(s, key);
            let mut source = s.clone();
            if matches!(s.mode, Mode::Avl | Mode::RedBlack) {
                target.ghost_edges = real_edges(&s.root);
                source.ghost_edges = real_edges(&target.root);
            }
            return Some(
                drop_target(
                    target,
                    Rect::new(
                        0.,
                        if s.mode == Mode::Heap { 60. } else { 70. },
                        if s.mode == Mode::Heap { 560. } else { 520. },
                        if s.mode == Mode::Heap { 272. } else { 320. },
                    ),
                )
                .with_floating()
                .when_far(fixed(source).with_floating(), 50.)
                .on_drop(clear_ghosts),
            );
        }
        match s.mode {
            Mode::Splay => {
                let (root, step) = bst::splay(&s.root, key)?;
                let mut target = s.clone();
                target.root = root;
                target.last_step = format!("{step} on {key}");
                target.ghost_edges = real_edges(&s.root);
                let mut source = s.clone();
                source.ghost_edges = real_edges(&target.root);
                Some(
                    between([source, target])
                        .on_drop(clear_ghosts)
                        .with_snap_radius_options(
                            10.,
                            SnapOptions {
                                chain: true,
                                ..Default::default()
                            },
                        ),
                )
            }
            Mode::Heap => {
                let i = s.heap.iter().position(|k| *k == key)?;
                let swaps = heap_swaps(s, i)
                    .into_iter()
                    .map(|n| {
                        between([s.clone(), n]).with_snap_radius_options(
                            10.,
                            SnapOptions {
                                chain: true,
                                ..Default::default()
                            },
                        )
                    })
                    .collect::<Vec<_>>();
                if i == 0 {
                    Some(
                        closest(swaps.into_iter().chain(std::iter::once(drop_target(
                            extract(s),
                            Rect::new(330., 14., 498., 58.),
                        ))))
                        .when_far(fixed(s.clone()).with_floating(), 40.),
                    )
                } else if swaps.is_empty() {
                    Some(wiggle(s))
                } else {
                    Some(closest(swaps))
                }
            }
            _ => None,
        }
    }
    fn controls(&self, s: &State) -> Vec<Control> {
        if matches!(s.mode, Mode::Avl | Mode::RedBlack) {
            vec![Control::Choice {
                id: "mode",
                label: "Tree".into(),
                options: vec!["AVL".into(), "Red-black".into()],
                selected: usize::from(s.mode == Mode::RedBlack),
            }]
        } else {
            vec![]
        }
    }
    fn action(&self, s: &State, id: &str, value: Value) -> Option<State> {
        match (id, value) {
            ("mode", Value::Index(i)) if matches!(s.mode, Mode::Avl | Mode::RedBlack) && i < 2 => {
                Some(initial(if i == 0 { Mode::Avl } else { Mode::RedBlack }))
            }
            _ => None,
        }
    }
    fn action_transition(&self, _: &State, id: &str) -> Option<dragology::Transition> {
        (id == "reset").then_some(dragology::Transition {
            duration: std::time::Duration::from_millis(300),
            ..Default::default()
        })
    }
    fn status(&self, _: &State) -> String {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn ordered(root: &bst::Link) -> Vec<i32> {
        let mut nodes = Vec::new();
        bst::inorder(root, &mut nodes);
        nodes.into_iter().map(|n| n.key).collect()
    }
    fn avl_valid(root: &bst::Link) -> bool {
        root.as_ref()
            .is_none_or(|n| bst::balance(n).abs() <= 1 && avl_valid(&n.left) && avl_valid(&n.right))
    }
    fn rb_height(root: &bst::Link) -> usize {
        let Some(n) = root else { return 1 };
        if n.red {
            assert!(n.left.as_ref().is_none_or(|c| !c.red));
            assert!(n.right.as_ref().is_none_or(|c| !c.red));
        }
        let left = rb_height(&n.left);
        assert_eq!(left, rb_height(&n.right));
        left + usize::from(!n.red)
    }
    fn orders() -> Vec<Vec<i32>> {
        let mut output = vec![(1..=10).collect(), (1..=10).rev().collect(), KEYS.to_vec()];
        let mut seed = 5723u64;
        for _ in 0..64 {
            let mut order = (1..=10).collect::<Vec<_>>();
            for i in (1..order.len()).rev() {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                order.swap(i, ((seed >> 32) as usize) % (i + 1));
            }
            output.push(order);
        }
        output
    }
    #[test]
    fn source_structure_panels_route_controls_and_drags_independently() {
        let mut page = balancing_page();
        page.set_animated(false);
        let splay = page.scene().object("splay/node-6").unwrap().clone();
        let tt = page.scene().object("two-three/node-50").unwrap().clone();
        let heap = page.scene().object("heap/node-88").unwrap().clone();
        let start = page.scene().object("balanced/node-5").unwrap().transform * Point::ORIGIN;
        let offset = start - Point::new(90., 36.);
        let target = Point::new(260., 140.) + offset;
        page.press(start, Modifiers::default(), 1);
        page.move_to(target, Modifiers::default());
        page.advance(std::time::Duration::ZERO);
        page.release(target, Modifiers::default());
        let inserted = page.scene().object("balanced/node-5").unwrap().transform * Point::ORIGIN;
        assert!((inserted.y - (offset.y + 110.)).abs() < 1e-6);
        assert_eq!(page.scene().object("splay/node-6"), Some(&splay));
        assert_eq!(page.scene().object("two-three/node-50"), Some(&tt));
        assert_eq!(page.scene().object("heap/node-88"), Some(&heap));
        page.action("balanced/mode", Value::Index(1));
        assert_eq!(
            page.scene().object("balanced/node-5").unwrap().transform * Point::ORIGIN,
            start
        );
        assert_eq!(page.scene().object("heap/node-88"), Some(&heap));
        let start = heap.transform * Point::ORIGIN;
        let offset = start - heap_position(0);
        let tray = Point::new(350., 36.) + offset;
        page.press(start, Modifiers::default(), 1);
        page.move_to(tray, Modifiers::default());
        page.advance(std::time::Duration::ZERO);
        page.release(tray, Modifiers::default());
        assert_eq!(
            page.scene().object("heap/node-88").unwrap().transform * Point::ORIGIN,
            tray
        );
        let reset = page.scene().object("heap/reset").unwrap().transform * Point::new(23., 10.);
        page.press(reset, Modifiers::default(), 1);
        page.release(reset, Modifiers::default());
        assert_eq!(page.scene().object("heap/node-88"), Some(&heap));
    }
    #[test]
    fn avl_and_red_black_insertions_preserve_invariants_for_different_orders() {
        for order in orders() {
            let mut avl = None;
            let mut rb = None;
            let mut expected = Vec::new();
            for key in order {
                expected.push(key);
                expected.sort_unstable();
                avl = bst::avl(avl, key);
                rb = bst::red_black(rb, key);
                assert_eq!(ordered(&avl), expected);
                assert_eq!(ordered(&rb), expected);
                assert!(avl_valid(&avl));
                assert!(!rb.as_ref().unwrap().red);
                rb_height(&rb);
            }
        }
    }
    #[test]
    fn splay_steps_preserve_search_order_and_eventually_promote_every_key() {
        for order in orders() {
            let mut original = None;
            for key in order {
                original = bst::plain(original, key);
            }
            for key in 1..=10 {
                let mut root = original.clone();
                for _ in 0..10 {
                    let Some((next, _)) = bst::splay(&root, key) else {
                        break;
                    };
                    assert_eq!(ordered(&next), (1..=10).collect::<Vec<_>>());
                    root = next;
                }
                assert_eq!(root.unwrap().key, key);
            }
        }
        for (order, key, expected) in [
            (vec![2, 1], 1, "zig"),
            (vec![3, 2, 1], 1, "zig-zig"),
            (vec![3, 1, 2], 2, "zig-zag"),
        ] {
            let mut root = None;
            for key in order {
                root = bst::plain(root, key);
            }
            assert_eq!(bst::splay(&root, key).unwrap().1, expected);
        }
    }
    fn check_tt(
        n: &two_three::Node,
        low: i32,
        high: i32,
        depth: usize,
        leaf_depth: &mut Option<usize>,
        keys: &mut Vec<i32>,
    ) {
        assert!((1..=2).contains(&n.keys.len()));
        assert!(n.keys.windows(2).all(|w| w[0] < w[1]));
        assert!(n.keys.iter().all(|&k| low < k && k < high));
        keys.extend(&n.keys);
        if n.children.is_empty() {
            if let Some(previous) = leaf_depth {
                assert_eq!(*previous, depth)
            } else {
                *leaf_depth = Some(depth)
            }
        } else {
            assert_eq!(n.children.len(), n.keys.len() + 1);
            for (i, c) in n.children.iter().enumerate() {
                check_tt(
                    c,
                    if i == 0 { low } else { n.keys[i - 1] },
                    if i == n.keys.len() { high } else { n.keys[i] },
                    depth + 1,
                    leaf_depth,
                    keys,
                )
            }
        }
    }
    #[test]
    fn two_three_splits_keep_all_leaves_at_the_same_depth() {
        for order in orders()
            .into_iter()
            .chain(std::iter::once(TT_KEYS.to_vec()))
        {
            let mut root = None;
            let mut next = 0;
            let mut expected = Vec::new();
            for key in order {
                expected.push(key);
                expected.sort_unstable();
                root = Some(two_three::insert(root, key, &mut next));
                let mut keys = Vec::new();
                check_tt(
                    root.as_ref().unwrap(),
                    i32::MIN,
                    i32::MAX,
                    0,
                    &mut None,
                    &mut keys,
                );
                keys.sort_unstable();
                assert_eq!(keys, expected);
            }
        }
    }
    fn heap_valid(heap: &[i32]) -> bool {
        (1..heap.len()).all(|i| heap[(i - 1) / 2] >= heap[i])
    }
    #[test]
    fn manual_heap_sifts_restore_order_after_append_and_extract() {
        let mut state = initial(Mode::Heap);
        for key in state.deck.clone() {
            state = inserted(&state, key);
            loop {
                let i = state.heap.iter().position(|k| *k == key).unwrap();
                if i == 0 || state.heap[(i - 1) / 2] >= key {
                    break;
                }
                state = heap_swaps(&state, i)
                    .into_iter()
                    .find(|n| n.heap[(i - 1) / 2] == key)
                    .unwrap();
            }
            assert!(heap_valid(&state.heap));
        }
        while !state.heap.is_empty() {
            let maximum = *state.heap.iter().max().unwrap();
            state = extract(&state);
            assert_eq!(state.out.last(), Some(&maximum));
            if let Some(&key) = state.heap.first() {
                loop {
                    let i = state.heap.iter().position(|k| *k == key).unwrap();
                    let Some(next) = heap_swaps(&state, i).into_iter().next() else {
                        break;
                    };
                    state = next;
                }
            }
            assert!(heap_valid(&state.heap));
        }
        assert!(state.out.windows(2).all(|w| w[0] >= w[1]));
        assert_eq!(state.out.len(), 12);
    }
    fn gesture(state: State, key: i32, target: Point) -> State {
        let id = format!("node-{key}");
        let scene = Balancing.scene(&state);
        let origin = scene.object(&id).unwrap().transform * Point::ORIGIN;
        let spec = Balancing.drag(&state, &id, Modifiers::default()).unwrap();
        let mut session = dragology::DragSession::begin(
            state,
            origin,
            move |state| Balancing.locate(state, &id),
            spec,
        )
        .unwrap();
        session.frame(origin).unwrap();
        session.frame(target).unwrap();
        session.finish(target).unwrap()
    }
    #[test]
    fn drag_drop_inserts_every_deck_key_and_splays_to_the_root() {
        for mode in [Mode::Avl, Mode::RedBlack, Mode::TwoThree, Mode::Heap] {
            let mut state = initial(mode);
            for key in state.deck.clone() {
                let cancelled = gesture(state.clone(), key, Point::new(530., 20.));
                assert_eq!(cancelled, state);
                state = gesture(state, key, Point::new(270., 140.));
                assert!(!state.deck.contains(&key));
                match mode {
                    Mode::Avl | Mode::RedBlack => assert!(ordered(&state.root).contains(&key)),
                    Mode::Heap => assert!(state.heap.contains(&key)),
                    Mode::TwoThree => assert!(
                        Balancing
                            .scene(&state)
                            .object(&format!("node-{key}"))
                            .is_some()
                    ),
                    _ => unreachable!(),
                }
            }
        }
        let mut state = initial(Mode::Splay);
        for _ in 0..5 {
            if state.root.as_ref().unwrap().key == 7 {
                break;
            }
            state = gesture(state, 7, Point::new(304., 40.));
        }
        assert_eq!(state.root.as_ref().unwrap().key, 7);
    }
}
