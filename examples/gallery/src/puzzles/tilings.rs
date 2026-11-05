use super::*;
use dragology::{ClosestOptions, closest_with, drop_target};
use std::collections::{BTreeMap, VecDeque};

pub fn entries() -> Vec<Entry> {
    vec![
        Entry::new(
            "plane-partition",
            "Plane partition",
            "A plane partition is a stack of cubes in the corner of a box – and, seen flat, it is a lozenge tiling of a hexagon. Drag a corner cube to move it or to toss it in the spare pile; drag a spare cube into any inner corner. Each move is one “flip” of the tiling. See also aztec-diamond, the domino cousin of this demo.",
            || demo(PlanePartition),
        ),
        Entry::new(
            "aztec-diamond",
            "Aztec diamond",
            "Domino tilings of the Aztec diamond. Two parallel dominoes side by side form a 2×2 block; drag one of them to turn the block like a knob, either way and as far round as you like; let go and it settles to the nearest quarter turn. The numbers are Thurston’s height function – each flip changes it at exactly one vertex. See also plane-partition, the lozenge cousin of this demo.",
            || demo(AztecDiamond),
        ),
    ]
}

fn random(seed: &mut u64) -> f64 {
    let a = (*seed as u32).wrapping_add(0x6d2b79f5);
    *seed = u64::from(a);
    let mut t = (a ^ (a >> 15)).wrapping_mul(a | 1);
    t ^= t.wrapping_add((t ^ (t >> 7)).wrapping_mul(t | 61));
    f64::from(t ^ (t >> 14)) / 4294967296.
}
fn new_seed() -> u64 {
    use std::hash::{BuildHasher, Hasher};
    1 + std::collections::hash_map::RandomState::new()
        .build_hasher()
        .finish()
        % 1_000_000_000
}

#[derive(Clone, PartialEq, Debug)]
struct Cube {
    id: usize,
    i: usize,
    j: usize,
    k: usize,
}
#[derive(Clone, PartialEq, Debug)]
struct PlaneState {
    a: usize,
    b: usize,
    c: usize,
    cubes: Vec<Cube>,
    next_id: usize,
    creating: Option<usize>,
    hints: bool,
    seed: u64,
}
struct PlanePartition;

