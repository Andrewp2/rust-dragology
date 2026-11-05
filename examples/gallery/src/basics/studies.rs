use super::*;
use dragology::{substate, vary, vary_constrained};

struct Slider;
impl Example for Slider {
    type State = f64;
    fn initial(&self) -> f64 {
        100.
    }
    fn scene(&self, value: &f64) -> Scene {
        let mut scene = Scene::new(320., 130.);
        scene
            .push(Object::rect("track", Rect::new(30., 57., 270., 63.), 3., GRAY_200).decorative());
        scene.push(
            Object::rect("fill", Rect::new(30., 57., 30. + value, 63.), 3., BLUE).decorative(),
        );
        scene.push(stroke(
            Object::circle("thumb", (30. + value, 60.), 12., WHITE),
            GRAY_300,
            1.5,
        ));
        scene.push(Object::text(
            "value",
            (value + 30., 88.),
            rounded(*value),
            13.,
            INK,
        ));
        for object in &mut scene.objects {
            if object.id == "thumb" {
                *object = shadow(object.clone(), 1., 3., 38);
            }
            if object.id == "value" {
                object.text_style.anchor = TextAnchor::Middle;
                object.text_style.baseline = TextBaseline::Alphabetic;
                object.text_style.weight = 500;
                for shape in &mut object.shapes {
                    if let Shape::Text { color, .. } = shape {
                        *color = GRAY_700;
                    }
                }
            }
        }
        scene
    }
    fn drag(&self, value: &f64, id: &str, _: Modifiers) -> Option<DragSpec<'static, f64>> {
        (id == "thumb").then(|| vary_constrained([*value], |[x]| x, |x| [-x, x - 240.]))
    }
}

struct Dial;
impl Example for Dial {
    type State = f64;
    fn initial(&self) -> f64 {
        0.
    }
    fn scene(&self, angle: &f64) -> Scene {
        let mut scene = Scene::new(400., 250.);
        let center = Point::new(200., 120.);
        for i in 0..12 {
            let a = i as f64 * std::f64::consts::TAU / 12.;
            let direction = Vec2::new(a.cos(), a.sin());
            scene.push(Object::line(
                format!("tick-{i}"),
                center + direction * 66.,
                center + direction * 76.,
                MUTED,
                2.,
            ));
        }
        let mut dial = stroke(
            Object::circle("dial", center, 60., [249, 250, 251, 255]),
            GRAY_300,
            2.,
        );
        dial.transform = Affine::translate(center.to_vec2())
            * Affine::rotate(angle.to_radians() - std::f64::consts::FRAC_PI_2);
        let mut hand = BezPath::new();
        hand.move_to((0., 0.));
        hand.line_to((46., 0.));
        dial.shapes.push(Shape::Path {
            path: hand,
            fill: None,
            stroke: Some((GRAY_700, 3.)),
        });
        scene.push(dial);
        scene.push(Object::text(
            "angle",
            (200., 220.),
            format!("{}°", rounded(*angle)),
            13.,
            INK,
        ));
        for object in &mut scene.objects {
            object.stroke_style.cap = LineCap::Round;
            if object.id == "dial" {
                *object = shadow(object.clone(), 0., 4., 38);
            }
            if object.id == "angle" {
                object.text_style.anchor = TextAnchor::Middle;
                object.text_style.baseline = TextBaseline::Alphabetic;
                object.text_style.weight = 500;
                for shape in &mut object.shapes {
                    if let Shape::Text { color, .. } = shape {
                        *color = GRAY_700;
                    }
                }
            }
        }
        scene
    }
    fn drag(&self, angle: &f64, id: &str, _: Modifiers) -> Option<DragSpec<'static, f64>> {
        (id == "dial").then(|| vary([*angle], |[a]| a))
    }
}

