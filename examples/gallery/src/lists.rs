//! Collection demos ported from the upstream MIT-licensed Dragology gallery.
use crate::*;
use dragology::{between, closest, drop_target, fixed, vary};

mod components;
mod nested;

pub fn entries() -> Vec<Entry> {
    vec![
        Entry::new(
            "perm",
            "Permutation",
            "Drag a tile to interpolate between reorderings.",
            || demo(Rows(Mode::Perm)),
        ),
        Entry::new(
            "perm-floating",
            "Floating permutation",
            "Move a tile freely; the row previews its insertion. Pull away to cancel the reordering.",
            || demo(Rows(Mode::Floating)),
        ),
        Entry::new(
            "perm-double",
            "Rows and columns",
            "A tile changes position within its row while the whole row changes order.",
            || demo(Rows(Mode::Double)),
        ),
        Entry::new(
            "lists-2d",
            "Lists in two dimensions",
            "Drag tiles between rows. Drag a row by its grip. Pull a tile or row far away to remove it.",
            || demo(Rows(Mode::Lists)),
        ),
        Entry::new(
            "lists-2d-sizes",
            "Lists with different sizes",
            "Move differently sized tiles between rows and watch their containers adapt.",
            || demo(Rows(Mode::Sizes)),
        ),
        Entry::new(
            "canvas-of-lists",
            "Canvas of lists",
            "Move row grips freely. Pull a tile away from its row to create a new row.",
            || demo(Rows(Mode::Canvas)),
        ),
        Entry::new(
            "lists-in-lists",
            "Nested lists",
            "Move tiles and whole rows between nesting levels. Pull away to create a top-level row.",
            || demo(nested::Nested(false)),
        ),
        Entry::new(
            "lists-in-lists-vanilla",
            "Nested lists with drop indicators",
            "The upstream imperative comparison uses insertion indicators and a floating item without rearranging the destination during the drag.",
            || demo(nested::Nested(true)),
        ),
        Entry::new(
            "kanban",
            "Kanban",
            "Drag cards between columns, or drag a column header to reorder columns.",
            || demo(Rows(Mode::Kanban)),
        ),
        Entry::new(
            "ring-of-beads",
            "Ring of beads",
            "Reorder the same ring using four different drag specifications.",
            || demo(Beads),
        ),
        Entry::new(
            "insert-and-remove",
            "Insert and remove",
            "Pull fruit from the palette to create tiles. Reorder them or drop them in the trash.",
            || demo(Palette(false)),
        ),
        Entry::new(
            "insert-and-remove-original",
            "Insert and remove, original",
            "The original version keeps a tile visible over the trash until release and removes tiles pulled far away.",
            || demo(Palette(true)),
        ),
        Entry::new(
            "todo",
            "To-do list",
            "Add tasks, toggle completion, and reorder them. Floating mode also allows pulling a task out of the list.",
            || demo(components::Todo),
        ),
        Entry::new(
            "text-box",
            "Text box",
            "Edit the text field to update the text in the drawing.",
            || demo(components::TextBox),
        ),
        Entry::new(
            "carousel",
            "Carousel",
            "Swipe slides, drag the active dot, or click the arrows and other dots.",
            || demo(components::Carousel),
        ),
        Entry::new(
            "bluefish-static",
            "Bluefish static layout",
            "A native rendering of the upstream Bluefish planet diagram, including the Mercury reference. This example is static.",
            || demo(BluefishStatic),
        ),
        Entry::new(
            "bluefish-perm",
            "Bluefish permutation",
            "The Bluefish layout variant of tile reordering, translated to native geometry.",
            || demo(Rows(Mode::BluefishPerm)),
        ),
        Entry::new(
            "bluefish-lists-2d-sizes",
            "Bluefish sized lists",
            "The Bluefish layout variant of differently sized tiles and draggable rows.",
            || demo(Rows(Mode::BluefishSizes)),
        ),
    ]
}

const WHITE: Color = [255, 255, 255, 255];
const BORDER: Color = [170, 170, 170, 255];
const BLACK: Color = [0, 0, 0, 255];
const LABEL: Color = [85, 85, 85, 255];
const ROW_COLORS: [Color; 3] = [
    [240, 244, 255, 255],
    [255, 244, 240, 255],
    [240, 255, 244, 255],
];
const CANVAS_COLORS: [Color; 8] = [
    [201, 228, 240, 255],
    [245, 213, 216, 255],
    [212, 237, 207, 255],
    [245, 234, 201, 255],
    [228, 212, 232, 255],
    [245, 220, 201, 255],
    [201, 240, 237, 255],
    [232, 212, 240, 255],
];
fn stroked(mut object: Object, color: Color, width: f64) -> Object {
    if let Shape::Rect { stroke, .. } | Shape::Circle { stroke, .. } = &mut object.shapes[0] {
        *stroke = Some((color, width));
    }
    object
}
fn square_tile(id: &str, label: &str, bounds: Rect) -> Object {
    stroked(Object::rect(id, bounds, 0., WHITE), BLACK, 2.)
        .label(
            label,
            (bounds.width() / 2., bounds.height() / 2.),
            20.,
            BLACK,
        )
        .text_anchor(TextAnchor::Middle)
        .text_baseline(TextBaseline::Central)
}

