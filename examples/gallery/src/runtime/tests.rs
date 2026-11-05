//! Lifecycle contracts shared by every gallery port.
use super::*;
use dragology::{Preview, between, closest, vary, vary_constrained};

const EPS: f64 = 0.02;
fn near(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < EPS,
        "expected {expected}, got {actual}"
    );
}
fn origin(scene: &Scene, id: &str) -> Point {
    scene.object(id).unwrap().transform * Point::ORIGIN
}
fn x<E: Example>(demo: &ModelDemo<E>, id: &str) -> f64 {
    origin(demo.scene(), id).x
}

#[derive(Clone, Copy)]
enum Behavior {
    Continuous,
    Branches,
    Chain,
}
#[derive(Clone, PartialEq, Debug)]
struct State {
    position: f64,
    step: usize,
}
struct Interaction {
    behavior: Behavior,
    delay: Duration,
    center_anchor: bool,
}
impl Interaction {
    fn new(behavior: Behavior) -> Self {
        Self {
            behavior,
            delay: Duration::ZERO,
            center_anchor: false,
        }
    }
}
impl Example for Interaction {
    type State = State;
    fn initial(&self) -> State {
        State {
            position: 10.,
            step: 0,
        }
    }
    fn scene(&self, s: &State) -> Scene {
        let mut scene = Scene::new(400., 130.);
        // Grandchild precedes both its ancestors to exercise transitive lookup.
        let mut grand = Object::circle("grandchild", (s.position + 25., 30.), 3., GREEN);
        grand.parent = Some("child".into());
        let mut child = Object::circle("child", (s.position + 15., 30.), 3., BLUE);
        child.parent = Some("root".into());
        scene.push(grand);
        scene.push(child);
        scene.push(Object::circle("root", (s.position, 30.), 8., RED));
        scene.push(Object::circle("backing", (s.position * 2., 85.), 8., MUTED).decorative());
        scene
    }
    fn locate(&self, s: &State, id: &str) -> Option<Affine> {
        Some(Affine::translate((
            s.position
                + match id {
                    "root" => 0.,
                    "child" => 15.,
                    "grandchild" => 25.,
                    _ => return None,
                },
            30.,
        )))
    }
    fn drag(&self, s: &State, id: &str, mods: Modifiers) -> Option<DragSpec<'static, State>> {
        if id != "root" {
            return None;
        }
        match self.behavior {
            Behavior::Continuous => {
                let base = s.clone();
                if mods.alt {
                    Some(vary_constrained(
                        [s.position],
                        move |[position]| State {
                            position,
                            ..base.clone()
                        },
                        |s: &State| [s.position - 30.],
                    ))
                } else {
                    Some(vary([s.position], move |[position]| State {
                        position,
                        ..base.clone()
                    }))
                }
            }
            Behavior::Branches => Some(
                closest([
                    State {
                        position: 10.,
                        step: 0,
                    },
                    State {
                        position: 110.,
                        step: 1,
                    },
                ])
                .with_floating()
                .with_branch_transition(Some(dragology::Transition::default())),
            ),
            Behavior::Chain => Some(
                between([
                    s.clone(),
                    State {
                        position: s.position + 50.,
                        step: s.step + 1,
                    },
                ])
                .with_snap_radius_options(
                    2.,
                    dragology::SnapOptions {
                        chain: true,
                        transition: Some(dragology::Transition::default()),
                    },
                ),
            ),
        }
    }
    fn anchor(&self, _: &State, _: &str, local: Point) -> Point {
        if self.center_anchor {
            Point::ORIGIN
        } else {
            local
        }
    }
    fn pointer_delay(&self, _: &State) -> Duration {
        self.delay
    }
    fn action(&self, s: &State, id: &str, value: Value) -> Option<State> {
        match (id, value) {
            ("position", Value::Number(position)) => Some(State {
                position,
                ..s.clone()
            }),
            _ => None,
        }
    }
}

