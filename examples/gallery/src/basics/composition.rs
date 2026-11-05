use super::*;
use dragology::{ClosestOptions, closest_with, fixed, vary, vary_constrained};
use std::time::Duration;

#[derive(Clone, PartialEq)]
enum IslandState {
    Slider(f64),
    Island(usize),
}
struct BetweenVary;
impl Example for BetweenVary {
    type State = IslandState;
    fn initial(&self) -> Self::State {
        IslandState::Slider(50.)
    }
    fn scene(&self, s: &Self::State) -> Scene {
        let mut scene = Scene::new(300., 240.);
        let on_slider = matches!(s, IslandState::Slider(_));
        scene.push(Object::line(
            "track",
            (50., 100.),
            (250., 100.),
            if on_slider {
                [204, 204, 204, 255]
            } else {
                [238, 238, 238, 255]
            },
            4.,
        ));
        for i in 0..2 {
            let y = if i == 0 { 20. } else { 180. };
            let active = *s == IslandState::Island(i);
            scene.push(
                stroke(
                    Object::rect(
                        format!("island-{i}"),
                        Rect::new(110., y - 20., 190., y + 20.),
                        8.,
                        if active {
                            [208, 232, 255, 255]
                        } else {
                            [232, 232, 232, 255]
                        },
                    ),
                    if active {
                        [136, 136, 187, 255]
                    } else {
                        [187, 187, 187, 255]
                    },
                    1.,
                )
                .label(
                    if i == 0 { "top" } else { "bottom" },
                    (40., 25.),
                    13.,
                    [153, 153, 153, 255],
                )
                .decorative(),
            );
        }
        let pos = match s {
            IslandState::Slider(v) => Point::new(50. + 2. * v, 100.),
            IslandState::Island(i) => Point::new(150., if *i == 0 { 20. } else { 180. }),
        };
        if let IslandState::Slider(v) = s {
            scene.push(Object::text(
                "value",
                (pos.x, pos.y - 22.),
                rounded(*v),
                12.,
                INK,
            ));
        }
        scene.push(Object::circle("knob", pos, 14., [51, 51, 51, 255]));
        for object in &mut scene.objects {
            if object.id == "track" {
                object.stroke_style.cap = LineCap::Round;
            }
            if object.id.starts_with("island-") || object.id == "value" {
                object.text_style.anchor = TextAnchor::Middle;
                object.text_style.baseline = TextBaseline::Alphabetic;
            }
            if object.id == "value" {
                for shape in &mut object.shapes {
                    if let Shape::Text { color, .. } = shape {
                        *color = [102, 102, 102, 255];
                    }
                }
            }
        }
        scene
    }
    fn drag(
        &self,
        _: &Self::State,
        id: &str,
        _: Modifiers,
    ) -> Option<DragSpec<'static, Self::State>> {
        (id == "knob").then(|| {
            between([
                vary_constrained(
                    [50.],
                    |[v]| IslandState::Slider(v),
                    |s| match s {
                        IslandState::Slider(v) => [-v, v - 100.],
                        _ => [0., 0.],
                    },
                ),
                fixed(IslandState::Island(0)),
                fixed(IslandState::Island(1)),
            ])
        })
    }
}

#[derive(Clone, PartialEq)]
struct StickyState {
    position: usize,
    amount: usize,
}
struct Sticky;
impl Example for Sticky {
    type State = StickyState;
    fn initial(&self) -> Self::State {
        StickyState {
            position: 0,
            amount: 0,
        }
    }
    fn scene(&self, s: &Self::State) -> Scene {
        let mut scene = Scene::new(320., 140.);
        for i in 0..3 {
            let x = 30. + 100. * i as f64;
            scene.push(
                outlined_rect(
                    format!("slot-{i}"),
                    Rect::new(x, 50., x + 40., 90.),
                    6.,
                    [0; 4],
                )
                .decorative(),
            );
        }
        let x = 30. + 100. * s.position as f64;
        scene.push(Object::rect(
            "knob",
            Rect::new(x, 50., x + 40., 90.),
            6.,
            INK,
        ));
        scene
    }
    fn drag(
        &self,
        s: &Self::State,
        id: &str,
        _: Modifiers,
    ) -> Option<DragSpec<'static, Self::State>> {
        (id == "knob").then(|| {
            closest_with(
                (0..3).map(|position| StickyState {
                    position,
                    ..s.clone()
                }),
                ClosestOptions {
                    stickiness: [0., 10., 30., 60.][s.amount],
                    lock_past: None,
                },
            )
        })
    }
    fn controls(&self, s: &Self::State) -> Vec<Control> {
        vec![Control::Choice {
            id: "amount",
            label: "Stickiness".into(),
            options: ["0", "10", "30", "60"].map(str::to_string).to_vec(),
            selected: s.amount,
        }]
    }
    fn action(&self, s: &Self::State, id: &str, v: Value) -> Option<Self::State> {
        if let ("amount", Value::Index(amount)) = (id, v)
            && amount < 4
        {
            Some(StickyState {
                amount,
                ..s.clone()
            })
        } else {
            None
        }
    }
}

