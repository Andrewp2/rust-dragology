use dragology_gallery::{Demo, Modifiers, Point, entries};
use std::collections::BTreeSet;

fn point(demo: &dyn Demo, id: &str) -> Point {
    demo.scene().object(id).unwrap().transform * Point::new(8., 8.)
}

fn unique_ids(demo: &dyn Demo) {
    let mut ids = BTreeSet::new();
    for object in &demo.scene().objects {
        assert!(ids.insert(&object.id), "duplicate object ID: {}", object.id);
    }
}

fn gesture(demo: &mut dyn Demo, id: &str, to: Point) {
    demo.press(point(demo, id), Modifiers::default(), 1);
    unique_ids(demo);
    demo.move_to(to, Modifiers::default());
    demo.advance(std::time::Duration::ZERO);
    unique_ids(demo);
    demo.release(to, Modifiers::default());
    unique_ids(demo);
}

// Keep this startup regression in its own test binary: other demos consuming
// IDs first used to hide collisions between the allocator and palette seeds.
#[test]
fn native_builder_constructs_removes_and_recovers_an_expression() {
    let entry = entries()
        .into_iter()
        .find(|entry| entry.id == "animate-algebra-stage-builder")
        .unwrap();
    let mut demo = (entry.create)();
    demo.set_animated(false);
    for _ in 0..2 {
        let d = demo.as_mut();
        let empty = point(d, "empty-stage");
        gesture(d, "brush-1", empty);
        assert!(point(d, "brush-1").distance(empty) < 1e-8);

        let hole = point(d, "brush-1-c0");
        gesture(d, "brush-8", hole);
        let hole = point(d, "brush-1-c1");
        gesture(d, "brush-9", hole);
        for id in ["brush-8", "brush-9"] {
            assert_eq!(
                d.scene().object(id).unwrap().parent.as_deref(),
                Some("brush-1")
            );
        }

        gesture(d, "brush-1", Point::new(780., 400.));
        assert!(
            d.scene()
                .object("brush-8")
                .is_none_or(|object| object.opacity == 0.)
        );
        let empty = point(d, "empty-stage");
        gesture(d, "brush-1", empty);
        for id in ["brush-8", "brush-9"] {
            assert_eq!(
                d.scene().object(id).unwrap().parent.as_deref(),
                Some("brush-1")
            );
        }
        d.reset();
        unique_ids(d);
    }
}