#[test]
fn floating_moves_all_descendants_once_and_removes_stale_backdrop_members() {
    struct Group;
    impl Example for Group {
        type State = bool;
        fn initial(&self) -> bool {
            false
        }
        fn scene(&self, new: &bool) -> Scene {
            let mut s = Interaction::new(Behavior::Continuous).scene(&State {
                position: if *new { 100. } else { 10. },
                step: 0,
            });
            if !new {
                let mut old = Object::circle("stale", (40., 30.), 3., GOLD);
                old.parent = Some("grandchild".into());
                s.push(old);
            }
            s.push(Object::circle("root-note", (7., 90.), 3., GOLD));
            s
        }
        fn drag(&self, _: &bool, _: &str, _: Modifiers) -> Option<DragSpec<'static, bool>> {
            None
        }
    }
    let preview = Preview::Floating {
        backdrop: Rc::new(Preview::State(Rc::new(false))),
        source: Rc::new(Preview::State(Rc::new(true))),
        translation: Vec2::new(13., -7.),
        ghost_opacity: None,
    };
    let scene = scene::resolve_interaction(&Group, "root", None, &preview);
    for (id, expected) in [("root", 113.), ("child", 128.), ("grandchild", 138.)] {
        near(origin(&scene, id).x, expected);
        near(origin(&scene, id).y, 23.);
        assert_eq!(scene.objects.iter().filter(|o| o.id == id).count(), 1);
    }
    assert!(scene.object("stale").is_none());
    assert_eq!(origin(&scene, "root-note"), Point::new(7., 90.));
    near(origin(&scene, "backing").x, 20.);
}

#[test]
fn branch_animation_pins_dragged_group_while_backing_moves_and_cancel_restores_model() {
    let mut demo = ModelDemo::new(Interaction::new(Behavior::Branches));
    demo.press(Point::new(10., 30.), Modifiers::default(), 1);
    demo.move_to(Point::new(90., 30.), Modifiers::default());
    demo.advance(Duration::ZERO);
    assert!(demo.transition.is_some());
    for (id, expected) in [("root", 90.), ("child", 105.), ("grandchild", 115.)] {
        near(x(&demo, id), expected);
    }
    near(x(&demo, "backing"), 20.);
    demo.advance(Duration::from_millis(70));
    assert!(x(&demo, "backing") > 20. && x(&demo, "backing") < 220.);
    near(x(&demo, "root"), 90.);
    near(x(&demo, "grandchild"), 115.);
    demo.move_to(Point::new(95., 30.), Modifiers::default());
    demo.advance(Duration::ZERO);
    near(x(&demo, "root"), 95.);
    demo.cancel();
    assert_eq!(demo.state, demo.example.initial());
    assert!(demo.active.is_none());
    demo.advance(Duration::from_secs(1));
    near(x(&demo, "root"), 10.);
    near(x(&demo, "backing"), 20.);
}

#[test]
fn chained_steps_keep_the_local_anchor_and_cancel_to_the_original_press() {
    let mut demo = ModelDemo::new(Interaction::new(Behavior::Chain));
    demo.press(Point::new(12., 30.), Modifiers::default(), 1);
    for (step, pointer) in [(1, 62.), (2, 112.), (3, 162.)] {
        demo.move_to(Point::new(pointer, 30.), Modifiers::default());
        demo.advance(Duration::ZERO);
        assert_eq!(demo.active.as_ref().unwrap().anchor_state.step, step);
        // Upstream transitions the entire chained preview unless the object
        // explicitly opts out. The new target retains the original grab anchor.
        demo.advance(Duration::from_millis(200));
        near(x(&demo, "root"), pointer - 2.);
        near(x(&demo, "grandchild"), pointer + 23.);
    }
    demo.cancel();
    assert_eq!(demo.state.step, 0);
    near(demo.state.position, 10.);
    demo.advance(Duration::from_secs(1));
    near(x(&demo, "root"), 10.);
}

#[test]
fn delayed_moves_at_timestamp_zero_wait_the_full_delay() {
    let mut example = Interaction::new(Behavior::Continuous);
    example.delay = Duration::from_millis(100);
    let mut demo = ModelDemo::new(example);
    demo.set_animated(false);
    demo.press(Point::new(10., 30.), Modifiers::default(), 1);
    demo.move_to(Point::new(70., 30.), Modifiers::default());
    near(x(&demo, "root"), 10.);
    assert!(demo.advance(Duration::from_millis(99)));
    near(x(&demo, "root"), 10.);
    demo.advance(Duration::from_millis(1));
    near(x(&demo, "root"), 70.);
    demo.move_to(Point::new(95., 30.), Modifiers::default());
    near(x(&demo, "root"), 70.);
    demo.cancel();
    assert!(demo.pending.is_empty());
    demo.advance(Duration::from_millis(200));
    near(x(&demo, "root"), 10.);
}