struct Linked;
impl Example for Linked {
    type State = [f64; 5];
    fn initial(&self) -> Self::State {
        [40., 80., 120., 160., 200.]
    }
    fn scene(&self, values: &Self::State) -> Scene {
        let mut scene = Scene::new(340., 204.);
        for (i, value) in values.iter().enumerate() {
            let y = 30. + i as f64 * 36.;
            scene.push(
                Object::rect(
                    format!("track-{i}"),
                    Rect::new(40., y - 3., 280., y + 3.),
                    3.,
                    GRAY_200,
                )
                .decorative(),
            );
            scene.push(
                Object::rect(
                    format!("fill-{i}"),
                    Rect::new(40., y - 3., 40. + value, y + 3.),
                    3.,
                    BLUE,
                )
                .decorative(),
            );
            scene.push(stroke(
                Object::circle(format!("thumb-{i}"), (40. + value, y), 10., WHITE),
                GRAY_300,
                1.5,
            ));
            scene.push(Object::text(
                format!("number-{i}"),
                (30., y),
                format!("{}", i + 1),
                11.,
                MUTED,
            ));
            scene.push(Object::text(
                format!("value-{i}"),
                (292., y),
                rounded(*value),
                11.,
                INK,
            ));
        }
        for object in &mut scene.objects {
            if object.id.starts_with("thumb-") {
                *object = shadow(object.clone(), 1., 2., 38);
            }
            if object.id.starts_with("number-") {
                object.text_style.anchor = TextAnchor::End;
                object.text_style.baseline = TextBaseline::Central;
            }
            if object.id.starts_with("value-") {
                object.text_style.baseline = TextBaseline::Central;
                for shape in &mut object.shapes {
                    if let Shape::Text { color, .. } = shape {
                        *color = GRAY_700;
                    }
                }
            }
        }
        scene
    }
    fn drag(
        &self,
        values: &Self::State,
        id: &str,
        _: Modifiers,
    ) -> Option<DragSpec<'static, Self::State>> {
        let index = id.strip_prefix("thumb-")?.parse::<usize>().ok()?;
        let initial = *values;
        Some(vary_constrained(
            [0.],
            move |[delta]| {
                let mut next = initial;
                for v in &mut next[index..] {
                    *v += delta;
                }
                next
            },
            |values| {
                std::array::from_fn::<_, 10, _>(|i| {
                    if i % 2 == 0 {
                        -values[i / 2]
                    } else {
                        values[i / 2] - 240.
                    }
                })
            },
        ))
    }
}

#[derive(Clone, PartialEq)]
struct Block {
    name: &'static str,
    pos: f64,
    track: usize,
    color: Color,
}
#[derive(Clone, Copy)]
enum TimelineKind {
    Direct,
    Substate,
    Embedded,
}
struct Timeline(TimelineKind);
fn block_spec(block: Block) -> DragSpec<'static, Block> {
    closest((0..4).map(move |track| {
        let base = block.clone();
        vary_constrained(
            [base.pos],
            move |[pos]| Block {
                pos,
                track,
                ..base.clone()
            },
            |b| [-b.pos, b.pos - 260.],
        )
    }))
}
fn block_object(block: &Block) -> Object {
    let y = 10. + 42. * block.track as f64;
    Object::rect(
        block.name,
        Rect::new(10. + block.pos, y, 90. + block.pos, y + 36.),
        6.,
        block.color,
    )
    .label(block.name, (40., 18.), 13., WHITE)
    .text_anchor(TextAnchor::Middle)
    .text_baseline(TextBaseline::Central)
    .font_weight(600)
    .shadow(Shadow {
        offset: Vec2::new(0., 1.),
        blur: 2.,
        color: [0, 0, 0, 38],
    })
}
impl Example for Timeline {
    type State = Vec<Block>;
    fn initial(&self) -> Self::State {
        vec![
            Block {
                name: "Cats",
                pos: 10.,
                track: 0,
                color: BLUE,
            },
            Block {
                name: "Dogs",
                pos: 100.,
                track: 1,
                color: RED,
            },
            Block {
                name: "Clouds",
                pos: 50.,
                track: 3,
                color: GREEN,
            },
        ]
    }
    fn scene(&self, blocks: &Self::State) -> Scene {
        let mut scene = Scene::new(370., 200.);
        for t in 0..4 {
            let y = 10. + t as f64 * 42.;
            scene.push(
                stroke(
                    Object::rect(
                        format!("track-{t}"),
                        Rect::new(10., y, 350., y + 36.),
                        4.,
                        [243, 244, 246, 255],
                    ),
                    GRAY_200,
                    1.,
                )
                .decorative(),
            );
        }
        for block in blocks {
            scene.push(block_object(block));
        }
        scene
    }
    fn scene_dragging(&self, state: &Self::State, id: Option<&str>) -> Scene {
        let mut scene = self.scene(state);
        if let Some(id) = id {
            if let Some(i) = scene.objects.iter().position(|o| o.id == id) {
                let object = scene.objects.remove(i);
                scene.push(object);
            }
        }
        scene
    }