#[derive(Clone, PartialEq)]
struct SpokeState {
    at: Option<usize>,
    lock: usize,
}
struct Spokes;
fn leaf(i: usize) -> Point {
    let angle = [0.0_f64, 30., 60., 150., 240.][i].to_radians();
    Point::new(150. + 110. * angle.cos(), 130. + 110. * angle.sin())
}
impl Example for Spokes {
    type State = SpokeState;
    fn initial(&self) -> Self::State {
        SpokeState { at: None, lock: 2 }
    }
    fn scene(&self, s: &Self::State) -> Scene {
        let mut scene = Scene::new(300., 260.);
        for i in 0..5 {
            scene.push(Object::line(
                format!("spoke-{i}"),
                (150., 130.),
                leaf(i),
                TRACK,
                2.,
            ));
            scene.push(outlined_circle(format!("leaf-{i}"), leaf(i), 14., [0; 4]).decorative());
        }
        scene.push(outlined_circle("hub", (150., 130.), 14., [0; 4]).decorative());
        scene.push(Object::circle(
            "dot",
            s.at.map(leaf).unwrap_or(Point::new(150., 130.)),
            12.,
            BLUE,
        ));
        scene
    }
    fn drag(
        &self,
        s: &Self::State,
        id: &str,
        _: Modifiers,
    ) -> Option<DragSpec<'static, Self::State>> {
        if id != "dot" {
            return None;
        }
        Some(
            if s.at.is_none() {
                closest_with(
                    (0..5).map(|i| {
                        between([
                            s.clone(),
                            SpokeState {
                                at: Some(i),
                                ..s.clone()
                            },
                        ])
                    }),
                    ClosestOptions {
                        stickiness: 0.,
                        lock_past: [None, Some(10.), Some(20.), Some(30.)][s.lock],
                    },
                )
            } else {
                between([
                    s.clone(),
                    SpokeState {
                        at: None,
                        ..s.clone()
                    },
                ])
            }
            .with_chaining(),
        )
    }

    fn controls(&self, s: &Self::State) -> Vec<Control> {
        vec![Control::Choice {
            id: "lock",
            label: "lockPast".into(),
            options: ["off", "10", "20", "30"].map(str::to_string).to_vec(),
            selected: s.lock,
        }]
    }
    fn action(&self, s: &Self::State, id: &str, v: Value) -> Option<Self::State> {
        if let ("lock", Value::Index(lock)) = (id, v)
            && lock < 4
        {
            Some(SpokeState { lock, ..s.clone() })
        } else {
            None
        }
    }
}

struct ChangeFrame;
impl Example for ChangeFrame {
    type State = f64;
    fn initial(&self) -> f64 {
        -45.
    }
    fn scene(&self, angle: &f64) -> Scene {
        let mut scene = Scene::new(300., 300.);
        scene.push(
            stroke(
                Object::circle("circle", (150., 150.), 100., [0; 4]),
                [204, 204, 204, 255],
                2.,
            )
            .decorative(),
        );
        scene.push(Object::line(
            "axis",
            (30., 150.),
            (270., 150.),
            [238, 238, 238, 255],
            1.,
        ));
        scene.push(
            Object::circle("point", Point::ZERO, 12., INK).transforms(vec![
                TransformComponent::Translate(Vec2::new(150., 150.)),
                TransformComponent::Rotate {
                    radians: angle.to_radians(),
                    center: Point::ZERO,
                },
                TransformComponent::Translate(Vec2::new(100., 0.)),
            ]),
        );
        scene
    }
    fn drag(&self, angle: &f64, id: &str, _: Modifiers) -> Option<DragSpec<'static, f64>> {
        if id != "point" {
            return None;
        }
        let mut inner = vary([*angle], |[a]| a);
        Some(DragSpec::custom(move |ctx, pointer| {
            inner.evaluate(ctx, Point::new(pointer.x, 300. - pointer.y))
        }))
    }
}

