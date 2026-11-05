use super::*;
use dragology::drop_target;

pub fn entries() -> Vec<Entry> {
    vec![
        Entry::new(
            "card-piles",
            "Card piles",
            "Drag cards to move them. Overlapping cards form piles. Drag the grey background to move a whole pile.",
            || demo(CardPiles),
        ),
        Entry::new("tactile-tessellations", "Tactile tessellations", "", || {
            demo(Tessellations)
        }),
    ]
}

#[derive(Clone, PartialEq)]
struct CardsState {
    positions: Vec<Point>,
}
struct CardPiles;
fn card_groups(s: &CardsState) -> Vec<Vec<usize>> {
    let mut groups: Vec<Vec<usize>> = Vec::new();
    for i in 0..s.positions.len() {
        if groups.iter().any(|g| g.contains(&i)) {
            continue;
        }
        let mut group = vec![i];
        let mut cursor = 0;
        while cursor < group.len() {
            let a = s.positions[group[cursor]];
            for (j, b) in s.positions.iter().enumerate() {
                if !group.contains(&j) && (a.x - b.x).abs() < 70. && (a.y - b.y).abs() < 100. {
                    group.push(j);
                }
            }
            cursor += 1;
        }
        group.sort_unstable();
        groups.push(group);
    }
    groups
}
fn pile_bounds(s: &CardsState, ids: &[usize]) -> Rect {
    ids.iter().fold(
        Rect::new(
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ),
        |b, &i| {
            let p = s.positions[i];
            Rect::new(
                b.x0.min(p.x - 12.),
                b.y0.min(p.y - 12.),
                b.x1.max(p.x + 82.),
                b.y1.max(p.y + 112.),
            )
        },
    )
}
fn pile_indices(id: &str) -> Option<Vec<usize>> {
    id.strip_prefix("pile-")?
        .split('-')
        .map(|s| s.parse().ok())
        .collect()
}
impl Example for CardPiles {
    type State = CardsState;
    fn initial(&self) -> Self::State {
        CardsState {
            positions: vec![
                (120., 120.),
                (165., 150.),
                (320., 110.),
                (360., 140.),
                (220., 300.),
                (400., 300.),
            ]
            .into_iter()
            .map(Point::from)
            .collect(),
        }
    }
    fn scene(&self, s: &Self::State) -> Scene {
        let mut scene = Scene::new(500., 400.);
        for group in card_groups(s).into_iter().filter(|g| g.len() > 1) {
            let id = format!(
                "pile-{}",
                group
                    .iter()
                    .map(usize::to_string)
                    .collect::<Vec<_>>()
                    .join("-")
            );
            let mut pile = source_rect(id, pile_bounds(s, &group), 12., rgb(0xd1d5db), None);
            pile.opacity = 0.4;
            scene.push(pile);
        }
        for (i, &p) in s.positions.iter().enumerate() {
            scene.push(source_rect(
                format!("card-{i}"),
                Rect::from_origin_size(p, (70., 100.)),
                6.,
                WHITE,
                Some((rgb(0x9ca3af), 1.5)),
            ));
        }
        scene
    }
    fn scene_dragging(&self, s: &Self::State, id: Option<&str>) -> Scene {
        let mut scene = self.scene(s);
        if let Some(id) = id {
            if id.starts_with("pile-") {
                if let Some(pile) = scene.objects.iter_mut().find(|o| o.id == id) {
                    pile.opacity = 0.7;
                }
            } else {
                raise_dragged(&mut scene, Some(id));
            }
        }
        scene
    }
    fn drag(
        &self,
        s: &Self::State,
        id: &str,
        _: Modifiers,
    ) -> Option<DragSpec<'static, Self::State>> {
        let ids = if let Some(i) = index(id, "card-") {
            vec![i]
        } else {
            pile_indices(id)?
        };
        if ids.iter().any(|&i| i >= s.positions.len()) {
            return None;
        }
        let base = s.clone();
        Some(vary([0., 0.], move |[dx, dy]| {
            let mut n = base.clone();
            for &i in &ids {
                n.positions[i] += Vec2::new(dx, dy);
            }
            n
        }))
    }
    fn locate(&self, s: &Self::State, id: &str) -> Option<Affine> {
        if let Some(i) = index(id, "card-") {
            return s.positions.get(i).map(|p| Affine::translate(p.to_vec2()));
        }
        let indices = pile_indices(id)?;
        if indices.iter().any(|&i| i >= s.positions.len()) {
            return None;
        }
        Some(Affine::translate(
            pile_bounds(s, &indices).origin().to_vec2(),
        ))
    }
    fn status(&self, _: &Self::State) -> String {
        String::new()
    }
}

