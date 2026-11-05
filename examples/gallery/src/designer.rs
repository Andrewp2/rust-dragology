//! Typed drag-spec editor, ported from Dragology (MIT, Dragology contributors).
use crate::*;
use dragology::{between, closest, drop_target, fixed, vary, vary_constrained};
use std::collections::{BTreeMap, BTreeSet};

const ROOT: u64 = 0;
const WHITE: Color = [255; 4];
const TRASH: Rect = Rect::new(558., 10., 590., 42.);
const DOTS: [Point; 3] = [
    Point::new(510., 97.),
    Point::new(465., 172.),
    Point::new(555., 172.),
];
const fn rgb(hex: u32) -> Color {
    [(hex >> 16) as u8, (hex >> 8) as u8, hex as u8, 255]
}
const STATE_COLORS: [Color; 3] = [rgb(0xf97316), rgb(0x3b82f6), rgb(0x22c55e)];
#[derive(Clone, Debug, PartialEq)]
enum Kind {
    State(usize),
    Between,
    Closest,
    Fixed,
    Floating,
    Snap(f64),
    Active,
}

#[derive(Clone, Copy, PartialEq)]
enum SlotKind {
    State,
    Spec,
}

impl Kind {
    fn label(&self) -> &'static str {
        match self {
            Self::State(i) => ["A", "B", "C"][*i],
            Self::Between => "between",
            Self::Closest => "closest",
            Self::Fixed => "fixed",
            Self::Floating => "withFloating",
            Self::Snap(_) => "withSnapRadius",
            Self::Active => "active spec",
        }
    }

    fn output(&self) -> SlotKind {
        if matches!(self, Self::State(_)) {
            SlotKind::State
        } else {
            SlotKind::Spec
        }
    }

    fn slot(&self) -> Option<SlotKind> {
        match self {
            Self::State(_) => None,
            Self::Between | Self::Fixed => Some(SlotKind::State),
            _ => Some(SlotKind::Spec),
        }
    }

    fn multiple(&self) -> bool {
        matches!(self, Self::Between | Self::Closest)
    }

    fn height(&self) -> f64 {
        if matches!(self, Self::Snap(_)) {
            66.
        } else {
            44.
        }
    }
    fn style(&self) -> (Color, Color, Color, f64) {
        let (bg, stroke, text, fs) = match self {
            Self::State(i) => return (STATE_COLORS[*i], WHITE, WHITE, 11.),
            Self::Between => (0xede9fe, 0xc4b5fd, 0x7c3aed, 11.),
            Self::Closest => (0xfef3c7, 0xfcd34d, 0xb45309, 11.),
            Self::Fixed => (0xfce7f3, 0xf9a8d4, 0xbe185d, 11.),
            Self::Floating => (0xccfbf1, 0x5eead4, 0x0f766e, 10.),
            Self::Snap(_) => (0xe0f2fe, 0x93c5fd, 0x2563eb, 10.),
            Self::Active => (0xf1f5f9, 0x94a3b8, 0x475569, 10.),
        };
        (rgb(bg), rgb(stroke), rgb(text), fs)
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Node {
    kind: Kind,
    children: Vec<u64>,
    position: Point,
}

#[derive(Clone, Debug, PartialEq)]
struct State {
    nodes: BTreeMap<u64, Node>,
    next: u64,
    preview: usize,
}

impl State {
    fn add(&mut self, kind: Kind, position: Point) -> u64 {
        let id = self.next;
        self.next += 1;
        self.nodes.insert(
            id,
            Node {
                kind,
                children: Vec::new(),
                position,
            },
        );
        id
    }

    fn object_id(&self, id: u64) -> String {
        format!("node-{id}")
    }

    fn parent(&self, id: u64) -> Option<u64> {
        self.nodes
            .iter()
            .find_map(|(parent, node)| node.children.contains(&id).then_some(*parent))
    }

    fn descendants(&self, id: u64) -> BTreeSet<u64> {
        let mut found = BTreeSet::new();
        let mut pending = vec![id];
        while let Some(id) = pending.pop() {
            if found.insert(id)
                && let Some(node) = self.nodes.get(&id)
            {
                pending.extend(&node.children);
            }
        }
        found
    }

    fn detach(&mut self, id: u64) {
        if let Some(parent) = self.parent(id) {
            // Preserve the source's stored-parent coordinates, even when the
            // rendered snapped position differs from those stored coordinates.
            let position = self.nodes[&parent].position;
            self.nodes
                .get_mut(&parent)
                .unwrap()
                .children
                .retain(|child| *child != id);
            self.nodes.get_mut(&id).unwrap().position = position;
        }
    }
}

struct Designer;

pub fn entry() -> Entry {
    Entry::new("drag-spec-designer", "drag-spec-designer", "", || {
        demo(Designer)
    })
}

fn tools() -> Vec<(Kind, Point, f64)> {
    vec![
        (Kind::State(0), Point::new(20., 33.), 20.),
        (Kind::State(1), Point::new(52., 33.), 20.),
        (Kind::State(2), Point::new(84., 33.), 20.),
        (Kind::Between, Point::new(114., 21.), 56.),
        (Kind::Closest, Point::new(182., 21.), 56.),
        (Kind::Fixed, Point::new(258., 21.), 56.),
        (Kind::Floating, Point::new(326., 21.), 72.),
        (Kind::Snap(15.), Point::new(410., 21.), 84.),
    ]
}
fn palette_id(kind: &Kind) -> String {
    format!(
        "tool-{}",
        if matches!(kind, Kind::Snap(_)) {
            "wsr"
        } else {
            kind.label()
        }
    )
}

#[derive(Clone, Copy)]
struct Layout {
    origin: Point,
    width: f64,
}

fn node_width(state: &State, id: u64, widths: &mut BTreeMap<u64, f64>) -> f64 {
    if let Some(width) = widths.get(&id) {
        return *width;
    }
    let node = &state.nodes[&id];
    let children: Vec<_> = node
        .children
        .iter()
        .map(|child| node_width(state, *child, widths))
        .collect();
    let width = match node.kind {
        Kind::State(_) => 28.,
        Kind::Between => 28. + 38. * (children.len() + 1) as f64,
        Kind::Closest => {
            28. + children.iter().sum::<f64>()
                + children.len() as f64 * 8.
                + if children.is_empty() { 60. } else { 32. }
        }
        Kind::Fixed => 80.,
        Kind::Floating => (children.first().copied().unwrap_or(48.) + 24.).max(100.),
        Kind::Snap(_) => (children.first().copied().unwrap_or(48.) + 24.).max(160.),
        Kind::Active => (children.first().copied().unwrap_or(48.) + 24.).max(120.),
    };
    widths.insert(id, width);
    width
}

fn child_positions(node: &Node, width: f64, widths: &BTreeMap<u64, f64>) -> Vec<Point> {
    let height = node.kind.height();
    match node.kind {
        Kind::Between => node
            .children
            .iter()
            .enumerate()
            .map(|(i, _)| Point::new(33. + i as f64 * 38., height))
            .collect(),
        Kind::Fixed => node
            .children
            .iter()
            .map(|_| Point::new(width / 2., height))
            .collect(),
        Kind::Closest => {
            let mut x = 14.;
            node.children
                .iter()
                .map(|child| {
                    let point = Point::new(x, height - 10.);
                    x += widths[child] + 8.;
                    point
                })
                .collect()
        }
        _ => node
            .children
            .iter()
            .map(|child| Point::new((width - widths[child]) / 2., height - 10.))
            .collect(),
    }
}

fn place(
    state: &State,
    id: u64,
    origin: Point,
    widths: &BTreeMap<u64, f64>,
    result: &mut BTreeMap<u64, Layout>,
) {
    result.insert(
        id,
        Layout {
            origin,
            width: widths[&id],
        },
    );
    let node = &state.nodes[&id];
    for (child, offset) in node
        .children
        .iter()
        .zip(child_positions(node, widths[&id], widths))
    {
        place(state, *child, origin + offset.to_vec2(), widths, result);
    }
}

fn layout(state: &State) -> BTreeMap<u64, Layout> {
    let mut widths = BTreeMap::new();
    let mut result = BTreeMap::new();
    let children: BTreeSet<_> = state
        .nodes
        .values()
        .flat_map(|n| &n.children)
        .copied()
        .collect();
    for (id, node) in &state.nodes {
        if !children.contains(id) {
            node_width(state, *id, &mut widths);
            place(state, *id, node.position, &widths, &mut result);
        }
    }
    result
}

fn diamond(radius: f64) -> BezPath {
    let mut p = BezPath::new();
    p.move_to((0., -radius));
    p.line_to((radius, 0.));
    p.line_to((0., radius));
    p.line_to((-radius, 0.));
    p.close_path();
    p
}
fn state_shape(id: String, position: Point, index: usize, palette: bool) -> Object {
    let path = diamond(if palette { 10. } else { 14. });
    Object::new(
        id,
        Affine::translate(position.to_vec2()),
        Some(Hit::Path(path.clone(), 0.)),
        vec![
            Shape::Path {
                path,
                fill: Some(STATE_COLORS[index]),
                stroke: None,
            },
            Shape::Text {
                origin: Point::ORIGIN,
                text: ["A", "B", "C"][index].into(),
                size: if palette { 8. } else { 11. },
                color: WHITE,
            },
        ],
    )
    .text_anchor(TextAnchor::Middle)
    .text_baseline(TextBaseline::Central)
    .font_weight(700)
    .line_join(LineJoin::Round)
}
fn arc(sweep: u8, x: f64, y: f64, r: f64) -> String {
    format!(" A {r},{r} 0 0 {sweep} {x},{y}")
}
fn rect_notch(cx: f64, hw: f64, h: f64) -> String {
    format!(
        " H {} V {}{} H {}{} V {h}",
        cx + hw,
        h - 10. + 8.,
        arc(0, cx + hw - 8., h - 10., 8.),
        cx - hw + 8.,
        arc(0, cx - hw, h - 10. + 8., 8.)
    )
}
fn block_path(node: &Node, width: f64, placed: &BTreeMap<u64, Layout>) -> BezPath {
    let h = node.kind.height();
    let r = 8.;
    let mut p = if matches!(node.kind, Kind::Active) {
        format!("M 0,0 H {width} V {}{}", h - r, arc(1, width - r, h, r))
    } else {
        format!(
            "M {r},0 H {}{} V {}{}",
            width - r,
            arc(1, width, r, r),
            h - r,
            arc(1, width - r, h, r)
        )
    };
    match node.kind {
        Kind::Between => {
            // Six path slots, including collapsed unused ones, match upstream.
            // Width and children remain unbounded; do not silently add notches.
            for i in (0..6).rev() {
                if i < node.children.len() + 1 {
                    let cx = 33. + i as f64 * 38.;
                    p += &format!(" H {} L {cx},{} L {},{h}", cx + 14., h - 14., cx - 14.);
                } else {
                    p += &format!(" H {} L {},{h} L {},{h}", width - r, width - r, width - r);
                }
            }
        }
        Kind::Fixed => {
            let cx = width / 2.;
            p += &format!(" H {} L {cx},{} L {},{h}", cx + 14., h - 14., cx - 14.);
        }
        Kind::Closest => {
            let mut centers = vec![];
            let mut x = 14.;
            for child in &node.children {
                let hw = placed[child].width / 2.;
                centers.push((x + hw, hw));
                x += hw * 2. + 8.;
            }
            let hw = if node.children.is_empty() { 30. } else { 16. };
            centers.push((x + hw, hw));
            for i in (0..6).rev() {
                if let Some((cx, hw)) = centers.get(i) {
                    p += &rect_notch(*cx, *hw, h);
                } else {
                    let x = width - r;
                    p += &format!(
                        " H {x} V {h}{} H {x}{} V {h}",
                        arc(0, x, h, 0.01),
                        arc(0, x, h, 0.01)
                    );
                }
            }
        }
        _ => {
            let hw = node
                .children
                .first()
                .map(|id| placed[id].width / 2.)
                .unwrap_or(24.);
            p += &rect_notch(width / 2., hw, h);
        }
    }
    p += &format!(" H {r}{}", arc(1, 0., h - r, r));
    if matches!(node.kind, Kind::Active) {
        p += " Z";
    } else {
        p += &format!(" V {r}{} Z", arc(1, r, 0., r));
    }
    BezPath::from_svg(&p).expect("source block SVG path")
}
fn render_node(state: &State, id: u64, placed: &BTreeMap<u64, Layout>, scene: &mut Scene) {
    let node = &state.nodes[&id];
    let Layout { origin, width } = placed[&id];
    let object_id = state.object_id(id);
    let mut object = if let Kind::State(index) = node.kind {
        state_shape(object_id.clone(), origin, index, false)
    } else {
        let (fill, stroke, text, fs) = node.kind.style();
        let path = block_path(node, width, placed);
        Object::new(
            object_id.clone(),
            Affine::translate(origin.to_vec2()),
            (id != ROOT).then(|| Hit::Path(path.clone(), 1.5)),
            vec![
                Shape::Path {
                    path,
                    fill: Some(fill),
                    stroke: Some((stroke, 1.5)),
                },
                Shape::Text {
                    origin: Point::new(width / 2., 13.),
                    text: node.kind.label().into(),
                    size: fs,
                    color: text,
                },
            ],
        )
        .text_anchor(TextAnchor::Middle)
        .text_baseline(TextBaseline::Central)
        .font_weight(600)
        .line_join(LineJoin::Round)
    };
    object.parent = state.parent(id).map(|id| state.object_id(id));
    scene.push(object);
    if let Kind::Snap(radius) = node.kind {
        let mut track = Object::line(
            format!("track-{id}"),
            origin + Vec2::new(12., 46.),
            origin + Vec2::new(width - 12., 46.),
            rgb(0xdddddd),
            3.,
        )
        .line_cap(LineCap::Round);
        track.parent = Some(object_id.clone());
        scene.push(track);
        let x = 12. + radius / 30. * (width - 24.);
        let mut knob = Object::circle(
            format!("radius-{id}"),
            origin + Vec2::new(x, 46.),
            7.,
            rgb(0x2563eb),
        );
        if let Shape::Circle { stroke, .. } = &mut knob.shapes[0] {
            *stroke = Some((WHITE, 2.));
        }
        knob.parent = Some(object_id.clone());
        scene.push(knob);
        let mut label = Object::text(
            format!("radius-label-{id}"),
            origin + Vec2::new(x, 34.),
            format!("{:.0}", (radius + 0.5).floor()),
            9.,
            rgb(0x2563eb),
        )
        .text_anchor(TextAnchor::Middle)
        .text_baseline(TextBaseline::Alphabetic)
        .font_weight(500);
        label.parent = Some(object_id);
        scene.push(label);
    }
    for child in &node.children {
        render_node(state, *child, placed, scene);
    }
}

/// Candidate states are typed trees. Descendants are excluded from attachment
/// and splice targets, so moving a whole subtree cannot introduce a cycle.
fn attachment_states(base: &State, id: u64) -> Vec<(State, bool)> {
    let subtree = base.descendants(id);
    let node = &base.nodes[&id];
    let mut targets = Vec::new();
    let mut next_adapter = base.next;
    for (parent, target) in &base.nodes {
        if subtree.contains(parent) {
            continue;
        }
        let Some(slot) = target.kind.slot() else {
            continue;
        };
        let slots = if target.kind.multiple() {
            target.children.len() + 1
        } else {
            usize::from(target.children.is_empty())
        };
        for index in 0..slots {
            if slot == node.kind.output()
                || node.kind.output() == SlotKind::State && slot == SlotKind::Spec
            {
                let mut candidate = base.clone();
                let child = if slot == node.kind.output() {
                    id
                } else {
                    candidate.next = next_adapter;
                    let adapter = candidate.add(Kind::Fixed, target.position);
                    next_adapter = candidate.next;
                    candidate.nodes.get_mut(&adapter).unwrap().children.push(id);
                    adapter
                };
                candidate
                    .nodes
                    .get_mut(parent)
                    .unwrap()
                    .children
                    .insert(index, child);
                targets.push((candidate, true));
            }
        }
    }
    // Source puts all direct attachment branches before chain insertion branches.
    for (parent, target) in &base.nodes {
        if subtree.contains(parent) {
            continue;
        }
        let Some(slot) = target.kind.slot() else {
            continue;
        };
        // Empty unary modifiers can be spliced into a compatible occupied edge.
        if node.kind.output() == SlotKind::Spec
            && !node.kind.multiple()
            && node.kind.slot().is_some()
            && node.children.is_empty()
            && slot == SlotKind::Spec
        {
            for (index, child) in target.children.iter().enumerate() {
                if subtree.contains(child)
                    || node.kind.slot() != Some(base.nodes[child].kind.output())
                {
                    continue;
                }
                let mut candidate = base.clone();
                candidate.nodes.get_mut(parent).unwrap().children[index] = id;
                candidate.nodes.get_mut(&id).unwrap().children.push(*child);
                targets.push((candidate, false));
            }
        }
    }
    for (candidate, _) in &mut targets {
        candidate.next = next_adapter;
    }
    targets
}

fn node_drag(mut base: State, id: u64) -> DragSpec<'static, State> {
    base.detach(id);
    let attachments = attachment_states(&base, id);
    base.next = attachments
        .iter()
        .map(|(s, _)| s.next)
        .max()
        .unwrap_or(base.next);
    let mut targets: Vec<_> = attachments
        .into_iter()
        .map(|(state, floating)| {
            let spec = fixed(state);
            if floating { spec.with_floating() } else { spec }
        })
        .collect();
    let mut deleted = base.clone();
    for child in base.descendants(id) {
        deleted.nodes.remove(&child);
    }
    targets.push(drop_target(deleted, TRASH));
    let position = base.nodes[&id].position;
    let free = vary([position.x, position.y], move |[x, y]| {
        let mut candidate = base.clone();
        candidate.nodes.get_mut(&id).unwrap().position = Point::new(x, y);
        candidate
    });
    closest(targets).when_far(free, 40.)
}

