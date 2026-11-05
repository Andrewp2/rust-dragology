use dragology::*;

fn close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 0.01, "{actual} vs {expected}");
}

#[test]
fn slider_finds_the_nearest_feasible_value_with_an_off_center_grab() {
    let slider = vary_constrained([60.], |[x]| x, |&x| [-x, x - 240.]);
    let mut drag = DragSession::begin(
        60.,
        (65., 4.),
        |&x| Some(Affine::translate((x, 0.))),
        slider,
    )
    .unwrap();
    for (x, y, expected) in [
        (150., 50., 145.),
        (152., 10., 147.),
        (300., 10., 240.),
        (-50., 10., 0.),
        (45., 5., 40.),
    ] {
        let f = drag.frame((x, y)).unwrap();
        close(*f.drop_state, expected);
        close(f.grab_position.unwrap().x, expected + 5.);
        close(
            f.gap,
            Point::new(x, y).distance(Point::new(expected + 5., 4.)),
        );
    }
}

#[test]
fn varying_tracks_compose_inside_closest() {
    let track = |y| vary([0.], move |[x]| Point::new(x, y));
    let spec = closest([track(0.), track(40.)]);
    let mut drag = DragSession::begin(
        Point::ORIGIN,
        (0., 0.),
        |p| Some(Affine::translate(p.to_vec2())),
        spec,
    )
    .unwrap();
    for (x, y, track) in [
        (10., 10., 0.),
        (12., 30., 40.),
        (-5., 50., 40.),
        (-4., 0., 0.),
    ] {
        let f = drag.frame((x, y)).unwrap();
        close(f.drop_state.x, x);
        close(f.drop_state.y, track);
        close(f.gap, (track - y).abs());
    }
}

#[test]
fn scaled_parameters_respect_a_coupled_constraint() {
    let model = |[x, y]: [f64; 2]| Point::new(x * 100., y * 0.1);
    let spec = vary_constrained([0., 0.], model, |p| [p.x + p.y - 10.]);
    let mut drag = DragSession::begin(
        Point::ORIGIN,
        (0., 0.),
        |p| Some(Affine::translate(p.to_vec2())),
        spec,
    )
    .unwrap();
    let f = drag.frame((20., 10.)).unwrap();
    close(f.drop_state.x, 10.);
    close(f.drop_state.y, 0.);
    assert!(f.drop_state.x + f.drop_state.y <= 10. + 1e-8);
}

#[test]
fn impossible_constraints_return_the_final_iterate_like_upstream() {
    let spec = vary_constrained([0.], |[x]| x, |_| [1.]);
    let mut drag =
        DragSession::begin(0., (0., 0.), |&x| Some(Affine::translate((x, 0.))), spec).unwrap();
    let frame = drag.frame((100., 10.)).unwrap();
    close(*frame.drop_state, 100.);
    assert!(frame.gap.is_finite());
    assert_eq!(drag.cancel(), 0.);
}
