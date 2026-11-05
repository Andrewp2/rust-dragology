//! A host may interpolate transforms rather than their grabbed endpoints.
//! Differential fixtures: `node tools/upstream-parity/generate-preview-geometry.mjs <upstream>`.
//! Source between/withFloating read that rendered geometry; closest/snap read
//! the operator cache first and render only when its cache was invalidated.
use dragology::*;
use std::{
    cell::Cell,
    f64::consts::{FRAC_PI_2, PI},
    rc::Rc,
};

#[derive(Clone, Debug, PartialEq)]
struct Pose {
    angle: f64,
    offset: Vec2,
}
impl Pose {
    fn angle(angle: f64) -> Self {
        Self {
            angle,
            offset: Vec2::ZERO,
        }
    }
    fn at(x: f64, y: f64) -> Self {
        Self {
            angle: 0.,
            offset: Vec2::new(x, y),
        }
    }
    fn transform(&self) -> Affine {
        Affine::translate(self.offset) * Affine::rotate(self.angle)
    }
}
fn locate(p: &Pose) -> Option<Affine> {
    Some(p.transform())
}
// A small renderer model: blend decomposed angles/translation, then apply the
// local anchor. This is deliberately different from blending endpoint points.
fn render(preview: &Preview<Pose>) -> Pose {
    match preview {
        Preview::State(p) => (**p).clone(),
        Preview::Blend(parts) => parts.iter().fold(Pose::angle(0.), |mut out, (p, w)| {
            let child = render(p);
            out.angle += child.angle * *w;
            out.offset += child.offset * *w;
            out
        }),
        Preview::Floating {
            source,
            translation,
            ..
        } => {
            let mut p = render(source);
            p.offset += *translation;
            p
        }
    }
}
fn geometry(preview: &Preview<Pose>, anchor: Point) -> Option<Point> {
    Some(render(preview).transform() * anchor)
}
fn close(a: Point, b: Point) {
    assert!(a.distance(b) < 1e-9, "{a:?} != {b:?}");
}
fn rotation() -> DragSpec<'static, Pose> {
    between([Pose::angle(0.), Pose::angle(2. * PI / 3.)])
}

#[test]
fn outer_between_measures_the_rendered_inner_rotation() {
    let spec = between([rotation(), fixed(Pose::angle(-FRAC_PI_2))]);
    let mut drag = DragSession::begin(Pose::angle(0.), (10., 0.), locate, spec)
        .unwrap()
        .with_preview_geometry(geometry);
    let frame = drag.frame(Point::ORIGIN).unwrap();
    // Inner between caches a chord midpoint, but its rendered60° rotation
    // stays on the10px circle. The other child is at(0,-10); weights are equal.
    let Preview::Blend(parts) = frame.preview.as_ref() else {
        panic!("outer interpolation missing")
    };
    assert_eq!(parts.len(), 2);
    assert!((parts[0].1 - 0.5).abs() < 1e-9);
    assert!((parts[1].1 - 0.5).abs() < 1e-9);
    assert!(
        parts
            .iter()
            .any(|(p, _)| matches!(p.as_ref(), Preview::Blend(_)))
    );
}

#[test]
fn floating_extracts_rendered_rotation_instead_of_its_cached_endpoint_average() {
    let mut drag = DragSession::begin(
        Pose::angle(0.),
        (10., 0.),
        locate,
        rotation().with_floating(),
    )
    .unwrap()
    .with_preview_geometry(geometry);
    let frame = drag.frame(Point::ORIGIN).unwrap();
    close(
        geometry(&frame.preview, Point::new(10., 0.)).unwrap(),
        Point::ORIGIN,
    );
    let Preview::Floating { translation, .. } = frame.preview.as_ref() else {
        panic!("not floating")
    };
    close(translation.to_point(), Point::new(-5., -5. * 3_f64.sqrt()));
}