fn compile(state: &State, id: u64) -> Option<DragSpec<'static, State>> {
    let node = state.nodes.get(&id)?;
    match node.kind {
        Kind::State(_) => None,
        Kind::Between => {
            let candidates: Vec<_> = node
                .children
                .iter()
                .filter_map(|id| match state.nodes[id].kind {
                    Kind::State(index) => Some(index),
                    _ => None,
                })
                .map(|index| {
                    let mut s = state.clone();
                    s.preview = index;
                    s
                })
                .collect();
            (!candidates.is_empty()).then(|| between(candidates))
        }
        Kind::Closest => {
            let children: Vec<_> = node
                .children
                .iter()
                .filter_map(|child| compile(state, *child))
                .collect();
            (!children.is_empty()).then(|| closest(children))
        }
        Kind::Fixed => {
            let Kind::State(index) = state.nodes.get(node.children.first()?)?.kind else {
                return None;
            };
            let mut candidate = state.clone();
            candidate.preview = index;
            Some(fixed(candidate))
        }
        Kind::Floating => Some(compile(state, *node.children.first()?)?.with_floating()),
        Kind::Snap(radius) => {
            Some(compile(state, *node.children.first()?)?.with_snap_radius(radius))
        }
        Kind::Active => compile(state, *node.children.first()?),
    }
}