fn heights(s: &PlaneState) -> Vec<Vec<usize>> {
    let mut h = vec![vec![0; s.b]; s.a];
    for cube in &s.cubes {
        h[cube.i][cube.j] = h[cube.i][cube.j].max(cube.k + 1);
    }
    h
}
fn removable(s: &PlaneState) -> Vec<usize> {
    let h = heights(s);
    s.cubes
        .iter()
        .filter(|cube| {
            cube.k + 1 == h[cube.i][cube.j]
                && (cube.i + 1 == s.a || h[cube.i + 1][cube.j] <= cube.k)
                && (cube.j + 1 == s.b || h[cube.i][cube.j + 1] <= cube.k)
        })
        .map(|cube| cube.id)
        .collect()
}
fn addable(s: &PlaneState) -> Vec<(usize, usize, usize)> {
    let h = heights(s);
    let mut positions = Vec::new();
    for i in 0..s.a {
        for j in 0..s.b {
            let k = h[i][j];
            if k < s.c && (i == 0 || h[i - 1][j] > k) && (j == 0 || h[i][j - 1] > k) {
                positions.push((i, j, k));
            }
        }
    }
    positions
}
fn plane_initial(a: usize, b: usize, c: usize, hints: bool, seed: u64) -> PlaneState {
    let mut state = PlaneState {
        a,
        b,
        c,
        cubes: vec![],
        next_id: 0,
        creating: None,
        hints,
        seed,
    };
    if seed == 0 {
        for i in 0..a {
            for j in 0..b {
                for k in 0..(c + 1).saturating_sub(i + j).min(c) {
                    state.cubes.push(Cube {
                        id: state.next_id,
                        i,
                        j,
                        k,
                    });
                    state.next_id += 1;
                }
            }
        }
    } else {
        let mut rng = seed;
        for _ in 0..a * b * c * 8 {
            let add = addable(&state);
            let remove = removable(&state);
            if random(&mut rng)
                < if (state.cubes.len() as f64) < (a * b * c) as f64 / 2. {
                    0.8
                } else {
                    0.4
                }
                && !add.is_empty()
            {
                let (i, j, k) = add[(random(&mut rng) * add.len() as f64) as usize % add.len()];
                state.cubes.push(Cube {
                    id: state.next_id,
                    i,
                    j,
                    k,
                });
                state.next_id += 1;
            } else if !remove.is_empty() {
                let id = remove[(random(&mut rng) * remove.len() as f64) as usize % remove.len()];
                state.cubes.retain(|cube| cube.id != id);
            }
        }
    }
    state
}
const EDGE: f64 = 30.;
fn project(i: f64, j: f64, k: f64) -> Point {
    Point::new(
        (i - j) * 3f64.sqrt() / 2. * EDGE,
        (i + j) * EDGE / 2. - k * EDGE,
    )
}
fn plane_layout(s: &PlaneState) -> (Point, Point, f64, f64) {
    let left = -(s.b as f64) * 3f64.sqrt() / 2. * EDGE;
    let right = s.a as f64 * 3f64.sqrt() / 2. * EDGE;
    let top = -(s.c as f64) * EDGE;
    let bottom = (s.a + s.b) as f64 * EDGE / 2.;
    let origin = Point::new(24. - left, 24. - top);
    let spare = origin + Vec2::new(right + 90., (top + bottom) / 2. + 15.);
    (
        origin,
        spare,
        right + 90. - left + 3f64.sqrt() * EDGE + 48.,
        bottom - top + 48.,
    )
}
fn face(points: &[(f64, f64, f64)], color: Color) -> Shape {
    Shape::Path {
        path: polygon(points.iter().map(|&(i, j, k)| project(i, j, k))),
        fill: Some(color),
        stroke: Some((rgb(0x333333), 1.)),
    }
}
fn cube_object(id: String, position: Point, hint: bool, removable: bool) -> Object {
    let top = if hint {
        [255, 255, 255, 115]
    } else {
        [246, 213, 92, 255]
    };
    let front = if hint {
        [255, 255, 255, 115]
    } else {
        [60, 174, 163, 255]
    };
    let side = if hint {
        [255, 255, 255, 115]
    } else {
        [237, 85, 59, 255]
    };
    let mut shapes = vec![
        face(
            &[(0., 0., 1.), (1., 0., 1.), (1., 1., 1.), (0., 1., 1.)],
            top,
        ),
        face(
            &[(1., 0., 0.), (1., 1., 0.), (1., 1., 1.), (1., 0., 1.)],
            front,
        ),
        face(
            &[(0., 1., 0.), (1., 1., 0.), (1., 1., 1.), (0., 1., 1.)],
            side,
        ),
    ];
    if removable && !hint {
        shapes.push(Shape::Circle {
            center: project(0.5, 0.5, 1.),
            radius: 3.,
            fill: [51, 51, 51, 128],
            stroke: None,
        });
    }
    let outline = polygon(
        [
            (0., 0., 1.),
            (1., 0., 1.),
            (1., 0., 0.),
            (1., 1., 0.),
            (0., 1., 0.),
            (0., 1., 1.),
        ]
        .map(|(i, j, k)| project(i, j, k)),
    );
    let object = Object::new(
        id,
        Affine::translate(position.to_vec2()),
        if hint {
            None
        } else {
            Some(Hit::Path(outline, 2.))
        },
        shapes,
    )
    .line_join(LineJoin::Round);
    if hint {
        object.dash(vec![3., 3.])
    } else {
        object
    }
}