#[derive(Clone)]
struct Panel {
    offset: Vec2,
    child: Pose,
}
fn parent_render(preview: &Preview<Panel>) -> Pose {
    match preview {
        Preview::State(p) => {
            let mut child = p.child.clone();
            child.offset += p.offset;
            child
        }
        Preview::Blend(parts) => parts.iter().fold(Pose::angle(0.), |mut out, (p, w)| {
            let child = parent_render(p);
            out.angle += child.angle * *w;
            out.offset += child.offset * *w;
            out
        }),
        Preview::Floating {
            source,
            translation,
            ..
        } => {
            let mut pose = parent_render(source);
            pose.offset += *translation;
            pose
        }
    }
}
#[test]
fn substate_lifts_preview_geometry_into_the_parent_layout() {
    let panel = Panel {
        offset: Vec2::new(70., 30.),
        child: Pose::angle(0.),
    };
    let spec = substate(panel.clone(), |p| &mut p.child, rotation().with_floating());
    let mut drag = DragSession::begin(
        panel,
        (80., 30.),
        |p| Some(Affine::translate(p.offset) * p.child.transform()),
        spec,
    )
    .unwrap()
    .with_preview_geometry(|p, anchor| Some(parent_render(p).transform() * anchor));
    let frame = drag.frame((70., 30.)).unwrap();
    close(
        parent_render(&frame.preview).transform() * Point::new(10., 0.),
        Point::new(70., 30.),
    );
    let Preview::Floating { translation, .. } = frame.preview.as_ref() else {
        panic!("not floating")
    };
    close(translation.to_point(), Point::new(-5., -5. * 3_f64.sqrt()));
}

fn replace_preview() -> DragSpec<'static, Pose> {
    fixed(Pose::at(0., 0.)).change_result(|_| DragResultPatch {
        preview: Some(Rc::new(Preview::State(Rc::new(Pose::at(100., 0.))))),
        ..Default::default()
    })
}
#[test]
fn changed_preview_invalidates_the_cache_for_snapping_and_branch_locking() {
    let mut snap = DragSession::begin(
        Pose::at(0., 0.),
        Point::ORIGIN,
        locate,
        replace_preview().with_snap_radius(5.),
    )
    .unwrap()
    .with_preview_geometry(geometry);
    let frame = snap.frame(Point::ORIGIN).unwrap();
    close(frame.grab_position.unwrap(), Point::new(100., 0.));
    close(
        geometry(&frame.preview, Point::ORIGIN).unwrap(),
        Point::new(100., 0.),
    );
    let options = ClosestOptions {
        lock_past: Some(20.),
        ..Default::default()
    };
    let mut locked = DragSession::begin(
        Pose::at(0., 0.),
        Point::ORIGIN,
        locate,
        closest_with([replace_preview(), fixed(Pose::at(30., 0.))], options),
    )
    .unwrap()
    .with_preview_geometry(geometry);
    locked.frame(Point::ORIGIN).unwrap();
    // The actual100px preview locked branch0, despite its drop model staying0.
    assert_eq!(
        *locked.frame((30., 0.)).unwrap().drop_state,
        Pose::at(0., 0.)
    );
}
#[test]
fn explicit_patch_cache_overrides_preview_and_omitted_fields_are_preserved() {
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    let spec = fixed(Pose::at(0., 0.))
        .with_chaining()
        .change_result(|_| DragResultPatch {
            preview: Some(Rc::new(Preview::State(Rc::new(Pose::at(100., 0.))))),
            grab_position: Some(Some(Point::ORIGIN)),
            ..Default::default()
        })
        .with_snap_radius(5.);
    let mut drag = DragSession::begin(Pose::at(0., 0.), Point::ORIGIN, locate, spec)
        .unwrap()
        .with_preview_geometry(move |p, a| {
            count.set(count.get() + 1);
            geometry(p, a)
        });
    let frame = drag.frame(Point::ORIGIN).unwrap();
    close(
        geometry(&frame.preview, Point::ORIGIN).unwrap(),
        Point::ORIGIN,
    );
    assert_eq!(calls.get(), 0, "snap should use explicit cached position");
    // Test patch preservation separately: snap itself owns chaining policy.
    let mut drag = DragSession::begin(
        Pose::at(0., 0.),
        Point::ORIGIN,
        locate,
        fixed(Pose::at(3., 0.))
            .with_chaining()
            .change_result(|_| DragResultPatch {
                gap: Some(7.),
                ..Default::default()
            }),
    )
    .unwrap();
    let frame = drag.frame(Point::ORIGIN).unwrap();
    assert_eq!(frame.gap, 7.);
    assert_eq!(frame.grab_position, Some(Point::new(3., 0.)));
    assert!(frame.chain.is_some());
}
#[test]
fn missing_rendered_element_is_authoritative_even_with_a_cached_grab() {
    let mut drag = DragSession::begin(
        Pose::angle(0.),
        (10., 0.),
        locate,
        between([Pose::angle(0.), Pose::angle(2. * PI / 3.)]),
    )
    .unwrap()
    .with_preview_geometry(|_, _| None);
    assert!(matches!(
        drag.frame(Point::ORIGIN),
        Err(DragError::MissingElement)
    ));
    let mut drag = DragSession::begin(Pose::angle(0.), (10., 0.), locate, rotation())
        .unwrap()
        .with_preview_geometry(|_, _| Some(Point::new(f64::NAN, 0.)));
    assert!(matches!(
        drag.frame(Point::ORIGIN),
        Err(DragError::InvalidTransform)
    ));
}