impl Example for Designer {
    type State = State;

    fn initial(&self) -> State {
        State {
            nodes: BTreeMap::from([(
                ROOT,
                Node {
                    kind: Kind::Active,
                    children: Vec::new(),
                    position: Point::new(0., 52.),
                },
            )]),
            next: 1,
            preview: 0,
        }
    }

    fn scene(&self, state: &State) -> Scene {
        self.scene_dragging(state, None)
    }
    fn scene_dragging(&self, state: &State, dragged: Option<&str>) -> Scene {
        let mut scene = Scene::new(600., 400.);
        let mut toolbar = Object::rect(
            "toolbar-bg",
            Rect::new(0., 0., 600., 52.),
            0.,
            rgb(0xf9fafb),
        )
        .decorative();
        if let Shape::Rect { stroke, .. } = &mut toolbar.shapes[0] {
            *stroke = Some((rgb(0xe5e7eb), 1.));
        }
        scene.push(toolbar);
        scene.push(Object::line(
            "toolbar-edge",
            (0., 52.),
            (600., 52.),
            rgb(0xe5e7eb),
            1.,
        ));
        for (x, label) in [(10., "STATES"), (114., "COMBINATORS"), (258., "MODIFIERS")] {
            scene.push(
                Object::text(
                    format!("heading-{label}"),
                    (x, 14.),
                    label,
                    9.,
                    rgb(0xaaaaaa),
                )
                .text_baseline(TextBaseline::Alphabetic)
                .font_weight(500),
            );
        }
        for (kind, position, width) in tools() {
            let id = palette_id(&kind);
            if let Kind::State(index) = kind {
                scene.push(state_shape(id, position, index, true));
            } else {
                let (fill, stroke, text, _) = kind.style();
                let fs = if matches!(kind, Kind::Floating | Kind::Snap(_)) {
                    8.
                } else {
                    9.
                };
                let mut object =
                    Object::rect(id, Rect::from_origin_size(position, (width, 24.)), 6., fill)
                        .label(kind.label(), (width / 2., 12.), fs, text)
                        .text_anchor(TextAnchor::Middle)
                        .text_baseline(TextBaseline::Central)
                        .font_weight(500);
                if let Shape::Rect { stroke: s, .. } = &mut object.shapes[0] {
                    *s = Some((stroke, 1.5));
                }
                scene.push(object);
            }
        }
        let mut trash = Object::rect("trash-bin", TRASH, 6., rgb(0xfee2e2))
            .decorative()
            .dash(vec![4., 3.]);
        if let Shape::Rect { stroke, .. } = &mut trash.shapes[0] {
            *stroke = Some((rgb(0xfca5a5), 1.5));
        }
        scene.push(trash);
        scene.push(
            Object::text("trash-icon", TRASH.center(), "🗑", 16., rgb(0))
                .text_anchor(TextAnchor::Middle)
                .text_baseline(TextBaseline::Central),
        );
        let placed = layout(state);
        render_node(state, ROOT, &placed, &mut scene);
        let mut panel =
            Object::rect("preview-panel", Rect::new(430., 62., 590., 202.), 8., WHITE).decorative();
        if let Shape::Rect { stroke, .. } = &mut panel.shapes[0] {
            *stroke = Some((rgb(0xe5e7eb), 1.5));
        }
        scene.push(panel);
        scene.push(
            Object::text("preview-title", (510., 76.), "PREVIEW", 9., rgb(0xaaaaaa))
                .text_anchor(TextAnchor::Middle)
                .text_baseline(TextBaseline::Alphabetic)
                .font_weight(500),
        );
        for (index, point) in DOTS.into_iter().enumerate() {
            let mut color = STATE_COLORS[index];
            color[3] = 64;
            scene.push(
                Object::circle(format!("preview-target-{index}"), point, 12., color).decorative(),
            );
            scene.push(
                Object::text(
                    format!("preview-label-{index}"),
                    point,
                    ["A", "B", "C"][index],
                    9.,
                    STATE_COLORS[index],
                )
                .text_anchor(TextAnchor::Middle)
                .text_baseline(TextBaseline::Central)
                .font_weight(700),
            );
        }
        let mut preview = Object::circle(
            "preview-dot",
            DOTS[state.preview],
            12.,
            STATE_COLORS[state.preview],
        );
        if let Shape::Circle { stroke, .. } = &mut preview.shapes[0] {
            *stroke = Some((WHITE, 2.));
        }
        if compile(state, ROOT).is_none() {
            preview.hit = None;
        }
        scene.push(preview);
        for id in state
            .nodes
            .keys()
            .filter(|id| **id != ROOT && state.parent(**id).is_none())
        {
            render_node(state, *id, &placed, &mut scene);
        }
        // Upstream extracts snapped nodes and slider knobs into higher layers.
        let dragged_node = dragged.and_then(|id| id.strip_prefix("node-")?.parse::<u64>().ok());
        let raised = dragged_node
            .map(|id| state.descendants(id))
            .unwrap_or_default();
        scene.objects.sort_by_key(|o| {
            if o.id == "toolbar-bg" {
                -10
            } else if let Some(id) =
                o.id.strip_prefix("radius-")
                    .and_then(|id| id.parse::<u64>().ok())
            {
                if dragged_node == Some(id) { 11 } else { 2 }
            } else {
                let node =
                    o.id.strip_prefix("node-")
                        .and_then(|id| id.parse::<u64>().ok())
                        .or_else(|| {
                            o.parent
                                .as_ref()
                                .and_then(|p| p.strip_prefix("node-")?.parse::<u64>().ok())
                        });
                if node.is_some_and(|id| raised.contains(&id)) {
                    10
                } else if node.is_some_and(|id| state.parent(id).is_some()) {
                    1
                } else {
                    0
                }
            }
        });
        for object in &mut scene.objects {
            if object.hit.is_some() {
                object.cursor = if object.id.starts_with("radius-") {
                    Cursor::ResizeHorizontal
                } else {
                    Cursor::Grab
                };
            }
        }
        scene
    }
    fn begin_drag(&self, state: &State, id: &str, _: Modifiers) -> Option<(State, String)> {
        let (kind, _, _) = tools()
            .into_iter()
            .find(|(kind, _, _)| palette_id(kind) == id)?;
        let mut next = state.clone();
        let node = next.add(kind, Point::ORIGIN);
        Some((next, format!("node-{node}")))
    }

