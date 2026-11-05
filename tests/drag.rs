use dragology::*;

fn at(p: &Point) -> Option<Affine> {
    Some(Affine::translate(p.to_vec2()))
}

fn near(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-8, "{actual} vs {expected}");
}

// A headless consumer that draws just one point. Checks the public preview
// instructions independently of the engine's reported grab position.
fn draw<S>(preview: &Preview<S>, locate: &impl Fn(&S) -> Point) -> Point {
    match preview {
        Preview::State(s) => locate(s),
        Preview::Blend(parts) => {
            near(parts.iter().map(|(_, w)| w).sum(), 1.0);
            parts
                .iter()
                .map(|(view, w)| draw(view, locate).to_vec2() * *w)
                .fold(Vec2::ZERO, |acc, v| acc + v)
                .to_point()
        }
        Preview::Floating {
            source,
            translation,
            ..
        } => draw(source, locate) + *translation,
    }
}

#[test]
fn toggle_blends_the_view_and_commits_at_release() {
    let locate = |on: &bool| Point::new(if *on { 90.0 } else { 30.0 }, 30.0);
    let mut drag = DragSession::begin(
        false,
        (33.0, 34.0),
        |s| at(&locate(s)),
        between([false, true]),
    )
    .unwrap();
    for (x, knob, on) in [
        (0., 30., false),
        (48., 45., false),
        (63., 60., false),
        (64., 61., true),
        (150., 90., true),
    ] {
        let f = drag.frame((x, 44.)).unwrap();
        near(draw(&f.preview, &locate).x, knob);
        near(f.grab_position.unwrap().x, knob + 3.);
        near(
            f.gap,
            Point::new(knob + 3., 34.).distance(Point::new(x, 44.)),
        );
        assert_eq!(*f.drop_state, on);
    }
    assert!(!drag.finish((34., 34.)).unwrap());
}

#[test]
fn starting_transform_is_inverted_once_and_applied_to_each_target() {
    let start = Affine::translate((8., 6.)) * Affine::scale(2.);
    let end = Affine::translate((30., 20.)) * Affine::rotate(std::f64::consts::FRAC_PI_2);
    let local = Point::new(3., -4.);
    let target = end * local;
    let mut drag = DragSession::begin(start, start * local, |s| Some(*s), fixed(end)).unwrap();
    assert_eq!(drag.frame(target).unwrap().grab_position, Some(target));
    assert_eq!(drag.cancel(), start);
}

#[test]
fn interpolation_reproduces_affine_layout_inside_and_projects_outside() {
    let sites = [(10., 10.), (0., 0.), (0., 10.), (10., 0.)].map(Point::from);
    for interpolation in [Interpolation::Delaunay, Interpolation::NaturalNeighbor] {
        let spec = between_with(
            sites,
            BetweenOptions {
                interpolation,
                ..Default::default()
            },
        );
        let mut drag = DragSession::begin(sites[0], sites[0], at, spec).unwrap();
        for x in [-1., 0., 2., 5., 8., 10., 11.] {
            for y in [-3., 0., 3., 5., 9., 10., 12.] {
                let p = Point::new(x, y);
                let expected = Point::new(x.clamp(0., 10.), y.clamp(0., 10.));
                let f = drag.frame(p).unwrap();
                near(draw(&f.preview, &|p| *p).distance(expected), 0.);
                near(f.grab_position.unwrap().distance(expected), 0.);
                near(f.gap, p.distance(expected));
                near(
                    f.drop_state.distance(p),
                    sites
                        .iter()
                        .map(|s| s.distance(p))
                        .fold(f64::INFINITY, f64::min),
                );
            }
        }
    }
}

