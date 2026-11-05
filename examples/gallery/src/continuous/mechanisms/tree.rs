use super::*;
use std::collections::BTreeMap;

#[derive(Clone, PartialEq)]
struct Tree {
    id: usize,
    children: Vec<Tree>,
    emerge: Option<usize>,
}
#[derive(Clone, PartialEq)]
pub(super) struct TreeState {
    root: Tree,
    next: usize,
    compact: bool,
}
pub(super) struct SproutingTree;
fn leaf_count(node: &Tree) -> usize {
    if node.children.is_empty() {
        1
    } else {
        node.children.iter().map(leaf_count).sum()
    }
}
fn standard_layout(node: &Tree, x: f64, y: f64, positions: &mut BTreeMap<usize, Point>) {
    positions.insert(node.id, Point::new(x, y));
    let mut left = x - leaf_count(node) as f64 * 25.;
    for child in &node.children {
        let width = leaf_count(child) as f64 * 50.;
        standard_layout(child, left + width / 2., y + 60., positions);
        left += width;
    }
}
fn compact_layout(node: &Tree) -> (BTreeMap<usize, Point>, Vec<(f64, f64)>) {
    let mut positions = BTreeMap::from([(node.id, Point::ORIGIN)]);
    if node.children.is_empty() {
        return (positions, vec![(0., 0.)]);
    }
    let children = node.children.iter().map(compact_layout).collect::<Vec<_>>();
    let mut shifts = vec![0.];
    for i in 1..children.len() {
        let mut shift = 0_f64;
        for (a, b) in children[i - 1].1.iter().zip(&children[i].1) {
            shift = shift.max(a.1 - b.0 + 50.);
        }
        shifts.push(shifts[i - 1] + shift);
    }
    let offset = -shifts.last().copied().unwrap() / 2.;
    let mut contour: Vec<(f64, f64)> = vec![(0., 0.)];
    for (i, (points, extents)) in children.into_iter().enumerate() {
        let dx = shifts[i] + offset;
        for (id, p) in points {
            positions.insert(id, p + Vec2::new(dx, 60.));
        }
        for (d, (min, max)) in extents.into_iter().enumerate() {
            if let Some(current) = contour.get_mut(d + 1) {
                current.0 = current.0.min(min + dx);
                current.1 = current.1.max(max + dx);
            } else {
                contour.push((min + dx, max + dx));
            }
        }
    }
    (positions, contour)
}
fn positions(s: &TreeState) -> BTreeMap<usize, Point> {
    if s.compact {
        compact_layout(&s.root)
            .0
            .into_iter()
            .map(|(id, p)| (id, p + Vec2::new(250., 40.)))
            .collect()
    } else {
        let mut positions = BTreeMap::new();
        standard_layout(&s.root, 250., 40., &mut positions);
        positions
    }
}
fn find(node: &Tree, id: usize) -> Option<&Tree> {
    if node.id == id {
        Some(node)
    } else {
        node.children.iter().find_map(|c| find(c, id))
    }
}
fn sprout(node: &Tree, id: usize, keep: usize, next: usize) -> Tree {
    if node.id == id && node.children.is_empty() {
        let kept = Tree {
            id,
            children: vec![],
            emerge: None,
        };
        let sibling = Tree {
            id: next + 1,
            children: vec![],
            emerge: Some(id),
        };
        return Tree {
            id: next,
            children: if keep == 0 {
                vec![kept, sibling]
            } else {
                vec![sibling, kept]
            },
            emerge: Some(id),
        };
    }
    Tree {
        id: node.id,
        emerge: None,
        children: node
            .children
            .iter()
            .map(|c| sprout(c, id, keep, next))
            .collect(),
    }
}
fn prune(node: &Tree, id: usize) -> Tree {
    if node.children.iter().any(|child| child.id == id) {
        Tree {
            id,
            children: vec![],
            emerge: None,
        }
    } else {
        Tree {
            id: node.id,
            emerge: None,
            children: node.children.iter().map(|c| prune(c, id)).collect(),
        }
    }
}
fn draw_node(scene: &mut Scene, node: &Tree, positions: &BTreeMap<usize, Point>) {
    let p = positions[&node.id];
    for (i, child) in node.children.iter().enumerate() {
        let mut edge = Object::line(
            format!("edge-{}-{i}", node.id),
            p,
            positions[&child.id],
            [204, 204, 204, 255],
            2.,
        );
        edge.emerge_from = node.emerge.or(child.emerge).map(|id| format!("n{id}"));
        scene.push(edge);
        draw_node(scene, child, positions);
    }
    let leaf = node.children.is_empty();
    let mut circle = Object::circle(
        format!("n{}", node.id),
        p,
        14.,
        if leaf {
            [124, 179, 240, 255]
        } else {
            [232, 232, 232, 255]
        },
    );
    if let Some(Shape::Circle { stroke, .. }) = circle.shapes.first_mut() {
        *stroke = Some((if leaf { rgb(0x4a90d9) } else { rgb(0xbbbbbb) }, 2.));
    }
    if !leaf {
        circle.hit = None;
    }
    circle.emerge_from = node.emerge.map(|id| format!("n{id}"));
    scene.push(circle);
}
impl Example for SproutingTree {
    type State = TreeState;
    fn reset(&self, s: &Self::State) -> Self::State {
        TreeState {
            compact: s.compact,
            ..self.initial()
        }
    }
    fn action_transition(&self, _: &Self::State, _: &str) -> Option<dragology::Transition> {
        None
    }
    fn initial(&self) -> TreeState {
        TreeState {
            root: Tree {
                id: 0,
                children: vec![],
                emerge: None,
            },
            next: 1,
            compact: false,
        }
    }
    fn scene(&self, s: &TreeState) -> Scene {
        self.scene_dragging(s, None)
    }
    fn scene_dragging(&self, s: &TreeState, dragged: Option<&str>) -> Scene {
        let mut scene = Scene::new(500., 300.);
        draw_node(&mut scene, &s.root, &positions(s));
        scene.objects.sort_by_key(|o| {
            if o.id.starts_with("edge-") {
                0
            } else if dragged == Some(o.id.as_str()) {
                3
            } else {
                1
            }
        });
        source_cursors(scene)
    }
    fn locate(&self, s: &TreeState, id: &str) -> Option<Affine> {
        Some(Affine::translate(
            positions(s).get(&index(id, "n")?)?.to_vec2(),
        ))
    }
    fn drag(&self, s: &TreeState, id: &str, _: Modifiers) -> Option<DragSpec<'static, TreeState>> {
        let id = index(id, "n")?;
        if !find(&s.root, id)?.children.is_empty() {
            return None;
        }
        let mut branches = Vec::new();
        for keep in 0..2 {
            let mut next = s.clone();
            next.root = sprout(&s.root, id, keep, s.next);
            next.next += 2;
            branches.push(between([s.clone(), next]));
        }
        if s.root.id != id {
            let mut next = s.clone();
            next.root = prune(&s.root, id);
            branches.push(between([s.clone(), next]));
        }
        Some(closest(branches).with_snap_radius_options(
            5.,
            dragology::SnapOptions {
                chain: true,
                ..Default::default()
            },
        ))
    }
    fn controls(&self, s: &TreeState) -> Vec<Control> {
        vec![Control::toggle("compact", "Compact layout", s.compact)]
    }
    fn action(&self, s: &TreeState, id: &str, value: Value) -> Option<TreeState> {
        let Value::Bool(compact) = value else {
            return None;
        };
        (id == "compact").then(|| TreeState {
            compact,
            ..s.clone()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sprouting_and_pruning_keep_dragged_identity() {
        let s = SproutingTree.initial();
        let grown = sprout(&s.root, 0, 0, 1);
        assert_eq!(leaf_count(&grown), 2);
        assert!(find(&grown, 0).unwrap().children.is_empty());
        assert_eq!(grown.emerge, Some(0));
        let pruned = prune(&grown, 0);
        assert_eq!(pruned.id, 0);
        assert!(pruned.children.is_empty());
    }
}
