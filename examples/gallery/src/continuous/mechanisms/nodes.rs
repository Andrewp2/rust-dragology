use super::*;

#[derive(Clone, PartialEq)]
enum End {
    Port { node: usize, port: usize },
    Free(Point),
}
#[derive(Clone, PartialEq)]
struct Wire {
    ends: [End; 2],
}
#[derive(Clone, PartialEq)]
pub(super) struct NodeState {
    positions: [Point; 3],
    wires: Vec<Option<Wire>>,
}
pub(super) struct Nodes;
fn count(node: usize, end: usize) -> usize {
    if end == 0 {
        usize::from(node != 2)
    } else if node == 0 {
        2
    } else {
        1
    }
}
fn height(node: usize) -> f64 {
    if node == 0 { 70. } else { 48. }
}
fn port_position(s: &NodeState, node: usize, end: usize, port: usize) -> Point {
    let h = height(node);
    let n = count(node, end);
    s.positions[node]
        + Vec2::new(
            if end == 0 { 90. } else { 0. },
            20. + (h - 20. - n as f64 * 22.) / 2. + 11. + port as f64 * 22.,
        )
}
fn end_position(s: &NodeState, end: usize, value: &End) -> Point {
    match value {
        End::Port { node, port } => port_position(s, *node, end, *port),
        End::Free(p) => *p,
    }
}
fn port_id(node: usize, end: usize, port: usize) -> String {
    format!("port-{node}-{end}-{port}")
}
fn parse_port(id: &str) -> Option<(usize, usize, usize)> {
    let mut parts = id
        .strip_prefix("port-")
        .or_else(|| id.strip_prefix("portlabel-"))?
        .split('-');
    Some((
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
    ))
}
fn parse_end(id: &str) -> Option<(usize, usize)> {
    let (wire, end) = id.strip_prefix("wire-")?.split_once('-')?;
    Some((wire.parse().ok()?, end.parse().ok()?))
}
fn occupied(s: &NodeState, node: usize, end: usize, port: usize) -> bool {
    s.wires
        .iter()
        .flatten()
        .any(|w| w.ends[end] == End::Port { node, port })
}
fn end_drag(s: &NodeState, wire: usize, end: usize) -> DragSpec<'static, NodeState> {
    let mut candidates = Vec::new();
    for node in 0..3 {
        for port in 0..count(node, end) {
            let mut next = s.clone();
            next.wires[wire].as_mut().unwrap().ends[end] = End::Port { node, port };
            candidates.push(fixed(next));
        }
    }
    let p = end_position(s, end, &s.wires[wire].as_ref().unwrap().ends[end]);
    let base = s.clone();
    let free = vary([p.x, p.y], move |[x, y]| {
        let mut next = base.clone();
        next.wires[wire].as_mut().unwrap().ends[end] = End::Free(Point::new(x, y));
        next
    })
    .on_drop(move |s| {
        let mut next = s.clone();
        if next.wires[wire]
            .as_ref()
            .is_some_and(|w| w.ends.iter().all(|e| matches!(e, End::Free(_))))
        {
            next.wires[wire] = None;
        }
        next
    });
    closest(candidates).when_far(free, 20.)
}
impl Example for Nodes {
    type State = NodeState;
    fn anchor(&self, _: &NodeState, id: &str, local: Point) -> Point {
        if parse_port(id).is_some() {
            Point::ORIGIN
        } else {
            local
        }
    }
    fn initial(&self) -> NodeState {
        NodeState {
            positions: [
                Point::new(20., 30.),
                Point::new(200., 10.),
                Point::new(380., 40.),
            ],
            wires: vec![
                None,
                Some(Wire {
                    ends: [
                        End::Port { node: 0, port: 0 },
                        End::Port { node: 1, port: 0 },
                    ],
                }),
                Some(Wire {
                    ends: [
                        End::Port { node: 1, port: 0 },
                        End::Free(Point::new(340., 130.)),
                    ],
                }),
            ],
        }
    }
    fn scene(&self, s: &NodeState) -> Scene {
        self.scene_dragging(s, None)
    }
    fn scene_dragging(&self, s: &NodeState, dragged: Option<&str>) -> Scene {
        let mut scene = Scene::new(500., 200.);
        for node in 0..3 {
            let mut object = Object::rect(
                format!("node-{node}"),
                Rect::from_origin_size(s.positions[node], (90., height(node))),
                5.,
                rgb(0xfdfdfd),
            );
            if let Shape::Rect { stroke, .. } = &mut object.shapes[0] {
                *stroke = Some((rgb(0xbbbbbb), 1.2));
            }
            object
                .shapes
                .extend(Object::line("header", (0., 20.), (90., 20.), rgb(0xdddddd), 1.).shapes);
            scene.push(
                object
                    .label(
                        ["Mix", "Filter", "Output"][node],
                        (45., 11.),
                        11.,
                        rgb(0x444444),
                    )
                    .text_anchor(TextAnchor::Middle)
                    .text_baseline(TextBaseline::Middle)
                    .font_weight(600),
            );
            for end in 0..2 {
                for port in 0..count(node, end) {
                    let connected = occupied(s, node, end, port);
                    let p = port_position(s, node, end, port);
                    let color = rgb(if end == 0 {
                        if connected { 0xff6b4a } else { 0xf0c8c0 }
                    } else {
                        if connected { 0x4a9eff } else { 0xc0d8f0 }
                    });
                    let mut object =
                        stroked_circle(port_id(node, end, port), p, 5., color, [255; 4], 1.5);
                    object.parent = Some(format!("node-{node}"));
                    object.cursor = Cursor::Crosshair;
                    if connected {
                        object.hit = None;
                    }
                    scene.push(object);
                    let label = if end == 0 {
                        "out"
                    } else if node == 0 {
                        ["a", "b"][port]
                    } else {
                        "in"
                    };
                    let x = if end == 0 { -9. } else { 9. };
                    let style = TextStyle {
                        anchor: if end == 0 {
                            TextAnchor::End
                        } else {
                            TextAnchor::Start
                        },
                        baseline: TextBaseline::Middle,
                        ..Default::default()
                    };
                    let metrics = crate::text_layout::measure_text(label, 9., &style);
                    let logical = crate::text_layout::outlines(label, 9., &style)
                        .logical_bounds
                        .unwrap_or_default();
                    let w = metrics.advance;
                    let mut label_object = Object::new(
                        format!("portlabel-{node}-{end}-{port}"),
                        Affine::translate(p.to_vec2()),
                        if connected {
                            None
                        } else {
                            Some(Hit::Rect(Rect::new(
                                if end == 0 { x - w } else { x },
                                logical.y0,
                                if end == 0 { x } else { x + w },
                                logical.y1,
                            )))
                        },
                        vec![Shape::Text {
                            origin: Point::new(x, 0.),
                            text: label.into(),
                            size: 9.,
                            color: rgb(0x999999),
                        }],
                    )
                    .text_style(style);
                    label_object.parent = Some(format!("node-{node}"));
                    label_object.cursor = Cursor::Crosshair;
                    scene.push(label_object);
                }
            }
        }
        for (i, wire) in s.wires.iter().enumerate() {
            let Some(wire) = wire else { continue };
            let a = end_position(s, 0, &wire.ends[0]);
            let b = end_position(s, 1, &wire.ends[1]);
            let dx = ((b.x - a.x).abs() * 0.4).max(30.);
            let mut curve = BezPath::new();
            curve.move_to(a);
            curve.curve_to(a + Vec2::new(dx, 0.), b - Vec2::new(dx, 0.), b);
            scene.push(Object::path(
                format!("noodle-{i}"),
                curve,
                rgb(0xaaaaaa),
                2.,
            ));
            for end in 0..2 {
                let free = matches!(wire.ends[end], End::Free(_));
                scene.push(stroked_circle(
                    format!("wire-{i}-{end}"),
                    end_position(s, end, &wire.ends[end]),
                    if free { 6. } else { 5. },
                    if free { rgb(0xcccccc) } else { [0; 4] },
                    if free { rgb(0x999999) } else { [0; 4] },
                    if free { 1. } else { 0. },
                ));
            }
        }
        for o in &mut scene.objects {
            if let Some((i, e)) = parse_end(&o.id) {
                if matches!(s.wires[i].as_ref().unwrap().ends[e], End::Port { .. }) {
                    o.cursor = Cursor::Crosshair;
                }
            }
        }
        scene.objects.sort_by_key(|o| {
            if o.id.starts_with("port-") {
                10
            } else if o.id.starts_with("wire-") || o.id.starts_with("noodle-") {
                3
            } else if dragged == Some(o.id.as_str())
                || o.parent.as_deref() == dragged && dragged.is_some()
            {
                5
            } else {
                1
            }
        });
        source_cursors(scene)
    }
    fn locate(&self, s: &NodeState, id: &str) -> Option<Affine> {
        let p = if let Some(node) = index(id, "node-") {
            *s.positions.get(node)?
        } else if let Some((node, end, port)) = parse_port(id) {
            if node >= 3 || end >= 2 || port >= count(node, end) {
                return None;
            }
            port_position(s, node, end, port)
        } else {
            let (wire, end) = parse_end(id)?;
            if end >= 2 {
                return None;
            }
            end_position(s, end, &s.wires.get(wire)?.as_ref()?.ends[end])
        };
        Some(Affine::translate(p.to_vec2()))
    }
    fn begin_drag(&self, s: &NodeState, id: &str, _: Modifiers) -> Option<(NodeState, String)> {
        let (node, end, port) = parse_port(id)?;
        if node >= 3 || end >= 2 || port >= count(node, end) || occupied(s, node, end, port) {
            return None;
        }
        let mut next = s.clone();
        let wire = next
            .wires
            .iter()
            .position(Option::is_none)
            .unwrap_or(next.wires.len());
        let mut ends = [
            End::Free(port_position(s, node, end, port)),
            End::Free(port_position(s, node, end, port)),
        ];
        ends[end] = End::Port { node, port };
        if wire == next.wires.len() {
            next.wires.push(Some(Wire { ends }));
        } else {
            next.wires[wire] = Some(Wire { ends });
        }
        Some((next, format!("wire-{wire}-{}", 1 - end)))
    }
    fn drag(&self, s: &NodeState, id: &str, _: Modifiers) -> Option<DragSpec<'static, NodeState>> {
        if let Some(node) = index(id, "node-") {
            let p = *s.positions.get(node)?;
            let base = s.clone();
            return Some(vary([p.x, p.y], move |[x, y]| {
                let mut next = base.clone();
                next.positions[node] = Point::new(x, y);
                next
            }));
        }
        if let Some((wire, end)) = parse_end(id) {
            s.wires.get(wire)?.as_ref()?;
            return (end < 2).then(|| end_drag(s, wire, end));
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dragology::DragSession;

    #[test]
    fn port_label_hit_bounds_use_the_renderers_shaped_text_metrics() {
        let scene = Nodes.scene(&Nodes.initial());
        for object in scene
            .objects
            .iter()
            .filter(|object| object.id.starts_with("portlabel-"))
        {
            let Some(Hit::Rect(hit)) = object.hit else {
                continue;
            };
            let Shape::Text { text, size, .. } = &object.shapes[0] else {
                panic!("port label missing")
            };
            let metrics = crate::text_layout::measure_text(text, *size, &object.text_style);
            let bounds = crate::text_layout::outlines(text, *size, &object.text_style)
                .logical_bounds
                .unwrap();
            assert!((hit.width() - metrics.advance).abs() < 1e-9);
            assert!((hit.y0 - bounds.y0).abs() < 1e-9);
            assert!((hit.y1 - bounds.y1).abs() < 1e-9);
        }
    }

    #[test]
    fn port_label_creates_wire_immediately_and_cancel_restores_original() {
        let mut d = demo(Nodes);
        d.set_animated(false);
        let m = Modifiers::default();
        // The input label is separate from the five-pixel port circle.
        d.press(Point::new(31., 64.), m, 1);
        assert!(d.scene().object("wire-0-0").is_some());
        d.move_to(Point::new(140., 160.), m);
        d.advance(std::time::Duration::ZERO);
        let tip = d.scene().object("wire-0-0").unwrap().transform * Point::ORIGIN;
        assert!(tip.distance(Point::new(140., 160.)) < 0.01);
        d.cancel();
        assert!(d.scene().object("wire-0-0").is_none());
        assert_eq!(
            d.scene().object("node-0").unwrap().transform * Point::ORIGIN,
            Point::new(20., 30.)
        );
    }
    #[test]
    fn unused_port_creates_a_wire_and_detaching_both_ends_deletes_it() {
        let state = Nodes.initial();
        let id = "port-0-1-0";
        let press = Nodes.locate(&state, id).unwrap() * Point::ORIGIN;
        let (state, follow) = Nodes.begin_drag(&state, id, Modifiers::default()).unwrap();
        let spec = Nodes.drag(&state, &follow, Modifiers::default()).unwrap();
        let next = DragSession::begin(state, press, move |s| Nodes.locate(s, &follow), spec)
            .unwrap()
            .finish(Point::new(100., 180.))
            .unwrap();
        let wire = next.wires[0].as_ref().unwrap();
        assert!(matches!(wire.ends[0], End::Free(_)));
        assert!(matches!(wire.ends[1], End::Port { node: 0, port: 0 }));
        let id = "wire-0-1";
        let press = Nodes.locate(&next, id).unwrap() * Point::ORIGIN;
        let spec = Nodes.drag(&next, id, Modifiers::default()).unwrap();
        let removed = DragSession::begin(next, press, move |s| Nodes.locate(s, id), spec)
            .unwrap()
            .finish(Point::new(100., 180.))
            .unwrap();
        assert!(removed.wires[0].is_none());
    }

    #[test]
    fn moving_a_node_carries_its_connected_end() {
        let mut s = Nodes.initial();
        let wire = s.wires[1].as_ref().unwrap().clone();
        let before = end_position(&s, 0, &wire.ends[0]);
        s.positions[0] += Vec2::new(45., -17.);
        assert_eq!(
            end_position(&s, 0, &wire.ends[0]) - before,
            Vec2::new(45., -17.)
        );
        assert_eq!(end_position(&s, 1, &wire.ends[1]), Point::new(200., 44.));
    }
}