impl Example for PlanePartition {
    type State = PlaneState;
    fn initial(&self) -> Self::State {
        plane_initial(4, 4, 4, true, 0)
    }
    fn scene(&self, s: &Self::State) -> Scene {
        let (origin, spare, width, height) = plane_layout(s);
        let mut scene = Scene::new(width, height);
        let mut room = Object::new("room", Affine::translate(origin.to_vec2()), None, vec![]);
        for i in 0..s.a {
            for j in 0..s.b {
                let (i, j) = (i as f64, j as f64);
                room.shapes.push(face(
                    &[
                        (i, j, 0.),
                        (i + 1., j, 0.),
                        (i + 1., j + 1., 0.),
                        (i, j + 1., 0.),
                    ],
                    [246, 213, 92, 255],
                ));
            }
        }
        for j in 0..s.b {
            for k in 0..s.c {
                let (j, k) = (j as f64, k as f64);
                room.shapes.push(face(
                    &[
                        (0., j, k),
                        (0., j + 1., k),
                        (0., j + 1., k + 1.),
                        (0., j, k + 1.),
                    ],
                    [60, 174, 163, 255],
                ));
            }
        }
        for i in 0..s.a {
            for k in 0..s.c {
                let (i, k) = (i as f64, k as f64);
                room.shapes.push(face(
                    &[
                        (i, 0., k),
                        (i + 1., 0., k),
                        (i + 1., 0., k + 1.),
                        (i, 0., k + 1.),
                    ],
                    [237, 85, 59, 255],
                ));
            }
        }
        scene.push(room.line_join(LineJoin::Round));
        let remove = removable(s);
        let mut cubes = s.cubes.iter().collect::<Vec<_>>();
        cubes.sort_by_key(|cube| cube.i + cube.j + cube.k);
        for cube in cubes {
            let id = format!("cube-{}", cube.id);
            scene.push(cube_object(
                id,
                origin + project(cube.i as f64, cube.j as f64, cube.k as f64).to_vec2(),
                false,
                s.hints && remove.contains(&cube.id),
            ));
        }
        if s.hints {
            for (i, j, k) in addable(s) {
                scene.push(cube_object(
                    format!("hint-{i}-{j}-{k}"),
                    origin + project(i as f64, j as f64, k as f64).to_vec2(),
                    true,
                    false,
                ));
            }
        }
        use kurbo::Shape as _;
        scene.push(
            Object::new(
                "spare-bin",
                Affine::translate(spare.to_vec2()),
                None,
                vec![Shape::Path {
                    path: kurbo::Ellipse::new((0., 15.), (39., 22.5), 0.).to_path(0.1),
                    fill: Some(rgb(0xf5f5f5)),
                    stroke: Some((rgb(0xbbbbbb), 1.)),
                }],
            )
            .dash(vec![4., 4.]),
        );
        scene.push(
            Object::text(
                "spare-label",
                spare + Vec2::new(0., 52.5),
                "spare cubes",
                11.,
                rgb(0x888888),
            )
            .text_anchor(TextAnchor::Middle)
            .text_baseline(TextBaseline::Alphabetic),
        );
        scene.push(cube_object("spare".into(), spare, false, false));
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
        let (_, spare, _, _) = plane_layout(s);
        let cube_id = index(id, "cube-")?;
        if s.creating == Some(cube_id) {
            let mut without = s.clone();
            without.cubes.retain(|c| c.id != cube_id);
            without.creating = None;
            let targets = addable(&without).into_iter().map(|(i, j, k)| {
                let mut n = without.clone();
                n.cubes.push(Cube {
                    id: cube_id,
                    i,
                    j,
                    k,
                });
                n
            });
            Some(
                closest(targets)
                    .with_floating()
                    .when_far(fixed(without).with_floating(), 50.),
            )
        } else {
            let id = index(id, "cube-")?;
            if !removable(s).contains(&id) {
                return None;
            }
            let mut without = s.clone();
            without.cubes.retain(|cube| cube.id != id);
            let targets = addable(&without)
                .into_iter()
                .map(|(i, j, k)| {
                    let mut n = without.clone();
                    n.cubes.push(Cube { id, i, j, k });
                    n
                })
                .collect::<Vec<_>>();
            Some(
                closest([
                    closest(targets).with_floating(),
                    drop_target(
                        without,
                        Rect::new(spare.x - 39., spare.y - 30., spare.x + 39., spare.y + 54.5),
                    ),
                ])
                .when_far(fixed(s.clone()).with_floating(), 50.),
            )
        }
    }
    fn begin_drag(&self, s: &Self::State, id: &str, _: Modifiers) -> Option<(Self::State, String)> {
        if id != "spare" {
            return None;
        }
        let (i, j, k) = addable(s).first().copied()?;
        let mut n = s.clone();
        let id = n.next_id;
        n.next_id += 1;
        n.cubes.push(Cube { id, i, j, k });
        n.creating = Some(id);
        Some((n, format!("cube-{id}")))
    }
    fn controls(&self, s: &Self::State) -> Vec<Control> {
        vec![
            slider("a", "a", s.a as f64, 1., 6.),
            slider("b", "b", s.b as f64, 1., 6.),
            slider("c", "c", s.c as f64, 1., 6.),
            Control::toggle("hints", "Show hints", s.hints),
            Control::button("randomize", "Randomize"),
            Control::button("reset-board", "Reset"),
        ]
    }
    fn action(&self, s: &Self::State, id: &str, v: Value) -> Option<Self::State> {
        let mut n = s.clone();
        match (id, v) {
            ("hints", Value::Bool(b)) => {
                n.hints = b;
                return Some(n);
            }
            ("a", Value::Number(v)) => n.a = v.round().clamp(1., 6.) as usize,
            ("b", Value::Number(v)) => n.b = v.round().clamp(1., 6.) as usize,
            ("c", Value::Number(v)) => n.c = v.round().clamp(1., 6.) as usize,
            ("randomize", _) => n.seed = new_seed(),
            ("reset-board", _) => n.seed = 0,
            _ => return None,
        }
        Some(plane_initial(n.a, n.b, n.c, n.hints, n.seed))
    }
    fn status(&self, _: &Self::State) -> String {
        String::new()
    }
}