fn tile_object(id: &str, label: &str, bounds: Rect) -> Object {
    let mut object = Object::rect(id, bounds, 4., WHITE);
    if let Shape::Rect { stroke, .. } = &mut object.shapes[0] {
        *stroke = Some((BORDER, 1.5));
    }
    object
        .label(
            label,
            (bounds.width() / 2., bounds.height() / 2.),
            18.,
            LABEL,
        )
        .text_anchor(TextAnchor::Middle)
        .text_baseline(TextBaseline::Middle)
        .font_weight(500)
}

fn grip(object: &mut Object, height: f64) {
    for y in [-8., 0., 8.] {
        for x in [8., 16.] {
            object.shapes.push(Shape::Circle {
                center: Point::new(x, height * 0.5 + y),
                radius: 1.5,
                fill: [51, 51, 51, 89],
                stroke: None,
            });
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Tile {
    id: String,
    label: String,
    width: f64,
    height: f64,
}
impl Tile {
    fn new(id: impl Into<String>) -> Self {
        let id = id.into();
        Self {
            label: id.clone(),
            id,
            width: 50.,
            height: 50.,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
struct Row {
    id: String,
    title: String,
    items: Vec<Tile>,
    color: Color,
    position: Point,
}
#[derive(Clone, Debug, PartialEq)]
struct RowsState {
    rows: Vec<Row>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Perm,
    Floating,
    Double,
    Lists,
    Sizes,
    Canvas,
    Kanban,
    BluefishPerm,
    BluefishSizes,
}
struct Rows(Mode);

fn moved<T: Clone>(items: &[T], from: usize, to: usize) -> Vec<T> {
    let mut result = items.to_vec();
    let item = result.remove(from);
    result.insert(to, item);
    result
}

impl Rows {
    fn sized(&self) -> bool {
        matches!(self.0, Mode::Sizes | Mode::BluefishSizes)
    }
    fn plain(&self) -> bool {
        matches!(
            self.0,
            Mode::Perm | Mode::Floating | Mode::Double | Mode::BluefishPerm
        )
    }
    fn candidates(
        &self,
        state: &RowsState,
        row: usize,
        item: usize,
    ) -> (RowsState, Vec<RowsState>) {
        let mut without = state.clone();
        let tile = without.rows[row].items.remove(item);
        if self.0 == Mode::Canvas && without.rows[row].items.is_empty() {
            without.rows.remove(row);
        }
        let mut candidates = Vec::new();
        for r in 0..without.rows.len() {
            for i in 0..=without.rows[r].items.len() {
                let mut next = without.clone();
                next.rows[r].items.insert(i, tile.clone());
                candidates.push(next);
            }
        }
        (without, candidates)
    }
}

impl Example for Rows {
    type State = RowsState;
    fn initial(&self) -> RowsState {
        let rows = if matches!(self.0, Mode::Perm | Mode::Floating | Mode::BluefishPerm) {
            vec![Row {
                id: "row1".into(),
                title: String::new(),
                items: ["A", "B", "C", "D", "E"]
                    .into_iter()
                    .map(Tile::new)
                    .collect(),
                color: ROW_COLORS[0],
                position: Point::ORIGIN,
            }]
        } else if self.0 == Mode::Kanban {
            ["To Do", "Doing", "Done"]
                .into_iter()
                .enumerate()
                .map(|(r, title)| Row {
                    id: format!("row{}", r + 1),
                    title: title.into(),
                    color: [240, 240, 240, 255],
                    position: Point::ORIGIN,
                    items: (if r == 0 {
                        vec!["Task A", "Task B"]
                    } else if r == 1 {
                        vec!["Task C"]
                    } else {
                        vec!["Task D"]
                    })
                    .into_iter()
                    .map(Tile::new)
                    .collect(),
                })
                .collect()
        } else {
            (0..3)
                .map(|r| {
                    let items = (0..3)
                        .map(|i| {
                            let mut t = Tile::new(format!("{}{}", char::from(b'A' + i), r + 1));
                            if self.sized() {
                                if r == 0 && i == 0 {
                                    t.width = 108.;
                                    t.height = 100.;
                                }
                                if r == 1 && i == 1 {
                                    t.width = 108.;
                                }
                                if r == 2 && i == 2 {
                                    t.height = 108.;
                                }
                            }
                            t
                        })
                        .collect();
                    Row {
                        id: format!("row{}", r + 1),
                        title: String::new(),
                        items,
                        color: if self.0 == Mode::Canvas {
                            CANVAS_COLORS[r]
                        } else {
                            ROW_COLORS[r]
                        },
                        position: Point::new([0., 20., 100.][r], r as f64 * 100.),
                    }
                })
                .collect()
        };
        RowsState { rows }
    }
    fn scene(&self, state: &RowsState) -> Scene {
        let (width, height) = match self.0 {
            Mode::Perm | Mode::Floating => (350., 100.),
            Mode::Double => (250., 200.),
            Mode::Lists => (400., 300.),
            Mode::Sizes | Mode::BluefishSizes => (400., 450.),
            Mode::Canvas => (600., 400.),
            Mode::Kanban => (400., 200.),
            Mode::BluefishPerm => (300., 120.),
        };
        let mut scene = Scene::new(width, height);
        let mut next_y = if self.0 == Mode::BluefishSizes {
            10.
        } else {
            0.
        };
        for (r, row) in state.rows.iter().enumerate() {
            let height = row.items.iter().map(|t| t.height).fold(
                if self.0 == Mode::BluefishSizes {
                    17.
                } else {
                    50.
                },
                f64::max,
            ) + 16.;
            let width = 34. + row.items.iter().map(|t| t.width + 8.).sum::<f64>()
                - if self.0 == Mode::BluefishSizes && row.items.is_empty() {
                    0.
                } else {
                    8.
                };
            let (x, y) = if self.0 == Mode::Canvas {
                (row.position.x, row.position.y)
            } else if self.0 == Mode::Kanban {
                (r as f64 * 130., 0.)
            } else {
                (
                    if matches!(self.0, Mode::BluefishPerm | Mode::BluefishSizes) {
                        10.
                    } else {
                        0.
                    },
                    next_y,
                )
            };
            if !self.plain() {
                let (w, h) = if self.0 == Mode::Kanban {
                    (120., 35. + row.items.len() as f64 * 35.)
                } else {
                    (width, height)
                };
                let mut bg = stroked(
                    Object::rect(
                        &row.id,
                        Rect::new(x, y, x + w, y + h),
                        if self.0 == Mode::Kanban { 4. } else { 6. },
                        row.color,
                    ),
                    if self.0 == Mode::Kanban {
                        [204, 204, 204, 255]
                    } else {
                        BORDER
                    },
                    if self.0 == Mode::Kanban { 1. } else { 1.5 },
                );
                if self.0 == Mode::Kanban {
                    bg = bg
                        .label(&row.title, (60., 16.5), 12., [51, 51, 51, 255])
                        .text_anchor(TextAnchor::Middle)
                        .text_baseline(TextBaseline::Alphabetic)
                        .font_weight(700);
                } else {
                    if self.0 == Mode::BluefishSizes {
                        for i in 0..3 {
                            for j in 0..2 {
                                bg.shapes.push(Shape::Circle {
                                    center: Point::new(
                                        9.5 + j as f64 * 7.,
                                        h / 2. + (i as f64 - 1.) * 7.,
                                    ),
                                    radius: 1.5,
                                    fill: [51, 51, 51, 89],
                                    stroke: None,
                                });
                            }
                        }
                    } else {
                        grip(&mut bg, h);
                    }
                }
                scene.push(bg);
            }
            let mut next_x = if self.0 == Mode::Double {
                5.
            } else if self.plain() {
                0.
            } else {
                26.
            };
            for (i, tile) in row.items.iter().enumerate() {
                let (tx, ty, w, h) = if self.0 == Mode::Kanban {
                    (x + 5., y + 30. + i as f64 * 35., 110., 30.)
                } else {
                    (
                        x + next_x,
                        y + if self.0 == Mode::Double {
                            5.
                        } else if self.0 == Mode::BluefishPerm {
                            30.
                        } else if self.plain() {
                            0.
                        } else {
                            8.
                        },
                        tile.width,
                        tile.height,
                    )
                };
                let bounds = Rect::new(tx, ty, tx + w, ty + h);
                let mut object = if self.plain() {
                    square_tile(&tile.id, &tile.label, bounds).text_baseline(TextBaseline::Middle)
                } else if self.0 == Mode::Kanban {
                    stroked(
                        Object::rect(&tile.id, bounds, 3., WHITE),
                        [221, 221, 221, 255],
                        1.,
                    )
                    .label(&tile.label, (55., 19.), 11., [51, 51, 51, 255])
                    .text_anchor(TextAnchor::Middle)
                    .text_baseline(TextBaseline::Alphabetic)
                } else {
                    tile_object(&tile.id, &tile.label, bounds)
                };
                if matches!(self.0, Mode::BluefishPerm | Mode::BluefishSizes) {
                    let font = if self.0 == Mode::BluefishPerm {
                        20.
                    } else {
                        18.
                    };
                    if let Some(Shape::Text { origin, .. }) = object.shapes.last_mut() {
                        *origin = Point::new(w / 2., h / 2. + font / 2.);
                    }
                    object = object
                        .text_baseline(TextBaseline::Alphabetic)
                        .font_family(FontFamily::Named("Alegreya Sans".into()));
                }
                if !self.plain() {
                    object.parent = Some(row.id.clone());
                }
                scene.push(object);
                next_x += tile.width + if self.plain() { 0. } else { 8. };
            }
            next_y += if self.plain() { 60. } else { height + 10. };
        }
        // Source ID layers place every row background [0] below every tile
        // [0, 1], including tiles belonging to earlier, overlapping rows.
        scene.objects.sort_by_key(|object| object.parent.is_some());
        scene
    }
    fn scene_dragging(&self, state: &RowsState, id: Option<&str>) -> Scene {
        let mut scene = self.scene(state);
        if let Some(id) = id {
            if matches!(self.0, Mode::Perm | Mode::BluefishPerm) {
                if let Some(item) = scene.objects.iter_mut().find(|o| o.id == id) {
                    let lift = if self.0 == Mode::Perm { 10. } else { 20. };
                    item.transform = Affine::translate((0., -lift)) * item.transform;
                }
            }
            let row = if self.0 == Mode::Double {
                state
                    .rows
                    .iter()
                    .find(|r| r.items.iter().any(|t| t.id == id))
            } else {
                None
            };
            if self.0 == Mode::Kanban {
                if let Some(object) = scene.objects.iter_mut().find(|o| o.id == id) {
                    if object.parent.is_some() {
                        if let Shape::Rect { fill, stroke, .. } = &mut object.shapes[0] {
                            *fill = [227, 242, 253, 255];
                            *stroke = Some(([33, 150, 243, 255], 1.));
                        }
                    }
                }
            }
            let row_drag = state.rows.iter().any(|r| r.id == id);
            scene.objects.sort_by_key(|o| {
                if row_drag && o.parent.as_deref() == Some(id) {
                    3
                } else if o.id == id {
                    2
                } else if row.is_some_and(|r| r.items.iter().any(|t| t.id == o.id)) {
                    1
                } else {
                    0
                }
            });
        }
        scene
    }
    fn drag(
        &self,
        state: &RowsState,
        id: &str,
        _: Modifiers,
    ) -> Option<DragSpec<'static, RowsState>> {
        if let Some(r) = state.rows.iter().position(|row| row.id == id) {
            if self.plain() {
                return None;
            }
            if self.0 == Mode::Canvas {
                let base = state.clone();
                let p = base.rows[r].position;
                return Some(vary([p.x, p.y], move |[x, y]| {
                    let mut s = base.clone();
                    s.rows[r].position = Point::new(x, y);
                    s
                }));
            }
            let mut states = (0..state.rows.len())
                .map(|to| {
                    let mut s = state.clone();
                    s.rows = moved(&s.rows, r, to);
                    s
                })
                .collect::<Vec<_>>();
            if self.0 == Mode::Kanban {
                // Source places the unchanged candidate first, before the
                // alternate column positions (relevant at equal-distance ties).
                let original = states.remove(r);
                states.insert(0, original);
                return Some(between(states).with_snap_radius_options(
                    20.,
                    dragology::SnapOptions {
                        transition: Some(dragology::Transition::default()),
                        chain: false,
                    },
                ));
            }
            let mut without = state.clone();
            without.rows.remove(r);
            return Some(closest(states).when_far(without, 50.).with_floating());
        }
        let (r, i) = state
            .rows
            .iter()
            .enumerate()
            .find_map(|(r, row)| row.items.iter().position(|t| t.id == id).map(|i| (r, i)))?;
        if matches!(self.0, Mode::Perm | Mode::Floating | Mode::BluefishPerm) {
            let states = (0..state.rows[r].items.len())
                .map(|to| {
                    let mut s = state.clone();
                    s.rows[r].items = moved(&s.rows[r].items, i, to);
                    s
                })
                .collect::<Vec<_>>();
            return Some(if self.0 == Mode::Floating {
                closest(states).when_far(state.clone(), 50.).with_floating()
            } else {
                between(states)
            });
        }
        if self.0 == Mode::Double {
            let mut states = Vec::new();
            for col in 0..state.rows[r].items.len() {
                for row in 0..state.rows.len() {
                    let mut s = state.clone();
                    s.rows[r].items = moved(&s.rows[r].items, i, col);
                    s.rows = moved(&s.rows, r, row);
                    states.push(s);
                }
            }
            return Some(between(states).with_snap_radius(10.));
        }
        let tile = state.rows[r].items[i].clone();
        let (without, states) = self.candidates(state, r, i);
        if self.0 == Mode::Canvas {
            let mut detached = without.clone();
            let mut n = 1;
            while detached.rows.iter().any(|r| r.id == format!("row{n}")) {
                n += 1;
            }
            detached.rows.push(Row {
                id: format!("row{n}"),
                title: String::new(),
                items: vec![tile],
                color: CANVAS_COLORS[(n - 1) % CANVAS_COLORS.len()],
                position: Point::ORIGIN,
            });
            let last = detached.rows.len() - 1;
            let free = vary([0., 0.], move |[x, y]| {
                let mut s = detached.clone();
                s.rows[last].position = Point::new(x, y);
                s
            });
            return Some(closest(states).with_floating().when_far(free, 50.));
        }
        if self.0 == Mode::Kanban {
            return Some(
                closest(states).with_floating_options(dragology::FloatingOptions {
                    ghost_opacity: Some(0.3),
                    ..Default::default()
                }),
            );
        }
        Some(closest(states).when_far(without, 50.).with_floating())
    }
}

#[derive(Clone, PartialEq)]
struct BeadState {
    beads: Vec<String>,
    stage: usize,
}
struct Beads;
impl Example for Beads {
    type State = BeadState;
    fn initial(&self) -> BeadState {
        BeadState {
            beads: ["A", "B", "C", "D", "E"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
            stage: 0,
        }
    }
    fn scene(&self, s: &BeadState) -> Scene {
        let mut scene = Scene::new(300., 300.);
        let mut ring = Object::circle("ring", (150., 150.), 100., [0; 4]).decorative();
        if let Shape::Circle { stroke, .. } = &mut ring.shapes[0] {
            *stroke = Some(([224, 224, 224, 255], 1.5));
        }
        scene.push(ring);
        for (i, label) in s.beads.iter().enumerate() {
            let angle = i as f64 / s.beads.len() as f64 * std::f64::consts::TAU
                - std::f64::consts::FRAC_PI_2;
            let color = [
                [232, 180, 184, 255],
                [180, 212, 232, 255],
                [184, 232, 180, 255],
                [232, 212, 180, 255],
                [212, 180, 232, 255],
            ][(label.as_bytes()[0] - b'A') as usize];
            scene.push(
                stroked(
                    Object::circle(
                        label,
                        (150. + 100. * angle.cos(), 150. + 100. * angle.sin()),
                        24.,
                        color,
                    ),
                    [153, 153, 153, 255],
                    1.5,
                )
                .label(label, (0., 0.), 16., [51, 51, 51, 255])
                .text_anchor(TextAnchor::Middle)
                .text_baseline(TextBaseline::Central)
                .font_weight(500),
            );
        }
        scene
    }
    fn drag(&self, s: &BeadState, id: &str, _: Modifiers) -> Option<DragSpec<'static, BeadState>> {
        let from = s.beads.iter().position(|b| b == id)?;
        let states = (0..s.beads.len())
            .map(|to| {
                let mut t = s.clone();
                t.beads = moved(&s.beads, from, to);
                t
            })
            .collect::<Vec<_>>();
        Some(match s.stage {
            0 => closest(states),
            1 => closest(states).with_floating(),
            2 => {
                let mut removed = s.clone();
                removed.beads.remove(from);
                let original = s.clone();
                closest(states)
                    .when_far(fixed(removed).on_drop(move |_| original.clone()), 65.)
                    .with_floating()
            }
            _ => between(states),
        })
    }
    fn scene_dragging(&self, s: &BeadState, id: Option<&str>) -> Scene {
        let mut scene = self.scene(s);
        if let Some(id) = id {
            for o in &mut scene.objects {
                if o.id == id {
                    *o = o.clone().shadow(Shadow {
                        offset: Vec2::new(0., 2.),
                        blur: 3.,
                        color: [0, 0, 0, 64],
                    });
                }
            }
            scene.objects.sort_by_key(|o| o.id == id);
        }
        scene
    }
    fn controls(&self, s: &BeadState) -> Vec<Control> {
        vec![Control::Choice {
            id: "stage",
            label: "Stage".into(),
            options: [
                "d.closest()",
                "d.closest().withFloating()",
                "d.closest().whenFar().withFloating()",
                "d.between()",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect(),
            selected: s.stage,
        }]
    }
    fn action(&self, s: &BeadState, id: &str, v: Value) -> Option<BeadState> {
        if let ("stage", Value::Index(stage)) = (id, v) {
            let mut s = s.clone();
            s.stage = stage.min(3);
            Some(s)
        } else {
            None
        }
    }
}

#[derive(Clone, PartialEq)]
struct PaletteState {
    items: Vec<Tile>,
    store: Vec<Tile>,
    deleted: Option<Tile>,
    next: usize,
    creating: Option<String>,
}
struct Palette(bool);
impl Example for Palette {
    type State = PaletteState;
    fn initial(&self) -> PaletteState {
        let fruit = |id: &str, label: &str| Tile {
            id: id.into(),
            label: label.into(),
            width: 50.,
            height: 50.,
        };
        PaletteState {
            items: vec![fruit("A", "🍎"), fruit("B", "🍎"), fruit("C", "🍌")],
            store: vec![fruit("D", "🍎"), fruit("E", "🍌"), fruit("F", "🍇")],
            deleted: None,
            next: 0,
            creating: None,
        }
    }
    fn scene(&self, s: &PaletteState) -> Scene {
        let mut scene = Scene::new(400., 200.);
        scene.push(
            stroked(
                Object::rect(
                    "toolbar",
                    Rect::new(-5., -5., 165., 55.),
                    4.,
                    [245, 245, 245, 255],
                ),
                [204, 204, 204, 255],
                1.,
            )
            .decorative(),
        );
        for (i, t) in s.store.iter().enumerate() {
            scene.push(square_tile(
                &t.id,
                &t.label,
                Rect::new(5. + i as f64 * 50., 0., 55. + i as f64 * 50., 50.),
            ));
        }
        for (i, t) in s.items.iter().enumerate() {
            scene.push(square_tile(
                &t.id,
                &t.label,
                Rect::new(i as f64 * 50., 70., 50. + i as f64 * 50., 120.),
            ));
        }
        let trash = stroked(
            Object::rect(
                "trash",
                Rect::new(230., 0., 280., 50.),
                4.,
                [255, 238, 238, 255],
            ),
            [153, 153, 153, 255],
            2.,
        )
        .dash(vec![4., 4.])
        .label("🗑", (25., 25.), 30., BLACK)
        .text_anchor(TextAnchor::Middle)
        .text_baseline(TextBaseline::Central)
        .decorative();
        scene.push(trash);
        if let Some(t) = &s.deleted {
            scene.push(square_tile(&t.id, &t.label, Rect::new(230., 0., 280., 50.)));
        }
        scene
    }
    fn begin_drag(
        &self,
        s: &PaletteState,
        id: &str,
        _: Modifiers,
    ) -> Option<(PaletteState, String)> {
        if self.0 {
            return None;
        }
        let tile = s.store.iter().find(|t| t.id == id)?;
        let mut next = s.clone();
        let new_id = format!("created-{}", s.next);
        next.next += 1;
        next.items.push(Tile {
            id: new_id.clone(),
            ..tile.clone()
        });
        next.creating = Some(new_id.clone());
        Some((next, new_id))
    }
    fn drag(
        &self,
        s: &PaletteState,
        id: &str,
        _: Modifiers,
    ) -> Option<DragSpec<'static, PaletteState>> {
        let mut without = s.clone();
        without.creating = None;
        let store = without.store.iter().position(|t| t.id == id);
        let tile = if let Some(i) = store {
            let tile = without.store[i].clone();
            without.store[i].id = if self.0 {
                format!("{}-1", tile.id)
            } else {
                format!("stock-{}", without.next)
            };
            without.next += 1;
            tile
        } else {
            let i = without.items.iter().position(|t| t.id == id)?;
            without.items.remove(i)
        };
        let states = (0..=without.items.len())
            .map(|i| {
                let mut n = without.clone();
                n.items.insert(i, tile.clone());
                n
            })
            .collect::<Vec<_>>();
        if self.0 {
            if store.is_some() {
                return Some(closest(states).when_far(without, 50.).with_floating());
            }
            let mut deleted = without.clone();
            // Preserve the original demo's second splice, including its
            // removal of the following tile. The newer demo removes only one.
            let index = s.items.iter().position(|t| t.id == tile.id).unwrap();
            if index < deleted.items.len() {
                deleted.items.remove(index);
            }
            deleted.deleted = Some(tile);
            let mut drop = deleted.clone();
            drop.deleted = None;
            return Some(
                closest([
                    closest(states),
                    fixed(deleted).on_drop(move |_| drop.clone()),
                ])
                .when_far(without, 50.)
                .with_floating(),
            );
        }
        let original = if s.creating.as_deref() == Some(id) {
            without.clone()
        } else {
            s.clone()
        };
        let far = if store.is_some() {
            fixed(without.clone()).on_drop(move |_| original.clone())
        } else {
            fixed(original)
        };
        Some(
            closest([
                closest(states).with_floating(),
                drop_target(without, Rect::new(230., 0., 280., 50.)),
            ])
            .when_far(far.with_floating(), 50.),
        )
    }
}

struct BluefishStatic;
impl Example for BluefishStatic {
    type State = ();
    fn initial(&self) {}
    fn scene(&self, _: &()) -> Scene {
        let mut scene = Scene::new(500., 250.);
        scene.push(
            Object::rect(
                "planet-background",
                Rect::new(10., 10., 460., 166.),
                0.,
                [133, 159, 201, 255],
            )
            .decorative(),
        );
        for (i, (x, r, color)) in [
            (65., 15., [235, 227, 207, 255]),
            (166., 36., [220, 147, 60, 255]),
            (290., 38., [23, 157, 215, 255]),
            (399., 21., [241, 207, 142, 255]),
        ]
        .into_iter()
        .enumerate()
        {
            scene.push(
                stroked(
                    Object::circle(format!("planet-{i}"), (x, 88.), r, color),
                    BLACK,
                    3.,
                )
                .decorative(),
            );
        }
        // Bluefish's StackV measures a 14px bold label; the reference shares
        // Mercury's position rather than drawing a second planet.
        let label_style = TextStyle {
            anchor: TextAnchor::Middle,
            baseline: TextBaseline::Alphabetic,
            family: FontFamily::Named("Alegreya Sans".into()),
            weight: 700,
            ..TextStyle::default()
        };
        let label_width = crate::text_layout::measure_text("Mercury", 14., &label_style).advance;
        scene.push(
            stroked(
                Object::rect(
                    "mercury-reference",
                    Rect::new(
                        65. - label_width / 2. - 10.,
                        19.,
                        65. + label_width / 2. + 10.,
                        113.,
                    ),
                    10.,
                    [0; 4],
                ),
                BLACK,
                3.,
            )
            .decorative(),
        );
        scene.push(
            Object::text("mercury-label", (65., 43.), "Mercury", 14., BLACK)
                .text_style(label_style),
        );

        scene
    }
    fn drag(&self, _: &(), _: &str, _: Modifiers) -> Option<DragSpec<'static, ()>> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn release<E: Example>(example: E, state: E::State, id: &str, destination: Point) -> E::State {
        let (state, id) = example
            .begin_drag(&state, id, Modifiers::default())
            .unwrap_or((state, id.to_owned()));
        let press = example.locate(&state, &id).unwrap() * Point::new(20., 20.);
        let spec = example.drag(&state, &id, Modifiers::default()).unwrap();
        let id = id.to_string();
        dragology::DragSession::begin(state, press, move |s| example.locate(s, &id), spec)
            .unwrap()
            .finish(destination)
            .unwrap()
    }
    fn tile_ids(s: &RowsState) -> Vec<String> {
        let mut ids = s
            .rows
            .iter()
            .flat_map(|r| r.items.iter().map(|t| t.id.clone()))
            .collect::<Vec<_>>();
        ids.sort();
        ids
    }
    #[test]
    fn source_layouts_keep_the_distinct_origins_and_empty_row_sizes() {
        for (mode, size, origin) in [
            (Mode::Perm, (350., 100.), Point::ORIGIN),
            (Mode::Floating, (350., 100.), Point::ORIGIN),
            (Mode::BluefishPerm, (300., 120.), Point::new(10., 30.)),
        ] {
            let e = Rows(mode);
            let s = e.initial();
            let scene = e.scene(&s);
            assert_eq!((scene.width, scene.height), size);
            assert_eq!(scene.object("A").unwrap().transform * Point::ORIGIN, origin);
        }
        for (mode, expected) in [(Mode::Lists, (26., 66.)), (Mode::BluefishSizes, (34., 33.))] {
            let e = Rows(mode);
            let mut s = e.initial();
            s.rows[0].items.clear();
            let scene = e.scene(&s);
            let Shape::Rect { bounds, .. } = scene.object("row1").unwrap().shapes[0] else {
                panic!()
            };
            assert_eq!((bounds.width(), bounds.height()), expected);
        }
        let e = Rows(Mode::BluefishPerm);
        let s = e.initial();
        let a = e
            .scene_dragging(&s, Some("A"))
            .object("A")
            .unwrap()
            .transform
            * Point::ORIGIN;
        assert_eq!(a, Point::new(10., 10.));
    }
    #[test]
    fn runtime_store_creation_keeps_the_source_stock_identity_and_cancel_restores() {
        let mut app = crate::runtime::ModelDemo::new(Palette(false));
        app.set_animated(false);
        let before = app.scene().clone();
        app.press(Point::new(25., 20.), Modifiers::default(), 1);
        app.move_to(Point::new(20., 90.), Modifiers::default());
        app.advance(std::time::Duration::ZERO);
        assert!(app.scene().object("D").is_some());
        assert!(app.scene().object("created-0").is_some());
        app.cancel();
        assert_eq!(app.scene(), &before);
        app.press(Point::new(25., 20.), Modifiers::default(), 1);
        app.release(Point::new(20., 90.), Modifiers::default());
        assert!(app.scene().object("created-0").is_some());
        assert!(app.scene().object("D").is_some());
    }
    #[test]
    fn reordering_preserves_identity_and_moves_to_requested_slot() {
        let e = Rows(Mode::Perm);
        let s = e.initial();
        let dest = e.locate(&s, "E").unwrap() * Point::new(20., 20.);
        let out = release(e, s.clone(), "A", dest);
        assert_eq!(out.rows[0].items.last().unwrap().id, "A");
        assert_eq!(tile_ids(&s), tile_ids(&out));
    }
    #[test]
    fn variable_sized_tiles_transfer_between_rows_without_duplication() {
        let e = Rows(Mode::Sizes);
        let s = e.initial();
        let (_, candidates) = e.candidates(&s, 0, 0);
        let target = candidates
            .iter()
            .find(|c| c.rows[2].items.first().is_some_and(|t| t.id == "A1"))
            .unwrap();
        let dest = e.locate(target, "A1").unwrap() * Point::new(20., 20.);
        let out = release(e, s.clone(), "A1", dest);
        assert_eq!(out.rows[2].items[0].id, "A1");
        assert_eq!(out.rows[2].items[0].height, 100.);
        assert_eq!(tile_ids(&s), tile_ids(&out));
    }
    #[test]
    fn canvas_detachment_creates_a_new_row_without_losing_tiles() {
        let e = Rows(Mode::Canvas);
        let s = e.initial();
        let out = release(e, s.clone(), "A1", Point::new(550., 350.));
        assert_eq!(out.rows.len(), 4);
        assert_eq!(tile_ids(&s), tile_ids(&out));
        assert_eq!(out.rows.last().unwrap().items[0].id, "A1");
    }
    #[test]
    fn overlapping_canvas_rows_keep_tiles_above_other_row_backgrounds() {
        let mut app = crate::runtime::ModelDemo::new(Rows(Mode::Canvas));
        app.set_animated(false);
        // Move row2 so its grip lies over A1. Upstream's row layer [0]
        // remains beneath A1's tile layer [0, 1] after the row is released.
        app.press(Point::new(28., 133.), Modifiers::default(), 1);
        app.release(Point::new(48., 33.), Modifiers::default());
        let row = app.scene().object("row2").unwrap().transform;
        assert!((row * Point::ORIGIN).distance(Point::new(40., 0.)) < 0.01);
        app.press(Point::new(50., 25.), Modifiers::default(), 1);
        app.release(Point::new(550., 350.), Modifiers::default());
        assert_eq!(app.scene().object("row2").unwrap().transform, row);
        assert!(
            (app.scene().object("A1").unwrap().transform * Point::ORIGIN).x > 400.,
            "the visible tile must receive the drag instead of the overlapping row"
        );
    }
    #[test]
    fn palette_creation_replenishes_stock_and_far_release_cancels() {
        let e = Palette(false);
        let s = e.initial();
        let out = release(e, s.clone(), "D", Point::new(20., 90.));
        assert_eq!(out.items.len(), 4);
        assert_eq!(out.store.len(), 3);
        assert_eq!(out.items[0].id, "created-0");
        assert_eq!(out.store[0].id, "D");
        let ids = out
            .items
            .iter()
            .chain(&out.store)
            .map(|t| &t.id)
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(ids.len(), 7);
        let canceled = release(Palette(false), s.clone(), "D", Point::new(600., 500.));
        assert_eq!(canceled.items, s.items);
        assert_eq!(canceled.store, s.store);
    }
    #[test]
    fn trash_preserves_the_distinct_original_and_modern_source_behavior() {
        for original in [false, true] {
            let e = Palette(original);
            let s = e.initial();
            let out = release(e, s, "B", Point::new(250., 20.));
            assert_eq!(
                out.items.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(),
                if original { vec!["A"] } else { vec!["A", "C"] }
            );
            assert!(out.deleted.is_none());
        }
    }
}
