use super::*;
#[derive(Clone, Copy, Debug)]
pub enum Role {
    Term {
        owner: u64,
        index: usize,
        across: Option<usize>,
    },
    Factor {
        owner: u64,
        part: Option<bool>,
        index: usize,
        across: Option<usize>,
    },
    Exponent {
        owner: u64,
        across: Option<usize>,
        log: Option<u64>,
    },
    PowerBase {
        owner: u64,
        side: usize,
    },
    LogBase {
        owner: u64,
        across: Option<usize>,
    },
}
#[derive(Clone, Copy, Default)]
struct Place {
    role: Option<Role>,
    inert: bool,
    atomic: bool,
    side_level: bool,
    parens: bool,
    factor: Option<(u64, bool, Option<usize>)>,
    log: Option<u64>,
}
struct BoxNode {
    id: u64,
    w: f64,
    asc: f64,
    desc: f64,
    size: f64,
    lead: f64,
    role: Option<Role>,
    shapes: Vec<Shape>,
    lead_shape: Option<Shape>,
    italic: bool,
    children: Vec<(f64, f64, BoxNode)>,
}
pub struct Layout {
    pub scene: Scene,
    pub roles: HashMap<String, Role>,
    pub bounds: HashMap<u64, NodeBounds>,
}
fn txt(origin: (f64, f64), text: impl Into<String>, size: f64) -> Shape {
    Shape::Text {
        origin: Point::new(origin.0, origin.1),
        text: text.into(),
        size,
        color: [31, 41, 55, 255],
    }
}
fn term_lead(sign: i8, index: usize, size: f64) -> (String, f64) {
    if index == 0 {
        if sign < 0 {
            ("−".into(), 0.68 * 34. * size)
        } else {
            (String::new(), 0.)
        }
    } else {
        ((if sign < 0 { "−" } else { "+" }).into(), 1.1 * 34. * size)
    }
}
fn digit(e: &Expr) -> bool {
    match &e.kind {
        Kind::Num(_) => true,
        Kind::Pow(a, _) => digit(a),
        _ => false,
    }
}
fn build(e: &Expr, p: Place, side: usize, root_id: u64, size: f64, lead: (String, f64)) -> BoxNode {
    let em = 34. * size;
    let inert = p.inert || p.atomic;
    let mut node = BoxNode {
        id: e.id,
        w: 0.,
        asc: 0.72 * em,
        desc: 0.24 * em,
        size,
        lead: lead.1,
        role: p.role,
        shapes: Vec::new(),
        lead_shape: None,
        italic: false,
        children: Vec::new(),
    };
    if !lead.0.is_empty() {
        node.lead_shape = Some(txt((-lead.1 / 2., 0.), lead.0, em));
    }
    match &e.kind {
        Kind::Num(n) => {
            let text = n.to_string();
            node.w = text.len() as f64 * 0.5 * em;
            node.shapes.push(txt((node.w / 2., 0.), text, em));
        }
        Kind::Var(v) => {
            node.w = 0.5 * em;
            node.shapes.push(txt((node.w / 2., 0.), v, em));
            node.italic = true;
        }
        Kind::Sum(ts) => {
            let multi = ts.len() >= 2;
            let pw = if p.parens { 0.36 * em } else { 0. };
            let mut x = pw;
            for (i, t) in ts.iter().enumerate() {
                let role = (!inert).then_some(Role::Term {
                    owner: e.id,
                    index: i,
                    across: p.side_level.then_some(side),
                });
                let child = build(
                    &t.expr,
                    Place {
                        role,
                        inert,
                        atomic: multi,
                        side_level: p.side_level && !multi,
                        ..Place::default()
                    },
                    side,
                    root_id,
                    size,
                    term_lead(t.sign, i, size),
                );
                node.asc = node.asc.max(child.asc);
                node.desc = node.desc.max(child.desc);
                let w = child.w;
                node.children.push((x, 0., child));
                x += w;
            }
            node.w = x + pw;
            if p.parens {
                node.shapes.push(txt((pw / 2., 0.), "(", em));
                node.shapes.push(txt((x + pw / 2., 0.), ")", em));
            }
        }
        Kind::Prod(fs) => {
            let mut x = 0.;
            for (i, f) in fs.iter().enumerate() {
                let (owner, part, across) = p.factor.map_or(
                    (e.id, None, p.side_level.then_some(side)),
                    |(id, top, across)| (id, Some(top), across),
                );
                let role = (!inert).then_some(Role::Factor {
                    owner,
                    part,
                    index: i,
                    across,
                });
                let lead = if i > 0 && digit(f) {
                    ("·".into(), 0.42 * em)
                } else {
                    (String::new(), 0.)
                };
                let child = build(
                    f,
                    Place {
                        role,
                        inert,
                        parens: matches!(f.kind, Kind::Sum(_)),
                        ..Place::default()
                    },
                    side,
                    root_id,
                    size,
                    lead,
                );
                node.asc = node.asc.max(child.asc);
                node.desc = node.desc.max(child.desc);
                let w = child.w;
                node.children.push((x, 0., child));
                x += w;
            }
            node.w = x;
        }
        Kind::Frac(a, b) => {
            let part = |expr: &Expr, top: bool| {
                let across = p.side_level.then_some(side);
                let is_prod = matches!(expr.kind, Kind::Prod(_));
                build(
                    expr,
                    Place {
                        role: (!inert && !is_prod).then_some(Role::Factor {
                            owner: e.id,
                            part: Some(top),
                            index: 0,
                            across,
                        }),
                        inert,
                        factor: is_prod.then_some((e.id, top, across)),
                        ..Place::default()
                    },
                    side,
                    root_id,
                    size,
                    (String::new(), 0.),
                )
            };
            let top = part(a, true);
            let bottom = part(b, false);
            let bar_w = top.w.max(bottom.w) + 0.24 * em;
            let bar_y = -0.27 * em;
            let top_y = bar_y - 0.1 * em - top.desc;
            let bottom_y = bar_y + 0.1 * em + bottom.asc;
            let pad = 0.06 * em;
            node.w = bar_w + 2. * pad;
            node.asc = -top_y + top.asc;
            node.desc = bottom_y + bottom.desc;
            let mut path = BezPath::new();
            path.move_to((pad, bar_y));
            path.line_to((pad + bar_w, bar_y));
            node.shapes.push(Shape::Path {
                path,
                fill: None,
                stroke: Some(([31, 41, 55, 255], 2. * size)),
            });
            node.children.push((pad + (bar_w - top.w) / 2., top_y, top));
            node.children
                .push((pad + (bar_w - bottom.w) / 2., bottom_y, bottom));
        }
        Kind::Pow(a, b) => {
            let across = (p.side_level && e.id == root_id).then_some(side);
            let wrap = !matches!(a.kind, Kind::Num(_) | Kind::Var(_) | Kind::Sum(_));
            let pw = if wrap { 0.36 * em } else { 0. };
            let base = build(
                a,
                Place {
                    role: if !inert && matches!(a.kind, Kind::Num(_)) {
                        across.map(|side| Role::PowerBase { owner: e.id, side })
                    } else {
                        None
                    },
                    inert,
                    parens: matches!(a.kind, Kind::Sum(_)),
                    ..Place::default()
                },
                side,
                root_id,
                size,
                (String::new(), 0.),
            );
            let exp = build(
                b,
                Place {
                    role: (!inert).then_some(Role::Exponent {
                        owner: e.id,
                        across,
                        log: p.log,
                    }),
                    inert,
                    atomic: true,
                    ..Place::default()
                },
                side,
                root_id,
                size * 0.62,
                (String::new(), 0.),
            );
            let exp_x = 2. * pw + base.w + 0.04 * em;
            let exp_y = -(0.42 * em)
                .max(base.asc - 0.3 * em)
                .max(exp.desc + 0.2 * em);
            node.w = exp_x + exp.w;
            node.asc = base.asc.max(exp.asc - exp_y);
            node.desc = base.desc.max(exp.desc + exp_y);
            if wrap {
                node.shapes.push(txt((pw / 2., 0.), "(", em));
                node.shapes.push(txt((pw * 1.5 + base.w, 0.), ")", em));
            }
            node.children.push((pw, 0., base));
            node.children.push((exp_x, exp_y, exp));
        }
        Kind::Log(a, b) => {
            let across = (p.side_level && e.id == root_id).then_some(side);
            let base = build(
                a,
                Place {
                    role: (!inert).then_some(Role::LogBase {
                        owner: e.id,
                        across,
                    }),
                    inert,
                    atomic: true,
                    ..Place::default()
                },
                side,
                root_id,
                size * 0.62,
                (String::new(), 0.),
            );
            let arg = build(
                b,
                Place {
                    inert,
                    log: Some(e.id),
                    ..Place::default()
                },
                side,
                root_id,
                size,
                (String::new(), 0.),
            );
            let base_x = 1.32 * em;
            let base_y = 0.3 * em;
            let pw = 0.36 * em;
            let arg_x = base_x + base.w + 0.04 * em + pw;
            node.w = arg_x + arg.w + pw;
            node.asc = arg.asc;
            node.desc = arg.desc.max(base_y + base.desc);
            node.shapes.extend([
                txt((0.65 * em, 0.), "log", em),
                txt((arg_x - pw / 2., 0.), "(", em),
                txt((arg_x + arg.w + pw / 2., 0.), ")", em),
            ]);
            node.children.push((base_x, base_y, base));
            node.children.push((arg_x, 0., arg));
        }
    }
    node.w += node.lead;
    node
}
fn emit(
    layout: &mut Layout,
    node: BoxNode,
    x: f64,
    y: f64,
    parent: Option<String>,
    hovered: Option<&str>,
) {
    let id = format!("node-{}", node.id);
    let origin = Point::new(x + node.lead, y);
    let bounds = Rect::new(-node.lead, -node.asc, node.w - node.lead, node.desc);
    let mut object = Object::new(
        id.clone(),
        Affine::translate(origin.to_vec2()),
        node.role.map(|_| Hit::Rect(bounds)),
        node.shapes,
    );
    object.parent = parent;
    object = object
        .font_family(FontFamily::Serif)
        .text_anchor(TextAnchor::Middle)
        .text_baseline(TextBaseline::Alphabetic);
    if node.italic {
        object = object.italic();
    }
    object.shapes.insert(
        0,
        Shape::Rect {
            bounds,
            radius: 6.,
            fill: if hovered == Some(id.as_str()) && node.role.is_some() {
                [14, 165, 233, 26]
            } else {
                [0, 0, 0, 0]
            },
            stroke: None,
        },
    );
    layout.bounds.insert(
        node.id,
        NodeBounds {
            origin,
            bounds: bounds + origin.to_vec2(),
            size: node.size,
        },
    );
    if let Some(role) = node.role {
        layout.roles.insert(id.clone(), role);
    }
    layout.scene.push(object);
    if let Some(lead) = node.lead_shape {
        let mut o = Object::new(
            format!("{id}-lead"),
            Affine::translate(origin.to_vec2()),
            None,
            vec![lead],
        )
        .font_family(FontFamily::Serif)
        .text_anchor(TextAnchor::Middle)
        .text_baseline(TextBaseline::Alphabetic);
        o.parent = Some(id.clone());
        layout.scene.push(o);
    }
    for (cx, cy, child) in node.children {
        emit(
            layout,
            child,
            origin.x + cx,
            origin.y + cy,
            Some(id.clone()),
            hovered,
        );
    }
}
pub fn render(s: &State) -> Layout {
    render_interaction(s, None, None)
}
pub fn render_interaction(s: &State, dragged: Option<&str>, hovered: Option<&str>) -> Layout {
    let hovered = if dragged.is_none() { hovered } else { None };
    let build_side = |side: usize| {
        let e = &s.sides[side];
        build(
            e,
            Place {
                role: (!matches!(e.kind, Kind::Sum(_))).then_some(Role::Term {
                    owner: e.id,
                    index: 0,
                    across: Some(side),
                }),
                side_level: true,
                ..Place::default()
            },
            side,
            e.id,
            1.,
            (String::new(), 0.),
        )
    };
    let lhs = build_side(0);
    let rhs = build_side(1);
    let left = 320. - 18.7 - lhs.w;
    let mut layout = Layout {
        scene: Scene::new(640., 240.),
        roles: HashMap::new(),
        bounds: HashMap::new(),
    };
    if s.solved() && dragged.is_none() {
        let pad = 17.;
        let x = left - pad;
        let w = lhs.w + rhs.w + 37.4 + 2. * pad;
        let y = 128. - lhs.asc.max(rhs.asc) - pad / 2.;
        let h = lhs.asc.max(rhs.asc) + lhs.desc.max(rhs.desc) + pad;
        for (id, bounds, blur, color) in [
            (
                "solved-glow",
                Rect::new(x, y, x + w, y + h),
                14.,
                [251, 191, 36, 153],
            ),
            (
                "solved-core",
                Rect::new(
                    x + pad / 2.,
                    y + pad / 4.,
                    x + w - pad / 2.,
                    y + h - pad / 4.,
                ),
                5.,
                [255, 251, 235, 255],
            ),
        ] {
            layout.scene.push(
                Object::rect(id, bounds, h / 2., color)
                    .blur(blur)
                    .decorative(),
            );
        }
    }
    emit(&mut layout, lhs, left, 128., None, hovered);
    layout.scene.push(
        Object::text("equals", (320., 128.), "=", 34., [31, 41, 55, 255])
            .font_family(FontFamily::Serif)
            .text_anchor(TextAnchor::Middle)
            .text_baseline(TextBaseline::Alphabetic),
    );
    emit(&mut layout, rhs, 338.7, 128., None, hovered);
    if let Some(puff) = &s.puff {
        let lead = puff.sign.map_or((String::new(), 0.), |sign| {
            (
                (if sign < 0 { "−" } else { "+" }).into(),
                0.68 * 34. * puff.size,
            )
        });
        let inner = build(
            &puff.expr,
            Place {
                inert: true,
                atomic: true,
                parens: matches!(puff.expr.kind, Kind::Sum(_)),
                ..Place::default()
            },
            0,
            puff.expr.id,
            puff.size,
            lead,
        );
        let em = 34. * puff.size;
        let center = Point::new((inner.w - inner.lead) / 2., -0.3 * em);
        let at = puff.at - Vec2::new(0., equation_buttons(&mut Scene::new(640., 240.), s.preset));
        let hidden_start = layout.scene.objects.len();
        let lead_width = inner.lead;
        emit(&mut layout, inner, at.x - lead_width, at.y, None, None);
        // Keep every part at its destination while it fades into the cloud.
        // Dropping the hidden subtree would make its children fade at their old positions.
        for object in &mut layout.scene.objects[hidden_start..] {
            object.opacity = 0.;
        }
        let root_id = format!("node-{}", puff.expr.id);
        let mut cloud = Object::new(
            format!("{root_id}-puff"),
            Affine::translate(at.to_vec2()),
            None,
            Vec::new(),
        );
        cloud.parent = Some(root_id);
        cloud.opacity = 0.75;
        let bumps = [
            (-0.2, 0.04, 0.16),
            (-0.04, -0.11, 0.2),
            (0.17, -0.03, 0.17),
            (0.09, 0.13, 0.13),
            (-0.09, 0.13, 0.11),
        ];
        for filled in [false, true] {
            for (x, y, r) in bumps {
                cloud.shapes.push(Shape::Circle {
                    center: center + Vec2::new(x * em, y * em),
                    radius: r * em,
                    fill: if filled {
                        [248, 250, 252, 255]
                    } else {
                        [0, 0, 0, 0]
                    },
                    stroke: (!filled).then_some(([148, 163, 184, 255], 3.)),
                });
            }
        }
        for (x, y, r) in [
            (0.47, -0.26, 0.09),
            (-0.47, -0.2, 0.065),
            (0.39, 0.24, 0.055),
        ] {
            cloud.shapes.push(Shape::Circle {
                center: center + Vec2::new(x * em, y * em),
                radius: r * em,
                fill: [248, 250, 252, 255],
                stroke: Some(([148, 163, 184, 255], 2.)),
            });
        }
        layout.scene.push(cloud);
    }
    let offset = equation_buttons(&mut layout.scene, s.preset);
    layout.scene.height += offset;
    for object in &mut layout.scene.objects {
        object.transform = Affine::translate((0., offset)) * object.transform;
    }
    for bounds in layout.bounds.values_mut() {
        bounds.origin.y += offset;
        bounds.bounds = bounds.bounds + Vec2::new(0., offset);
    }
    layout
}