    fn drag(
        &self,
        blocks: &Self::State,
        id: &str,
        _: Modifiers,
    ) -> Option<DragSpec<'static, Self::State>> {
        let index = blocks.iter().position(|b| b.name == id)?;
        Some(
            match self.0 {
                TimelineKind::Direct => closest((0..4).map(|track| {
                    let base = blocks.clone();
                    vary_constrained(
                        [base[index].pos],
                        move |[pos]| {
                            let mut next = base.clone();
                            next[index].pos = pos;
                            next[index].track = track;
                            next
                        },
                        move |next| [-next[index].pos, next[index].pos - 260.],
                    )
                })),
                TimelineKind::Substate | TimelineKind::Embedded => substate(
                    blocks.clone(),
                    move |items| &mut items[index],
                    block_spec(blocks[index].clone()),
                ),
            }
            .with_branch_transition(Some(dragology::Transition {
                duration: std::time::Duration::from_millis(100),
                easing: dragology::Easing::CubicOut,
            })),
        )
    }
}

#[derive(Clone, Copy)]
enum ListKind {
    Study,
    Error,
    Components,
}
#[derive(Clone, PartialEq)]
struct ListState {
    order: Vec<usize>,
    floating: bool,
    snap: bool,
    counts: [usize; 4],
    notes: [String; 4],
    accent: usize,
    mounted_at: String,
}
struct Reorder(ListKind);
const FRUITS: [&str; 4] = ["Apples", "Bananas", "Cherries", "Dates"];
const NOTES: [&str; 4] = ["alpha", "beta", "gamma", "delta"];
const NOTE_INPUTS: [&str; 4] = ["note-0", "note-1", "note-2", "note-3"];
const INCREMENTS: [&str; 4] = ["increment-0", "increment-1", "increment-2", "increment-3"];
const ACCENTS: [Color; 4] = [BLUE, [16, 185, 129, 255], [245, 158, 11, 255], RED];
fn components_scene(s: &ListState) -> Scene {
    let mut scene = Scene::new(480., 340.);
    scene.push(
        Object::text(
            "accent-label",
            (0., 10.),
            "Accent (React context from outside the draggable):",
            14.,
            INK,
        )
        .text_baseline(TextBaseline::Central),
    );
    for (i, color) in ACCENTS.into_iter().enumerate() {
        let dot = Object::circle(
            format!("accent-{i}"),
            (363. + 24. * i as f64, 10.),
            10.,
            color,
        )
        .clickable();
        scene.push(if s.accent == i {
            stroke(dot, INK, 2.)
        } else {
            dot
        });
    }
    for (row, &item) in s.order.iter().enumerate() {
        let x = 10.;
        let y = 44. + row as f64 * 74.;
        let id = format!("item-{item}");
        // The source's items-center row shares a center with its 20 px button
        // and 22 px input. SVG text-before-edge is not a CSS line-box top.
        let row_center_y = y + 22.;
        scene.push(stroke(
            Object::rect(id.clone(), Rect::new(x, y, x + 340., y + 64.), 8., WHITE)
                .cursor(Cursor::Grab),
            GRAY_300,
            1.,
        ));
        let mut grip = Object::text(format!("grip-{item}"), (x + 14., y + 32.), "⠿", 18., MUTED)
            .text_anchor(TextAnchor::Middle)
            .text_baseline(TextBaseline::Middle);
        grip.parent = Some(id.clone());
        scene.push(grip);
        let mut name = Object::text(
            format!("name-{item}"),
            (x + 28., row_center_y),
            NOTES[item],
            14.,
            ACCENTS[s.accent],
        )
        .font_weight(600)
        .text_baseline(TextBaseline::Central);
        name.parent = Some(id.clone());
        scene.push(name);
        let mut count = Object::text(
            format!("count-{item}"),
            (x + 124., row_center_y),
            s.counts[item].to_string(),
            14.,
            INK,
        )
        .text_baseline(TextBaseline::Central);
        count.parent = Some(id.clone());
        scene.push(count);
        let mut mounted = Object::text(
            format!("mounted-{item}"),
            (x + 28., y + 45.),
            format!("mounted at {}", s.mounted_at),
            12.,
            MUTED,
        )
        .text_baseline(TextBaseline::Central);
        mounted.parent = Some(id.clone());
        scene.push(mounted);
        scene
            .inline_control_for(INCREMENTS[item], id.clone(), Rect::new(84., 12., 116., 32.))
            .background(ACCENTS[s.accent])
            .foreground(WHITE)
            .font_size(14.);
        scene
            .inline_control_for(NOTE_INPUTS[item], id, Rect::new(156., 11., 332., 33.))
            .background(WHITE)
            .foreground(INK)
            .border(GRAY_300)
            .font_size(14.)
            .placeholder("type a note");
    }
    scene
}

