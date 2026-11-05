use super::*;
use dragology::{BetweenOptions, between_with};

#[derive(Clone, Copy)]
enum SwitchKind {
    Track,
    Chained,
    Study,
    Clickable,
    Sharpness,
    ClickAndDrag,
    ThreeWay,
}

#[derive(Clone, PartialEq)]
struct SwitchState {
    value: usize,
    color: usize,
    floating: bool,
    clockwise: bool,
}
struct Switch(SwitchKind);
const THREE: [(f64, f64); 3] = [(110., 50.), (76., 108.), (144., 108.)];
const SHARPNESS: [f64; 6] = [1., 1.5, 2., 3., 5., 10.];

impl Example for Switch {
    type State = SwitchState;
    fn initial(&self) -> SwitchState {
        SwitchState {
            value: 0,
            color: 0,
            floating: false,
            clockwise: false,
        }
    }
    fn scene(&self, s: &SwitchState) -> Scene {
        let mut scene = match self.0 {
            SwitchKind::Track | SwitchKind::ClickAndDrag => {
                let mut scene = Scene::new(150., 80.);
                scene.push(Object::line("track", (20., 20.), (80., 20.), TRACK, 6.));
                let x = s.value as f64 * 60.;
                scene.push(Object::rect(
                    "knob",
                    Rect::new(x, 0., x + 40., 40.),
                    4.,
                    if matches!(self.0, SwitchKind::ClickAndDrag) {
                        [RED, GREEN, BLUE, GOLD][s.color]
                    } else {
                        INK
                    },
                ));
                scene
            }
            SwitchKind::Chained => {
                let mut scene = Scene::new(400., 100.);
                let pos = |v: usize| (20. + v as f64 * 100., if v % 2 == 0 { 60. } else { 20. });
                for i in 0..3 {
                    scene.push(Object::line(
                        format!("segment-{i}"),
                        pos(i),
                        pos(i + 1),
                        TRACK,
                        6.,
                    ));
                }
                let (x, y) = pos(s.value);
                scene.push(Object::rect(
                    "knob",
                    Rect::new(x - 20., y - 20., x + 20., y + 20.),
                    4.,
                    INK,
                ));
                scene
            }
            SwitchKind::Study | SwitchKind::Clickable | SwitchKind::Sharpness => {
                let multiple = matches!(self.0, SwitchKind::Sharpness);
                let mut scene = Scene::new(
                    if multiple { 520. } else { 200. },
                    if multiple { 210. } else { 200. },
                );
                for i in 0..if multiple { 6 } else { 1 } {
                    let (x, y) = if multiple {
                        (20. + 180. * (i % 3) as f64, 20. + 110. * (i / 3) as f64)
                    } else {
                        (50., 50.)
                    };
                    if multiple {
                        scene.push(Object::text(
                            format!("label-{i}"),
                            (x, y - 6.),
                            format!("sharpness: {}", SHARPNESS[i]),
                            11.,
                            MUTED,
                        ));
                    }
                    scene.push(Object::rect(
                        format!("track-{i}"),
                        Rect::new(x, y, x + 120., y + 60.),
                        30.,
                        if s.value == 1 { GREEN } else { GRAY_300 },
                    ));
                    scene.push(stroke(
                        Object::circle(
                            format!("knob-{i}"),
                            (x + 30. + 60. * s.value as f64, y + 30.),
                            26.,
                            WHITE,
                        ),
                        GRAY_200,
                        1.,
                    ));
                }
                scene
            }
            SwitchKind::ThreeWay => {
                let mut scene = Scene::new(200., 200.);
                for (i, &p) in THREE.iter().enumerate() {
                    let mut color = [RED, GREEN, BLUE][i];
                    color[3] = 64;
                    scene.push(Object::circle(format!("target-{i}"), p, 16., color).decorative());
                }
                scene.push(stroke(
                    Object::circle("knob", THREE[s.value], 16., [RED, GREEN, BLUE][s.value]),
                    WHITE,
                    3.,
                ));
                scene
            }
        };
        for object in &mut scene.objects {
            if (matches!(self.0, SwitchKind::Clickable)
                && (object.id.starts_with("track-") || object.id.starts_with("knob-")))
                || (matches!(self.0, SwitchKind::ClickAndDrag) && object.id == "knob")
            {
                object.click_handler = true;
                if matches!(self.0, SwitchKind::Clickable) {
                    object.cursor = Cursor::Pointer;
                }
            }
            if object.id == "track" || object.id.starts_with("segment-") {
                object.stroke_style.cap = LineCap::Round;
                object.stroke_style.join = LineJoin::Round;
            }
            if object.id.starts_with("label-") {
                object.text_style.baseline = TextBaseline::Alphabetic;
                object.text_style.family = FontFamily::Mono;
                for shape in &mut object.shapes {
                    if let Shape::Text { color, .. } = shape {
                        *color = [153, 153, 153, 255];
                    }
                }
            }
            if object.id.starts_with("knob-") {
                *object = shadow(object.clone(), 1., 2., 38);
            }
            if matches!(self.0, SwitchKind::ThreeWay) && object.id == "knob" {
                *object = shadow(object.clone(), 2., 4., 51);
            }
        }
        scene
    }
    fn drag(
        &self,
        s: &SwitchState,
        id: &str,
        _: Modifiers,
    ) -> Option<DragSpec<'static, SwitchState>> {
        if !id.starts_with("knob") {
            return None;
        }
        let with_value = |v| {
            let mut next = s.clone();
            next.value = v;
            next
        };
        let spec = match self.0 {
            SwitchKind::Chained => {
                let mut branches = Vec::new();
                if s.value > 0 {
                    branches.push(between([s.clone(), with_value(s.value - 1)]));
                }
                if s.value < 3 {
                    branches.push(between([s.clone(), with_value(s.value + 1)]));
                }
                closest(branches).with_snap_radius_options(
                    10.,
                    dragology::SnapOptions {
                        chain: true,
                        ..Default::default()
                    },
                )
            }
            SwitchKind::ThreeWay => {
                let spec = if s.clockwise {
                    let next = [2, 0, 1][s.value];
                    between([s.clone(), with_value(next)]).with_snap_radius_options(
                        15.,
                        dragology::SnapOptions {
                            chain: true,
                            ..Default::default()
                        },
                    )
                } else {
                    between((0..3).map(with_value))
                };
                if s.floating {
                    spec.with_floating()
                } else {
                    spec
                }
            }
            SwitchKind::Sharpness => {
                let i = id.strip_prefix("knob-")?.parse::<usize>().ok()?;
                between_with(
                    [with_value(0), with_value(1)],
                    BetweenOptions {
                        sharpness: SHARPNESS[i],
                        ..Default::default()
                    },
                )
            }
            SwitchKind::Track => between([with_value(1), with_value(0)])
                .with_snap_radius(10.)
                .with_drop_transition(Some(elastic())),
            SwitchKind::ClickAndDrag => {
                between([with_value(1), with_value(0)]).with_drop_transition(Some(elastic()))
            }
            _ => between([with_value(0), with_value(1)]),
        };
        Some(spec)
    }
    fn click(
        &self,
        s: &SwitchState,
        id: &str,
        _: Point,
        _: Modifiers,
        _: usize,
    ) -> Option<SwitchState> {
        let mut next = s.clone();
        match self.0 {
            SwitchKind::Clickable if id.starts_with("track") || id.starts_with("knob") => {
                next.value = 1 - next.value
            }
            SwitchKind::ClickAndDrag if id == "knob" => next.color = (next.color + 1) % 4,
            _ => return None,
        }
        Some(next)
    }
    fn controls(&self, s: &SwitchState) -> Vec<Control> {
        if matches!(self.0, SwitchKind::ThreeWay) {
            vec![
                Control::toggle("floating", "Use withFloating", s.floating),
                Control::toggle(
                    "clockwise",
                    "Clockwise drag only (with chaining)",
                    s.clockwise,
                ),
            ]
        } else {
            Vec::new()
        }
    }
    fn action(&self, s: &SwitchState, id: &str, value: Value) -> Option<SwitchState> {
        let Value::Bool(v) = value else { return None };
        let mut next = s.clone();
        match id {
            "floating" => next.floating = v,
            "clockwise" => next.clockwise = v,
            _ => return None,
        }
        Some(next)
    }
}

