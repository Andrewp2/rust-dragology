use super::*;

pub fn entries() -> Vec<Entry> {
    vec![
        Entry::new(
            "tiles",
            "Tiles",
            "I'm trying to make dragging feel right here. Goal is for the tile to only drag orthogonally, AND to not jump discontinuously. This seems to require 'Relative Pointer Motion' mode (or divergent approaches).",
            tiles_page,
        ),
        Entry::new("grid-poly", "Grid polygon", "", || {
            demo(GridPoly { continuous: false })
        }),
        Entry::new(
            "grid-poly-vary",
            "Grid polygon · continuous",
            "Re-implementation of grid-poly in a somewhat more traditional way: d.vary lets x & y be dragged freely. spec.during keeps the dragged point inside the grid. spec.onDrop rounds the point to the nearest grid point on drop. spec.withSnapRadius works on top of all of this – which is kinda cool because we never enumerate the drop locations! But note that we don't stop points from colliding, which grid-poly does.",
            || demo(GridPoly { continuous: true }),
        ),
        Entry::new("rush-hour", "Rush Hour", "", || demo(RushHour)),
        Entry::new(
            "fifteen",
            "Fifteen",
            "Weird experiment: I made the blank draggable",
            || demo(Fifteen),
        ),
        Entry::new(
            "hanoi",
            "Tower of Hanoi",
            "Uses floating. Only top disks can be dragged.",
            hanoi_page,
        ),
        Entry::new("sokoban", "Sokoban", "", || demo(Sokoban)),
        Entry::new(
            "spinny",
            "Spinny",
            "Tests interpolation of rotations.",
            || demo(Spinny),
        ),
        Entry::new(
            "tromino",
            "Tromino",
            "Inspired by a neat Reddit post.",
            || demo(Tromino),
        ),
    ]
}

/// Independent figures reproduce the source's simultaneous examples.
struct Figure<E: Example> {
    example: E,
    start: E::State,
}
impl<E: Example> Example for Figure<E> {
    type State = E::State;
    fn initial(&self) -> Self::State {
        self.start.clone()
    }
    fn scene(&self, s: &Self::State) -> Scene {
        self.example.scene(s)
    }
    fn scene_dragging(&self, s: &Self::State, id: Option<&str>) -> Scene {
        self.example.scene_dragging(s, id)
    }
    fn drag(
        &self,
        s: &Self::State,
        id: &str,
        m: Modifiers,
    ) -> Option<DragSpec<'static, Self::State>> {
        self.example.drag(s, id, m)
    }
    fn locate(&self, s: &Self::State, id: &str) -> Option<Affine> {
        self.example.locate(s, id)
    }
}
fn tiles_page() -> Box<dyn Demo> {
    Box::new(CompositeDemo::new(vec![
        Panel {
            id: "lonely",
            title: "lonely",
            description: "",
            demo: demo(Figure {
                example: Tiles,
                start: TilesState {
                    positions: vec![(2, 2)],
                },
            }),
        },
        Panel {
            id: "friendly",
            title: "friendly",
            description: "",
            demo: demo(Figure {
                example: Tiles,
                start: TilesState {
                    positions: vec![(2, 2), (4, 4)],
                },
            }),
        },
    ]))
}
fn hanoi_page() -> Box<dyn Demo> {
    Box::new(CompositeDemo::new(vec![
        Panel {
            id: "three",
            title: "3 disks",
            description: "",
            demo: demo(Figure {
                example: Hanoi,
                start: HanoiState { pegs: vec![0; 3] },
            }),
        },
        Panel {
            id: "four",
            title: "4 disks",
            description: "",
            demo: demo(Figure {
                example: Hanoi,
                start: HanoiState { pegs: vec![0; 4] },
            }),
        },
    ]))
}

const DIRECTIONS: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
const TILE: f64 = 50.;
const OFFSET: f64 = 20.;