impl Example for Reorder {
    type State = ListState;
    fn initial(&self) -> Self::State {
        ListState {
            order: vec![0, 1, 2, 3],
            floating: matches!(self.0, ListKind::Components),
            snap: false,
            counts: [0; 4],
            notes: Default::default(),
            accent: 0,
            mounted_at: chrono::Local::now().format("%-I:%M:%S %p").to_string(),
        }
    }
    fn scene(&self, s: &Self::State) -> Scene {
        if matches!(self.0, ListKind::Components) {
            return components_scene(s);
        }
        let mut scene = Scene::new(220., 230.);
        for (row, &item) in s.order.iter().enumerate() {
            let id = if matches!(self.0, ListKind::Error) {
                format!("slot-{row}")
            } else {
                format!("item-{item}")
            };
            scene.push(
                stroke(
                    Object::rect(
                        id,
                        Rect::new(20., 20. + row as f64 * 48., 180., 60. + row as f64 * 48.),
                        6.,
                        WHITE,
                    ),
                    GRAY_300,
                    1.5,
                )
                .label(FRUITS[item], (80., 20.), 16., GRAY_700)
                .text_anchor(TextAnchor::Middle)
                .text_baseline(TextBaseline::Central),
            );
        }
        scene
    }
    fn scene_dragging(&self, s: &Self::State, id: Option<&str>) -> Scene {
        let mut scene = self.scene(s);
        if !matches!(self.0, ListKind::Error) {
            if let Some(id) = id {
                let (front, back): (Vec<_>, Vec<_>) = scene
                    .objects
                    .into_iter()
                    .partition(|o| o.id == id || o.parent.as_deref() == Some(id));
                scene.objects = back;
                scene.objects.extend(front);
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
        let row = if matches!(self.0, ListKind::Error) {
            id.strip_prefix("slot-")?.parse::<usize>().ok()?
        } else {
            let item = id.strip_prefix("item-")?.parse::<usize>().ok()?;
            s.order.iter().position(|&v| v == item)?
        };
        let targets = (0..s.order.len()).map(|to| {
            let mut next = s.clone();
            next.order = move_item(&s.order, row, to);
            next
        });
        let spec = if s.floating {
            if matches!(self.0, ListKind::Components) {
                closest(targets).when_far(s.clone(), 50.).with_floating()
            } else {
                closest(targets).with_floating()
            }
        } else {
            between(targets)
        };
        Some(if s.snap {
            spec.with_snap_radius(10.)
        } else {
            spec
        })
    }
    fn click(
        &self,
        s: &Self::State,
        id: &str,
        _: Point,
        _: Modifiers,
        _: usize,
    ) -> Option<Self::State> {
        if !matches!(self.0, ListKind::Components) {
            return None;
        }
        let accent = id.strip_prefix("accent-")?.parse::<usize>().ok()?;
        (accent < 4).then(|| ListState {
            accent,
            ..s.clone()
        })
    }
    fn controls(&self, s: &Self::State) -> Vec<Control> {
        match self.0 {
            ListKind::Components => (0..4)
                .flat_map(|i| {
                    [
                        Control::button(INCREMENTS[i], "+1"),
                        Control::Text {
                            id: NOTE_INPUTS[i],
                            label: "type a note".into(),
                            value: s.notes[i].clone(),
                        },
                    ]
                })
                .collect(),
            ListKind::Study => vec![
                Control::toggle("floating", "Use d.closest with floating", s.floating),
                Control::toggle("snap", "Use withSnapRadius", s.snap),
            ],
            ListKind::Error => Vec::new(),
        }
    }
    fn action(&self, s: &Self::State, id: &str, value: Value) -> Option<Self::State> {
        let mut next = s.clone();
        if let Some(i) = NOTE_INPUTS.iter().position(|input| *input == id) {
            let Value::Text(text) = value else {
                return None;
            };
            next.notes[i] = text;
        } else if let Some(i) = INCREMENTS.iter().position(|button| *button == id) {
            next.counts[i] += 1;
        } else {
            match (id, value) {
                ("floating", Value::Bool(v)) => next.floating = v,
                ("snap", Value::Bool(v)) => next.snap = v,
                _ => return None,
            }
        }
        Some(next)
    }
}

struct LerpKeys;
impl Example for LerpKeys {
    type State = usize;
    fn initial(&self) -> usize {
        3
    }
    fn scene(&self, value: &usize) -> Scene {
        let mut scene = Scene::new(320., 200.);
        scene
            .push(Object::rect("track", Rect::new(30., 37., 240., 43.), 3., GRAY_200).decorative());
        for i in 0..8 {
            scene.push(
                Object::circle(format!("notch-{i}"), (30. + 30. * i as f64, 40.), 2., MUTED)
                    .decorative(),
            );
        }
        scene.push(outlined_circle(
            "thumb",
            (30. + 30. * *value as f64, 40.),
            12.,
            WHITE,
        ));
        scene.push(Object::text(
            "unkeyed-label",
            (23., 84.),
            "no dragologyKey",
            11.,
            MUTED,
        ));
        scene.push(Object::text(
            "keyed-label",
            (23., 134.),
            "with dragologyKey",
            11.,
            MUTED,
        ));
        for (slot, i) in (*value..8).enumerate() {
            let x = 30. + i as f64 * 30.;
            scene.push(
                Object::circle(
                    format!("slot-{slot}"),
                    (x, 107.),
                    7.,
                    [
                        [239, 68, 68, 255],
                        [249, 115, 22, 255],
                        [234, 179, 8, 255],
                        [34, 197, 94, 255],
                        [20, 184, 166, 255],
                        [59, 130, 246, 255],
                        [139, 92, 246, 255],
                        [236, 72, 153, 255],
                    ][i],
                )
                .decorative(),
            );
            scene.push(
                Object::circle(
                    format!("key-{i}"),
                    (x, 157.),
                    7.,
                    [
                        [239, 68, 68, 255],
                        [249, 115, 22, 255],
                        [234, 179, 8, 255],
                        [34, 197, 94, 255],
                        [20, 184, 166, 255],
                        [59, 130, 246, 255],
                        [139, 92, 246, 255],
                        [236, 72, 153, 255],
                    ][i],
                )
                .decorative(),
            );
        }
        for object in &mut scene.objects {
            if object.id == "thumb" {
                *object = stroke(object.clone(), GRAY_300, 1.5);
            }
            if object.id.ends_with("-label") {
                object.text_style.baseline = TextBaseline::Alphabetic;
                for shape in &mut object.shapes {
                    if let Shape::Text { color, .. } = shape {
                        *color = GRAY_500;
                    }
                }
            }
        }
        scene
    }
    fn drag(&self, _: &usize, id: &str, _: Modifiers) -> Option<DragSpec<'static, usize>> {
        (id == "thumb").then(|| between(0..8))
    }
}

pub(super) fn entries() -> Vec<Entry> {
    vec![
        Entry::new("study-slider", "study-slider", "", || demo(Slider)),
        Entry::new("study-rotary-dial", "study-rotary-dial", "", || demo(Dial)),
        Entry::new(
            "linked-sliders",
            "linked-sliders",
            "Contrived demo of d.varyFunc: each slider controls itself & all lower sliders. They're constrained to the tracks.",
            || demo(Linked),
        ),
        Entry::new("study-timeline", "study-timeline", "", || {
            demo(Timeline(TimelineKind::Direct))
        }),
        Entry::new(
            "study-timeline-substate",
            "study-timeline-substate",
            "An approach to the \"Timeline\" study task using the experimental d.substate operator, which lets you write a drag-spec for a particular piece of the state so you don't need to keep drilling down repetitively.",
            || demo(Timeline(TimelineKind::Substate)),
        ),
        Entry::new(
            "study-timeline-embed",
            "study-timeline-embed",
            "An approach to the \"Timeline\" study task using embedded block renderers and drag specifications within the parent timeline. The upstream version demonstrates the experimental embed() operator.",
            || demo(Timeline(TimelineKind::Embedded)),
        ),
        Entry::new(
            "study-reorderable-list",
            "study-reorderable-list",
            "",
            || demo(Reorder(ListKind::Study)),
        ),
        Entry::new(
            "error-study-reorderable-list",
            "error-study-reorderable-list",
            "This tests the \"coincident targets detected\" error (because id is missing).",
            || demo(Reorder(ListKind::Error)),
        ),
        Entry::new(
            "component-elements",
            "component-elements",
            "Each card holds a component with its own counter, editable note, and mount time. An accent color comes from outside the draggable. Component state survives dragging and reordering. Drag a card by its widget; the button and input stop pointer propagation so they work as controls instead.",
            || demo(Reorder(ListKind::Components)),
        ),
        Entry::new(
            "lerping-keys",
            "lerping-keys",
            "Drag the knob; it transitions between notches via d.between. Both rows below show dots to the right of the knob. Without dragologyKey, children are keyed by position — so as one dot disappears, the rest shift and lerp through each other's colors. With dragologyKey, each dot is matched by its key and fades in/out in place.",
            || demo(LerpKeys),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use dragology::DragSession;
    #[test]
    fn linked_sliders_respect_all_bounds_and_move_only_the_suffix() {
        let example = Linked;
        let state = example.initial();
        let spec = example
            .drag(&state, "thumb-2", Modifiers::default())
            .unwrap();
        let session = DragSession::begin(
            state,
            (160., 102.),
            move |s| example.locate(s, "thumb-2"),
            spec,
        )
        .unwrap();
        let next = session.finish((500., 102.)).unwrap();
        assert_eq!(next[..2], [40., 80.]);
        assert!(next.iter().all(|v| *v >= -1e-8 && *v <= 240. + 1e-8));
        assert!((next[4] - 240.).abs() < 0.01);
        assert!((next[3] - next[2] - 40.).abs() < 0.01);
    }
    #[test]
    fn card_local_state_survives_reordering() {
        let example = Reorder(ListKind::Components);
        let state = example.initial();
        let state = example
            .action(&state, "increment-0", Value::Trigger)
            .unwrap();
        let state = example
            .action(&state, "note-0", Value::Text("Keep me".into()))
            .unwrap();
        let spec = example
            .drag(&state, "item-0", Modifiers::default())
            .unwrap();
        let session = DragSession::begin(
            state,
            (35., 65.),
            move |s| example.locate(s, "item-0"),
            spec,
        )
        .unwrap();
        let next = session.finish((35., 287.)).unwrap();
        assert_eq!(next.order, [1, 2, 3, 0]);
        assert_eq!(next.counts[0], 1);
        assert_eq!(next.notes[0], "Keep me");
    }

    #[test]
    fn floating_list_and_component_list_keep_their_different_far_drop_policies() {
        for kind in [ListKind::Study, ListKind::Components] {
            let example = Reorder(kind);
            let mut state = example.initial();
            state.floating = true;
            let (press, target) = if matches!(kind, ListKind::Study) {
                ((35., 35.), (300., 179.))
            } else {
                ((25., 59.), (300., 281.))
            };
            let spec = example
                .drag(&state, "item-0", Modifiers::default())
                .unwrap();
            let session =
                DragSession::begin(state, press, move |s| example.locate(s, "item-0"), spec)
                    .unwrap();
            let next = session.finish(target).unwrap();
            assert_eq!(
                next.order,
                if matches!(kind, ListKind::Study) {
                    vec![1, 2, 3, 0]
                } else {
                    vec![0, 1, 2, 3]
                }
            );
        }
    }

    #[test]
    fn timeline_variants_share_constraints_and_source_branch_timing() {
        for kind in [
            TimelineKind::Direct,
            TimelineKind::Substate,
            TimelineKind::Embedded,
        ] {
            let example = Timeline(kind);
            let state = example.initial();
            let spec = example.drag(&state, "Cats", Modifiers::default()).unwrap();
            let mut session =
                DragSession::begin(state, (30., 20.), move |s| example.locate(s, "Cats"), spec)
                    .unwrap();
            let frame = session.frame((400., 146.)).unwrap();
            assert_eq!(frame.drop_state[0].track, 3);
            assert!((frame.drop_state[0].pos - 260.).abs() < 0.01);
            assert_eq!(frame.drop_state[1].pos, 100.);
            assert_eq!(frame.drop_state[2].pos, 50.);
            assert_eq!(
                frame.branch_transition,
                Some(Some(dragology::Transition {
                    duration: std::time::Duration::from_millis(100),
                    easing: dragology::Easing::CubicOut,
                }))
            );
        }
    }

    #[test]
    fn source_missing_identity_example_still_reports_coincident_targets() {
        let example = Reorder(ListKind::Error);
        let state = example.initial();
        let spec = example
            .drag(&state, "slot-0", Modifiers::default())
            .unwrap();
        let mut session = DragSession::begin(
            state,
            (35., 35.),
            move |s| example.locate(s, "slot-0"),
            spec,
        )
        .unwrap();
        assert!(matches!(
            session.frame((35., 100.)),
            Err(dragology::DragError::CoincidentTargets { .. })
        ));
    }

    #[test]
    fn component_controls_are_anchored_to_card_identity_and_follow_context_accent() {
        let example = Reorder(ListKind::Components);
        let state = example.initial();
        let state = example
            .click(&state, "accent-2", Point::ZERO, Modifiers::default(), 1)
            .unwrap();
        let scene = example.scene(&state);
        assert_eq!(scene.inline_controls.len(), 8);
        for i in 0..4 {
            let button = scene
                .inline_controls
                .iter()
                .find(|c| c.id == INCREMENTS[i])
                .unwrap();
            let input = scene
                .inline_controls
                .iter()
                .find(|c| c.id == NOTE_INPUTS[i])
                .unwrap();
            assert_eq!(button.parent.as_deref(), Some(format!("item-{i}").as_str()));
            assert_eq!(input.parent, button.parent);
            assert_eq!(button.style.background, Some(ACCENTS[2]));
        }
    }
}