struct Clicker;
impl Example for Clicker {
    type State = usize;
    fn initial(&self) -> usize {
        0
    }
    fn scene(&self, s: &usize) -> Scene {
        let mut scene = Scene::new(300., 150.);
        scene.push(
            Object::rect(
                "square",
                Rect::new(40. * *s as f64, 0., 40. * *s as f64 + 100., 100.),
                0.,
                [
                    [255, 0, 0, 255],
                    [0, 128, 0, 255],
                    [0, 0, 255, 255],
                    [255, 255, 0, 255],
                ][*s],
            )
            .clickable()
            .cursor(Cursor::Pointer),
        );
        scene
    }
    fn drag(&self, _: &usize, _: &str, _: Modifiers) -> Option<DragSpec<'static, usize>> {
        None
    }
    fn click(&self, s: &usize, id: &str, _: Point, _: Modifiers, _: usize) -> Option<usize> {
        (id == "square").then_some((s + 1) % 4)
    }
}

#[derive(Clone, Copy)]
enum TriangleKind {
    Four,
    Custom,
    Text,
}
struct Triangle(TriangleKind);
impl Example for Triangle {
    type State = [usize; 4];
    fn initial(&self) -> Self::State {
        [0; 4]
    }
    fn scene(&self, s: &Self::State) -> Scene {
        let mut scene = match self.0 {
            TriangleKind::Four => Scene::new(370., 800.).scrolling(),
            TriangleKind::Custom => Scene::new(200., 150.),
            TriangleKind::Text => Scene::new(300., 270.),
        };
        if matches!(self.0, TriangleKind::Text) {
            let points = [(50., 50.), (250., 50.), (150., 220.)];
            for i in 0..3 {
                scene.push(Object::line(
                    format!("edge-{i}"),
                    points[i],
                    points[(i + 1) % 3],
                    TRACK,
                    2.,
                ));
                scene.push(
                    outlined_circle(format!("target-{i}"), points[i], 22., [0; 4]).decorative(),
                );
                scene.push(Object::text(
                    format!("label-{i}"),
                    (
                        points[i].0 + [-30., 30., 0.][i],
                        points[i].1 + [-24., -24., 36.][i],
                    ),
                    ["A", "B", "C"][i],
                    16.,
                    MUTED,
                ));
            }
            scene.push(Object::circle("knob-0", points[s[0]], 18., BLUE).label(
                ["A", "B", "C"][s[0]],
                (0., 0.),
                18.,
                WHITE,
            ));
            for object in &mut scene.objects {
                if object.id.starts_with("target-") {
                    object.stroke_style.dash = vec![4., 3.];
                }
                if object.id.starts_with("label-") || object.id == "knob-0" {
                    object.text_style.anchor = TextAnchor::Middle;
                    object.text_style.baseline = TextBaseline::Central;
                }
                if object.id.starts_with("label-") {
                    for shape in &mut object.shapes {
                        if let Shape::Text { color, .. } = shape {
                            *color = [100, 116, 139, 255];
                        }
                    }
                }
                if object.id == "knob-0" {
                    object.text_style.weight = 700;
                }
            }
            return scene;
        }
        let labels = [
            "d.closest(states)",
            "d.closest(states).withFloating()",
            "d.between(states)",
            "d.between(states).withFloating()",
        ];
        for panel in 0..if matches!(self.0, TriangleKind::Four) {
            4
        } else {
            1
        } {
            let offset = if matches!(self.0, TriangleKind::Four) {
                Vec2::new(0., 40. + 200. * panel as f64)
            } else {
                Vec2::ZERO
            };
            let pos = [
                Point::new(10., 10.),
                Point::new(100., 10.),
                Point::new(55., 90.),
            ];
            if matches!(self.0, TriangleKind::Four) {
                scene.push(Object::text(
                    format!("heading-{panel}"),
                    (offset.x, offset.y - 25.),
                    labels[panel],
                    16.,
                    INK,
                ));
            }
            for (i, p) in pos.iter().enumerate() {
                let p = *p + offset;
                scene.push(
                    outlined_rect(
                        format!("target-{panel}-{i}"),
                        Rect::new(p.x, p.y, p.x + 40., p.y + 40.),
                        4.,
                        [0; 4],
                    )
                    .decorative(),
                );
            }
            let p = pos[s[panel]] + offset;
            scene.push(Object::rect(
                format!("knob-{panel}"),
                Rect::new(p.x, p.y, p.x + 40., p.y + 40.),
                4.,
                INK,
            ));
            scene.push(Object::line(
                format!("indicator-{panel}"),
                Point::new(75., 110. / 3. + 20.) + offset,
                p + Vec2::new(20., 20.),
                TRACK,
                6.,
            ));
        }
        for object in &mut scene.objects {
            if object.id.starts_with("indicator-") {
                object.stroke_style.cap = LineCap::Round;
            }
            if object.id.starts_with("heading-") {
                object.text_style.family = FontFamily::Mono;
                object.text_style.weight = 500;
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
        let panel = id.strip_prefix("knob-")?.parse::<usize>().ok()?;
        let targets: Vec<_> = (0..3)
            .map(|i| {
                let mut next = *s;
                next[panel] = i;
                next
            })
            .collect();
        if matches!(self.0, TriangleKind::Custom) {
            return Some(DragSpec::custom(move |ctx, pointer| {
                let mut best = None;
                for target in &targets {
                    let point = ctx
                        .grab_in(target)?
                        .ok_or(dragology::DragError::MissingElement)?;
                    let gap = point.distance(pointer);
                    if best.as_ref().is_none_or(|(_, distance)| gap < *distance) {
                        best = Some((*target, gap));
                    }
                }
                let selected = best.unwrap().0;
                let mut frame = ctx.state_frame(Rc::new(selected), pointer)?;
                frame.active_path = selected[panel].to_string();
                Ok(frame)
            }));
        }
        let spec = if panel < 2 && matches!(self.0, TriangleKind::Four) {
            closest(targets)
        } else {
            between(targets)
        };
        Some(if panel % 2 == 1 {
            spec.with_floating()
        } else {
            spec
        })
    }
}

pub(super) fn entries() -> Vec<Entry> {
    vec![
        Entry::new("linear-track", "linear-track", "", || {
            demo(Switch(SwitchKind::Track))
        }),
        Entry::new("linear-track-chained", "linear-track-chained", "", || {
            demo(Switch(SwitchKind::Chained))
        }),
        Entry::new(
            "simple-triangle",
            "simple-triangle",
            "An interface with three states, using different drag specs built on these states.",
            || demo(Triangle(TriangleKind::Four)),
        ),
        Entry::new(
            "simple-triangle-custom",
            "simple-triangle-custom",
            "Reproduces the d.closest(states) simple-triangle demo using only d.custom.",
            || demo(Triangle(TriangleKind::Custom)),
        ),
        Entry::new("simplest-clicker", "simplest-clicker", "", || demo(Clicker)),
        Entry::new("click-and-drag", "click-and-drag", "", || {
            demo(Switch(SwitchKind::ClickAndDrag))
        }),
        Entry::new("switch-with-sharpness", "switch-with-sharpness", "", || {
            demo(Switch(SwitchKind::Sharpness))
        }),
        Entry::new("study-switch", "study-switch", "", || {
            demo(Switch(SwitchKind::Study))
        }),
        Entry::new(
            "study-switch-clickable",
            "study-switch-clickable",
            "Same as study-slider, but it's clickable.",
            || demo(Switch(SwitchKind::Clickable)),
        ),
        Entry::new(
            "study-three-way-switch",
            "study-three-way-switch",
            "",
            || demo(Switch(SwitchKind::ThreeWay)),
        ),
        Entry::new(
            "lerping-text",
            "lerping-text",
            "Drag the mark around the triangle. Its position blends between the corners, but its label is text, which can't be blended, so it shows the nearest corner's letter.",
            || demo(Triangle(TriangleKind::Text)),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use dragology::DragSession;

    #[test]
    fn custom_triangle_reports_the_selected_branch() {
        let example = Triangle(TriangleKind::Custom);
        let state = example.initial();
        let spec = example
            .drag(&state, "knob-0", Modifiers::default())
            .unwrap();
        let mut session = DragSession::begin(
            state,
            (20., 20.),
            move |s| example.locate(s, "knob-0"),
            spec,
        )
        .unwrap();
        let frame = session.frame((65., 100.)).unwrap();
        assert_eq!(frame.drop_state[0], 2);
        assert!(frame.active_path.ends_with("/2"));
        let frame = session.frame((110., 20.)).unwrap();
        assert_eq!(frame.drop_state[0], 1);
        assert!(frame.active_path.ends_with("/1"));
    }

    #[test]
    fn chained_track_rebuilds_only_when_snapped_to_an_endpoint() {
        let example = Switch(SwitchKind::Chained);
        let state = example.initial();
        let spec = example.drag(&state, "knob", Modifiers::default()).unwrap();
        let mut session =
            DragSession::begin(state, (20., 60.), move |s| example.locate(s, "knob"), spec)
                .unwrap();
        assert!(session.frame((70., 40.)).unwrap().chain.is_none());
        let frame = session.frame((120., 20.)).unwrap();
        assert_eq!(frame.drop_state.value, 1);
        assert!(frame.chain.is_some());
    }

    #[test]
    fn linear_track_and_click_drag_have_the_source_elastic_drop_policy() {
        for kind in [SwitchKind::Track, SwitchKind::ClickAndDrag] {
            let example = Switch(kind);
            let state = example.initial();
            let spec = example.drag(&state, "knob", Modifiers::default()).unwrap();
            let mut session =
                DragSession::begin(state, (20., 20.), move |s| example.locate(s, "knob"), spec)
                    .unwrap();
            assert_eq!(
                session.frame((70., 20.)).unwrap().drop_transition,
                Some(Some(elastic()))
            );
        }
    }
}
