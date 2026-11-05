//! Fixtures execute the original TypeScript implementation at the pinned
//! revision. Regenerate with tools/upstream-parity/generate.mjs.
use dragology::*;
use std::{rc::Rc, time::Duration};

fn layout(p: &Point) -> Option<Affine> {
    Some(Affine::translate(p.to_vec2()))
}

fn preview_point(preview: &Preview<Point>, anchor: Point) -> Point {
    match preview {
        Preview::State(p) => *p.as_ref() + anchor.to_vec2(),
        Preview::Blend(parts) => parts
            .iter()
            .map(|(p, w)| preview_point(p, anchor).to_vec2() * *w)
            .fold(Vec2::ZERO, |a, b| a + b)
            .to_point(),
        Preview::Floating {
            source,
            translation,
            ..
        } => preview_point(source, anchor) + *translation,
    }
}

fn spec(name: &str) -> DragSpec<'static, Point> {
    let sites = [Point::ORIGIN, Point::new(100., 0.), Point::new(0., 100.)];
    match name {
        "fixed" => fixed(sites[1]),
        "closest" => closest(sites),
        "sticky" => closest_with(
            sites,
            ClosestOptions {
                stickiness: 15.,
                ..Default::default()
            },
        ),
        "locked" => closest_with(
            [between([sites[0], sites[1]]), between([sites[0], sites[2]])],
            ClosestOptions {
                lock_past: Some(20.),
                ..Default::default()
            },
        ),
        "between" => between(sites),
        "sharp" => between_with(
            sites,
            BetweenOptions {
                sharpness: 3.,
                ..Default::default()
            },
        ),
        "natural" => between_with(
            sites,
            BetweenOptions {
                interpolation: Interpolation::NaturalNeighbor,
                ..Default::default()
            },
        ),
        "far" => between(sites).when_far_with(Point::new(-50., -50.), 10., 30.),
        "snap" => between(sites).with_snap_radius(20.),
        "snapChain" => between(sites).with_snap_radius_options(
            20.,
            SnapOptions {
                transition: Some(Transition::default()),
                chain: true,
            },
        ),
        "floating" => closest(sites).with_floating(),
        "tether" => closest(sites).with_floating_options(FloatingOptions {
            ghost_opacity: Some(0.35),
            tether: Some(Box::new(|distance| distance.min(20.))),
        }),
        "during" => closest(sites).during(|p| *p + Vec2::new(0., 10.)),
        "onDrop" => closest(sites).on_drop(|p| *p + Vec2::new(0., 10.)),
        "gap" => closest(sites).change_gap(|gap| gap / 2.),
        "frame" => closest(sites).change_frame(|p| p + Vec2::new(20., -5.)),
        "chain" => closest(sites).with_chaining(),
        "vary" => vary([0.], |[x]| Point::new(x, 0.)),
        "constrained" => vary_constrained([0.], |[x]| Point::new(x, 0.), |p| [-p.x, p.x - 80.]),
        "coupled" => vary_constrained(
            [10., 20.],
            |[x, y]| Point::new(x, y),
            |p| [-p.x, -p.y, p.x + p.y - 80.],
        ),
        _ => panic!("unimplemented source fixture: {name}"),
    }
}

fn point(value: &serde_json::Value) -> Point {
    Point::new(value[0].as_f64().unwrap(), value[1].as_f64().unwrap())
}

#[test]
fn operators_match_executed_upstream_traces() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/upstream-operators.json")).unwrap();
    assert_eq!(
        fixture["revision"],
        "b145bdd56530f7cfa9a61bf6a7170459d2f140e1"
    );
    let anchor = Point::new(2., 3.);
    for case in fixture["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let tolerance = if matches!(name, "vary" | "constrained" | "coupled") {
            0.01
        } else {
            1e-8
        };
        let mut drag = DragSession::begin(Point::ORIGIN, anchor, layout, spec(name)).unwrap();
        for (i, expected) in case["frames"].as_array().unwrap().iter().enumerate() {
            let actual = drag.frame(point(&expected["pointer"])).unwrap();
            assert!(
                actual.drop_state.distance(point(&expected["drop"])) < tolerance,
                "{name} frame{i}: drop {:?} vs {}",
                actual.drop_state,
                expected["drop"]
            );
            let preview = preview_point(&actual.preview, anchor);
            assert!(
                preview.distance(point(&expected["preview"])) < tolerance,
                "{name} frame{i}: preview {preview:?} vs {}",
                expected["preview"]
            );
            if let Some(gap) = expected["gap"].as_f64() {
                assert!(
                    (actual.gap - gap).abs() < tolerance,
                    "{name} frame{i}: gap {} vs {gap}",
                    actual.gap
                );
            } else {
                assert!(actual.gap.is_infinite());
            }
            assert_eq!(
                actual.active_path,
                expected["path"].as_str().unwrap(),
                "{name} frame{i}"
            );
            assert_eq!(
                actual.chain.is_some(),
                expected["chain"].as_bool().unwrap(),
                "{name} frame{i}"
            );
        }
    }
}

#[test]
fn easing_matches_upstream_d3_curves_including_overshoot() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/upstream-operators.json")).unwrap();
    for row in fixture["easing"].as_array().unwrap() {
        for (name, easing) in [("cubic", Easing::CubicOut), ("elastic", Easing::ElasticOut)] {
            let transition = Transition {
                duration: Duration::from_secs(1),
                easing,
            };
            let actual = transition.progress(Duration::from_secs_f64(row["t"].as_f64().unwrap()));
            assert!(
                (actual - row[name].as_f64().unwrap()).abs() < 1e-12,
                "{name} at {}",
                row["t"]
            );
        }
    }
}

#[test]
fn reactive_operator_preserves_memory_until_value_changes() {
    use std::cell::Cell;
    let value = Rc::new(Cell::new(false));
    let builds = Rc::new(Cell::new(0));
    let getter = value.clone();
    let count = builds.clone();
    let spec = react_to(
        move || getter.get(),
        move |value| {
            count.set(count.get() + 1);
            let home = if *value { 100. } else { 0. };
            closest_with(
                [Point::new(home, 0.), Point::new(home + 10., 0.)],
                ClosestOptions {
                    stickiness: 3.,
                    ..Default::default()
                },
            )
        },
    );
    let mut drag = DragSession::begin(Point::ORIGIN, Point::ORIGIN, layout, spec).unwrap();
    assert_eq!(drag.frame((6., 0.)).unwrap().drop_state.x, 10.);
    assert_eq!(drag.frame((4., 0.)).unwrap().drop_state.x, 10.);
    assert_eq!(builds.get(), 1);
    value.set(true);
    assert_eq!(drag.frame((100., 0.)).unwrap().drop_state.x, 100.);
    assert_eq!(builds.get(), 2);
}
