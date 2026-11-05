use super::*;
use dragology::{between, closest, vary_constrained};

#[derive(Clone, PartialEq)]
enum Kind {
    Leaf {
        w: f64,
        h: f64,
        color: Color,
    },
    Box {
        direction: usize,
        cols: usize,
        gap: f64,
        pad: f64,
        children: Vec<Node>,
    },
}
#[derive(Clone, PartialEq)]
struct Node {
    id: String,
    kind: Kind,
}
#[derive(Clone, PartialEq)]
pub struct State {
    root: Node,
}
pub struct Layout;
#[derive(Clone)]
struct Geometry {
    w: f64,
    h: f64,
    slots: Vec<(f64, f64, Geometry)>,
    cols: Vec<f64>,
    rows: Vec<f64>,
}
fn geometry(n: &Node) -> Geometry {
    match &n.kind {
        Kind::Leaf { w, h, .. } => Geometry {
            w: *w,
            h: *h,
            slots: vec![],
            cols: vec![],
            rows: vec![],
        },
        Kind::Box {
            direction,
            cols,
            gap,
            pad,
            children,
        } => {
            let layouts: Vec<_> = children.iter().map(geometry).collect();
            let count = layouts.len();
            let columns = match direction {
                0 => count.max(1),
                1 => 1,
                _ => (*cols).clamp(1, count.max(1)),
            };
            let mut cw = vec![0f64; columns];
            let mut rh = vec![0f64; count.div_ceil(columns).max(1)];
            for (i, l) in layouts.iter().enumerate() {
                cw[i % columns] = cw[i % columns].max(l.w);
                rh[i / columns] = rh[i / columns].max(l.h);
            }
            let slots = layouts
                .into_iter()
                .enumerate()
                .map(|(i, l)| {
                    let c = i % columns;
                    let r = i / columns;
                    (
                        pad + cw[..c].iter().sum::<f64>() + gap * c as f64,
                        pad + rh[..r].iter().sum::<f64>() + gap * r as f64,
                        l,
                    )
                })
                .collect();
            Geometry {
                w: (pad * 2. + cw.iter().sum::<f64>() + gap * columns.saturating_sub(1) as f64)
                    .max(44.),
                h: (pad * 2. + rh.iter().sum::<f64>() + gap * rh.len().saturating_sub(1) as f64)
                    .max(44.),
                slots,
                cols: cw,
                rows: rh,
            }
        }
    }
}
fn node<'a>(n: &'a Node, id: &str) -> Option<&'a Node> {
    if n.id == id {
        return Some(n);
    }
    if let Kind::Box { children, .. } = &n.kind {
        for c in children {
            if let Some(v) = node(c, id) {
                return Some(v);
            }
        }
    }
    None
}
fn node_mut<'a>(n: &'a mut Node, id: &str) -> Option<&'a mut Node> {
    if n.id == id {
        return Some(n);
    }
    if let Kind::Box { children, .. } = &mut n.kind {
        for c in children {
            if let Some(v) = node_mut(c, id) {
                return Some(v);
            }
        }
    }
    None
}
fn remove(n: &mut Node, id: &str) -> Option<Node> {
    if let Kind::Box { children, .. } = &mut n.kind {
        if let Some(i) = children.iter().position(|c| c.id == id) {
            return Some(children.remove(i));
        }
        for c in children {
            if let Some(v) = remove(c, id) {
                return Some(v);
            }
        }
    }
    None
}
fn targets(n: &Node, out: &mut Vec<(String, usize)>) {
    if let Kind::Box { children, .. } = &n.kind {
        for i in 0..=children.len() {
            out.push((n.id.clone(), i));
        }
        for c in children {
            targets(c, out)
        }
    }
}
fn leaf(id: &str, w: f64, h: f64, color: Color) -> Node {
    Node {
        id: id.into(),
        kind: Kind::Leaf { w, h, color },
    }
}
fn container(id: &str, direction: usize, gap: f64, pad: f64, children: Vec<Node>) -> Node {
    Node {
        id: id.into(),
        kind: Kind::Box {
            direction,
            cols: 2,
            gap,
            pad,
            children,
        },
    }
}
fn html(n: &Node, depth: usize) -> String {
    let indent = "  ".repeat(depth);
    match &n.kind {
        Kind::Leaf { .. } => format!("{indent}<div class=\"leaf\" id=\"{}\"></div>", n.id),
        Kind::Box { children, .. } => {
            if children.is_empty() {
                format!("{indent}<div class=\"box\" id=\"{}\"></div>", n.id)
            } else {
                format!(
                    "{indent}<div class=\"box\" id=\"{}\">\n{}\n{indent}</div>",
                    n.id,
                    children
                        .iter()
                        .map(|c| html(c, depth + 1))
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            }
        }
    }
}
fn px(value: f64) -> String {
    if value.round() == 0. {
        "0".into()
    } else {
        format!("{}px", value.round())
    }
}
fn css_rule(selector: &str, declarations: Vec<(&str, String)>) -> String {
    format!(
        "{selector} {{\n{}\n}}",
        declarations
            .into_iter()
            .map(|(k, v)| format!("  {k}: {v};"))
            .collect::<Vec<_>>()
            .join("\n")
    )
}
fn css(root: &Node) -> String {
    let mut rules = vec![
        css_rule("*", vec![("box-sizing", "border-box".into())]),
        css_rule(
            ".box",
            vec![
                ("width", "max-content".into()),
                ("min-width", "44px".into()),
                ("min-height", "44px".into()),
                ("align-items", "start".into()),
                ("justify-items", "start".into()),
                ("border-radius", "6px".into()),
                ("background", "rgba(99, 102, 241, 0.08)".into()),
                ("outline", "1.5px dashed rgba(99, 102, 241, 0.5)".into()),
                ("outline-offset", "-1.5px".into()),
            ],
        ),
        css_rule(
            ".leaf",
            vec![
                ("border-radius", "4px".into()),
                ("outline", "1px solid rgba(0, 0, 0, 0.25)".into()),
                ("outline-offset", "-1px".into()),
            ],
        ),
    ];
    fn visit(n: &Node, rules: &mut Vec<String>) {
        match &n.kind {
            Kind::Leaf { w, h, color } => rules.push(css_rule(
                &format!("#{}", n.id),
                vec![
                    ("width", px(*w)),
                    ("height", px(*h)),
                    (
                        "background",
                        format!("#{:02x}{:02x}{:02x}", color[0], color[1], color[2]),
                    ),
                ],
            )),
            Kind::Box {
                direction,
                cols,
                gap,
                pad,
                children,
            } => {
                let mut declarations = match direction {
                    0 => vec![("display", "flex".into())],
                    1 => vec![
                        ("display", "flex".into()),
                        ("flex-direction", "column".into()),
                    ],
                    _ => vec![
                        ("display", "grid".into()),
                        (
                            "grid-template-columns",
                            format!(
                                "repeat({}, max-content)",
                                (*cols).min(children.len()).max(1)
                            ),
                        ),
                    ],
                };
                declarations.extend([("gap", px(*gap)), ("padding", px(*pad))]);
                rules.push(css_rule(&format!("#{}", n.id), declarations));
                for child in children {
                    visit(child, rules)
                }
            }
        }
    }
    visit(root, &mut rules);
    rules.join("\n\n")
}
fn render(
    n: &Node,
    g: &Geometry,
    x: f64,
    y: f64,
    parent: Option<&str>,
    scene: &mut Scene,
    preview: bool,
) {
    let root = parent.is_none();
    let (fill, stroke, radius) = match &n.kind {
        Kind::Leaf { color, .. } => (*color, ([0, 0, 0, 64], 1.), 4.),
        Kind::Box { .. } => (
            if root && !preview {
                hex_color("f8fafc")
            } else {
                [99, 102, 241, 20]
            },
            (
                if root && !preview {
                    hex_color("cbd5e1")
                } else {
                    [99, 102, 241, 128]
                },
                1.5,
            ),
            6.,
        ),
    };
    let mut o = Object::rect(&n.id, Rect::new(x, y, x + g.w, y + g.h), radius, fill);
    o.parent = parent.map(str::to_owned);
    if let Shape::Rect { stroke: s, .. } = &mut o.shapes[0] {
        *s = Some(stroke);
    }
    if matches!(n.kind, Kind::Box { .. }) && (!root || preview) {
        o = o.dash(vec![4., 3.]);
    }
    if preview {
        o.hit = None;
    }
    scene.push(o);
    let handle = |id: String, p: Point, path: BezPath, stroke: Color, width: f64| {
        Object::new(
            id,
            Affine::translate(p.to_vec2()),
            Some(Hit::Rect(Rect::new(-5., -5., 5., 5.))),
            vec![Shape::Path {
                path,
                fill: None,
                stroke: Some((stroke, width)),
            }],
        )
        .line_cap(LineCap::Round)
    };
    match &n.kind {
        Kind::Leaf { .. } => {
            if !preview {
                let mut path = BezPath::new();
                path.move_to((8., 2.));
                path.line_to((8., 8.));
                path.line_to((2., 8.));
                let mut o = Object::new(
                    format!("{}:resize", n.id),
                    Affine::translate((x + g.w - 10., y + g.h - 10.)),
                    Some(Hit::Rect(Rect::new(0., 0., 10., 10.))),
                    vec![Shape::Path {
                        path,
                        fill: None,
                        stroke: Some(([0, 0, 0, 115], 1.5)),
                    }],
                )
                .line_cap(LineCap::Round);
                o.parent = Some(n.id.clone());
                scene.push(o);
            }
        }
        Kind::Box {
            direction,
            pad,
            gap,
            children,
            ..
        } => {
            for (child, (dx, dy, geom)) in children.iter().zip(&g.slots) {
                render(child, geom, x + dx, y + dy, Some(&n.id), scene, preview)
            }
            if preview {
                return;
            }
            let color = hex_color("6366f1");
            let mut bracket = BezPath::new();
            bracket.move_to((-4., 1.));
            bracket.line_to((-4., -4.));
            bracket.line_to((1., -4.));
            scene.push(handle(
                format!("{}:pad", n.id),
                Point::new(x + pad, y + pad),
                bracket,
                color,
                2.,
            ));
            if children.len() > 1 {
                let (sx, sy, first) = &g.slots[0];
                let vertical = *direction == 1 || (*direction == 2 && g.cols.len() == 1);
                let p = if vertical {
                    Point::new(x + sx + first.w / 2., y + sy + g.rows[0] + gap / 2.)
                } else {
                    Point::new(x + sx + g.cols[0] + gap / 2., y + sy + first.h / 2.)
                };
                let mut tick = BezPath::new();
                if *direction == 1 {
                    tick.move_to((-4., 0.));
                    tick.line_to((4., 0.));
                } else {
                    tick.move_to((0., -4.));
                    tick.line_to((0., 4.));
                }
                scene.push(handle(format!("{}:gap", n.id), p, tick, color, 2.));
            }
            if *direction == 2 && children.len() > 1 {
                let p = Point::new(x + g.w - pad + 6., y + pad + g.rows[0] / 2.);
                let mut o = Object::circle(format!("{}:cols", n.id), p, 3.5, WHITE);
                o.hit = Some(Hit::Rect(Rect::new(-5., -5., 5., 5.)));
                if let Shape::Circle { stroke, .. } = &mut o.shapes[0] {
                    *stroke = Some((color, 2.));
                }
                scene.push(o);
            }
            let p = match direction {
                0 => Point::new(x + g.w - 5., y + g.h / 2.),
                1 => Point::new(x + g.w / 2., y + g.h - 5.),
                _ => Point::new(x + g.w - 5., y + g.h - 5.),
            };
            let mut o = Object::circle(format!("{}:axis", n.id), p, 7., WHITE);
            if let Shape::Circle { stroke, .. } = &mut o.shapes[0] {
                *stroke = Some((color, 1.5));
            }
            let mut glyph = BezPath::new();
            match direction {
                0 => {
                    glyph.move_to((-4., 0.));
                    glyph.line_to((4., 0.));
                    glyph.move_to((1.5, -2.5));
                    glyph.line_to((4., 0.));
                    glyph.line_to((1.5, 2.5));
                }
                1 => {
                    glyph.move_to((0., -4.));
                    glyph.line_to((0., 4.));
                    glyph.move_to((-2.5, 1.5));
                    glyph.line_to((0., 4.));
                    glyph.line_to((2.5, 1.5));
                }
                _ => {
                    for (dx, dy) in [(-3.5, -3.5), (0.5, -3.5), (-3.5, 0.5), (0.5, 0.5)] {
                        glyph.move_to((dx, dy));
                        glyph.line_to((dx + 3., dy));
                        glyph.line_to((dx + 3., dy + 3.));
                        glyph.line_to((dx, dy + 3.));
                        glyph.close_path();
                    }
                }
            }
            o.shapes.push(Shape::Path {
                path: glyph,
                fill: (*direction == 2).then_some(color),
                stroke: if *direction == 2 {
                    None
                } else {
                    Some((color, 1.5))
                },
            });
            scene.push(o.line_cap(LineCap::Round));
        }
    }
}
fn rounded(n: &Node) -> Node {
    let mut out = n.clone();
    match &mut out.kind {
        Kind::Leaf { w, h, .. } => {
            *w = w.round();
            *h = h.round();
        }
        Kind::Box {
            gap, pad, children, ..
        } => {
            *gap = gap.round();
            *pad = pad.round();
            *children = children.iter().map(rounded).collect();
        }
    }
    out
}
impl Example for Layout {
    type State = State;
    fn initial(&self) -> State {
        State {
            root: container(
                "root",
                1,
                12.,
                12.,
                vec![
                    container(
                        "header",
                        0,
                        8.,
                        8.,
                        vec![
                            leaf("logo", 36., 36., hex_color("f87171")),
                            leaf("nav-a", 60., 24., hex_color("fb923c")),
                            leaf("nav-b", 60., 24., hex_color("fb923c")),
                            leaf("nav-c", 60., 24., hex_color("fb923c")),
                        ],
                    ),
                    container(
                        "body",
                        0,
                        12.,
                        8.,
                        vec![
                            container(
                                "sidebar",
                                1,
                                6.,
                                6.,
                                (1..=3)
                                    .map(|i| {
                                        leaf(&format!("side-{i}"), 56., 28., hex_color("4ade80"))
                                    })
                                    .collect(),
                            ),
                            container(
                                "gallery",
                                2,
                                8.,
                                8.,
                                (0..4)
                                    .map(|i| {
                                        leaf(
                                            &format!("img-{}", i + 1),
                                            64.,
                                            48.,
                                            [
                                                hex_color("60a5fa"),
                                                hex_color("a78bfa"),
                                                hex_color("f472b6"),
                                                hex_color("60a5fa"),
                                            ][i],
                                        )
                                    })
                                    .collect(),
                            ),
                        ],
                    ),
                ],
            ),
        }
    }
    fn scene(&self, s: &State) -> Scene {
        let mut scene = Scene::new(704., 760.).scrolling();
        scene.push(baseline_text(
            "editor-heading",
            (0., 12.),
            "Editor",
            12.,
            hex_color("64748b"),
        ));
        scene.push(baseline_text(
            "html-preview-heading",
            (360., 12.),
            "Native HTML preview",
            12.,
            hex_color("64748b"),
        ));
        render(
            &s.root,
            &geometry(&s.root),
            20.,
            40.,
            None,
            &mut scene,
            false,
        );
        for object in &mut scene.objects {
            if object.id != "editor-heading" && object.id != "html-preview-heading" {
                object.clip = Some(Clip::Rect(
                    object
                        .transform
                        .inverse()
                        .transform_rect_bbox(Rect::new(0., 20., 344., 380.)),
                ));
            }
            if let Some((parent, _)) = object.id.split_once(':') {
                object.parent = Some(parent.into());
            }
        }
        let preview = rounded(&s.root);
        let mut second = Scene::new(344., 360.);
        render(
            &preview,
            &geometry(&preview),
            380.,
            40.,
            None,
            &mut second,
            true,
        );
        for mut object in second.objects {
            object.clip = Some(Clip::Rect(
                object
                    .transform
                    .inverse()
                    .transform_rect_bbox(Rect::new(360., 20., 704., 380.)),
            ));
            object.id = format!("html-preview-{}", object.id);
            object.parent = object.parent.map(|id| format!("html-preview-{id}"));
            scene.push(object);
        }
        scene.push(baseline_text(
            "css-heading",
            (0., 392.),
            "CSS",
            12.,
            hex_color("64748b"),
        ));
        scene.push(baseline_text(
            "html-heading",
            (360., 392.),
            "HTML",
            12.,
            hex_color("64748b"),
        ));
        scene.inline_control("css-code", Rect::new(0., 400., 344., 760.));
        scene.inline_control("html-code", Rect::new(360., 400., 704., 760.));
        scene
    }
    fn controls(&self, s: &State) -> Vec<Control> {
        vec![
            Control::Code {
                id: "css-code",
                label: "CSS".into(),
                content: css(&s.root),
            },
            Control::Code {
                id: "html-code",
                label: "HTML".into(),
                content: html(&s.root, 0),
            },
        ]
    }
    fn locate(&self, s: &State, id: &str) -> Option<Affine> {
        let mut scene = Scene::new(344., 360.);
        render(
            &s.root,
            &geometry(&s.root),
            20.,
            40.,
            None,
            &mut scene,
            false,
        );
        scene.object(id).map(|object| object.transform)
    }
    fn drag(&self, s: &State, id: &str, _: Modifiers) -> Option<DragSpec<'static, State>> {
        if let Some((target, handle)) = id.rsplit_once(':') {
            let n = node(&s.root, target)?;
            let target = target.to_string();
            match handle {
                "axis" => {
                    let states = (0..3)
                        .map(|d| {
                            let mut t = s.clone();
                            if let Kind::Box { direction, .. } =
                                &mut node_mut(&mut t.root, &target).unwrap().kind
                            {
                                *direction = d
                            }
                            t
                        })
                        .collect::<Vec<_>>();
                    return Some(between(states).with_drop_transition(Some(transition(150))));
                }
                "cols" => {
                    if let Kind::Box { children, .. } = &n.kind {
                        let states = (1..=children.len())
                            .map(|c| {
                                let mut t = s.clone();
                                if let Kind::Box { cols, .. } =
                                    &mut node_mut(&mut t.root, &target).unwrap().kind
                                {
                                    *cols = c
                                }
                                t
                            })
                            .collect::<Vec<_>>();
                        return Some(closest(states).with_branch_transition(Some(transition(120))));
                    }
                }
                "resize" => {
                    let Kind::Leaf { w, h, .. } = &n.kind else {
                        return None;
                    };
                    let initial = [*w, *h];
                    let base = s.clone();
                    let constraint_id = target.clone();
                    return Some(vary_constrained(
                        initial,
                        move |[w, h]| {
                            let mut t = base.clone();
                            if let Kind::Leaf { w: tw, h: th, .. } =
                                &mut node_mut(&mut t.root, &target).unwrap().kind
                            {
                                *tw = w;
                                *th = h;
                            }
                            t
                        },
                        move |s| {
                            let Kind::Leaf { w, h, .. } =
                                &node(&s.root, &constraint_id).unwrap().kind
                            else {
                                unreachable!()
                            };
                            [20. - w, w - 240., 20. - h, h - 240.]
                        },
                    ));
                }
                "pad" | "gap" => {
                    let Kind::Box { pad, gap, .. } = &n.kind else {
                        return None;
                    };
                    let padding = handle == "pad";
                    let value = if padding { *pad } else { *gap };
                    let base = s.clone();
                    let constraint_id = target.clone();
                    return Some(vary_constrained(
                        [value],
                        move |[value]| {
                            let mut t = base.clone();
                            if let Kind::Box { pad, gap, .. } =
                                &mut node_mut(&mut t.root, &target).unwrap().kind
                            {
                                if padding { *pad = value } else { *gap = value }
                            }
                            t
                        },
                        move |s| {
                            let Kind::Box { pad, gap, .. } =
                                &node(&s.root, &constraint_id).unwrap().kind
                            else {
                                unreachable!()
                            };
                            let value = if padding { *pad } else { *gap };
                            [-value, value - if padding { 40. } else { 48. }]
                        },
                    ));
                }
                _ => {}
            }
            return None;
        }
        if id == "root" {
            return None;
        }
        let mut base = s.clone();
        let moved = remove(&mut base.root, id)?;
        let mut dests = vec![];
        targets(&base.root, &mut dests);
        let mut choices = vec![s.clone()];
        for (target, i) in dests {
            let mut t = base.clone();
            if let Kind::Box { children, .. } = &mut node_mut(&mut t.root, &target).unwrap().kind {
                children.insert(i, moved.clone());
            }
            choices.push(t)
        }
        Some(
            closest(choices)
                .with_floating()
                .when_far(s.clone(), 40.)
                .with_branch_transition(Some(transition(120))),
        )
    }
    fn export(&self, s: &State) -> Option<(String, String)> {
        Some((
            "layout.html".into(),
            format!(
                "<!doctype html><meta charset=\"utf-8\"><style>\n{}\n</style>\n{}",
                css(&s.root),
                html(&s.root, 0)
            ),
        ))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn moving_preserves_tree_and_blocks_cycles() {
        let s = Layout.initial();
        let mut root = s.root.clone();
        let gallery = remove(&mut root, "gallery").unwrap();
        let mut ts = vec![];
        targets(&root, &mut ts);
        assert!(
            !ts.iter()
                .any(|(id, _)| id == "gallery" || id.starts_with("img"))
        );
        assert_eq!(geometry(&gallery).w, 152.);
        assert!(css(&s.root).contains("grid-template-columns: repeat(2, max-content);"));
    }
}