    fn locate(&self, state: &State, id: &str) -> Option<Affine> {
        if id == "preview-dot" {
            return Some(Affine::translate(DOTS[state.preview].to_vec2()));
        }
        let node_id = id
            .strip_prefix("node-")
            .and_then(|id| id.parse::<u64>().ok());
        if let Some(node) = node_id {
            return layout(state)
                .get(&node)
                .map(|node| Affine::translate(node.origin.to_vec2()));
        }
        if let Some(node) = id
            .strip_prefix("radius-")
            .and_then(|id| id.parse::<u64>().ok())
        {
            let Kind::Snap(radius) = state.nodes.get(&node)?.kind else {
                return None;
            };
            let placed = layout(state);
            let node = placed.get(&node)?;
            return Some(Affine::translate(
                (node.origin + Vec2::new(12. + radius / 30. * (node.width - 24.), 46.)).to_vec2(),
            ));
        }
        tools()
            .into_iter()
            .find(|(kind, _, _)| palette_id(kind) == id)
            .map(|(_, p, _)| Affine::translate(p.to_vec2()))
    }

    fn drag(&self, state: &State, id: &str, _: Modifiers) -> Option<DragSpec<'static, State>> {
        if id == "preview-dot" {
            return compile(state, ROOT);
        }
        if let Some(node) = id
            .strip_prefix("node-")
            .and_then(|id| id.parse::<u64>().ok())
        {
            return (node != ROOT && state.nodes.contains_key(&node))
                .then(|| node_drag(state.clone(), node));
        }
        if let Some(node) = id
            .strip_prefix("radius-")
            .and_then(|id| id.parse::<u64>().ok())
        {
            let Kind::Snap(radius) = state.nodes.get(&node)?.kind else {
                return None;
            };
            let base = state.clone();
            return Some(vary_constrained(
                [radius],
                move |[radius]| {
                    let mut candidate = base.clone();
                    candidate.nodes.get_mut(&node).unwrap().kind = Kind::Snap(radius);
                    candidate
                },
                move |state: &State| {
                    let Kind::Snap(radius) = state.nodes[&node].kind else {
                        unreachable!()
                    };
                    [-radius, radius - 30.]
                },
            ));
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dragology::DragSession;

    fn triangle() -> State {
        let mut state = Designer.initial();
        let node = state.add(Kind::Between, Point::new(60., 150.));
        state.nodes.get_mut(&ROOT).unwrap().children.push(node);
        for index in 0..3 {
            let child = state.add(Kind::State(index), Point::ORIGIN);
            state.nodes.get_mut(&node).unwrap().children.push(child);
        }
        state
    }

    #[test]
    fn palette_creates_permanent_block_immediately_and_can_build_active_spec() {
        let mut d = demo(Designer);
        d.set_animated(false);
        let m = Modifiers::default();
        d.press(Point::new(139., 36.), m, 1);
        assert!(d.scene().object("node-1").is_some());
        assert!(d.scene().object("tool-between").is_some());
        d.move_to(Point::new(52., 101.), m);
        d.advance(std::time::Duration::ZERO);
        d.release(Point::new(52., 101.), m);
        assert_eq!(
            d.scene().object("node-1").unwrap().transform * Point::ORIGIN,
            Point::new(27., 86.)
        );
        d.press(Point::new(20., 33.), m, 1);
        d.move_to(Point::new(60., 130.), m);
        d.advance(std::time::Duration::ZERO);
        d.release(Point::new(60., 130.), m);
        assert!(d.scene().object("preview-dot").unwrap().hit.is_some());
        assert_eq!(
            d.scene().object("node-2").unwrap().transform * Point::ORIGIN,
            Point::new(45., 130.)
        );
        assert!(d.status().is_empty(), "{}", d.status());
    }
    #[test]
    fn repeated_state_leaves_retain_source_coincident_target_error() {
        let mut s = triangle();
        let parent = s.nodes[&ROOT].children[0];
        let duplicate = s.add(Kind::State(0), Point::ORIGIN);
        s.nodes.get_mut(&parent).unwrap().children.push(duplicate);
        let spec = compile(&s, ROOT).unwrap();
        let mut session =
            DragSession::begin(s, DOTS[0], |s| Designer.locate(s, "preview-dot"), spec).unwrap();
        assert!(matches!(
            session.frame(DOTS[1]),
            Err(dragology::DragError::CoincidentTargets { .. })
        ));
    }
    #[test]
    fn compiled_triangle_uses_real_barycentric_preview_and_commits_a_state() {
        let state = triangle();
        let mut session = DragSession::begin(
            state.clone(),
            DOTS[0],
            |s| Designer.locate(s, "preview-dot"),
            compile(&state, ROOT).unwrap(),
        )
        .unwrap();
        let middle = Point::new(
            (DOTS[0].x + DOTS[1].x + DOTS[2].x) / 3.,
            (DOTS[0].y + DOTS[1].y + DOTS[2].y) / 3.,
        );
        let frame = session.frame(middle).unwrap();
        assert!(frame.grab_position.unwrap().distance(middle) < 1e-6);
        assert!(matches!(&*frame.preview, dragology::Preview::Blend(parts) if parts.len() == 3));
        assert_eq!(session.finish(DOTS[2]).unwrap().preview, 2);
    }

    #[test]
    fn floating_snap_composition_and_radius_slider_use_core_operators() {
        let mut state = triangle();
        let between = state.nodes[&ROOT].children[0];
        let floating = state.add(Kind::Floating, Point::ORIGIN);
        state
            .nodes
            .get_mut(&floating)
            .unwrap()
            .children
            .push(between);
        let snap = state.add(Kind::Snap(15.), Point::ORIGIN);
        state.nodes.get_mut(&snap).unwrap().children.push(floating);
        state.nodes.get_mut(&ROOT).unwrap().children = vec![snap];
        let mut session = DragSession::begin(
            state.clone(),
            DOTS[0],
            |s| Designer.locate(s, "preview-dot"),
            compile(&state, ROOT).unwrap(),
        )
        .unwrap();
        let near = session.frame(DOTS[0] + Vec2::new(8., -4.)).unwrap();
        assert!(matches!(&*near.preview, dragology::Preview::State(_)));
        assert_eq!(near.grab_position, Some(DOTS[0]));
        let far = session.frame(DOTS[0] + Vec2::new(70., -40.)).unwrap();
        assert!(matches!(&*far.preview, dragology::Preview::Floating { .. }));

        let key = format!("radius-{snap}");
        let press = Designer.locate(&state, &key).unwrap() * Point::ORIGIN;
        let spec = Designer.drag(&state, &key, Modifiers::default()).unwrap();
        let dragged_key = key.clone();
        let slider = DragSession::begin(
            state,
            press,
            move |s| Designer.locate(s, &dragged_key),
            spec,
        )
        .unwrap();
        let next = slider.finish(press + Vec2::new(500., 0.)).unwrap();
        let Kind::Snap(radius) = next.nodes[&snap].kind else {
            panic!("expected snap block")
        };
        assert!(radius.is_finite());
        let actual = Designer.locate(&next, &key).unwrap() * Point::ORIGIN;
        let mut boundary = next.clone();
        boundary.nodes.get_mut(&snap).unwrap().kind = Kind::Snap(30.);
        let expected = Designer.locate(&boundary, &key).unwrap() * Point::ORIGIN;
        // Upstream uses the numerical solver's final iterate, not an exact clamp.
        assert!(actual.distance(expected) < 0.01, "radius {radius}");
    }

    #[test]
    fn attaching_state_to_spec_makes_fixed_adapter_and_excludes_cycles() {
        let mut state = Designer.initial();
        let leaf = state.add(Kind::State(1), Point::new(200., 200.));
        let attached = attachment_states(&state, leaf)
            .into_iter()
            .map(|(state, _)| state)
            .find(|s| !s.nodes[&ROOT].children.is_empty())
            .unwrap();
        let adapter = attached.nodes[&ROOT].children[0];
        assert_eq!(attached.nodes[&adapter].kind, Kind::Fixed);
        assert_eq!(attached.nodes[&adapter].children, [leaf]);
        assert!(compile(&attached, ROOT).is_some());
        let mut detached = attached;
        detached.detach(adapter);
        for (candidate, _) in attachment_states(&detached, adapter) {
            assert!(!candidate.descendants(adapter).contains(&ROOT));
            assert_eq!(layout(&candidate).len(), candidate.nodes.len());
        }
        let mut state = triangle();
        let outer = state.add(Kind::Floating, Point::new(300., 260.));
        let inner = state.add(Kind::Floating, Point::ORIGIN);
        state.nodes.get_mut(&outer).unwrap().children.push(inner);
        for (candidate, _) in attachment_states(&state, outer) {
            assert_ne!(candidate.parent(outer), Some(inner));
            assert_eq!(layout(&candidate).len(), candidate.nodes.len());
        }
    }

    #[test]
    fn empty_modifier_splices_existing_edge_and_detach_uses_stored_parent_position() {
        let mut state = triangle();
        let between = state.nodes[&ROOT].children[0];
        let modifier = state.add(Kind::Floating, Point::new(300., 300.));
        let mut spliced = attachment_states(&state, modifier)
            .into_iter()
            .map(|(state, _)| state)
            .find(|s| s.nodes[&ROOT].children == [modifier])
            .unwrap();
        assert_eq!(spliced.nodes[&modifier].children, [between]);
        let before = spliced.nodes[&modifier].position;
        spliced.detach(between);
        assert_eq!(layout(&spliced)[&between].origin, before);
        assert!(spliced.nodes[&modifier].children.is_empty());
    }

    #[test]
    fn palette_clone_follows_pointer_and_trash_removes_whole_subtree() {
        let state = Designer.initial();
        let id = "tool-between";
        let press = Point::new(139., 36.);
        let anchor = Designer.locate(&state, id).unwrap().inverse() * press;
        let (state, follow) = Designer
            .begin_drag(&state, id, Modifiers::default())
            .unwrap();
        let spec = Designer
            .drag(&state, &follow, Modifiers::default())
            .unwrap();
        let followed = follow.clone();
        let session = DragSession::begin(
            state.clone(),
            anchor,
            move |s| Designer.locate(s, &followed),
            spec,
        )
        .unwrap();
        let free = session.finish((300., 340.)).unwrap();
        assert_eq!(free.nodes.len(), 2);
        let root = free
            .nodes
            .iter()
            .find_map(|(id, n)| matches!(n.kind, Kind::Between).then_some(*id))
            .unwrap();
        assert!(layout(&free)[&root].origin.distance(Point::new(275., 325.)) < 0.02);
        let state = triangle();
        let root = state.nodes[&ROOT].children[0];
        let key = state.object_id(root);
        let press = layout(&state)[&root].origin + Vec2::new(20., 15.);
        let session = DragSession::begin(
            state.clone(),
            press,
            move |s| Designer.locate(s, &key),
            node_drag(state.clone(), root),
        )
        .unwrap();
        let deleted = session.finish(TRASH.center()).unwrap();
        assert_eq!(deleted.nodes.len(), 1);
        assert!(deleted.nodes[&ROOT].children.is_empty());
    }
}
