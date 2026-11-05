//! Native port of Dragology's knot demo. Original contributors retain MIT credit.
use crate::*;
mod geometry;
mod model;
mod moves;
use geometry::*;
use model::*;
use moves::{Kind, Move};

pub fn entry() -> Entry {
    Entry::new(
        "knot",
        "Knot",
        "⚠️ Just a seed! A knot diagram you can manipulate by Reidemeister moves. Each strand between two crossings is a single curve. Grab one by its middle: drag it onto a ring beside it to twist a loop (R1), push it across a neighboring strand (R2; hold Alt to pass under), or slide it across a crossing (R3; or drag the crossing across the strand). Drag a loop back into the strand or a bigon back across its partner to undo. Drag in other directions to bend the strand, or drag a crossing to move it. The Jones polynomial never changes; crossing number and writhe do. Tangle A is the Culprit, an unknot that can’t be simplified until you first make it worse. Tangle B is a trefoil in disguise.",
        || demo(KnotDemo),
    )
}
#[derive(Clone, PartialEq)]
struct State {
    knot: Knot,
    past: Vec<Knot>,
    preset: usize,
    targets: Vec<Point>,
}
struct KnotDemo;
impl Example for KnotDemo {
    type State = State;
    fn initial(&self) -> State {
        State {
            knot: preset(0),
            past: Vec::new(),
            preset: 0,
            targets: Vec::new(),
        }
    }
    fn scene(&self, s: &State) -> Scene {
        drawing(s)
    }
    fn scene_dragging(&self, s: &State, id: Option<&str>) -> Scene {
        self.scene_interaction(s, id, None)
    }
    fn scene_interaction(&self, s: &State, dragged: Option<&str>, hovered: Option<&str>) -> Scene {
        let mut scene = self.scene(s);
        if let Some(id) = hovered {
            if let Some(object) = scene.objects.iter_mut().find(|o| o.id == id) {
                if let Some(Shape::Circle { fill, .. }) = object.shapes.first_mut() {
                    *fill = [15, 23, 42, 31];
                }
            }
        }
        if let Some(id) = dragged {
            if let Some(object) = scene.objects.iter_mut().find(|o| o.id == id) {
                if let Some(Shape::Circle { radius, fill, .. }) = object.shapes.first_mut() {
                    *radius = 7.;
                    *fill = [15, 23, 42, 64];
                }
            }
        }
        scene
    }
    fn drag(&self, s: &State, id: &str, mods: Modifiers) -> Option<DragSpec<'static, State>> {
        drag_spec(s, id, mods)
    }
    fn locate(&self, s: &State, id: &str) -> Option<Affine> {
        let p = if let Some(n) = id.strip_prefix("node-") {
            s.knot.nodes.get(n)?.pos()
        } else if let Some(e) = id.strip_prefix("h-") {
            edge_index(&s.knot, e)?;
            edge_mid(&s.knot, e)
        } else {
            return None;
        };
        Some(Affine::translate(p.to_vec2()))
    }
    fn controls(&self, s: &State) -> Vec<Control> {
        let mut controls = vec![Control::action_button(
            "undo",
            "↶ undo",
            !s.past.is_empty(),
            false,
        )];
        controls.extend(
            ["tangle A", "tangle B", "trefoil", "figure-eight", "unknot"]
                .into_iter()
                .enumerate()
                .map(|(i, label)| {
                    Control::button(
                        ["preset-0", "preset-1", "preset-2", "preset-3", "preset-4"][i],
                        label,
                    )
                }),
        );
        controls
    }
    fn action(&self, s: &State, id: &str, value: Value) -> Option<State> {
        if let Some(i) = id
            .strip_prefix("preset-")
            .and_then(|s| s.parse::<usize>().ok())
        {
            return self.action(s, "preset", Value::Index(i));
        }
        match (id, value) {
            ("undo", _) => {
                let mut t = s.clone();
                t.knot = t.past.pop()?;
                t.targets.clear();
                Some(t)
            }
            ("preset", Value::Index(i)) if i < 5 => {
                let mut t = s.clone();
                t.knot = preset(i);
                t.preset = i;
                t.targets.clear();
                remember(s, &mut t);
                Some(t)
            }
            _ => None,
        }
    }
    fn status(&self, s: &State) -> String {
        let inv = invariants(&s.knot);
        format!(
            "{} crossings · writhe {:+} · {} · Jones V(t) = {}",
            inv.n,
            inv.writhe,
            if inv.tricolorable {
                "tricolorable"
            } else {
                "not tricolorable"
            },
            inv.jones
                .as_ref()
                .map(format_jones)
                .unwrap_or_else(|| "too many crossings (limit 13)".into())
        )
    }
}
fn drawing(s: &State) -> Scene {
    let k = &s.knot;
    let inv = invariants(k);
    let cs = cubics(k);
    let n = k.code.len();
    let palette = [[225, 29, 72, 255], [37, 99, 235, 255], [22, 163, 74, 255]];
    let colors = (0..n)
        .map(|i| {
            if inv.tricolorable {
                palette[inv.colors.get(&arc(k, i)).copied().unwrap_or(0)]
            } else {
                [51, 65, 85, 255]
            }
        })
        .collect::<Vec<_>>();
    let mut scene = Scene::new(560., 730.).scrolling();
    for (i, v) in k.code.iter().enumerate() {
        scene.push(
            Object::path(format!("edge-{}", v.e), path(cs[i]), colors[i], 5.)
                .line_cap(LineCap::Round),
        );
    }
    for (i, v) in k.code.iter().enumerate() {
        let nd = &k.nodes[&v.n];
        let ghost = if !nd.crossing() {
            nd.ghost_of.as_ref()
        } else {
            None
        };
        if ghost.is_none() && (!v.over || !nd.crossing()) {
            continue;
        }
        let mut p = if ghost.is_some() {
            let q = cs[i][0];
            let mut p = BezPath::new();
            p.move_to(q);
            p.curve_to(q, q, q);
            p.curve_to(q, q, q);
            p
        } else {
            bridge(cs[(i + n - 1) % n], cs[i])
        };
        let mut object = Object::path(
            format!("bridge-{}", ghost.unwrap_or(&v.n)),
            p.clone(),
            [255, 255, 255, 255],
            13.,
        );
        object.shapes.push(Shape::Path {
            path: std::mem::take(&mut p),
            fill: None,
            stroke: Some((colors[i], 5.)),
        });
        if ghost.is_some() {
            object.opacity = 0.;
        }
        scene.push(object);
    }
    let m = bez(cs[0], 0.5);
    let mut marker = BezPath::new();
    marker.move_to((-4., -5.));
    marker.line_to((5., 0.));
    marker.line_to((-4., 5.));
    marker.close_path();
    scene.push(Object::new(
        "orientation",
        Affine::translate(m.to_vec2()) * Affine::rotate(angle(tangent(cs[0], 0.5))),
        None,
        vec![Shape::Path {
            path: marker,
            fill: Some([255; 4]),
            stroke: Some((colors[0], 1.5)),
        }],
    ));
    for (i, v) in k.code.iter().enumerate() {
        scene.push(
            Object::circle(format!("h-{}", v.e), bez(cs[i], 0.5), 10., [0; 4])
                .hit_when_transparent(),
        );
    }
    for (id, nd) in &k.nodes {
        scene.push(
            Object::circle(format!("node-{id}"), nd.pos(), 9., [0; 4]).hit_when_transparent(),
        );
    }
    for (i, p) in s.targets.iter().enumerate() {
        let mut o = Object::circle(format!("twist-target-{i}"), *p, 6., [0; 4]).decorative();
        if let Shape::Circle { stroke, .. } = &mut o.shapes[0] {
            *stroke = Some(([148, 163, 184, 255], 1.5));
        }
        scene.push(o);
    }
    scene.push(
        Object::text(
            "counts",
            (16., 412.),
            format!(
                "crossings {}   writhe {:+}   {}",
                inv.n,
                inv.writhe,
                if inv.tricolorable {
                    "tricolorable"
                } else {
                    "not tricolorable"
                }
            ),
            13.,
            [100, 116, 139, 255],
        )
        .font_family(FontFamily::Mono)
        .text_baseline(TextBaseline::Alphabetic),
    );
    scene.push(
        Object::text(
            "jones",
            (16., 434.),
            format!(
                "Jones  V(t) = {}",
                inv.jones
                    .as_ref()
                    .map(format_jones)
                    .unwrap_or_else(|| "(too many crossings)".into())
            ),
            15.,
            [15, 23, 42, 255],
        )
        .font_family(FontFamily::Mono)
        .text_baseline(TextBaseline::Alphabetic),
    );
    draw_move_legend(&mut scene);
    scene
}
fn path(c: Cubic) -> BezPath {
    let mut p = BezPath::new();
    p.move_to(c[0]);
    p.curve_to(c[1], c[2], c[3]);
    p
}
fn bridge(c: Cubic, d: Cubic) -> BezPath {
    let lc = length(c, 24);
    let ld = length(d, 24);
    let a = sub(
        c,
        if lc > 0. {
            at_length(c, (lc - 13.).max(0.))
        } else {
            1.
        },
        1.,
    );
    let b = sub(
        d,
        0.,
        if ld > 0. {
            at_length(d, ld.min(13.))
        } else {
            0.
        },
    );
    let mut p = path(a);
    p.curve_to(b[1], b[2], b[3]);
    p
}
fn same(a: &Knot, b: &Knot) -> bool {
    a.code == b.code
        && a.nodes.len() == b.nodes.len()
        && a.edges.len() == b.edges.len()
        && a.nodes.iter().all(|(id, n)| {
            b.nodes.get(id).is_some_and(|m| {
                (n.x - m.x).abs() < 1e-3 && (n.y - m.y).abs() < 1e-3 && (n.rot - m.rot).abs() < 1e-3
            })
        })
        && a.edges.iter().all(|(id, e)| {
            b.edges
                .get(id)
                .is_some_and(|f| (e.a - f.a).abs() < 1e-3 && (e.b - f.b).abs() < 1e-3)
        })
}
fn remember(original: &State, next: &mut State) {
    if !same(&original.knot, &next.knot) {
        next.past = original.past.clone();
        next.past.push(original.knot.clone());
        if next.past.len() > 100 {
            next.past.remove(0);
        }
    }
}
fn move_spec(
    m: Move,
    base: &State,
    node: Option<&str>,
    edge: Option<&str>,
) -> DragSpec<'static, State> {
    let at = |k: &Knot| {
        if let Some(id) = node {
            k.nodes[id].pos()
        } else {
            edge_mid(k, edge.unwrap())
        }
    };
    let a = at(&m.from);
    let b = at(&m.to);
    let len = a.distance(b);
    let dir = (b - a) / if len > 0. { len } else { 1. };
    let start = m.meets.map_or(a, |p| p - dir * 20.);
    let end = b + dir * 40.;
    let mut from = base.clone();
    from.knot = m.from;
    let mut to = base.clone();
    to.knot = m.to;
    let mut spec = if len < 1e-9 {
        dragology::fixed(to)
    } else {
        dragology::between([from, to])
    };
    DragSpec::custom(move |ctx, pointer| {
        let past = (pointer - b).dot(dir);
        let adjusted = if past > 0. {
            pointer - dir * past
        } else {
            pointer
        };
        let mut frame = spec.evaluate(ctx, adjusted)?;
        frame.gap = distance_segment(pointer, start, end);
        Ok(frame)
    })
}
fn clamp_valid(mut spec: DragSpec<'static, State>, start: State) -> DragSpec<'static, State> {
    let mut last = start;
    DragSpec::custom(move |ctx, pointer| {
        let frame = spec.evaluate(ctx, pointer)?;
        if valid(&frame.drop_state.knot) {
            last = (*frame.drop_state).clone();
        }
        ctx.state_frame(std::rc::Rc::new(last.clone()), pointer)
    })
}
fn drag_spec(s: &State, id: &str, mods: Modifiers) -> Option<DragSpec<'static, State>> {
    let original = s.clone();
    let mut base = s.clone();
    let (spec, cleanup) = if let Some(node) = id.strip_prefix("node-") {
        let nd = s.knot.nodes.get(node)?;
        let p = nd.pos();
        let possibilities = moves::crossing_moves(&s.knot, node);
        let free_base = base.clone();
        let key = node.to_string();
        let free = dragology::vary([p.x, p.y], move |[x, y]| {
            let mut t = free_base.clone();
            t.knot
                .nodes
                .get_mut(&key)
                .unwrap()
                .set_pos(Point::new(x, y));
            t
        });
        let free = clamp_valid(free, base.clone());
        let specs = possibilities
            .into_iter()
            .map(|m| move_spec(m, &base, Some(node), None))
            .collect::<Vec<_>>();
        (
            dragology::closest(specs).when_far_with(free, 12., 28.),
            false,
        )
    } else {
        let e = id.strip_prefix("h-")?;
        let i = edge_index(&s.knot, e)?;
        let n1 = s.knot.code[i].n.clone();
        let n2 = s.knot.code[(i + 1) % s.knot.code.len()].n.clone();
        let ed = s.knot.edges[e];
        let rot1 = s.knot.nodes[&n1].rot;
        let rot2 = s.knot.nodes[&n2].rot;
        let possible = moves::moves(&s.knot, e, mods.alt, None);
        base.targets = possible
            .iter()
            .filter(|m| m.kind == Kind::Twist)
            .map(|m| edge_mid(&m.to, e))
            .collect();
        let free_base = base.clone();
        let edge_key = e.to_string();
        let node1 = n1.clone();
        let node2 = n2.clone();
        let constraint_edge = e.to_string();
        let free = if n1 == n2 {
            let key = e.to_string();
            let node = n1.clone();
            let ci = node.clone();
            let ck = key.clone();
            let b = base.clone();
            dragology::vary_constrained(
                [ed.a, ed.b, rot1],
                move |[a, bv, r]| {
                    let mut t = b.clone();
                    t.knot.edges.insert(key.clone(), Edge { a, b: bv });
                    t.knot.nodes.get_mut(&node).unwrap().rot = r;
                    t
                },
                move |t: &State| {
                    let edge = t.knot.edges[&ck];
                    [
                        3. - edge.a,
                        3. - edge.b,
                        edge.a - 70.,
                        edge.b - 70.,
                        (t.knot.nodes[&ci].rot - rot1).abs() - std::f64::consts::PI / 3.,
                    ]
                },
            )
        } else {
            dragology::vary_constrained(
                [ed.a, ed.b, rot1, rot2],
                move |[a, b, r1, r2]| {
                    let mut t = free_base.clone();
                    t.knot.edges.insert(edge_key.clone(), Edge { a, b });
                    t.knot.nodes.get_mut(&node1).unwrap().rot = r1;
                    if node1 != node2 {
                        t.knot.nodes.get_mut(&node2).unwrap().rot = r2;
                    }
                    t
                },
                move |t: &State| {
                    let ed = t.knot.edges[&constraint_edge];
                    let p1 = &t.knot.nodes[&n1];
                    let p2 = &t.knot.nodes[&n2];
                    let max = p1.pos().distance(p2.pos()) + 70.;
                    [
                        3. - ed.a,
                        3. - ed.b,
                        ed.a - max,
                        ed.b - max,
                        (p1.rot - rot1).abs() - std::f64::consts::PI / 3.,
                        (p2.rot - rot2).abs() - std::f64::consts::PI / 3.,
                    ]
                },
            )
        };
        let free = clamp_valid(free, base.clone());
        let mut possible = possible;
        possible.sort_by_key(|m| m.kind == Kind::Twist);
        let specs = possible
            .into_iter()
            .map(|m| {
                if m.kind == Kind::Twist {
                    let mut t = base.clone();
                    t.knot = m.to;
                    dragology::fixed(t)
                } else {
                    move_spec(m, &base, None, Some(e))
                }
            })
            .collect::<Vec<_>>();
        (
            dragology::closest(specs).when_far_with(free, 12., 28.),
            true,
        )
    };
    Some(spec.on_drop(move |t: &State| {
        let mut t = t.clone();
        if cleanup {
            t.knot = moves::cleanup(&t.knot);
        }
        t.targets.clear();
        remember(&original, &mut t);
        t
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dragging_a_twist_target_commits_one_move_and_undo_restores_geometry() {
        let example = KnotDemo;
        let mut state = example.initial();
        state.knot = preset(2);
        state.preset = 2;
        let edge = state.knot.code[0].e.clone();
        let twist = moves::moves(&state.knot, &edge, false, Some(&[Kind::Twist])).remove(0);
        let id = format!("h-{edge}");
        let start = edge_mid(&state.knot, &edge);
        let destination = edge_mid(&twist.to, &edge);
        let spec = example.drag(&state, &id, Modifiers::default()).unwrap();
        let mut session = dragology::DragSession::begin(
            state.clone(),
            start,
            move |s| KnotDemo.locate(s, &id),
            spec,
        )
        .unwrap();
        let frame = session.frame(start).unwrap();
        assert!(!frame.drop_state.targets.is_empty() || same(&frame.drop_state.knot, &state.knot));
        let result = session.finish(destination).unwrap();
        assert_eq!(invariants(&result.knot).n, 4);
        assert_eq!(result.past.len(), 1);
        assert!(result.targets.is_empty());
        assert_eq!(
            invariants(&result.knot).jones,
            invariants(&state.knot).jones
        );
        let restored = example.action(&result, "undo", Value::Trigger).unwrap();
        assert_eq!(restored.knot, state.knot);
        assert!(restored.past.is_empty());
    }
    #[test]
    fn dragging_a_crossing_preserves_topology_and_rejects_invalid_geometry() {
        let mut state = KnotDemo.initial();
        state.knot = preset(2);
        let node = state.knot.code[0].n.clone();
        let id = format!("node-{node}");
        let start = state.knot.nodes[&node].pos();
        let spec = KnotDemo.drag(&state, &id, Modifiers::default()).unwrap();
        let mut session = dragology::DragSession::begin(
            state.clone(),
            start,
            move |s| KnotDemo.locate(s, &id),
            spec,
        )
        .unwrap();
        for delta in [
            Vec2::new(3., 4.),
            Vec2::new(80., -150.),
            Vec2::new(-200., 100.),
        ] {
            let frame = session.frame(start + delta).unwrap();
            assert!(valid(&frame.drop_state.knot));
            assert_eq!(
                invariants(&frame.drop_state.knot).jones,
                invariants(&state.knot).jones
            );
        }
        assert_eq!(session.cancel().knot, state.knot);
    }
    #[test]
    fn runtime_reacts_to_alt_during_a_knot_drag() {
        let knot = preset(2);
        let edge = knot.code[0].e.clone();
        let over = moves::moves(&knot, &edge, false, Some(&[Kind::Twist])).remove(0);
        let destination = edge_mid(&over.to, &edge);
        let under = moves::moves(&knot, &edge, true, Some(&[Kind::Twist]))
            .into_iter()
            .min_by(|a, b| {
                edge_mid(&a.to, &edge)
                    .distance(destination)
                    .total_cmp(&edge_mid(&b.to, &edge).distance(destination))
            })
            .unwrap();
        assert_ne!(invariants(&over.to).writhe, invariants(&under.to).writhe);
        let mut runtime = crate::runtime::ModelDemo::new(KnotDemo);
        runtime.set_animated(false);
        runtime.action("preset", Value::Index(2));
        runtime.press(edge_mid(&knot, &edge), Modifiers::default(), 1);
        runtime.move_to(destination, Modifiers::default());
        runtime.advance(std::time::Duration::ZERO);
        let over_scene = runtime.scene().clone();
        let alt = Modifiers {
            alt: true,
            ..Modifiers::default()
        };
        runtime.move_to(destination, alt);
        runtime.advance(std::time::Duration::ZERO);
        let over_text = &over_scene.object("counts").unwrap().shapes;
        let under_text = &runtime.scene().object("counts").unwrap().shapes;
        assert_ne!(
            over_text, under_text,
            "Alt must change the preview without a new pointer press"
        );
        runtime.release(destination, alt);
        assert!(
            runtime
                .status()
                .contains(&format!("writhe {:+}", invariants(&under.to).writhe))
        );
        runtime.action("undo", Value::Trigger);
        assert!(runtime.status().contains("3 crossings"));
    }
    #[test]
    fn baked_puzzles_offer_valid_moves_including_the_culprits_required_twist() {
        let culprit = preset(0);
        let twists = moves::moves(&culprit, "e21", true, Some(&[Kind::Twist]));
        assert!(!twists.is_empty());
        let after = moves::cleanup(&twists[0].to);
        assert!(valid(&after));
        assert_eq!(invariants(&after).n, 11);
        assert_eq!(
            format_jones(invariants(&after).jones.as_ref().unwrap()),
            "1"
        );
        let hidden_trefoil = preset(1);
        assert!(
            hidden_trefoil
                .code
                .iter()
                .any(|v| !moves::moves(&hidden_trefoil, &v.e, false, None).is_empty())
        );
    }
    #[test]
    fn presets_have_source_invariants() {
        let expected = [
            (10, "1", false),
            (10, "− t⁴ + t³ + t", true),
            (3, "− t⁴ + t³ + t", true),
            (4, "t² − t + 1 − t⁻¹ + t⁻²", false),
            (0, "1", false),
        ];
        for (i, (n, jones, tricolor)) in expected.into_iter().enumerate() {
            let k = preset(i);
            assert!(valid(&k), "preset {i}");
            let inv = invariants(&k);
            assert_eq!(inv.n, n, "preset {i}");
            assert_eq!(inv.tricolorable, tricolor, "preset {i}");
            let actual = format_jones(inv.jones.as_ref().unwrap());
            assert!(
                actual == jones
                    || i == 1 && actual == "t⁻¹ + t⁻³ − t⁻⁴"
                    || i == 2 && actual == "t⁻¹ + t⁻³ − t⁻⁴",
                "preset {i}: {actual}"
            );
        }
    }
    #[test]
    fn all_basic_moves_preserve_the_jones_polynomial() {
        for preset_id in [2, 3, 4] {
            let k = preset(preset_id);
            let inv = invariants(&k);
            let mut counts = [0; 5];
            for under in [false, true] {
                for v in &k.code {
                    for m in moves::moves(&k, &v.e, under, None) {
                        assert!(valid(&m.to), "invalid {:?}", m.kind);
                        let result = invariants(&m.to);
                        assert_eq!(result.jones, inv.jones, "{:?}", m.kind);
                        let (index, delta) = match m.kind {
                            Kind::Twist => (0, 1),
                            Kind::Untwist => (1, -1),
                            Kind::Push => (2, 2),
                            Kind::Pull => (3, -2),
                            Kind::Slide => (4, 0),
                        };
                        counts[index] += 1;
                        assert_eq!(result.n as i32, inv.n as i32 + delta);
                        assert_eq!(diagram_key(&m.from), diagram_key(&k));
                        let restored = moves::cleanup(&m.from);
                        assert_eq!(restored.code, k.code);
                        let clean = moves::cleanup(&m.to);
                        assert!(valid(&clean));
                        assert_eq!(invariants(&clean).jones, inv.jones);
                        if invariants(&clean).n == 0 {
                            assert_eq!(clean.code.len(), 2);
                        } else {
                            assert!(clean.code.iter().all(|v| clean.nodes[&v.n].crossing()));
                        }
                    }
                }
            }
            assert!(counts[0] > 0);
            if preset_id != 4 {
                assert!(counts[2] > 0);
            }
        }
    }
    #[test]
    fn twists_and_pushes_can_be_undone() {
        let k = preset(2);
        for v in &k.code {
            for m in moves::moves(&k, &v.e, false, Some(&[Kind::Twist, Kind::Push])) {
                let after = moves::cleanup(&m.to);
                let kind = if m.kind == Kind::Twist {
                    Kind::Untwist
                } else {
                    Kind::Pull
                };
                let back = moves::moves(&after, &v.e, false, Some(&[kind]));
                assert!(!back.is_empty(), "no inverse for {:?} {}", m.kind, v.e);
                let result = moves::cleanup(&back[0].to);
                assert_eq!(invariants(&result).n, 3);
                assert_eq!(invariants(&result).jones, invariants(&k).jones);
            }
        }
    }
    #[test]
    fn unknot_twist_and_untwist_restores_two_pass_nodes() {
        let k = preset(4);
        let twist = moves::moves(&k, &k.code[0].e, false, Some(&[Kind::Twist])).remove(0);
        let k1 = moves::cleanup(&twist.to);
        let e = k1
            .code
            .iter()
            .enumerate()
            .find(|(i, v)| v.n == k1.code[(i + 1) % k1.code.len()].n)
            .unwrap()
            .1
            .e
            .clone();
        let untwist = moves::moves(&k1, &e, false, Some(&[Kind::Untwist])).remove(0);
        let done = moves::cleanup(&untwist.to);
        assert!(valid(&done));
        assert_eq!(done.code.len(), 2);
        assert_eq!(invariants(&done).n, 0);
    }
    #[test]
    fn r3_strand_and_crossing_moves_preserve_knot_type() {
        let mut strand_count = 0;
        let mut crossing_count = 0;
        'presets: for p in [2, 3] {
            let k = preset(p);
            let original = invariants(&k).jones;
            for v in &k.code {
                for push in moves::moves(&k, &v.e, false, Some(&[Kind::Push])) {
                    let after = moves::cleanup(&push.to);
                    for e in &after.code {
                        for m in moves::moves(&after, &e.e, false, Some(&[Kind::Slide])) {
                            assert!(valid(&m.to));
                            assert_eq!(invariants(&m.to).jones, original);
                            assert_eq!(invariants(&m.to).n, invariants(&after).n);
                            strand_count += 1;
                        }
                    }
                    for n in after.nodes.keys() {
                        for m in moves::crossing_moves(&after, n) {
                            assert!(valid(&m.to));
                            assert_eq!(invariants(&m.to).jones, original);
                            assert!(m.to.nodes[n].pos().distance(after.nodes[n].pos()) > 5.);
                            crossing_count += 1;
                        }
                    }
                    if strand_count > 8 && crossing_count > 8 {
                        break 'presets;
                    }
                }
            }
        }
        assert!(strand_count > 0);
        assert!(crossing_count > 0);
    }
}

/// Literal 80×62 diagrams from upstream's MoveLegend. The surrounding native
/// cards share the original canvas width; the knot itself retains 560×470 units.
fn draw_move_legend(scene: &mut Scene) {
    const INK: Color = [51, 65, 85, 255];
    const ORANGE: Color = [234, 88, 12, 255];
    fn strand(scene: &mut Scene, id: String, origin: Point, d: &str, over: bool) {
        let path = BezPath::from_svg(d).expect("upstream legend SVG");
        let mut o = Object::path(id, path, INK, 3.).line_cap(LineCap::Round);
        o.transform = Affine::translate(origin.to_vec2());
        if over {
            let shape = o.shapes[0].clone();
            if let Shape::Path { stroke, .. } = &mut o.shapes[0] {
                *stroke = Some(([255; 4], 9.));
            }
            o.shapes.push(shape);
        }
        scene.push(o);
    }
    fn drag(scene: &mut Scene, id: String, origin: Point, from: Point, to: Point) {
        let dir = (to - from) / (to - from).hypot();
        scene.push(
            Object::line(
                format!("{id}-drag"),
                origin + from.to_vec2(),
                origin + to.to_vec2() - dir * 6.,
                ORANGE,
                2.,
            )
            .dash(vec![4., 3.]),
        );
        let side = Vec2::new(-dir.y, dir.x) * 4.;
        let tip = origin + to.to_vec2();
        let mut p = BezPath::new();
        p.move_to(tip);
        p.line_to(tip - dir * 7. + side);
        p.line_to(tip - dir * 7. - side);
        p.close_path();
        scene.push(Object::new(
            format!("{id}-arrow"),
            Affine::IDENTITY,
            None,
            vec![Shape::Path {
                path: p,
                fill: Some(ORANGE),
                stroke: None,
            }],
        ));
        scene.push(
            Object::circle(format!("{id}-grab"), origin + from.to_vec2(), 4.5, ORANGE).decorative(),
        );
    }
    let captions = [
        "Drag a strand onto one of the small rings beside it to make a loop. Drag the loop back in to undo.",
        "Push a strand across a neighbor (hold Alt to go under). Drag it back to undo.",
        "A strand over (or under) both sides of a triangle slides across the crossing opposite it, or drag that crossing across the strand.",
    ];
    for i in 0..3 {
        let x = i as f64 * 188.;
        let id = format!("move-{i}");
        let mut card = Object::rect(
            format!("{id}-card"),
            Rect::new(x, 482., x + 184., 722.),
            8.,
            [255; 4],
        )
        .decorative();
        if let Shape::Rect { stroke, .. } = &mut card.shapes[0] {
            *stroke = Some(([229, 231, 235, 255], 1.));
        }
        scene.push(card);
        scene.push(
            Object::text(
                format!("{id}-title"),
                (x + 8., 502.),
                ["Twist (R1)", "Push (R2)", "Slide (R3)"][i],
                12.,
                [55, 65, 81, 255],
            )
            .font_weight(600)
            .text_baseline(TextBaseline::Alphabetic),
        );
        scene.push(
            Object::text(
                format!("{id}-between"),
                (x + 92., 543.),
                "→",
                18.,
                [156, 163, 175, 255],
            )
            .text_anchor(TextAnchor::Middle)
            .text_baseline(TextBaseline::Middle),
        );
        for after in [false, true] {
            let origin = Point::new(x + if after { 100. } else { 4. }, 510.);
            let sub = format!("{id}-{}", usize::from(after));
            let mut add = |n, d, over| strand(scene, format!("{sub}-{n}"), origin, d, over);
            match (i, after) {
                (0, false) => {
                    add(0, "M4,46 C30,46 50,46 76,46", false);
                    let mut ring = Object::circle(
                        format!("{sub}-ring"),
                        origin + Vec2::new(40., 10.),
                        5.,
                        [0; 4],
                    )
                    .decorative();
                    if let Shape::Circle { stroke, .. } = &mut ring.shapes[0] {
                        *stroke = Some(([148, 163, 184, 255], 1.5));
                    }
                    scene.push(ring);
                    drag(
                        scene,
                        sub,
                        origin,
                        Point::new(40., 46.),
                        Point::new(40., 17.),
                    );
                }
                (0, true) => {
                    add(
                        0,
                        "M4,46 C18,46 32,44 40,38 C50,30 58,18 50,10 C43,3 30,6 29,14",
                        false,
                    );
                    add(1, "M29,14 C28,22 33,32 40,38 C47,44 62,46 76,46", true);
                    scene.push(
                        Object::circle(
                            format!("{sub}-grab"),
                            origin + Vec2::new(42., 6.),
                            4.5,
                            ORANGE,
                        )
                        .decorative(),
                    );
                }
                (1, false) => {
                    add(0, "M4,44 L76,44", false);
                    add(1, "M4,16 L76,16", false);
                    drag(
                        scene,
                        sub,
                        origin,
                        Point::new(40., 16.),
                        Point::new(40., 56.),
                    );
                }
                (1, true) => {
                    add(0, "M4,44 L76,44", false);
                    add(1, "M4,16 C24,16 26,56 40,56 C54,56 56,16 76,16", true);
                    scene.push(
                        Object::circle(
                            format!("{sub}-grab"),
                            origin + Vec2::new(40., 56.),
                            4.5,
                            ORANGE,
                        )
                        .decorative(),
                    );
                }
                (2, _) => {
                    add(0, "M12,6 L68,58", false);
                    add(1, "M68,6 L12,58", true);
                    add(
                        2,
                        if after {
                            "M4,46 L76,46"
                        } else {
                            "M4,18 L76,18"
                        },
                        true,
                    );
                    if after {
                        scene.push(
                            Object::circle(
                                format!("{sub}-grab"),
                                origin + Vec2::new(40., 46.),
                                4.5,
                                ORANGE,
                            )
                            .decorative(),
                        );
                    } else {
                        drag(
                            scene,
                            sub,
                            origin,
                            Point::new(40., 18.),
                            Point::new(40., 48.),
                        );
                    }
                }
                _ => unreachable!(),
            }
        }
        let mut lines = Vec::new();
        let mut line = String::new();
        for word in captions[i].split_whitespace() {
            if !line.is_empty() && line.len() + word.len() > 26 {
                lines.push(std::mem::take(&mut line));
            }
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(word);
        }
        if !line.is_empty() {
            lines.push(line);
        }
        for (row, line) in lines.into_iter().enumerate() {
            scene.push(
                Object::text(
                    format!("{id}-caption-{row}"),
                    (x + 8., 588. + row as f64 * 16.),
                    line,
                    12.,
                    [107, 114, 128, 255],
                )
                .text_baseline(TextBaseline::Alphabetic),
            );
        }
    }
}