#[derive(Clone, PartialEq)]
struct DelayedState {
    x: f64,
    y: f64,
    delay: f64,
}
struct Delayed;
impl Example for Delayed {
    type State = DelayedState;
    fn initial(&self) -> Self::State {
        DelayedState {
            x: 200.,
            y: 200.,
            delay: 250.,
        }
    }
    fn scene(&self, s: &Self::State) -> Scene {
        let mut scene = Scene::new(400., 400.);
        scene.push(Object::circle("dot", (s.x, s.y), 15., INK));
        scene
    }
    fn drag(
        &self,
        s: &Self::State,
        id: &str,
        _: Modifiers,
    ) -> Option<DragSpec<'static, Self::State>> {
        let delay = s.delay;
        (id == "dot").then(|| vary([s.x, s.y], move |[x, y]| DelayedState { x, y, delay }))
    }
    fn pointer_delay(&self, s: &Self::State) -> Duration {
        Duration::from_secs_f64(s.delay / 1000.)
    }
    fn controls(&self, s: &Self::State) -> Vec<Control> {
        vec![Control::Slider {
            id: "delay",
            label: "Delay (ms)".into(),
            value: s.delay,
            min: 0.,
            max: 1000.,
            step: 10.,
        }]
    }
    fn action(&self, s: &Self::State, id: &str, value: Value) -> Option<Self::State> {
        if let ("delay", Value::Number(delay)) = (id, value) {
            Some(DelayedState {
                delay: delay.clamp(0., 1000.),
                ..s.clone()
            })
        } else {
            None
        }
    }
}