#[derive(Clone, PartialEq, Debug)]
struct Domino {
    x: i32,
    y: i32,
    quarter: i32,
}
#[derive(Clone, PartialEq, Debug)]
struct Turn {
    bx: i32,
    by: i32,
    ids: [usize; 2],
    spin: f64,
}
#[derive(Clone, PartialEq, Debug)]
struct AztecState {
    n: i32,
    dominoes: Vec<Domino>,
    turn: Option<Turn>,
    show_heights: bool,
    dim_stuck: bool,
    seed: u64,
}
struct AztecDiamond;
const CELL: f64 = 32.;
fn in_diamond(n: i32, x: i32, y: i32) -> bool {
    (f64::from(x) + 0.5).abs() + (f64::from(y) + 0.5).abs() <= f64::from(n)
}
fn horizontal(d: &Domino) -> bool {
    d.quarter.rem_euclid(2) == 0
}
fn cells_of(d: &Domino) -> [(i32, i32); 2] {
    [
        (d.x, d.y),
        if horizontal(d) {
            (d.x + 1, d.y)
        } else {
            (d.x, d.y + 1)
        },
    ]
}
fn coverage(s: &AztecState) -> BTreeMap<(i32, i32), usize> {
    s.dominoes
        .iter()
        .enumerate()
        .flat_map(|(i, d)| cells_of(d).map(|p| (p, i)))
        .collect()
}
fn initial_aztec(n: i32, vertical: bool) -> AztecState {
    let mut dominoes = Vec::new();
    for x in -n..n {
        for y in -n..n {
            if !in_diamond(n, x, y) {
                continue;
            }
            let width =
                (f64::from(n) - (f64::from(if vertical { x } else { y }) + 0.5).abs() + 0.5) as i32;
            if (if vertical { y } else { x } + width).rem_euclid(2) == 0 {
                dominoes.push(Domino {
                    x,
                    y,
                    quarter: i32::from(vertical),
                });
            }
        }
    }
    AztecState {
        n,
        dominoes,
        turn: None,
        show_heights: true,
        dim_stuck: true,
        seed: 0,
    }
}
fn shuffled_aztec(order: i32, seed: u64, heights: bool, dim: bool) -> AztecState {
    let mut state = initial_aztec(order, false);
    state.seed = seed;
    state.show_heights = heights;
    state.dim_stuck = dim;
    if seed == 0 {
        return state;
    }
    let mut rng = seed;
    for _ in 0..order * order * 40 {
        let i = (random(&mut rng) * state.dominoes.len() as f64) as usize;
        let blocks = blocks_of(&state, i);
        if blocks.is_empty() {
            continue;
        }
        let (bx, by, j) = blocks[(random(&mut rng) * blocks.len() as f64) as usize];
        state = apply_turn(
            &state,
            &Turn {
                bx,
                by,
                ids: [i, j],
                spin: 0.,
            },
            if random(&mut rng) < 0.5 { 1 } else { -1 },
        );
    }
    state
}
fn blocks_of(s: &AztecState, i: usize) -> Vec<(i32, i32, usize)> {
    let d = &s.dominoes[i];
    let mut result = Vec::new();
    let positions = if horizontal(d) {
        [(d.x, d.y + 1), (d.x, d.y - 1)]
    } else {
        [(d.x + 1, d.y), (d.x - 1, d.y)]
    };
    for (x, y) in positions {
        if let Some(j) = s
            .dominoes
            .iter()
            .position(|p| p.x == x && p.y == y && horizontal(p) == horizontal(d))
        {
            result.push((d.x.min(x), d.y.min(y), j));
        }
    }
    result
}
fn domino_center(d: &Domino) -> Point {
    if horizontal(d) {
        Point::new(f64::from(d.x) + 1., -f64::from(d.y) - 0.5)
    } else {
        Point::new(f64::from(d.x) + 0.5, -f64::from(d.y) - 1.)
    }
}
fn apply_turn(s: &AztecState, turn: &Turn, quarter: i32) -> AztecState {
    let mut n = s.clone();
    let pivot = Point::new(f64::from(turn.bx) + 1., -f64::from(turn.by) - 1.);
    for &i in &turn.ids {
        let d = &s.dominoes[i];
        let center = pivot
            + (Affine::rotate(f64::from(quarter) * PI / 2.)
                * (domino_center(d) - pivot).to_point())
            .to_vec2();
        let angle = d.quarter + quarter;
        let h = angle.rem_euclid(2) == 0;
        n.dominoes[i] = Domino {
            x: (center.x - if h { 1. } else { 0.5 }).round() as i32,
            y: (-center.y - if h { 0.5 } else { 1. }).round() as i32,
            quarter: angle,
        };
    }
    n.turn = None;
    n
}
fn settle(s: &AztecState) -> AztecState {
    if let Some(turn) = &s.turn {
        apply_turn(s, turn, (turn.spin / (PI / 2.) + 0.5).floor() as i32)
    } else {
        s.clone()
    }
}
fn aztec_components(s: &AztecState, i: usize) -> Vec<TransformComponent> {
    let d = &s.dominoes[i];
    let center = domino_center(d);
    let origin = Vec2::new((f64::from(s.n) + 0.5) * CELL, (f64::from(s.n) + 0.5) * CELL);
    let (pivot, spin) = if let Some(t) = &s.turn
        && t.ids.contains(&i)
    {
        (
            Point::new(f64::from(t.bx) + 1., -f64::from(t.by) - 1.),
            t.spin,
        )
    } else {
        (center, 0.)
    };
    vec![
        TransformComponent::Translate(origin + pivot.to_vec2() * CELL),
        TransformComponent::Rotate {
            radians: spin,
            center: Point::ORIGIN,
        },
        TransformComponent::Translate((center - pivot) * CELL),
        TransformComponent::Rotate {
            radians: f64::from(d.quarter) * PI / 2.,
            center: Point::ORIGIN,
        },
    ]
}
fn aztec_transform(s: &AztecState, i: usize) -> Affine {
    aztec_components(s, i)
        .iter()
        .fold(Affine::IDENTITY, |m, c| m * c.affine())
}
fn height_function(s: &AztecState) -> BTreeMap<(i32, i32), i32> {
    let cov = coverage(s);
    let mut heights = BTreeMap::from([((-s.n, 0), 0)]);
    let mut queue = VecDeque::from([(-s.n, 0)]);
    while let Some((x, y)) = queue.pop_front() {
        let height = heights[&(x, y)];
        for (dx, dy) in [(1, 0), (0, 1), (-1, 0), (0, -1)] {
            let left = ((2 * x + dx - dy - 1) / 2, (2 * y + dy + dx - 1) / 2);
            let right = ((2 * x + dx + dy - 1) / 2, (2 * y + dy - dx - 1) / 2);
            let left_in = in_diamond(s.n, left.0, left.1);
            let right_in = in_diamond(s.n, right.0, right.1);
            if !left_in && !right_in {
                continue;
            }
            let crossed = left_in && right_in && cov.get(&left) == cov.get(&right);
            let sign = if (left.0 + left.1).rem_euclid(2) == 0 {
                1
            } else {
                -1
            };
            let next = (x + dx, y + dy);
            if let std::collections::btree_map::Entry::Vacant(entry) = heights.entry(next) {
                entry.insert(height + sign * if crossed { -3 } else { 1 });
                queue.push_back(next);
            }
        }
    }
    heights
}
impl Example for AztecDiamond {
    type State = AztecState;
    fn initial(&self) -> Self::State {
        initial_aztec(4, false)
    }
    fn scene(&self, s: &Self::State) -> Scene {
        let size = (2 * s.n + 1) as f64 * CELL;
        let mut scene = Scene::new(size, size);
        let offset = (f64::from(s.n) + 0.5) * CELL;
        for x in -s.n..s.n {
            for y in -s.n..s.n {
                if in_diamond(s.n, x, y) {
                    scene.push(
                        Object::rect(
                            format!("cell-{x}-{y}"),
                            Rect::from_origin_size(
                                (
                                    offset + f64::from(x) * CELL,
                                    offset - f64::from(y + 1) * CELL,
                                ),
                                (CELL, CELL),
                            ),
                            0.,
                            if (x + y).rem_euclid(2) == 0 {
                                [229, 231, 235, 255]
                            } else {
                                [249, 250, 251, 255]
                            },
                        )
                        .decorative(),
                    );
                }
            }
        }
        let settled = settle(s);
        for (i, d) in settled.dominoes.iter().enumerate() {
            let dark = (d.x + d.y).rem_euclid(2) == 0;
            let color = match (horizontal(d), dark) {
                (true, true) => [230, 57, 70, 255],
                (true, false) => [244, 162, 97, 255],
                (false, true) => [69, 123, 157, 255],
                (false, false) => [42, 157, 143, 255],
            };
            let bounds = Rect::new(-CELL + 2., -CELL / 2. + 2., CELL - 2., CELL / 2. - 2.);
            let mut o = Object::new(
                format!("domino-{i}"),
                aztec_transform(s, i),
                Some(Hit::Rect(bounds)),
                vec![Shape::Rect {
                    bounds,
                    radius: 4.,
                    fill: color,
                    stroke: Some((rgb(0x1f2937), 1.)),
                }],
            )
            .transforms(aztec_components(s, i));
            if s.dim_stuck && blocks_of(&settled, i).is_empty() {
                o.opacity = 0.45;
            }
            scene.push(o);
        }
        if s.show_heights {
            let lo = height_function(&initial_aztec(s.n, false));
            let hi = height_function(&initial_aztec(s.n, true));
            for ((x, y), h) in height_function(&settled) {
                let a = lo[&(x, y)].min(hi[&(x, y)]);
                let b = lo[&(x, y)].max(hi[&(x, y)]);
                let t = if a == b {
                    0.
                } else {
                    f64::from(h - a) / f64::from(b - a)
                };
                let color = [
                    (224. - 194. * t).round() as u8,
                    (242. - 184. * t).round() as u8,
                    (254. - 116. * t).round() as u8,
                    255,
                ];
                let mut circle = Object::circle(
                    format!("height-{x}-{y}"),
                    (offset + f64::from(x) * CELL, offset - f64::from(y) * CELL),
                    8.,
                    color,
                )
                .decorative();
                if let Shape::Circle { stroke, .. } = &mut circle.shapes[0] {
                    *stroke = Some((WHITE, 1.5));
                }
                scene.push(
                    centered_label(
                        circle,
                        h.to_string(),
                        Point::ORIGIN,
                        9.,
                        if t > 0.5 { WHITE } else { rgb(0x1e3a8a) },
                    )
                    .font_family(FontFamily::Mono),
                );
            }
        }
        scene
    }
    fn scene_dragging(&self, s: &Self::State, id: Option<&str>) -> Scene {
        let mut scene = self.scene(s);
        if let Some(turn) = &s.turn {
            for i in turn.ids {
                let name = format!("domino-{i}");
                if let Some(index) = scene.objects.iter().position(|o| o.id == name) {
                    let object = scene.objects.remove(index);
                    let before = scene
                        .objects
                        .iter()
                        .position(|o| o.id.starts_with("height-"))
                        .unwrap_or(scene.objects.len());
                    scene.objects.insert(before, object);
                }
            }
        }
        raise_dragged(&mut scene, id);
        scene
    }
    fn drag(
        &self,
        s: &Self::State,
        id: &str,
        _: Modifiers,
    ) -> Option<DragSpec<'static, Self::State>> {
        let i = index(id, "domino-").filter(|&i| i < s.dominoes.len())?;
        let base = settle(s);
        let blocks = blocks_of(&base, i);
        if blocks.is_empty() {
            return None;
        }
        let specs = blocks.into_iter().map(|(bx, by, j)| {
            let base = base.clone();
            vary([0.], move |[spin]| {
                let mut n = base.clone();
                n.turn = Some(Turn {
                    bx,
                    by,
                    ids: [i, j],
                    spin,
                });
                n
            })
        });
        Some(
            closest_with(
                specs,
                ClosestOptions {
                    stickiness: 12.,
                    ..Default::default()
                },
            )
            .with_branch_transition(Some(Transition {
                duration: std::time::Duration::from_millis(120),
                ..Default::default()
            }))
            .on_drop(settle),
        )
    }
    fn locate(&self, s: &Self::State, id: &str) -> Option<Affine> {
        let i = index(id, "domino-").filter(|&i| i < s.dominoes.len())?;
        Some(aztec_transform(s, i))
    }
    fn controls(&self, s: &Self::State) -> Vec<Control> {
        vec![
            slider("order", "Order", f64::from(s.n), 1., 8.),
            Control::toggle("heights", "Show height function", s.show_heights),
            Control::toggle("dim", "Dim stuck dominoes", s.dim_stuck),
            Control::button("shuffle", "Shuffle"),
            Control::button("reset-board", "Reset"),
        ]
    }
    fn action(&self, s: &Self::State, id: &str, v: Value) -> Option<Self::State> {
        let mut n = settle(s);
        match (id, v) {
            ("heights", Value::Bool(b)) => {
                n.show_heights = b;
                return Some(n);
            }
            ("dim", Value::Bool(b)) => {
                n.dim_stuck = b;
                return Some(n);
            }
            ("order", Value::Number(v)) => n.n = v.round().clamp(1., 8.) as i32,
            ("reset-board", _) => n.seed = 0,
            ("shuffle", _) => n.seed = new_seed(),
            _ => return None,
        }
        Some(shuffled_aztec(n.n, n.seed, n.show_heights, n.dim_stuck))
    }
    fn status(&self, _: &Self::State) -> String {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn valid_partition(s: &PlaneState) -> bool {
        let h = heights(s);
        s.cubes.len() == h.iter().flatten().sum::<usize>()
            && (0..s.a).all(|i| {
                (0..s.b).all(|j| {
                    h[i][j] <= s.c
                        && (i == 0 || h[i - 1][j] >= h[i][j])
                        && (j == 0 || h[i][j - 1] >= h[i][j])
                })
            })
    }
    // Fixtures produced by executing the pinned upstream functions with esbuild,
    // preserving its Math.imul generator and candidate ordering.
    #[test]
    fn taking_a_spare_keeps_the_bottomless_supply_visible_and_far_drop_cancels() {
        let mut demo = demo(PlanePartition);
        demo.set_animated(false);
        let spare = demo.scene().object("spare").unwrap().clone();
        let original_count = demo
            .scene()
            .objects
            .iter()
            .filter(|o| o.id.starts_with("cube-"))
            .count();
        let start = spare.transform * Point::new(8., 0.);
        let far = Point::new(600., 20.);
        demo.press(start, Modifiers::default(), 1);
        demo.move_to(far, Modifiers::default());
        demo.advance(std::time::Duration::ZERO);
        assert_eq!(demo.scene().object("spare"), Some(&spare));
        assert_eq!(
            demo.scene()
                .objects
                .iter()
                .filter(|o| o.id.starts_with("cube-"))
                .count(),
            original_count + 1
        );
        demo.release(far, Modifiers::default());
        assert_eq!(demo.scene().object("spare"), Some(&spare));
        assert_eq!(
            demo.scene()
                .objects
                .iter()
                .filter(|o| o.id.starts_with("cube-"))
                .count(),
            original_count
        );
    }
    #[test]
    fn seeded_tilings_match_the_pinned_typescript_source() {
        let mut cubes = plane_initial(3, 3, 3, true, 123)
            .cubes
            .into_iter()
            .map(|c| (c.i, c.j, c.k))
            .collect::<Vec<_>>();
        cubes.sort_unstable();
        assert_eq!(
            cubes,
            vec![
                (0, 0, 0),
                (0, 0, 1),
                (0, 0, 2),
                (0, 1, 0),
                (0, 1, 1),
                (0, 2, 0),
                (1, 0, 0),
                (1, 0, 1),
                (1, 0, 2),
                (1, 1, 0),
                (2, 0, 0),
                (2, 0, 1),
                (2, 0, 2),
                (2, 1, 0)
            ]
        );
        let state = shuffled_aztec(2, 123, true, true);
        assert_eq!(
            state
                .dominoes
                .iter()
                .map(|d| (d.x, d.y, d.quarter))
                .collect::<Vec<_>>(),
            vec![
                (-2, -1, 0),
                (1, -1, -7),
                (-1, -2, 0),
                (-1, 1, 4),
                (-2, 0, -6),
                (0, -1, -5)
            ]
        );
        assert_eq!(height_function(&state)[&(1, 0)], -3);
    }
    #[test]
    fn changing_aztec_order_rebuilds_from_the_preserved_seed() {
        let mut prior = shuffled_aztec(2, 123, false, false);
        let (bx, by, j) = blocks_of(&prior, 0)[0];
        prior = apply_turn(
            &prior,
            &Turn {
                bx,
                by,
                ids: [0, j],
                spin: 0.,
            },
            1,
        );
        let changed = AztecDiamond
            .action(&prior, "order", Value::Number(3.))
            .unwrap();
        assert_eq!(changed, shuffled_aztec(3, 123, false, false));
    }
    #[test]
    fn initial_models_accept_real_engine_gestures() {
        exercise(PlanePartition, vec![PlanePartition.initial()]);
        exercise(AztecDiamond, vec![AztecDiamond.initial()]);
    }
    #[test]
    fn every_offered_cube_move_preserves_a_plane_partition() {
        for seed in [0, 1, 901] {
            let s = plane_initial(4, 4, 4, true, seed);
            assert!(valid_partition(&s));
            for id in removable(&s) {
                let mut n = s.clone();
                n.cubes.retain(|c| c.id != id);
                assert!(valid_partition(&n));
            }
            for (i, j, k) in addable(&s) {
                let mut n = s.clone();
                n.cubes.push(Cube {
                    id: n.next_id,
                    i,
                    j,
                    k,
                });
                assert!(valid_partition(&n));
            }
        }
    }
    #[test]
    fn every_domino_flip_preserves_coverage_and_changes_one_height_vertex() {
        for order in 1..=5 {
            let s = initial_aztec(order, false);
            let original = coverage(&s);
            assert_eq!(original.len(), (2 * order * (order + 1)) as usize);
            let old_heights = height_function(&s);
            for i in 0..s.dominoes.len() {
                for (bx, by, j) in blocks_of(&s, i) {
                    let n = apply_turn(
                        &s,
                        &Turn {
                            bx,
                            by,
                            ids: [i, j],
                            spin: 0.,
                        },
                        1,
                    );
                    let after = coverage(&n);
                    assert_eq!(
                        after.keys().collect::<Vec<_>>(),
                        original.keys().collect::<Vec<_>>()
                    );
                    assert_eq!(after.len(), 2 * n.dominoes.len());
                    let changed = height_function(&n)
                        .into_iter()
                        .filter(|(p, h)| old_heights[p] != *h)
                        .collect::<Vec<_>>();
                    assert_eq!(changed.len(), 1);
                    assert_eq!((changed[0].1 - old_heights[&changed[0].0]).abs(), 4);
                }
            }
        }
    }
}