#[test]
fn alt_changes_rebuild_legal_moves_without_losing_the_original_anchor() {
    let mut demo = ModelDemo::new(Interaction::new(Behavior::Continuous));
    demo.set_animated(false);
    demo.press(Point::new(13., 30.), Modifiers::default(), 1);
    demo.move_to(Point::new(73., 30.), Modifiers::default());
    demo.advance(Duration::ZERO);
    near(x(&demo, "root"), 70.);
    let alt = Modifiers {
        alt: true,
        ..Modifiers::default()
    };
    demo.move_to(Point::new(73., 30.), alt);
    demo.advance(Duration::ZERO);
    near(x(&demo, "root"), 30.);
    assert!(demo.active.as_ref().unwrap().modifiers.alt);
    demo.move_to(Point::new(73., 30.), Modifiers::default());
    demo.advance(Duration::ZERO);
    near(x(&demo, "root"), 70.);
    demo.release(Point::new(83., 30.), Modifiers::default());
    near(demo.state.position, 80.);
    assert!(demo.error.is_none());
}

#[test]
fn center_anchor_override_ignores_the_pressed_edge() {
    let mut example = Interaction::new(Behavior::Continuous);
    example.center_anchor = true;
    let mut demo = ModelDemo::new(example);
    demo.set_animated(false);
    demo.press(Point::new(16., 30.), Modifiers::default(), 1);
    near(x(&demo, "root"), 16.);
    demo.move_to(Point::new(80., 30.), Modifiers::default());
    demo.advance(Duration::ZERO);
    near(x(&demo, "root"), 80.);
    assert_eq!(demo.active.as_ref().unwrap().anchor, Point::ORIGIN);
}

#[test]
fn regrabbing_uses_the_unsprung_target_transform_like_upstream_callbacks() {
    let mut demo = ModelDemo::new(Interaction::new(Behavior::Continuous));
    demo.action("position", Value::Number(130.));
    demo.advance(Duration::from_millis(55));
    let visible = x(&demo, "root");
    assert!(visible > 10. && visible < 130.);
    let press = Point::new(visible + 3., 30.);
    demo.press(press, Modifiers::default(), 1);
    near(x(&demo, "root"), 130.);
    assert!(demo.transition.is_none());
    demo.move_to(press + Vec2::new(20., 0.), Modifiers::default());
    demo.advance(Duration::ZERO);
    near(x(&demo, "root"), 150.);
    demo.cancel();
    near(demo.state.position, 130.);
    demo.advance(Duration::from_secs(1));
    near(x(&demo, "root"), 130.);
}

fn blended(a: Object, b: Object, t: f64) -> Object {
    let mut first = Scene::new(200., 200.);
    first.push(a);
    let mut second = Scene::new(200., 200.);
    second.push(b);
    scene::blend(vec![(first, 1. - t), (second, t)])
        .objects
        .remove(0)
}
fn triangle(size: f64) -> BezPath {
    let mut p = BezPath::new();
    p.move_to((0., 0.));
    p.line_to((size, 0.));
    p.line_to((0., size));
    p.close_path();
    p
}

#[test]
fn rect_circle_and_path_hits_follow_the_interpolated_visible_geometry() {
    let rect = blended(
        Object::rect("shape", Rect::new(0., 0., 20., 20.), 0., RED),
        Object::rect("shape", Rect::new(0., 0., 100., 100.), 0., BLUE),
        0.25,
    );
    assert!(scene::hit(&rect, Point::new(39., 20.)));
    assert!(!scene::hit(&rect, Point::new(41., 20.)));
    let Shape::Rect { bounds, .. } = rect.shapes[0] else {
        panic!()
    };
    near(bounds.width(), 40.);
    let circle = blended(
        Object::circle("shape", (0., 0.), 10., RED),
        Object::circle("shape", (20., 0.), 30., BLUE),
        0.5,
    );
    assert!(scene::hit(&circle, Point::new(29., 0.)));
    assert!(!scene::hit(&circle, Point::new(31., 0.)));
    let polygon = |size| {
        let p = triangle(size);
        Object::new(
            "shape",
            Affine::IDENTITY,
            Some(Hit::Path(p.clone(), 0.)),
            vec![Shape::Path {
                path: p,
                fill: Some(RED),
                stroke: None,
            }],
        )
    };
    let poly = blended(polygon(40.), polygon(80.), 0.25);
    assert!(scene::hit(&poly, Point::new(30., 5.)));
    assert!(!scene::hit(&poly, Point::new(40., 40.)));
    let line = |y, width| {
        let mut p = BezPath::new();
        p.move_to((0., y));
        p.line_to((50., y));
        Object::new(
            "shape",
            Affine::IDENTITY,
            Some(Hit::Path(p.clone(), width)),
            vec![Shape::Path {
                path: p,
                fill: None,
                stroke: Some((RED, width)),
            }],
        )
    };
    let stroke = blended(line(20., 10.), line(60., 20.), 0.5);
    assert!(scene::hit(&stroke, Point::new(25., 46.)));
    assert!(!scene::hit(&stroke, Point::new(25., 49.)));
}