#[test]
fn nested_rotations_match_executed_upstream_rendered_traces() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/upstream-preview-geometry.json")).unwrap();
    assert_eq!(
        fixture["revision"],
        "b145bdd56530f7cfa9a61bf6a7170459d2f140e1"
    );
    let point = |v: &serde_json::Value| Point::new(v[0].as_f64().unwrap(), v[1].as_f64().unwrap());
    for case in fixture["cases"].as_array().unwrap() {
        let spec = match case["name"].as_str().unwrap() {
            "inner" => rotation(),
            "outer" => between([rotation(), fixed(Pose::angle(-FRAC_PI_2))]),
            "floating" => rotation().with_floating(),
            _ => unreachable!(),
        };
        let mut drag = DragSession::begin(Pose::angle(0.), (10., 0.), locate, spec)
            .unwrap()
            .with_preview_geometry(geometry);
        for expected in case["frames"].as_array().unwrap() {
            let actual = drag.frame(point(&expected["pointer"])).unwrap();
            close(
                geometry(&actual.preview, Point::new(10., 0.)).unwrap(),
                point(&expected["position"]),
            );
            close(actual.grab_position.unwrap(), point(&expected["grab"]));
            assert!(
                (actual.drop_state.angle.to_degrees()
                    - expected["drop"]["angle"].as_f64().unwrap())
                .abs()
                    < 1e-9
            );
            assert!((actual.gap - expected["gap"].as_f64().unwrap()).abs() < 1e-9);
        }
    }
}
#[test]
fn floating_keeps_last_rendered_appearance_when_callback_reports_absence() {
    let mut drag = DragSession::begin(
        Pose::at(0., 0.),
        Point::ORIGIN,
        locate,
        closest([Pose::at(10., 0.), Pose::at(100., 0.)]).with_floating(),
    )
    .unwrap()
    .with_preview_geometry(|p, a| {
        let pose = render(p);
        (pose.offset.x != 100.).then(|| pose.transform() * a)
    });
    drag.frame((10., 0.)).unwrap();
    let frame = drag.frame((100., 0.)).unwrap();
    let Preview::Floating { source, .. } = frame.preview.as_ref() else {
        panic!("not floating")
    };
    assert_eq!(render(source), Pose::at(10., 0.));
    close(
        geometry(&frame.preview, Point::ORIGIN).unwrap(),
        Point::new(100., 0.),
    );
}