#[test]
fn natural_neighbors_and_collinear_sites_use_the_correct_models() {
    let sites = [(0., 0.), (10., 0.), (0., 10.), (10., 10.)].map(Point::from);
    let spec = between_with(
        sites,
        BetweenOptions {
            interpolation: Interpolation::NaturalNeighbor,
            ..Default::default()
        },
    );
    let mut drag = DragSession::begin(sites[0], sites[0], at, spec).unwrap();
    let frame = drag.frame((5., 5.)).unwrap();
    let Preview::Blend(weights) = frame.preview.as_ref() else {
        panic!("expected blend")
    };
    assert_eq!(weights.len(), 4);
    for (_, w) in weights {
        near(*w, 0.25);
    }

    let sites = [(20., 10.), (0., 0.), (40., 20.), (10., 5.)].map(Point::from);
    let mut drag = DragSession::begin(sites[0], sites[0], at, between(sites)).unwrap();
    // Perpendicular offset from the midpoint of two adjacent sites.
    let frame = drag.frame((13., 11.5)).unwrap();
    let Preview::Blend(parts) = frame.preview.as_ref() else {
        panic!("expected blend")
    };
    assert_eq!(parts.len(), 2);
    for (p, w) in parts {
        assert!(matches!(p.as_ref(), Preview::State(s) if **s == sites[0] || **s == sites[3]));
        near(*w, 0.5);
    }
}

#[test]
fn sharpness_and_snap_radius_change_only_the_preview() {
    let sites = [(0., 0.), (100., 0.)].map(Point::from);
    let spec = between_with(
        sites,
        BetweenOptions {
            sharpness: 2.,
            ..Default::default()
        },
    );
    let mut drag = DragSession::begin(sites[0], sites[0], at, spec.with_snap_radius(11.)).unwrap();
    let f = drag.frame((25., 30.)).unwrap(); // sharpened x = 10, inside snap radius
    assert_eq!(draw(&f.preview, &|p| *p), sites[0]);
    assert_eq!(f.grab_position, Some(sites[0]));
    near(f.gap, 30.);
    let f = drag.frame((40., 30.)).unwrap();
    near(draw(&f.preview, &|p| *p).x, 100. * 16. / 52.);
    assert_eq!(*f.drop_state, sites[0]);
}

#[test]
fn closest_locks_a_spoke_until_the_grab_returns_home() {
    let home = Point::ORIGIN;
    let right = Point::new(100., 0.);
    let down = Point::new(0., 100.);
    let spokes = [between([home, right]), between([home, down])];
    let spec = closest_with(
        spokes,
        ClosestOptions {
            lock_past: Some(10.),
            ..Default::default()
        },
    );
    let mut drag = DragSession::begin(home, home, at, spec).unwrap();
    for (pointer, expected) in [
        ((60., 5.), (60., 0.)),
        ((40., 60.), (40., 0.)),
        ((-10., 60.), (0., 60.)),
    ] {
        assert_eq!(
            drag.frame(pointer).unwrap().grab_position,
            Some(expected.into())
        );
    }
}

#[test]
fn closest_stickiness_and_when_far_have_independent_memory() {
    let sites = [(0., 0.), (10., 0.)].map(Point::from);
    let spec = closest_with(
        sites,
        ClosestOptions {
            stickiness: 2.,
            ..Default::default()
        },
    )
    .when_far_with(Point::new(100., 0.), 5., 10.);
    let mut drag = DragSession::begin(sites[0], sites[0], at, spec).unwrap();
    for (x, y, expected) in [
        (3., 0., 0.),
        (5.5, 0., 0.),
        (7., 0., 10.),
        (10., 10., 10.),
        (10., 11., 100.),
        (10., 8., 100.),
        (10., 5., 10.),
    ] {
        near(drag.frame((x, y)).unwrap().drop_state.x, expected);
    }
}

#[test]
fn reorder_floats_the_same_item_while_other_rows_preview_the_drop() {
    let orders = [['A', 'B', 'C'], ['B', 'A', 'C'], ['B', 'C', 'A']];
    let origin = |order: &[char; 3]| {
        let row = order.iter().position(|&key| key == 'A').unwrap();
        Point::new(20., 20. + row as f64 * 50.)
    };
    let spec = closest(orders).with_floating();
    let mut drag = DragSession::begin(orders[0], (28., 23.), |s| at(&origin(s)), spec).unwrap();
    let f = drag.frame((48., 102.)).unwrap();
    let Preview::Floating { backdrop, .. } = f.preview.as_ref() else {
        panic!("expected float")
    };
    assert!(matches!(backdrop.as_ref(), Preview::State(order) if **order == orders[2]));
    assert_eq!(draw(&f.preview, &origin), Point::new(40., 99.));
    assert_eq!(*f.drop_state, orders[2]);
    assert_eq!(drag.cancel(), orders[0]);
}