#[test]
fn affine_interpolation_preserves_rotation_and_rejects_singular_hit_transforms() {
    let a = Object::circle("shape", (40., 50.), 10., RED);
    let mut b = a.clone();
    b.transform = Affine::translate((40., 50.)) * Affine::rotate(std::f64::consts::FRAC_PI_2);
    let rotation = blended(a, b, 0.5);
    let p = rotation.transform * Point::new(10., 0.);
    near(p.x, 40. + 50_f64.sqrt());
    near(p.y, 50. + 50_f64.sqrt());
    near(rotation.transform.determinant(), 1.);
    let mut singular = rotation;
    singular.transform = Affine::scale(0.);
    assert!(!scene::hit(&singular, Point::ORIGIN));
}

#[test]
fn multiway_text_uses_the_strongest_sample_regardless_of_fold_order() {
    let make = |label: &str, x: f64, weight: f64| {
        let mut scene = Scene::new(200., 100.);
        scene.push(Object::text("text", (x, 20.), label, 12., INK));
        (scene, weight)
    };
    let a = make("A", 0., 0.30);
    let b = make("B", 100., 0.25);
    let c = make("C", 200., 0.45);
    for parts in [
        vec![a.clone(), b.clone(), c.clone()],
        vec![c.clone(), a.clone(), b.clone()],
        vec![b.clone(), c.clone(), a.clone()],
    ] {
        let result = scene::blend(parts);
        let object = result.object("text").unwrap();
        let Shape::Text { text, .. } = &object.shapes[0] else {
            panic!()
        };
        assert_eq!(text, "C");
        near((object.transform * Point::ORIGIN).x, 115.);
    }
}

#[test]
fn chaining_waits_for_a_frame_then_continues_without_pointer_motion() {
    struct Stepper;
    impl Example for Stepper {
        type State = usize;
        fn initial(&self) -> usize {
            0
        }
        fn scene(&self, _: &usize) -> Scene {
            let mut scene = Scene::new(100., 100.);
            scene.push(Object::circle("step", (30., 30.), 10., RED));
            scene
        }
        fn drag(&self, state: &usize, _: &str, _: Modifiers) -> Option<DragSpec<'static, usize>> {
            Some(
                dragology::fixed(state + 1).with_chaining_options(dragology::Chaining {
                    transition: None,
                    ..Default::default()
                }),
            )
        }
    }
    let mut demo = ModelDemo::new(Stepper);
    demo.press(Point::new(30., 30.), Modifiers::default(), 1);
    assert_eq!(demo.active.as_ref().unwrap().anchor_state, 0);
    for expected in 1..=3 {
        assert!(demo.advance(Duration::from_millis(16)));
        assert_eq!(demo.active.as_ref().unwrap().anchor_state, expected);
    }
    demo.cancel();
    assert_eq!(demo.state, 0);
}

#[test]
fn release_evaluates_a_stateful_operator_once_without_following_its_chain() {
    use std::cell::Cell;
    struct Counted(Rc<Cell<usize>>);
    impl Example for Counted {
        type State = usize;
        fn initial(&self) -> usize {
            0
        }
        fn scene(&self, _: &usize) -> Scene {
            let mut scene = Scene::new(100., 100.);
            scene.push(Object::circle("count", (30., 30.), 10., RED));
            scene
        }
        fn drag(&self, _: &usize, _: &str, _: Modifiers) -> Option<DragSpec<'static, usize>> {
            let count = self.0.clone();
            Some(
                DragSpec::custom(move |ctx, point| {
                    count.set(count.get() + 1);
                    ctx.state_frame(Rc::new(count.get()), point)
                })
                .with_chaining(),
            )
        }
    }
    let count = Rc::new(Cell::new(0));
    let mut demo = ModelDemo::new(Counted(count.clone()));
    demo.press(Point::new(30., 30.), Modifiers::default(), 1);
    assert_eq!(count.get(), 1);
    demo.release(Point::new(30., 30.), Modifiers::default());
    assert_eq!(count.get(), 2);
    assert_eq!(demo.state, 2);
    assert!(demo.active.is_none());
}