#[derive(Clone, PartialEq, Debug)]
struct TilesState {
    positions: Vec<(i32, i32)>,
}
struct Tiles;
impl Example for Tiles {
    type State = TilesState;
    fn initial(&self) -> Self::State {
        TilesState {
            positions: vec![(2, 2), (4, 4)],
        }
    }
    fn scene(&self, s: &Self::State) -> Scene {
        let mut scene = Scene::new(300., 300.);
        grid(&mut scene, 5, 5, 50., 0.);
        for (i, &(x, y)) in s.positions.iter().enumerate() {
            scene.push(centered_label(
                source_rect(
                    format!("tile-{i}"),
                    Rect::from_origin_size((f64::from(x) * 50., f64::from(y) * 50.), (50., 50.)),
                    0.,
                    rgb(0xeeeeee),
                    Some((BLACK, 2.)),
                ),
                ((b'A' + i as u8) as char).to_string(),
                Point::new(25., 25.),
                20.,
                BLACK,
            ));
        }
        scene
    }
    fn drag(
        &self,
        s: &Self::State,
        id: &str,
        _: Modifiers,
    ) -> Option<DragSpec<'static, Self::State>> {
        let i = index(id, "tile-").filter(|&i| i < s.positions.len())?;
        let (x, y) = s.positions[i];
        let targets = DIRECTIONS
            .iter()
            .filter_map(|&(dx, dy)| {
                let p = (x + dx, y + dy);
                if !(0..5).contains(&p.0) || !(0..5).contains(&p.1) || s.positions.contains(&p) {
                    return None;
                }
                let mut next = s.clone();
                next.positions[i] = p;
                Some(next)
            })
            .collect();
        Some(
            branches(s, targets)
                .with_snap_radius_options(
                    3.,
                    SnapOptions {
                        chain: true,
                        transition: Some(Transition::default()),
                    },
                )
                .with_branch_transition(None),
        )
    }
    fn controls(&self, _: &Self::State) -> Vec<Control> {
        vec![]
    }
    fn action(&self, _: &Self::State, _: &str, _: Value) -> Option<Self::State> {
        None
    }
}

#[derive(Clone, PartialEq, Debug)]
struct PolyState {
    points: Vec<Point>,
}
struct GridPoly {
    continuous: bool,
}
impl Example for GridPoly {
    type State = PolyState;
    fn initial(&self) -> Self::State {
        PolyState {
            points: vec![
                Point::new(1., 1.),
                Point::new(4., 2.),
                Point::new(3., 5.),
                Point::new(1., 4.),
            ],
        }
    }
    fn scene(&self, s: &Self::State) -> Scene {
        let mut scene = Scene::new(350., 350.);
        for x in 0..6 {
            for y in 0..6 {
                scene.push(
                    Object::circle(
                        format!("grid-{x}-{y}"),
                        (OFFSET + f64::from(x) * TILE, OFFSET + f64::from(y) * TILE),
                        5.,
                        GRAY,
                    )
                    .decorative(),
                );
            }
        }
        for (i, p) in s.points.iter().enumerate() {
            let next = s.points[(i + 1) % s.points.len()];
            scene.push(Object::line(
                format!("edge-{i}"),
                (p.x * TILE + OFFSET, p.y * TILE + OFFSET),
                (next.x * TILE + OFFSET, next.y * TILE + OFFSET),
                BLACK,
                2.,
            ));
        }
        for (i, p) in s.points.iter().enumerate() {
            scene.push(Object::circle(
                format!("point-{i}"),
                (p.x * TILE + OFFSET, p.y * TILE + OFFSET),
                10.,
                BLACK,
            ));
        }
        scene
    }
    fn drag(
        &self,
        s: &Self::State,
        id: &str,
        _: Modifiers,
    ) -> Option<DragSpec<'static, Self::State>> {
        let i = index(id, "point-").filter(|&i| i < s.points.len())?;
        if self.continuous {
            let base = s.clone();
            Some(
                vary([s.points[i].x, s.points[i].y], move |[x, y]| {
                    let mut n = base.clone();
                    n.points[i] = Point::new(x, y);
                    n
                })
                .during(move |s| {
                    let mut n = s.clone();
                    n.points[i] =
                        Point::new(n.points[i].x.clamp(0., 5.), n.points[i].y.clamp(0., 5.));
                    n
                })
                .on_drop(move |s| {
                    let mut n = s.clone();
                    n.points[i] = Point::new(n.points[i].x.round(), n.points[i].y.round());
                    n
                })
                .with_snap_radius(10.),
            )
        } else {
            let mut targets = Vec::new();
            for x in 0..6 {
                for y in 0..6 {
                    let p = Point::new(f64::from(x), f64::from(y));
                    if s.points.iter().enumerate().any(|(j, q)| j != i && *q == p) {
                        continue;
                    }
                    let mut n = s.clone();
                    n.points[i] = p;
                    targets.push(n);
                }
            }
            Some(between(targets))
        }
    }
    fn locate(&self, s: &Self::State, id: &str) -> Option<Affine> {
        let i = index(id, "point-")?;
        let p = *s.points.get(i)?;
        Some(Affine::translate((
            OFFSET + p.x * TILE,
            OFFSET + p.y * TILE,
        )))
    }
}