#[derive(Clone, PartialEq, Debug)]
struct TileShape {
    id: usize,
    n: usize,
    center: Point,
    angle: f64,
}
#[derive(Clone, PartialEq, Debug)]
struct TessState {
    shapes: Vec<TileShape>,
    next_id: usize,
}
struct Tessellations;
const SIDES: [usize; 4] = [3, 4, 6, 8];
fn vertices(n: usize) -> Vec<Point> {
    let radius = 50. / (2. * (PI / n as f64).sin());
    (0..n)
        .map(|i| {
            let angle = TAU * (i as f64 - 0.5) / n as f64 + PI / 2.;
            Point::new(radius * angle.cos(), radius * angle.sin())
        })
        .collect()
}
fn world_vertices(shape: &TileShape) -> Vec<Point> {
    let t = Affine::translate(shape.center.to_vec2()) * Affine::rotate(shape.angle);
    vertices(shape.n).into_iter().map(|p| t * p).collect()
}
fn polygons_overlap(a: &[Point], b: &[Point]) -> bool {
    for poly in [a, b] {
        for i in 0..poly.len() {
            let edge = poly[(i + 1) % poly.len()] - poly[i];
            let axis = Vec2::new(-edge.y, edge.x) / edge.hypot();
            let project = |points: &[Point]| {
                points
                    .iter()
                    .map(|p| p.to_vec2().dot(axis))
                    .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
                        (lo.min(v), hi.max(v))
                    })
            };
            let (amin, amax) = project(a);
            let (bmin, bmax) = project(b);
            if amax - 2. <= bmin || bmax - 2. <= amin {
                return false;
            }
        }
    }
    true
}
fn snaps(s: &TessState, moving: usize) -> Vec<TileShape> {
    let tile = &s.shapes[moving];
    let local = vertices(tile.n);
    let mut result = Vec::new();
    for other in s.shapes.iter().filter(|other| other.id != tile.id) {
        let world = world_vertices(other);
        for j in 0..world.len() {
            let a = world[j];
            let b = world[(j + 1) % world.len()];
            let target = a - b;
            let mut best = (f64::INFINITY, Point::ORIGIN);
            for i in 0..local.len() {
                let c = local[i];
                let d = local[(i + 1) % local.len()];
                let e = d - c;
                let angle = (target.atan2() - e.atan2() + PI).rem_euclid(TAU) - PI;
                if angle.abs() < best.0.abs() {
                    best = (angle, c.midpoint(d));
                }
            }
            let rotated_mid = Affine::rotate(best.0) * best.1;
            let center = a.midpoint(b) - rotated_mid.to_vec2();
            let candidate = TileShape {
                id: tile.id,
                n: tile.n,
                center,
                angle: best.0,
            };
            let candidate_vertices = world_vertices(&candidate);
            if !s
                .shapes
                .iter()
                .filter(|other| other.id != tile.id)
                .any(|other| polygons_overlap(&candidate_vertices, &world_vertices(other)))
            {
                result.push(candidate);
            }
        }
    }
    result
}
fn palette_center(i: usize) -> Point {
    let x = [50., 140., 245., 370.][i];
    let bottom = vertices(SIDES[i])
        .iter()
        .map(|p| p.y)
        .fold(f64::NEG_INFINITY, f64::max);
    Point::new(x, 135. - bottom)
}
fn polygon_object(id: impl Into<String>, shape: &TileShape) -> Object {
    let points = vertices(shape.n);
    let path = polygon(points.clone());
    let color = match shape.n {
        3 => [252, 165, 165, 255],
        4 => [147, 197, 253, 255],
        6 => [134, 239, 172, 255],
        _ => [253, 230, 138, 255],
    };
    Object::new(
        id,
        Affine::translate(shape.center.to_vec2()) * Affine::rotate(shape.angle),
        Some(Hit::Path(path.clone(), 3.)),
        vec![Shape::Path {
            path,
            fill: Some(color),
            stroke: Some((
                match shape.n {
                    3 => rgb(0xef4444),
                    4 => rgb(0x3b82f6),
                    6 => rgb(0x16a34a),
                    _ => rgb(0xca8a04),
                },
                2.,
            )),
        }],
    )
}

impl Example for Tessellations {
    type State = TessState;
    fn initial(&self) -> Self::State {
        TessState {
            shapes: vec![],
            next_id: 0,
        }
    }
    fn scene(&self, s: &Self::State) -> Scene {
        let mut scene = Scene::new(600., 450.);
        scene.push(
            source_rect(
                "trash",
                Rect::new(540., 10., 590., 60.),
                4.,
                rgb(0xffeeee),
                Some((rgb(0x999999), 2.)),
            )
            .label("🗑", (25., 25.), 28., BLACK)
            .text_anchor(TextAnchor::Middle)
            .text_baseline(TextBaseline::Central)
            .dash(vec![4., 4.])
            .decorative(),
        );
        scene.push(
            source_rect(
                "palette-background",
                Rect::new(0., 0., 480., 150.),
                0.,
                rgb(0xf9fafb),
                Some((rgb(0xe5e7eb), 1.)),
            )
            .decorative(),
        );
        scene.push(Object::line(
            "palette-line",
            (0., 150.),
            (480., 150.),
            rgb(0xe5e7eb),
            1.,
        ));
        scene.push(
            Object::text(
                "palette-label",
                (10., 14.),
                "DRAG TO ADD",
                9.,
                rgb(0xaaaaaa),
            )
            .text_baseline(TextBaseline::Alphabetic)
            .font_weight(500),
        );
        for i in 0..4 {
            let id = format!("palette-{i}");
            let shape = TileShape {
                id: usize::MAX,
                n: SIDES[i],
                center: palette_center(i),
                angle: 0.,
            };
            let mut object = polygon_object(id, &shape);
            object.opacity = 0.8;
            scene.push(object);
        }
        for shape in &s.shapes {
            scene.push(polygon_object(format!("shape-{}", shape.id), shape));
        }
        scene
    }
    fn scene_dragging(&self, s: &Self::State, id: Option<&str>) -> Scene {
        let mut scene = self.scene(s);
        raise_dragged(&mut scene, id);
        scene
    }