#[test]
fn releasing_into_a_new_branch_does_not_start_a_branch_spring() {
    struct Branches;
    impl Example for Branches {
        type State = bool;
        fn initial(&self) -> bool {
            false
        }
        fn scene(&self, state: &bool) -> Scene {
            let mut scene = Scene::new(200., 100.);
            scene.push(Object::circle(
                "root",
                (if *state { 130. } else { 30. }, 30.),
                10.,
                RED,
            ));
            scene
        }
        fn drag(&self, _: &bool, _: &str, _: Modifiers) -> Option<DragSpec<'static, bool>> {
            Some(closest([false, true]))
        }
    }
    let mut demo = ModelDemo::new(Branches);
    demo.press(Point::new(30., 30.), Modifiers::default(), 1);
    demo.release(Point::new(130., 30.), Modifiers::default());
    assert!(demo.state);
    near(x(&demo, "root"), 130.);
    assert!(demo.transition.is_none());
}

#[test]
fn click_and_drag_elements_wait_for_the_source_two_pixel_threshold() {
    struct ClickDrag;
    impl Example for ClickDrag {
        type State = f64;
        fn initial(&self) -> f64 {
            30.
        }
        fn scene(&self, state: &f64) -> Scene {
            self.scene_dragging(state, None)
        }
        fn scene_dragging(&self, state: &f64, dragged: Option<&str>) -> Scene {
            let mut scene = Scene::new(200., 100.);
            scene.push(
                Object::circle(
                    "root",
                    (*state, 30.),
                    10.,
                    if dragged.is_some() { RED } else { BLUE },
                )
                .clickable(),
            );
            scene
        }
        fn drag(&self, state: &f64, _: &str, _: Modifiers) -> Option<DragSpec<'static, f64>> {
            Some(vary([*state], |[position]| position))
        }
        fn click(&self, _: &f64, _: &str, _: Point, _: Modifiers, _: usize) -> Option<f64> {
            Some(130.)
        }
    }
    let mut demo = ModelDemo::new(ClickDrag);
    let idle = demo.scene.clone();
    demo.press(Point::new(30., 30.), Modifiers::default(), 1);
    demo.advance(Duration::from_millis(200));
    demo.move_to(Point::new(32., 30.), Modifiers::default());
    demo.advance(Duration::ZERO);
    assert_eq!(demo.scene, idle);
    demo.release(Point::new(32., 30.), Modifiers::default());
    near(demo.state, 130.);
    demo.reset();
    demo.press(Point::new(30., 30.), Modifiers::default(), 1);
    demo.move_to(Point::new(33., 30.), Modifiers::default());
    demo.advance(Duration::ZERO);
    assert_ne!(demo.scene, idle);
    demo.release(Point::new(33., 30.), Modifiers::default());
    near(demo.state, 33.);
}

#[test]
fn pointer_samples_coalesce_and_stateful_behavior_runs_once_per_frame() {
    use std::cell::Cell;
    struct Counted(Rc<Cell<usize>>);
    impl Example for Counted {
        type State = f64;
        fn initial(&self) -> f64 {
            30.
        }
        fn scene(&self, state: &f64) -> Scene {
            let mut scene = Scene::new(200., 100.);
            scene.push(Object::circle("root", (*state, 30.), 10., RED));
            scene
        }
        fn drag(&self, _: &f64, _: &str, _: Modifiers) -> Option<DragSpec<'static, f64>> {
            let count = self.0.clone();
            Some(DragSpec::custom(move |ctx, point| {
                count.set(count.get() + 1);
                ctx.state_frame(Rc::new(point.x), point)
            }))
        }
    }
    let count = Rc::new(Cell::new(0));
    let mut demo = ModelDemo::new(Counted(count.clone()));
    demo.set_animated(false);
    demo.press(Point::new(30., 30.), Modifiers::default(), 1);
    assert_eq!(count.get(), 1); // Source initDrag evaluates once.
    for pointer in [40., 50., 60.] {
        demo.move_to(Point::new(pointer, 30.), Modifiers::default());
    }
    assert_eq!(count.get(), 1);
    near(x(&demo, "root"), 30.);
    demo.advance(Duration::from_millis(16));
    assert_eq!(count.get(), 2);
    near(x(&demo, "root"), 60.);
    demo.advance(Duration::from_millis(16));
    assert_eq!(count.get(), 3); // A stationary pointer still advances behavior.
    demo.release(Point::new(80., 30.), Modifiers::default());
    assert_eq!(count.get(), 4);
    near(demo.state, 80.);
}