#[derive(Clone, PartialEq, Debug)]
struct Car {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    horizontal: bool,
}
#[derive(Clone, PartialEq, Debug)]
struct RushState {
    cars: Vec<Car>,
}
struct RushHour;
fn overlaps(a: &Car, b: &Car) -> bool {
    a.x < b.x + b.w && a.x + a.w > b.x && a.y < b.y + b.h && a.y + a.h > b.y
}
fn car_targets(s: &RushState, index: usize) -> Vec<RushState> {
    let car = &s.cars[index];
    let mut targets = vec![s.clone()];
    for sign in [-1, 1] {
        let mut n = s.clone();
        loop {
            if car.horizontal {
                n.cars[index].x += sign
            } else {
                n.cars[index].y += sign
            }
            let moved = &n.cars[index];
            let boundary = if index == 2 { 8 } else { 6 };
            if moved.x < 0
                || moved.x + moved.w > boundary
                || moved.y < 0
                || moved.y + moved.h > 6
                || n.cars
                    .iter()
                    .enumerate()
                    .any(|(j, c)| j != index && overlaps(moved, c))
            {
                break;
            }
            targets.push(n.clone());
        }
    }
    targets
}
impl Example for RushHour {
    type State = RushState;
    fn initial(&self) -> Self::State {
        RushState {
            cars: [
                (0, 0, 2, 1, true),
                (0, 1, 1, 3, false),
                (1, 2, 2, 1, true),
                (0, 4, 1, 2, false),
                (3, 1, 1, 3, false),
                (5, 0, 1, 3, false),
                (4, 4, 2, 1, true),
                (2, 5, 3, 1, true),
            ]
            .map(|(x, y, w, h, horizontal)| Car {
                x,
                y,
                w,
                h,
                horizontal,
            })
            .to_vec(),
        }
    }
    fn scene(&self, s: &Self::State) -> Scene {
        let mut scene = Scene::new(350., 350.);
        scene.push(
            source_rect(
                "border",
                Rect::new(5., 5., 315., 315.),
                0.,
                [0, 0, 0, 0],
                Some((GRAY, 10.)),
            )
            .decorative(),
        );
        grid(&mut scene, 6, 6, 50., 10.);
        scene.push(Object::line("exit", (315., 110.), (315., 160.), WHITE, 10.));
        let colors = [
            rgb(0x90ee90),
            rgb(0x800080),
            rgb(0xff0000),
            rgb(0xffa500),
            rgb(0x0000ff),
            rgb(0xffff00),
            rgb(0xadd8e6),
            rgb(0x008000),
        ];
        for (i, c) in s.cars.iter().enumerate() {
            scene.push(source_rect(
                format!("car-{i}"),
                Rect::from_origin_size(
                    (10. + f64::from(c.x) * 50., 10. + f64::from(c.y) * 50.),
                    (f64::from(c.w) * 50., f64::from(c.h) * 50.),
                ),
                0.,
                colors[i],
                Some((BLACK, 2.)),
            ));
        }
        scene
    }
    fn drag(
        &self,
        s: &Self::State,
        id: &str,
        _: Modifiers,
    ) -> Option<DragSpec<'static, Self::State>> {
        let i = index(id, "car-").filter(|&i| i < s.cars.len())?;
        Some(between(car_targets(s, i)).with_snap_radius(10.))
    }
    fn status(&self, _: &Self::State) -> String {
        String::new()
    }
}

#[derive(Clone, PartialEq, Debug)]
struct FifteenState {
    cells: [usize; 16],
}
struct Fifteen;
fn fifteen_targets(s: &FifteenState, id: usize) -> Vec<FifteenState> {
    let Some(pos) = s.cells.iter().position(|&x| x == id) else {
        return Vec::new();
    };
    let mut targets = Vec::new();
    for (dx, dy) in DIRECTIONS {
        let x = pos as i32 % 4 + dx;
        let y = pos as i32 / 4 + dy;
        if !(0..4).contains(&x) || !(0..4).contains(&y) {
            continue;
        }
        let adj = (y * 4 + x) as usize;
        if id != 0 && s.cells[adj] != 0 {
            continue;
        }
        let mut n = s.clone();
        n.cells.swap(pos, adj);
        targets.push(n);
    }
    targets
}
impl Example for Fifteen {
    type State = FifteenState;
    fn initial(&self) -> Self::State {
        FifteenState {
            cells: [12, 1, 2, 15, 11, 6, 5, 8, 7, 10, 9, 4, 0, 13, 14, 3],
        }
    }
    fn scene(&self, s: &Self::State) -> Scene {
        let mut scene = Scene::new(250., 250.);
        grid(&mut scene, 4, 4, 50., 0.);
        // Object.entries on the source's record visits integer keys in numeric
        // order, then the blank. Paint order must not change when cells swap.
        for id in (1..16).chain(std::iter::once(0)) {
            let pos = s.cells.iter().position(|&tile| tile == id).unwrap();
            let object = source_rect(
                format!("tile-{id}"),
                Rect::from_origin_size(
                    ((pos % 4) as f64 * 50., (pos / 4) as f64 * 50.),
                    (50., 50.),
                ),
                0.,
                if id == 0 { [0, 0, 0, 0] } else { rgb(0xeeeeee) },
                if id == 0 { None } else { Some((BLACK, 2.)) },
            )
            .cursor(Cursor::Grab);
            scene.push(if id == 0 {
                object
            } else {
                centered_label(object, id.to_string(), Point::new(25., 25.), 20., BLACK)
            });
        }
        scene
    }
    fn drag(
        &self,
        s: &Self::State,
        id: &str,
        _: Modifiers,
    ) -> Option<DragSpec<'static, Self::State>> {
        let i = index(id, "tile-").filter(|&i| i < 16)?;
        Some(branches(s, fifteen_targets(s, i)).with_snap_radius_options(
            10.,
            SnapOptions {
                chain: true,
                transition: None,
            },
        ))
    }
    fn status(&self, _: &Self::State) -> String {
        String::new()
    }
}

