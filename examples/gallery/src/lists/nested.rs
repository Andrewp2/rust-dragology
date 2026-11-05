use super::*;
use dragology::Preview;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
struct Node {
    id: String,
    label: String,
    children: Option<Vec<Node>>,
    color: Color,
    position: Point,
}
impl Node {
    fn tile(id: &str) -> Self {
        Self {
            id: id.into(),
            label: id.into(),
            children: None,
            color: WHITE,
            position: Point::ORIGIN,
        }
    }
    fn row(id: &str, children: Vec<Node>, color: Color, position: Point) -> Self {
        Self {
            id: id.into(),
            label: String::new(),
            children: Some(children),
            color,
            position,
        }
    }
    fn size(&self) -> (f64, f64) {
        match &self.children {
            None => (50., 50.),
            Some(children) => (
                34. + children.iter().map(|c| c.size().0).sum::<f64>()
                    + 8. * (children.len() as f64 - 1.),
                16. + children.iter().map(|c| c.size().1).fold(50., f64::max),
            ),
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub(super) struct NestedState {
    roots: Vec<Node>,
    serial: usize,
    indicator: Option<(Point, f64)>,
}
pub(super) struct Nested(pub bool);

fn take(nodes: &mut Vec<Node>, id: &str) -> Option<Node> {
    if let Some(i) = nodes.iter().position(|n| n.id == id) {
        return Some(nodes.remove(i));
    }
    for node in nodes {
        if let Some(children) = &mut node.children
            && let Some(found) = take(children, id)
        {
            return Some(found);
        }
    }
    None
}
fn find_mut<'a>(nodes: &'a mut [Node], id: &str) -> Option<&'a mut Node> {
    for n in nodes {
        if n.id == id {
            return Some(n);
        }
        if let Some(children) = &mut n.children
            && let Some(found) = find_mut(children, id)
        {
            return Some(found);
        }
    }
    None
}
fn destinations(nodes: &[Node], result: &mut Vec<(String, usize)>) {
    for node in nodes {
        if let Some(children) = &node.children {
            for i in 0..=children.len() {
                result.push((node.id.clone(), i));
            }
            destinations(children, result);
        }
    }
}
// The imperative reference scores insertion lines, rather than hypothetical
// layouts. The pointer can approach any point along a tall nested container.
fn zones(nodes: &[Node], out: &mut Vec<(String, usize, Point, f64)>, parent: Option<Point>) {
    fn visit(node: &Node, origin: Point, out: &mut Vec<(String, usize, Point, f64)>) {
        if let Some(children) = &node.children {
            let mut x = origin.x + 22.;
            let h = node.size().1;
            for i in 0..=children.len() {
                out.push((node.id.clone(), i, Point::new(x, origin.y), h));
                if let Some(child) = children.get(i) {
                    visit(child, Point::new(x + 4., origin.y + 8.), out);
                    x += child.size().0 + 8.;
                }
            }
        }
    }
    let mut next_x = parent.map_or(0., |p| p.x + 26.);
    for node in nodes {
        let origin = parent.map_or(node.position, |p| Point::new(next_x, p.y + 8.));
        visit(node, origin, out);
        next_x += node.size().0 + 8.;
    }
}

fn draw(scene: &mut Scene, node: &Node, position: Point, parent: Option<&str>) {
    let (w, h) = node.size();
    let mut object = if let Some(children) = &node.children {
        let mut object = stroked(
            Object::rect(
                &node.id,
                Rect::from_origin_size(position, (w, h)),
                6.,
                node.color,
            ),
            BORDER,
            1.5,
        );
        grip(&mut object, h);
        // The row shape precedes its children, which have their own hit targets.
        object.parent = parent.map(str::to_owned);
        scene.push(object);
        let mut x = position.x + 26.;
        for child in children {
            draw(scene, child, Point::new(x, position.y + 8.), Some(&node.id));
            x += child.size().0 + 8.;
        }
        return;
    } else {
        tile_object(
            &node.id,
            &node.label,
            Rect::from_origin_size(position, (w, h)),
        )
    };
    object.parent = parent.map(str::to_owned);
    scene.push(object);
}

impl Example for Nested {
    type State = NestedState;
    fn initial(&self) -> NestedState {
        let mut roots = vec![
            Node::row(
                "row1",
                vec![
                    Node::tile("A1"),
                    Node::tile("B1"),
                    Node::row(
                        "row1-1",
                        vec![Node::tile("A1-1"), Node::tile("B1-1")],
                        CANVAS_COLORS[3],
                        Point::ORIGIN,
                    ),
                ],
                CANVAS_COLORS[0],
                Point::ORIGIN,
            ),
            Node::row(
                "row2",
                ["A2", "B2", "C2"].into_iter().map(Node::tile).collect(),
                CANVAS_COLORS[1],
                Point::new(20., 100.),
            ),
        ];
        if self.0 {
            roots.push(Node::row(
                "row3",
                ["A3", "B3", "C3"].into_iter().map(Node::tile).collect(),
                CANVAS_COLORS[2],
                Point::new(100., 200.),
            ));
        }
        NestedState {
            roots,
            serial: 0,
            indicator: None,
        }
    }
    fn scene(&self, s: &NestedState) -> Scene {
        let mut scene = Scene::new(if self.0 { 600. } else { 400. }, 400.);
        for root in &s.roots {
            draw(&mut scene, root, root.position, None);
        }
        if !self.0 {
            // Each row and slot adds a zero to the source's stacking path.
            // Deeper ID layers therefore paint above all shallower layers;
            // the vanilla comparison instead keeps ordinary subtree order.
            let mut depths = std::collections::HashMap::new();
            for object in &scene.objects {
                let depth = object
                    .parent
                    .as_ref()
                    .map_or(0, |parent| depths[parent] + 1);
                depths.insert(object.id.clone(), depth);
            }
            scene.objects.sort_by_key(|object| depths[&object.id]);
        }
        if self.0 {
            for o in &mut scene.objects {
                if o.hit.is_some() {
                    o.cursor = Cursor::Grab;
                }
            }
        }
        if let Some((p, h)) = s.indicator {
            scene.push(
                Object::rect(
                    "drop-indicator",
                    Rect::new(p.x - 1.5, p.y + 4., p.x + 1.5, p.y + h - 4.),
                    1.5,
                    [102, 102, 102, 255],
                )
                .decorative(),
            );
        }
        scene
    }
    fn drop_transition(&self, _: &NestedState) -> Option<dragology::Transition> {
        if self.0 {
            None
        } else {
            Some(dragology::Transition::default())
        }
    }
    fn scene_dragging(&self, s: &NestedState, id: Option<&str>) -> Scene {
        let mut scene = self.scene(s);
        if let Some(id) = id {
            let mut lifted = std::collections::BTreeSet::from([id.to_owned()]);
            for object in &scene.objects {
                if object.parent.as_ref().is_some_and(|p| lifted.contains(p)) {
                    lifted.insert(object.id.clone());
                }
            }
            if self.0 {
                for object in &mut scene.objects {
                    if lifted.contains(&object.id) {
                        object.opacity = 0.7;
                    }
                }
            }
            scene.objects.sort_by_key(|o| lifted.contains(&o.id));
        }
        scene
    }
    fn drag(
        &self,
        s: &NestedState,
        id: &str,
        _: Modifiers,
    ) -> Option<DragSpec<'static, NestedState>> {
        let mut without = s.clone();
        without.indicator = None;
        let item = take(&mut without.roots, id)?;
        let mut targets = Vec::new();
        destinations(&without.roots, &mut targets);
        let candidates = targets
            .into_iter()
            .map(|(row, i)| {
                let mut candidate = without.clone();
                find_mut(&mut candidate.roots, &row)
                    .unwrap()
                    .children
                    .as_mut()
                    .unwrap()
                    .insert(i, item.clone());
                candidate
            })
            .collect::<Vec<_>>();
        let mut detached = without.clone();
        let dragged = item.clone();
        let top = if item.children.is_none() {
            let id = format!("detached-{}", detached.serial);
            detached.serial += 1;
            Node::row(
                &id,
                vec![item],
                CANVAS_COLORS[detached.roots.len() % CANVAS_COLORS.len()],
                Point::ORIGIN,
            )
        } else {
            Node {
                position: Point::ORIGIN,
                ..item
            }
        };
        detached.roots.push(top);
        let last = detached.roots.len() - 1;
        if self.0 {
            let mut drop_zones = Vec::new();
            zones(&without.roots, &mut drop_zones, None);
            let zone_states = drop_zones
                .iter()
                .map(|(row, index, _, _)| {
                    let mut state = without.clone();
                    find_mut(&mut state.roots, row)
                        .unwrap()
                        .children
                        .as_mut()
                        .unwrap()
                        .insert(*index, dragged.clone());
                    state
                })
                .collect::<Vec<_>>();
            return Some(
                DragSpec::custom(move |ctx, pointer| {
                    let closest = drop_zones
                        .iter()
                        .enumerate()
                        .map(|(i, (_, _, p, h))| {
                            let y = pointer.y.clamp(p.y, p.y + h);
                            (i, pointer.distance(Point::new(p.x, y)))
                        })
                        .min_by(|a, b| a.1.total_cmp(&b.1));
                    let mut backdrop = without.clone();
                    let next = if let Some((i, _)) = closest.filter(|(_, d)| *d < 50.) {
                        let (_, _, p, h) = drop_zones[i];
                        backdrop.indicator = Some((p, h));
                        zone_states[i].clone()
                    } else {
                        let mut next = detached.clone();
                        next.roots[last].position = pointer - ctx.anchor().to_vec2();
                        next
                    };
                    let mut frame = ctx.state_frame(Rc::new(next), pointer)?;
                    frame.preview = Rc::new(Preview::State(Rc::new(backdrop)));
                    frame.grab_position = None;
                    Ok(frame)
                })
                .with_floating(),
            );
        }
        let p = detached.roots[last].position;
        let free = vary([p.x, p.y], move |[x, y]| {
            let mut next = detached.clone();
            next.roots[last].position = Point::new(x, y);
            next
        });
        Some(closest(candidates).with_floating().when_far(free, 60.))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn declarative_nesting_keeps_tiles_above_overlapping_root_backgrounds() {
        let mut app = crate::runtime::ModelDemo::new(Nested(false));
        app.set_animated(false);
        // Detach row2 so a gap between its tiles lies over A1. Root
        // backgrounds remain below all tiles in the source's ID layers.
        app.press(Point::new(28., 133.), Modifiers::default(), 1);
        app.release(Point::new(-32., 13.), Modifiers::default());
        let row = app.scene().object("row2").unwrap().transform;
        assert!((row * Point::ORIGIN).distance(Point::new(-40., -20.)) < 0.01);
        app.press(Point::new(40., 20.), Modifiers::default(), 1);
        app.release(Point::new(550., 350.), Modifiers::default());
        assert_eq!(app.scene().object("row2").unwrap().transform, row);
        assert!(
            (app.scene().object("A1").unwrap().transform * Point::ORIGIN).x > 400.,
            "a tile must remain reachable above an overlapping root background"
        );
    }
    #[test]
    fn declarative_and_vanilla_nesting_preserve_their_distinct_stacking_rules() {
        for vanilla in [false, true] {
            let example = Nested(vanilla);
            let mut state = example.initial();
            state.roots[1].position = Point::new(180., 0.);
            let scene = example.scene(&state);
            let deep = scene.objects.iter().position(|o| o.id == "A1-1").unwrap();
            let shallow = scene.objects.iter().position(|o| o.id == "A2").unwrap();
            // The overlapping deeper tile rises above a later root's tiles
            // only in the declarative renderer. Vanilla retains DOM order.
            assert_eq!(deep > shallow, !vanilla);
        }
    }
    #[test]
    fn vanilla_target_scores_the_full_height_of_the_insertion_line() {
        let example = Nested(true);
        let start = example.initial();
        let source = example.locate(&start, "A3").unwrap() * Point::new(20., 20.);
        let mut without = start.clone();
        take(&mut without.roots, "A3").unwrap();
        let mut all = Vec::new();
        zones(&without.roots, &mut all, None);
        let (_, _, p, h) = all
            .iter()
            .find(|(id, i, _, _)| id == "row1" && *i == 0)
            .unwrap();
        let destination = Point::new(p.x, p.y + h - 1.);
        let spec = example.drag(&start, "A3", Modifiers::default()).unwrap();
        let result =
            dragology::DragSession::begin(start, source, |s| Nested(true).locate(s, "A3"), spec)
                .unwrap()
                .finish(destination)
                .unwrap();
        assert_eq!(result.roots[0].children.as_ref().unwrap()[0].id, "A3");
        assert_eq!(result.roots.len(), 3);
    }
    #[test]
    fn removing_a_subtree_excludes_its_descendants_from_drop_destinations() {
        let mut state = Nested(false).initial();
        let subtree = take(&mut state.roots, "row1").unwrap();
        assert!(subtree.children.is_some());
        let mut targets = Vec::new();
        destinations(&state.roots, &mut targets);
        assert!(targets.iter().all(|(id, _)| id != "row1" && id != "row1-1"));
        assert!(targets.iter().any(|(id, _)| id == "row2"));
    }
}