#[test]
fn chained_initialization_errors_cancel_and_restore_the_original_model() {
    struct BrokenChain {
        failure: u8,
    }
    impl Example for BrokenChain {
        type State = usize;
        fn initial(&self) -> usize {
            0
        }
        fn scene(&self, state: &usize) -> Scene {
            let mut scene = Scene::new(100., 100.);
            let mut object = Object::circle("root", (30., 30.), 10., RED);
            if *state == 1 && self.failure == 2 {
                return scene;
            }
            if *state == 1 && self.failure == 1 {
                object.transform = Affine::scale(0.);
            }
            scene.push(object);
            scene
        }
        fn drag(&self, state: &usize, _: &str, _: Modifiers) -> Option<DragSpec<'static, usize>> {
            if *state == 0 {
                Some(dragology::fixed(0).on_drop(|_| 1).with_chaining())
            } else {
                Some(DragSpec::custom(|_, _| {
                    Err(DragError::InvalidSpec("chained evaluation failed"))
                }))
            }
        }
    }
    for failure in [0, 1, 2] {
        let mut demo = ModelDemo::new(BrokenChain { failure });
        demo.press(Point::new(30., 30.), Modifiers::default(), 1);
        assert!(demo.error.is_none());
        assert!(demo.active.is_some());
        demo.advance(Duration::from_millis(16));
        assert!(demo.active.is_none());
        assert!(demo.press.is_none());
        assert_eq!(demo.state, 0);
        assert_eq!(
            demo.error.as_deref(),
            Some(match failure {
                0 => "chained evaluation failed",
                1 => "invalid element transform or grab position",
                _ => "the drag needs an element missing from this state",
            })
        );
        demo.release(Point::new(30., 30.), Modifiers::default());
        assert_eq!(demo.state, 0);
    }
}

#[test]
fn nested_operator_costs_use_the_actual_pivoted_preview_geometry() {
    #[derive(Clone, PartialEq)]
    enum PivotState {
        Angle(f64),
        Center,
    }
    struct Pivot;
    impl Example for Pivot {
        type State = PivotState;
        fn initial(&self) -> PivotState {
            PivotState::Angle(0.)
        }
        fn scene(&self, state: &PivotState) -> Scene {
            let mut scene = Scene::new(200., 200.);
            let mut transforms = vec![TransformComponent::Translate(Vec2::new(100., 100.))];
            if let PivotState::Angle(angle) = state {
                transforms.push(TransformComponent::Rotate {
                    radians: angle.to_radians(),
                    center: Point::ORIGIN,
                });
                transforms.push(TransformComponent::Translate(Vec2::new(50., 0.)));
            }
            scene.push(Object::circle("root", Point::ORIGIN, 10., RED).transforms(transforms));
            scene
        }
        fn drag(
            &self,
            _: &PivotState,
            _: &str,
            _: Modifiers,
        ) -> Option<DragSpec<'static, PivotState>> {
            Some(between([
                between([PivotState::Angle(0.), PivotState::Angle(180.)]),
                dragology::fixed(PivotState::Center),
            ]))
        }
    }
    let mut demo = ModelDemo::new(Pivot);
    demo.set_animated(false);
    demo.press(Point::new(150., 100.), Modifiers::default(), 1);
    demo.move_to(Point::new(100., 100.), Modifiers::default());
    demo.advance(Duration::ZERO);
    // The half-blended rotational preview remains on the radius-50 arc;
    // interpolating only endpoint positions would falsely put it at center.
    assert!(matches!(
        **demo.active.as_ref().unwrap().last_drop.as_ref().unwrap(),
        PivotState::Center
    ));
    assert_eq!(origin(demo.scene(), "root"), Point::new(100., 100.));
}