#[derive(Clone, PartialEq, Debug)]
struct HanoiState {
    pegs: Vec<usize>,
}
struct Hanoi;
fn hanoi_bounds(s: &HanoiState, d: usize) -> Rect {
    let peg = s.pegs[d];
    let below = s.pegs[d + 1..].iter().filter(|&&p| p == peg).count();
    let width = 40. + 25. * d as f64;
    let base_width = 40. + 25. * s.pegs.len() as f64;
    let x = 80. + peg as f64 * (base_width + 40.);
    let height = (s.pegs.len() + 1) as f64 * 20.;
    Rect::from_origin_size(
        (x - width / 2., height - (below + 1) as f64 * 20.),
        (width, 20.),
    )
}
impl Example for Hanoi {
    type State = HanoiState;
    fn initial(&self) -> Self::State {
        HanoiState { pegs: vec![0; 4] }
    }
    fn scene(&self, s: &Self::State) -> Scene {
        let mut scene = Scene::new(if s.pegs.len() == 3 { 500. } else { 600. }, 200.);
        let height = (s.pegs.len() + 1) as f64 * 20.;
        let base_width = 40. + 25. * s.pegs.len() as f64;
        for peg in 0..3 {
            let x = 80. + peg as f64 * (base_width + 40.);
            scene.push(
                source_rect(
                    format!("base-{peg}"),
                    Rect::new(
                        x - base_width / 2.,
                        height,
                        x + base_width / 2.,
                        height + 10.,
                    ),
                    0.,
                    rgb(0x8b4513),
                    Some((BLACK, 2.)),
                )
                .decorative(),
            );
            scene.push(
                source_rect(
                    format!("peg-{peg}"),
                    Rect::new(x - 5., 0., x + 5., height),
                    0.,
                    rgb(0x654321),
                    Some((BLACK, 2.)),
                )
                .decorative(),
            );
        }
        let colors = [rgb(0xff6b6b), rgb(0x4ecdc4), rgb(0x45b7d1), rgb(0xffa07a)];
        for d in (0..s.pegs.len()).rev() {
            let bounds = hanoi_bounds(s, d);
            let object = centered_label(
                source_rect(
                    format!("disk-{d}"),
                    bounds,
                    4.,
                    colors[d],
                    Some((BLACK, 2.)),
                ),
                (d + 1).to_string(),
                Point::new(bounds.width() / 2., 10.),
                14.,
                BLACK,
            )
            .font_weight(700);

            scene.push(object);
        }
        scene
    }
    fn drag(
        &self,
        s: &Self::State,
        id: &str,
        _: Modifiers,
    ) -> Option<DragSpec<'static, Self::State>> {
        let d = index(id, "disk-").filter(|&d| d < s.pegs.len())?;
        if s.pegs[..d].contains(&s.pegs[d]) {
            return None;
        }
        let targets = (0..3).filter(|p| !s.pegs[..d].contains(p)).map(|p| {
            let mut n = s.clone();
            n.pegs[d] = p;
            n
        });
        Some(closest(targets).with_floating_options(FloatingOptions {
            ghost_opacity: Some(0.5),
            ..Default::default()
        }))
    }
    fn controls(&self, _: &Self::State) -> Vec<Control> {
        vec![]
    }
    fn action(&self, _: &Self::State, _: &str, _: Value) -> Option<Self::State> {
        None
    }
}