#[derive(Clone, PartialEq)]
struct ChartState {
    preset: usize,
    selected: [Option<usize>; 2],
}
struct Charts;
const DATA: [[f64; 6]; 3] = [
    [20., 25., 22., 28., 24., 26.],
    [60., 35., 75., 50., 80., 45.],
    [95., 10., 85., 5., 100., 30.],
];
impl Example for Charts {
    type State = ChartState;
    fn initial(&self) -> Self::State {
        ChartState {
            preset: 0,
            selected: [None, None],
        }
    }
    fn scene(&self, s: &Self::State) -> Scene {
        let mut scene = Scene::new(360., 240.);
        let color = [BLUE, [245, 158, 11, 255], RED][s.preset];
        for chart in 0..2 {
            let x = 20. + chart as f64 * 180.;
            scene.push(Object::text(
                format!("label-{chart}"),
                (x, 20.),
                ["dragologyLerpProps", "no lerp"][chart],
                13.,
                GRAY_700,
            ));
            scene.push(Object::line(
                format!("baseline-{chart}"),
                (x, 140.),
                (x + 144., 140.),
                MUTED,
                1.,
            ));
            for i in 0..6 {
                let value = DATA[s.preset][i];
                let mut bar = Object::rect(
                    format!("bar-{chart}-{i}"),
                    Rect::new(
                        x + i as f64 * 24.,
                        140. - value,
                        x + i as f64 * 24. + 18.,
                        140.,
                    ),
                    0.,
                    if s.selected[chart] == Some(i) {
                        [17, 24, 39, 255]
                    } else {
                        color
                    },
                );
                bar.click_handler = true;
                bar.cursor = Cursor::Pointer;
                bar.interpolate = chart == 0;
                scene.push(bar);
            }
        }
        scene.push(Object::line(
            "track",
            (40., 190.),
            (280., 190.),
            GRAY_300,
            6.,
        ));
        for i in 0..3 {
            scene.push(Object::text(
                format!("preset-{i}"),
                (40. + i as f64 * 120., 220.),
                ["calm", "busy", "wild"][i],
                12.,
                MUTED,
            ));
        }
        scene.push(Object::circle(
            "knob",
            (40. + s.preset as f64 * 120., 190.),
            12.,
            color,
        ));
        for object in &mut scene.objects {
            if object.id.starts_with("label-") {
                object.text_style.baseline = TextBaseline::Alphabetic;
            }
            if object.id.starts_with("preset-") {
                object.text_style.anchor = TextAnchor::Middle;
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
    fn drag(
        &self,
        s: &Self::State,
        id: &str,
        _: Modifiers,
    ) -> Option<DragSpec<'static, Self::State>> {
        (id == "knob").then(|| {
            between((0..3).map(|preset| ChartState {
                preset,
                ..s.clone()
            }))
            .with_drop_transition(Some(elastic()))
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
        let parts = id.strip_prefix("bar-")?.split('-').collect::<Vec<_>>();
        let chart = parts.first()?.parse::<usize>().ok()?;
        let bar = parts.get(1)?.parse::<usize>().ok()?;
        let mut next = s.clone();
        next.selected[chart] = if next.selected[chart] == Some(bar) {
            None
        } else {
            Some(bar)
        };
        Some(next)
    }
}

pub(super) fn entries() -> Vec<Entry> {
    vec![
        Entry::new(
            "between-with-vary",
            "between-with-vary",
            "This demonstrates using d.between with a drag spec (d.vary) inside it. It's pretty weird!",
            || demo(BetweenVary),
        ),
        Entry::new(
            "stickiness",
            "stickiness",
            "Stickiness adds a bias toward the current branch in a closest, making it resist switching until the pointer is clearly closer to another option.",
            || demo(Sticky),
        ),
        Entry::new(
            "lock-past",
            "lock-past",
            "A closest of betweens, one per spoke. Without lockPast, a drag partway out one spoke can jump onto a neighboring spoke. With it, once the dot is more than lockPast pixels from the hub, it stays on its spoke until it comes back.",
            || demo(Spokes),
        ),
        Entry::new(
            "controlled",
            "controlled",
            "A few simple tests of a controlled draggable component: its parent ultimately controls its state. The two columns compare updates on drop with updates throughout dragging.",
            super::controlled::create,
        ),
        Entry::new(
            "with-pointer-delay",
            "with-pointer-delay",
            "pointerDelay lags the pointer before feeding it to the wrapped spec. Release preserves the last delayed position: the behavior cannot continue after drop. The upstream custom-modifier example additionally supplies debug tree and buffered-pointer overlay views.",
            || demo(Delayed),
        ),
        Entry::new(
            "change-frame",
            "change-frame",
            "spec.changeFrame changes the frame a spec is given. Here a d.vary moving a point around a circle is given the pointer flipped upside down, so the point follows the cursor's reflection. The upstream debug Overlay also shows the flipped pointer.",
            || demo(ChangeFrame),
        ),
        Entry::new(
            "component-elements-lerp",
            "component-elements-lerp",
            "Both charts use the same component, with independent selection state: click a bar to select it. Drag the knob. Mid-drag, the left chart's data and color properties are blended; the right chart shows the nearest preset. On drop, the knob and the left chart's bars overshoot and settle.",
            || demo(Charts),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use dragology::DragSession;

    #[test]
    fn spoke_chaining_is_available_before_snapping_to_a_leaf() {
        let example = Spokes;
        let state = example.initial();
        let spec = example.drag(&state, "dot", Modifiers::default()).unwrap();
        let mut session =
            DragSession::begin(state, (150., 130.), move |s| example.locate(s, "dot"), spec)
                .unwrap();
        let frame = session.frame((211., 130.)).unwrap();
        assert_eq!(frame.drop_state.at, Some(0));
        assert!(frame.chain.is_some());
        assert!(frame.grab_position.unwrap().distance(leaf(0)) > 40.);
    }

    #[test]
    fn reflected_frame_moves_to_the_mirrored_arc_point() {
        let example = ChangeFrame;
        let state = example.initial();
        let spec = example.drag(&state, "point", Modifiers::default()).unwrap();
        let p = Point::new(150. + 100. / 2_f64.sqrt(), 150. - 100. / 2_f64.sqrt());
        let session =
            DragSession::begin(state, p, move |s| example.locate(s, "point"), spec).unwrap();
        let next = session.finish((250., 150.)).unwrap();
        assert!(next.abs() < 0.01);
    }

    #[test]
    fn reflected_frame_rotates_an_off_center_grab_with_the_point() {
        let center = Point::new(150., 150.);
        // Upstream rotates the parent group, including the circle's local
        // grab offset. Start ten pixels off center and reflect each target.
        let grab_at = |angle: f64| {
            center + (Affine::rotate(angle.to_radians()) * Point::new(100., 10.)).to_vec2()
        };
        for target_angle in [-90., 0., 45., 90.] {
            let example = ChangeFrame;
            let state = example.initial();
            let spec = example.drag(&state, "point", Modifiers::default()).unwrap();
            let session = DragSession::begin(
                state,
                grab_at(state),
                move |s| example.locate(s, "point"),
                spec,
            )
            .unwrap();
            let target = grab_at(target_angle);
            let next = session.finish((target.x, 300. - target.y)).unwrap();
            assert!(
                (next - target_angle).abs() < 0.01,
                "off-center grab dropped at {next}°, expected {target_angle}°"
            );
        }
    }

    #[test]
    fn right_chart_opts_out_of_interpolation() {
        let example = Charts;
        let scene = example.scene(&example.initial());
        for object in scene.objects {
            if object.id.starts_with("bar-0-") {
                assert!(object.interpolate);
            }
            if object.id.starts_with("bar-1-") {
                assert!(!object.interpolate);
            }
        }
    }
}