#[test]
fn missing_element_can_still_float_over_a_deletion_target() {
    let spec = drop_target(None, Rect::new(20., 30., 40., 50.))
        .with_floating()
        .when_far(Some(4.), 0.);
    let mut drag = DragSession::begin(
        Some(4.),
        (6., 1.),
        |x: &Option<f64>| x.map(|v| Affine::translate((v, 0.))),
        spec,
    )
    .unwrap();
    let f = drag.frame((20., 30.)).unwrap();
    assert_eq!(*f.drop_state, None);
    assert_eq!(
        draw(&f.preview, &|s| Point::new(s.unwrap(), 0.)),
        Point::new(18., 29.)
    );
    assert_eq!(drag.finish((41., 51.)).unwrap(), Some(4.));
}

#[test]
fn empty_and_invalid_targets_have_explicit_outcomes() {
    let empty = closest(Vec::<Point>::new()).when_far(Point::new(4., 5.), 5.);
    let drag = DragSession::begin(Point::ORIGIN, Point::ORIGIN, at, empty).unwrap();
    assert_eq!(drag.finish(Point::ORIGIN).unwrap(), Point::new(4., 5.));

    for (sites, expected) in [
        (vec![], DragError::EmptyBetween),
        (
            vec![Point::ORIGIN; 2],
            DragError::CoincidentTargets {
                first: 0,
                second: 1,
            },
        ),
    ] {
        let mut drag =
            DragSession::begin(Point::ORIGIN, Point::ORIGIN, at, between(sites)).unwrap();
        assert_eq!(drag.frame(Point::ORIGIN).unwrap_err(), expected);
    }
    assert!(matches!(
        DragSession::begin(0., (0., 0.), |_| Some(Affine::scale(0.)), fixed(0.)),
        Err(DragError::InvalidTransform)
    ));
}

#[test]
fn a_child_spec_previews_and_drops_in_its_parents_layout() {
    #[derive(Clone, Debug, PartialEq)]
    struct Card {
        left: f64,
        step: u8,
    }

    let card = Card { left: 20., step: 0 };
    let origin = |card: &Card| Point::new(card.left + 40. * f64::from(card.step), 0.);
    let child = between([0, 1]).on_drop(|&step| step + 2);
    let spec = substate(card.clone(), |c| &mut c.step, child).with_floating();
    let mut drag = DragSession::begin(card, (23., 5.), |s| at(&origin(s)), spec).unwrap();
    let f = drag.frame((53., 10.)).unwrap();
    let Preview::Floating { backdrop, .. } = f.preview.as_ref() else {
        panic!("expected float")
    };
    near(draw(backdrop, &origin).x, 50.);
    assert_eq!(draw(&f.preview, &origin), Point::new(50., 5.));
    assert_eq!(*f.drop_state, Card { left: 20., step: 3 });
}

#[test]
fn a_custom_modifier_can_delegate_to_another_spec() {
    let mut child = fixed(Point::new(40., 0.)).during(|p| *p + Vec2::new(20., 0.));
    let inner = DragSpec::custom(move |ctx, pointer| {
        let mut frame = child.evaluate(ctx, pointer)?;
        frame.gap *= 0.5;
        Ok(frame)
    });
    let spec = inner.when_far(Point::ORIGIN, 5.);
    let mut drag = DragSession::begin(Point::ORIGIN, (0., 0.), at, spec).unwrap();
    assert_eq!(drag.frame((50., 0.)).unwrap().drop_state.x, 60.);
    assert_eq!(drag.finish((49., 0.)).unwrap(), Point::ORIGIN);
}