#[derive(Clone, PartialEq, Debug)]
struct SokobanState {
    walls: Vec<(i32, i32)>,
    boxes: Vec<(i32, i32)>,
    goals: Vec<(i32, i32)>,
    player: (i32, i32),
    editable: bool,
}
struct Sokoban;
fn floor(s: &SokobanState, p: (i32, i32)) -> bool {
    (0..8).contains(&p.0) && (0..9).contains(&p.1) && !s.walls.contains(&p)
}
fn sokoban_step(s: &SokobanState, d: (i32, i32)) -> Option<SokobanState> {
    let p = (s.player.0 + d.0, s.player.1 + d.1);
    if !floor(s, p) {
        return None;
    }
    let mut n = s.clone();
    if let Some(i) = s.boxes.iter().position(|&b| b == p) {
        let dest = (p.0 + d.0, p.1 + d.1);
        if !floor(s, dest) || s.boxes.contains(&dest) {
            return None;
        }
        n.boxes[i] = dest;
    }
    n.player = p;
    Some(n)
}
impl Example for Sokoban {
    type State = SokobanState;
    fn initial(&self) -> Self::State {
        let mut s = SokobanState {
            walls: vec![],
            boxes: vec![],
            goals: vec![],
            player: (0, 0),
            editable: false,
        };
        for (y, line) in [
            "  #####", "###   #", "#gpb  #", "### bg#", "#g##b #", "# # g ##", "#b Bbbg#",
            "#   g  #", "########",
        ]
        .iter()
        .enumerate()
        {
            for (x, ch) in line.chars().enumerate() {
                let p = (x as i32, y as i32);
                match ch {
                    '#' => s.walls.push(p),
                    'b' => s.boxes.push(p),
                    'g' => s.goals.push(p),
                    'p' => s.player = p,
                    'B' => {
                        s.boxes.push(p);
                        s.goals.push(p);
                    }
                    _ => {}
                }
            }
        }
        s
    }
    fn scene(&self, s: &Self::State) -> Scene {
        let mut scene = Scene::new(500., 500.);
        grid(&mut scene, 8, 9, 50., 0.);
        for (i, &(x, y)) in s.walls.iter().enumerate() {
            scene.push(source_rect(
                format!("wall-{i}"),
                Rect::from_origin_size((f64::from(x) * 50., f64::from(y) * 50.), (50., 50.)),
                0.,
                BLACK,
                None,
            ));
        }
        for (i, &(x, y)) in s.boxes.iter().enumerate() {
            scene.push(source_rect(
                format!("box-{i}"),
                Rect::from_origin_size((f64::from(x) * 50., f64::from(y) * 50.), (50., 50.)),
                0.,
                rgb(0xa52a2a),
                Some((BLACK, 2.)),
            ));
        }
        for (i, &(x, y)) in s.goals.iter().enumerate() {
            scene.push(source_rect(
                format!("goal-{i}"),
                Rect::from_origin_size(
                    (f64::from(x) * 50. + 12.5, f64::from(y) * 50. + 12.5),
                    (25., 25.),
                ),
                0.,
                rgb(0xffa500),
                None,
            ));
        }
        scene.push(
            source_rect(
                "player",
                Rect::from_origin_size(
                    (f64::from(s.player.0) * 50., f64::from(s.player.1) * 50.),
                    (50., 50.),
                ),
                0.,
                [0, 0, 0, 0],
                None,
            )
            .label("🧍", (25., 25.), 40., BLACK)
            .text_anchor(TextAnchor::Middle)
            .text_baseline(TextBaseline::Central)
            .cursor(Cursor::Grab),
        );
        if s.editable {
            for object in &mut scene.objects {
                if object.hit.is_some() {
                    object.cursor = Cursor::Grab;
                }
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
        let targets: Vec<SokobanState> = if id == "player" {
            DIRECTIONS
                .into_iter()
                .filter_map(|d| sokoban_step(s, d))
                .collect()
        } else if s.editable {
            let (kind, i) = id.rsplit_once('-')?;
            let i: usize = i.parse().ok()?;
            let source = match kind {
                "wall" => s.walls.get(i),
                "box" => s.boxes.get(i),
                "goal" => s.goals.get(i),
                _ => None,
            }?;
            DIRECTIONS
                .into_iter()
                .filter_map(|(dx, dy)| {
                    let p = (source.0 + dx, source.1 + dy);
                    if !(0..8).contains(&p.0) || !(0..9).contains(&p.1) {
                        return None;
                    }
                    let mut n = s.clone();
                    match kind {
                        "wall" => n.walls[i] = p,
                        "box" => n.boxes[i] = p,
                        "goal" => n.goals[i] = p,
                        _ => return None,
                    }
                    Some(n)
                })
                .collect()
        } else {
            return None;
        };
        Some(
            dragology::closest_with(
                targets.into_iter().map(|n| between([s.clone(), n])),
                dragology::ClosestOptions {
                    lock_past: Some(20.),
                    ..Default::default()
                },
            )
            .with_snap_radius_options(
                3.,
                SnapOptions {
                    chain: true,
                    transition: Some(Transition::default()),
                },
            ),
        )
    }
    fn controls(&self, s: &Self::State) -> Vec<Control> {
        vec![Control::toggle(
            "editable",
            "Make level editable",
            s.editable,
        )]
    }
    fn action(&self, s: &Self::State, id: &str, v: Value) -> Option<Self::State> {
        if let ("editable", Value::Bool(b)) = (id, v) {
            let mut n = s.clone();
            n.editable = b;
            Some(n)
        } else {
            None
        }
    }
    fn status(&self, _: &Self::State) -> String {
        String::new()
    }
}

#[derive(Clone, PartialEq)]
struct SpinState {
    offset: i32,
}
struct Spinny;
impl Example for Spinny {
    type State = SpinState;
    fn initial(&self) -> Self::State {
        SpinState { offset: 0 }
    }
    fn scene(&self, s: &Self::State) -> Scene {
        let mut scene = Scene::new(260., 260.);
        let mut ring = Object::circle("ring", (130., 130.), 100., [0, 0, 0, 0]).decorative();
        if let Shape::Circle { stroke, .. } = &mut ring.shapes[0] {
            *stroke = Some((rgb(0xeeeeee), 8.));
        }
        scene.push(ring);
        for i in 0..4 {
            let angle = PI + (i as f64 - s.offset as f64) * PI / 2.;
            let components = vec![
                TransformComponent::Translate(Vec2::new(130., 130.)),
                TransformComponent::Rotate {
                    radians: angle,
                    center: Point::ORIGIN,
                },
                TransformComponent::Translate(Vec2::new(100., 0.)),
                TransformComponent::Rotate {
                    radians: -angle,
                    center: Point::ORIGIN,
                },
            ];
            scene.push(centered_label(
                Object::new(
                    format!("letter-{i}"),
                    Affine::IDENTITY,
                    Some(Hit::Circle(Point::ORIGIN, 25.)),
                    vec![Shape::Circle {
                        center: Point::ORIGIN,
                        radius: 25.,
                        fill: WHITE,
                        stroke: Some((BLACK, 2.)),
                    }],
                )
                .transforms(components),
                ((b'A' + i as u8) as char).to_string(),
                Point::ORIGIN,
                14.,
                BLACK,
            ));
        }
        scene
    }
    fn drag(
        &self,
        s: &Self::State,
        id: &str,
        _: Modifiers,
    ) -> Option<DragSpec<'static, Self::State>> {
        index(id, "letter-")?;
        Some(
            branches(
                s,
                vec![
                    SpinState {
                        offset: s.offset - 1,
                    },
                    SpinState {
                        offset: s.offset + 1,
                    },
                ],
            )
            .with_snap_radius_options(
                10.,
                SnapOptions {
                    chain: true,
                    transition: None,
                },
            ),
        )
    }
}

#[derive(Clone, PartialEq)]
struct TrominoState {
    missing: (i32, i32),
    level: u32,
    snappy: bool,
    maze: bool,
}
struct Tromino;
fn draw_tromino(scene: &mut Scene, origin: (i32, i32), level: u32, missing: (i32, i32), cell: f64) {
    if level == 0 {
        return;
    }
    let half = 1i32 << (level - 1);
    let mx = usize::from(missing.0 >= origin.0 + half);
    let my = usize::from(missing.1 >= origin.1 + half);
    for qy in 0..2 {
        for qx in 0..2 {
            let o = (origin.0 + qx as i32 * half, origin.1 + qy as i32 * half);
            let center = (
                origin.0 + half - 1 + qx as i32,
                origin.1 + half - 1 + qy as i32,
            );
            let hole = if qx == mx && qy == my {
                missing
            } else {
                center
            };
            draw_tromino(scene, o, level - 1, hole, cell);
        }
    }
    let pad = cell * 0.15;
    let shape = polygon(
        [
            (pad, pad),
            (pad - cell, pad),
            (pad - cell, cell - pad),
            (cell - pad, cell - pad),
            (cell - pad, pad - cell),
            (pad, pad - cell),
        ]
        .map(Point::from),
    );
    let angle = match (mx, my) {
        (0, 0) => 0.,
        (1, 0) => PI / 2.,
        (0, 1) => -PI / 2.,
        _ => PI,
    };
    let transform = Affine::translate((
        f64::from(origin.0 + half) * cell,
        f64::from(origin.1 + half) * cell,
    )) * Affine::rotate(angle);
    scene.push(Object::new(
        format!("tromino-{}-{}-{level}", origin.0, origin.1),
        transform,
        None,
        vec![Shape::Path {
            path: shape,
            fill: Some(
                [rgb(0xadd8e6), rgb(0x90ee90), rgb(0xffb6c1), rgb(0xffffe0)][level as usize - 1],
            ),
            stroke: Some((BLACK, 1.)),
        }],
    ));
}
fn tromino_targets(s: &TrominoState) -> Vec<TrominoState> {
    let n = 1i32 << s.level;
    let mut locations = Vec::new();
    if s.maze {
        for level in 1..=s.level {
            let side = 1i32 << level;
            let half = side / 2;
            let x = s.missing.0.rem_euclid(side);
            let y = s.missing.1.rem_euclid(side);
            if (x == half - 1 || x == half) && (y == half - 1 || y == half) {
                locations.push((
                    s.missing.0 + if x == half - 1 { 1 } else { -1 },
                    s.missing.1,
                ));
                locations.push((
                    s.missing.0,
                    s.missing.1 + if y == half - 1 { 1 } else { -1 },
                ));
            }
        }
    } else {
        for x in 0..n {
            for y in 0..n {
                locations.push((x, y));
            }
        }
    }
    locations
        .into_iter()
        .map(|missing| TrominoState {
            missing,
            ..s.clone()
        })
        .collect()
}
impl Example for Tromino {
    type State = TrominoState;
    fn initial(&self) -> Self::State {
        TrominoState {
            missing: (0, 0),
            level: 3,
            snappy: false,
            maze: false,
        }
    }
    fn scene(&self, s: &Self::State) -> Scene {
        let count = 1u32 << s.level;
        let cell = 40f64.min(370. / f64::from(count));
        let size = f64::from(count) * cell;
        let mut scene = Scene::new(size, size);
        for x in 0..count {
            for y in 0..count {
                scene.push(
                    source_rect(
                        format!("cell-{x}-{y}"),
                        Rect::from_origin_size(
                            (f64::from(x) * cell, f64::from(y) * cell),
                            (cell, cell),
                        ),
                        0.,
                        WHITE,
                        Some((BLACK, 1.)),
                    )
                    .decorative(),
                );
            }
        }
        draw_tromino(&mut scene, (0, 0), s.level, s.missing, cell);
        scene.push(source_rect(
            "missing",
            Rect::from_origin_size(
                (
                    f64::from(s.missing.0) * cell + cell * 0.15,
                    f64::from(s.missing.1) * cell + cell * 0.15,
                ),
                (cell * 0.7, cell * 0.7),
            ),
            0.,
            BLACK,
            None,
        ));
        scene
    }
    fn scene_dragging(&self, s: &Self::State, _: Option<&str>) -> Scene {
        self.scene(s)
    }
    fn drag(
        &self,
        s: &Self::State,
        id: &str,
        _: Modifiers,
    ) -> Option<DragSpec<'static, Self::State>> {
        if id != "missing" {
            return None;
        }
        let targets = tromino_targets(s);
        Some(if s.snappy {
            let spec = closest(std::iter::once(s.clone()).chain(targets)).with_floating_options(
                FloatingOptions {
                    ghost_opacity: Some(0.2),
                    ..Default::default()
                },
            );
            if s.maze { spec.with_chaining() } else { spec }
        } else if s.maze {
            branches(s, targets).with_snap_radius_options(
                1.,
                SnapOptions {
                    chain: true,
                    ..Default::default()
                },
            )
        } else {
            between(targets)
        })
    }
    fn controls(&self, s: &Self::State) -> Vec<Control> {
        vec![
            Control::toggle("snappy", "Snappy mode", s.snappy),
            Control::toggle("maze", "Maze mode", s.maze),
            slider("level", "Board level", f64::from(s.level), 1., 4.),
        ]
    }
    fn action(&self, s: &Self::State, id: &str, v: Value) -> Option<Self::State> {
        let mut n = s.clone();
        match (id, v) {
            ("snappy", Value::Bool(b)) => n.snappy = b,
            ("maze", Value::Bool(b)) => n.maze = b,
            ("level", Value::Number(l)) => {
                n.level = l.round().clamp(1., 4.) as u32;
                n.missing = (0, 0);
            }
            _ => return None,
        }
        Some(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_tiles_figures_keep_independent_models() {
        let mut page = tiles_page();
        page.set_animated(false);
        let friendly = page
            .scene()
            .objects
            .iter()
            .filter(|o| o.id.starts_with("friendly/"))
            .cloned()
            .collect::<Vec<_>>();
        let start = page.scene().object("lonely/tile-0").unwrap().transform * Point::new(25., 25.);
        page.press(start, Modifiers::default(), 1);
        page.move_to(start + Vec2::new(50., 0.), Modifiers::default());
        page.advance(std::time::Duration::ZERO);
        page.release(start + Vec2::new(50., 0.), Modifiers::default());
        let moved = page.scene().object("lonely/tile-0").unwrap().transform * Point::new(25., 25.);
        assert!(moved.distance(start + Vec2::new(50., 0.)) < 1e-6);
        assert_eq!(
            page.scene()
                .objects
                .iter()
                .filter(|o| o.id.starts_with("friendly/"))
                .cloned()
                .collect::<Vec<_>>(),
            friendly
        );
    }
    #[test]
    fn hanoi_three_and_four_disk_figures_are_both_present_and_independent() {
        let mut page = hanoi_page();
        page.set_animated(false);
        let four = page
            .scene()
            .objects
            .iter()
            .filter(|o| o.id.starts_with("four/"))
            .cloned()
            .collect::<Vec<_>>();
        let initial = HanoiState { pegs: vec![0; 3] };
        let source = hanoi_bounds(&initial, 0);
        let object = page.scene().object("three/disk-0").unwrap();
        let offset = (object.transform * Point::ORIGIN) - source.origin();
        let mut next = initial;
        next.pegs[0] = 1;
        let target = hanoi_bounds(&next, 0).center() + offset;
        let start = source.center() + offset;
        page.press(start, Modifiers::default(), 1);
        page.move_to(target, Modifiers::default());
        page.advance(std::time::Duration::ZERO);
        page.release(target, Modifiers::default());
        assert!(
            (page.scene().object("three/disk-0").unwrap().transform * Point::new(20., 10.))
                .distance(target)
                < 1e-6
        );
        assert_eq!(
            page.scene()
                .objects
                .iter()
                .filter(|o| o.id.starts_with("four/"))
                .cloned()
                .collect::<Vec<_>>(),
            four
        );
        assert!(page.controls().is_empty());
    }
    #[test]
    fn all_board_variants_accept_real_engine_gestures() {
        exercise(Tiles, vec![Tiles.initial()]);
        exercise(
            GridPoly { continuous: false },
            vec![GridPoly { continuous: false }.initial()],
        );
        exercise(
            GridPoly { continuous: true },
            vec![GridPoly { continuous: true }.initial()],
        );
        exercise(RushHour, vec![RushHour.initial()]);
        exercise(Fifteen, vec![Fifteen.initial()]);
        exercise(Hanoi, vec![Hanoi.initial()]);
        exercise(
            Sokoban,
            vec![
                Sokoban.initial(),
                SokobanState {
                    editable: true,
                    ..Sokoban.initial()
                },
            ],
        );
        exercise(Spinny, vec![Spinny.initial()]);
        exercise(
            Tromino,
            [false, true]
                .into_iter()
                .flat_map(|maze| {
                    [false, true].map(|snappy| TrominoState {
                        maze,
                        snappy,
                        ..Tromino.initial()
                    })
                })
                .collect(),
        );
    }
    #[test]
    fn sokoban_pushes_only_one_box_into_clear_floor() {
        let s = SokobanState {
            walls: vec![],
            boxes: vec![(2, 1)],
            goals: vec![],
            player: (1, 1),
            editable: false,
        };
        let pushed = sokoban_step(&s, (1, 0)).unwrap();
        assert_eq!(pushed.player, (2, 1));
        assert_eq!(pushed.boxes, vec![(3, 1)]);
        let retreat = sokoban_step(&pushed, (-1, 0)).unwrap();
        assert_eq!(retreat.boxes, pushed.boxes);
        assert!(
            sokoban_step(
                &SokobanState {
                    boxes: vec![(2, 1), (3, 1)],
                    ..s.clone()
                },
                (1, 0)
            )
            .is_none()
        );
        assert!(
            sokoban_step(
                &SokobanState {
                    walls: vec![(3, 1)],
                    ..s
                },
                (1, 0)
            )
            .is_none()
        );
    }
    #[test]
    fn rush_hour_moves_stay_in_lanes_without_crossing_cars() {
        let s = RushHour.initial();
        for i in 0..s.cars.len() {
            for target in car_targets(&s, i) {
                let car = &target.cars[i];
                if car.horizontal {
                    assert_eq!(car.y, s.cars[i].y);
                } else {
                    assert_eq!(car.x, s.cars[i].x);
                }
                assert!(
                    !target
                        .cars
                        .iter()
                        .enumerate()
                        .any(|(j, other)| i != j && overlaps(car, other))
                );
            }
        }
    }
    #[test]
    fn fifteen_moves_exchange_the_gap_with_an_orthogonal_neighbor() {
        let s = Fifteen.initial();
        for i in 0..16 {
            for target in fifteen_targets(&s, i) {
                let changed = (0..16)
                    .filter(|&j| s.cells[j] != target.cells[j])
                    .collect::<Vec<_>>();
                assert_eq!(changed.len(), 2);
                assert!(changed.iter().any(|&j| s.cells[j] == 0));
                let (a, b) = (changed[0] as i32, changed[1] as i32);
                assert_eq!((a % 4 - b % 4).abs() + (a / 4 - b / 4).abs(), 1);
            }
        }
    }
}
