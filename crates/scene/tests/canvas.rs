use dragology_scene::{
    Canvas, Diagram, DragSpec, Item, Point, Pointer, Quad, Rect, Vec2, Viewport, dragology::between,
};

struct Switch;
impl Diagram for Switch {
    type Id = &'static str;
    type State = bool;
    fn layout(&self, on: &bool) -> Vec<Item<Self::Id>> {
        let thumb = Item {
            id: "thumb",
            quad: Quad {
                bounds: if *on {
                    Rect::new(80., 0., 120., 20.)
                } else {
                    Rect::new(0., 0., 20., 20.)
                },
                color: [10, 20, 30, 255],
                radius: 5.,
            },
        };
        let label = Item {
            id: "off label",
            quad: Quad {
                bounds: Rect::new(50., 30., 70., 50.),
                color: [255; 4],
                radius: 0.,
            },
        };
        if *on { vec![thumb] } else { vec![label, thumb] }
    }
    fn drag(&self, _: &bool, id: &Self::Id) -> Option<DragSpec<'static, bool>> {
        (*id == "thumb").then(|| between([false, true]))
    }
}

#[test]
fn keyed_blend_preserves_the_local_anchor_and_fades_unmatched_items() {
    let mut scene = Canvas::new(Switch, false, Vec2::new(120., 50.));
    assert!(!scene.handle(Pointer::Press(Point::new(0., 0.)))); // rounded corner
    assert!(scene.handle(Pointer::Press(Point::new(5., 10.))));
    // One quarter across either thumb: x=5 at rest and x=90 when on.
    scene.handle(Pointer::Move(Point::new(47.5, 10.)));
    let thumb = scene.items().iter().find(|i| i.id == "thumb").unwrap().quad;
    assert_eq!(thumb.bounds, Rect::new(40., 0., 70., 20.));
    let label = scene
        .items()
        .iter()
        .find(|i| i.id == "off label")
        .unwrap()
        .quad;
    assert_eq!(label.bounds, Rect::new(50., 30., 70., 50.));
    assert_eq!(label.color, [255, 255, 255, 128]);
    assert!(!scene.model());
    scene.handle(Pointer::Release(Point::new(92., 10.)));
    assert!(*scene.model());
    assert_eq!(scene.items().len(), 1);
}

#[test]
fn failed_release_and_external_model_changes_discard_the_transient_view() {
    let mut scene = Canvas::new(Switch, false, Vec2::new(120., 50.));
    scene.handle(Pointer::Press(Point::new(10., 10.)));
    scene.handle(Pointer::Release(Point::new(f64::NAN, 0.)));
    assert!(scene.last_error().is_some());
    assert!(!scene.is_dragging());
    assert!(!scene.model());
    scene.handle(Pointer::Press(Point::new(10., 10.)));
    scene.set_model(true);
    assert!(!scene.handle(Pointer::Release(Point::ORIGIN)));
    assert!(*scene.model());
    assert!(scene.last_error().is_none());
}

#[test]
fn viewport_centers_inside_an_offset_nonmatching_host_and_round_trips_input() {
    let view = Viewport::fit(Vec2::new(100., 100.), Rect::new(10., 20., 410., 220.)).unwrap();
    assert_eq!(view.to_host(Point::ORIGIN), Point::new(110., 20.));
    let point = Point::new(25., 65.);
    assert_eq!(view.to_scene(view.to_host(point)), point);
    assert!(Viewport::fit(Vec2::new(100., 100.), Rect::ZERO).is_none());
}

struct FloatingSwitch;
impl Diagram for FloatingSwitch {
    type Id = &'static str;
    type State = bool;
    fn layout(&self, on: &bool) -> Vec<Item<Self::Id>> {
        Switch.layout(on)
    }
    fn drag(&self, _: &bool, id: &Self::Id) -> Option<DragSpec<'static, bool>> {
        (*id == "thumb").then(|| dragology_scene::dragology::closest([false, true]).with_floating())
    }
}

#[test]
fn animated_drop_commits_immediately_and_can_be_grabbed_mid_flight() {
    use std::time::Duration;
    let mut scene = Canvas::new(FloatingSwitch, false, Vec2::new(120., 50.));
    scene.set_animated(true);
    scene.handle(Pointer::Press(Point::new(10., 10.)));
    scene.handle(Pointer::Move(Point::new(100., -30.)));
    scene.advance(Duration::from_millis(16));
    let thumb = |scene: &Canvas<FloatingSwitch>| {
        scene
            .items()
            .iter()
            .find(|i| i.id == "thumb")
            .unwrap()
            .quad
            .bounds
    };
    assert_eq!(thumb(&scene).center(), Point::new(100., -30.));
    scene.handle(Pointer::Release(Point::new(100., -30.)));
    assert!(*scene.model());
    assert_eq!(thumb(&scene).center(), Point::new(100., -30.));
    assert!(scene.advance(Duration::from_millis(60)));
    let mid = thumb(&scene).center();
    assert!(mid.y > -30. && mid.y < 10.);

    // A new gesture uses the local point in the visible, moving item.
    let grab = mid + Vec2::new(6., 0.);
    scene.handle(Pointer::Press(grab));
    assert!(thumb(&scene).center().distance(mid) < 1e-9);
    scene.handle(Pointer::Move(grab + Vec2::new(0., -20.)));
    assert!(thumb(&scene).center().distance(mid + Vec2::new(0., -20.)) < 1e-9);
    scene.handle(Pointer::Cancel);
    assert!(*scene.model());
    assert!(!scene.advance(Duration::from_secs(1)));
    assert_eq!(scene.items(), FloatingSwitch.layout(&true));
    assert!(!scene.is_animating());
}

#[test]
fn reduced_motion_and_external_reset_finish_pending_transitions() {
    use std::time::Duration;
    let mut scene = Canvas::new(FloatingSwitch, false, Vec2::new(120., 50.));
    scene.set_animated(true);
    scene.handle(Pointer::Press(Point::new(10., 10.)));
    scene.handle(Pointer::Release(Point::new(100., -30.)));
    assert!(scene.is_animating());
    scene.set_animated(false);
    assert!(!scene.advance(Duration::from_millis(16)));
    assert_eq!(scene.items(), FloatingSwitch.layout(&true));

    scene.set_animated(true);
    scene.handle(Pointer::Press(Point::new(100., 10.)));
    scene.handle(Pointer::Move(Point::new(30., -20.)));
    scene.set_model(false);
    assert!(!scene.is_dragging());
    assert!(!scene.is_animating());
    assert_eq!(scene.items(), FloatingSwitch.layout(&false));
}