    fn drag(
        &self,
        s: &Self::State,
        id: &str,
        _: Modifiers,
    ) -> Option<DragSpec<'static, Self::State>> {
        let base = s.clone();
        let number = index(id, "shape-")?;
        let moving = base.shapes.iter().position(|shape| shape.id == number)?;
        let tile = base.shapes[moving].clone();
        let free_base = base.clone();
        let free = vary([tile.center.x, tile.center.y], move |[x, y]| {
            let mut n = free_base.clone();
            n.shapes[moving].center = Point::new(x, y);
            n
        });
        let mut candidates: Vec<DragSpec<'static, TessState>> = snaps(&base, moving)
            .into_iter()
            .map(|shape| {
                let mut n = base.clone();
                n.shapes[moving] = shape;
                fixed(n)
            })
            .collect();
        let mut removed = base.clone();
        removed.shapes.remove(moving);
        candidates.push(drop_target(removed, Rect::new(540., 10., 590., 60.)));
        Some(closest(candidates).when_far(free, 30.))
    }
    fn anchor(&self, _: &Self::State, _: &str, _: Point) -> Point {
        Point::ORIGIN
    }
    fn begin_drag(&self, s: &Self::State, id: &str, _: Modifiers) -> Option<(Self::State, String)> {
        let palette = index(id, "palette-").filter(|&p| p < 4)?;
        let mut n = s.clone();
        let id = n.next_id;
        n.next_id += 1;
        n.shapes.push(TileShape {
            id,
            n: SIDES[palette],
            center: Point::ORIGIN,
            angle: 0.,
        });
        Some((n, format!("shape-{id}")))
    }
    fn locate(&self, s: &Self::State, id: &str) -> Option<Affine> {
        let id = index(id, "shape-")?;
        let shape = s.shapes.iter().find(|shape| shape.id == id)?;
        Some(Affine::translate(shape.center.to_vec2()) * Affine::rotate(shape.angle))
    }
    fn status(&self, _: &Self::State) -> String {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tessellation_grabs_follow_the_pointer_with_the_polygon_center() {
        let mut demo = demo(Tessellations);
        demo.set_animated(false);
        let start = palette_center(0) + Vec2::new(12., 0.);
        let target = Point::new(300., 300.);
        demo.press(start, Modifiers::default(), 1);
        demo.move_to(target, Modifiers::default());
        demo.advance(std::time::Duration::ZERO);
        demo.release(target, Modifiers::default());
        let center = demo.scene().object("shape-0").unwrap().transform * Point::ORIGIN;
        assert!(center.distance(target) < 0.2);
        demo.press(center + Vec2::new(12., 0.), Modifiers::default(), 1);
        let target = Point::new(390., 330.);
        demo.move_to(target, Modifiers::default());
        demo.advance(std::time::Duration::ZERO);
        demo.release(target, Modifiers::default());
        let center = demo.scene().object("shape-0").unwrap().transform * Point::ORIGIN;
        assert!(center.distance(target) < 0.2);
    }
    #[test]
    fn initial_models_accept_real_engine_gestures() {
        exercise(CardPiles, vec![CardPiles.initial()]);
        exercise(Tessellations, vec![Tessellations.initial()]);
    }
    #[test]
    fn overlap_groups_are_transitive_and_split_when_a_card_leaves() {
        let mut s = CardsState {
            positions: vec![
                Point::new(0., 0.),
                Point::new(60., 0.),
                Point::new(120., 0.),
            ],
        };
        assert_eq!(card_groups(&s), vec![vec![0, 1, 2]]);
        s.positions[1] = Point::new(300., 0.);
        assert_eq!(card_groups(&s).len(), 3);
    }
    #[test]
    fn edge_snaps_are_flush_and_never_overlap_existing_tiles() {
        let s = TessState {
            shapes: vec![
                TileShape {
                    id: 0,
                    n: 4,
                    center: Point::new(200., 250.),
                    angle: 0.,
                },
                TileShape {
                    id: 1,
                    n: 6,
                    center: Point::new(400., 250.),
                    angle: 0.,
                },
            ],
            next_id: 2,
        };
        let candidates = snaps(&s, 1);
        assert!(!candidates.is_empty());
        for candidate in candidates {
            assert!(!polygons_overlap(
                &world_vertices(&s.shapes[0]),
                &world_vertices(&candidate)
            ));
        }
    }
}
