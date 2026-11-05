//! Continuous motion references from the original Fifteen demo and frame lifecycle.
use dragology_gallery::{Modifiers, Point, entries};
use serde::Deserialize;
use std::time::Duration;

#[derive(Deserialize)]
struct Reference {
    revision: String,
    step_ms: u64,
    press: [f64; 2],
    paint_order: Vec<String>,
    frames: Vec<Frame>,
}
#[derive(Deserialize)]
struct Frame {
    pointer: [f64; 2],
    positions: [[f64; 2]; 16],
}

#[test]
fn fifteen_blank_matches_source_through_snap_exit_circles_and_backtracking() {
    let reference: Reference =
        serde_json::from_str(include_str!("fixtures/fifteen-frames.json")).unwrap();
    assert_eq!(
        reference.revision,
        "b145bdd56530f7cfa9a61bf6a7170459d2f140e1"
    );
    let mut demo = (entries()
        .into_iter()
        .find(|entry| entry.id == "fifteen")
        .unwrap()
        .create)();
    demo.set_animated(true);
    demo.press(
        Point::new(reference.press[0], reference.press[1]),
        Modifiers::default(),
        1,
    );
    for (frame_index, frame) in reference.frames.iter().enumerate() {
        demo.move_to(
            Point::new(frame.pointer[0], frame.pointer[1]),
            Modifiers::default(),
        );
        demo.advance(Duration::from_millis(reference.step_ms));
        assert!(
            demo.status().is_empty(),
            "frame {frame_index}: {}",
            demo.status()
        );
        for (id, expected) in frame.positions.iter().enumerate() {
            let actual = demo
                .scene()
                .object(&format!("tile-{id}"))
                .unwrap()
                .transform
                * Point::ORIGIN;
            assert!(
                actual.distance(Point::new(expected[0], expected[1])) < 1e-8,
                "frame {frame_index}, tile {id}, pointer {:?}: expected {expected:?}, got {actual:?}",
                frame.pointer
            );
        }
        let order: Vec<_> = demo
            .scene()
            .objects
            .iter()
            .filter(|object| object.id.starts_with("tile-"))
            .map(|object| object.id.clone())
            .collect();
        assert_eq!(
            order, reference.paint_order,
            "paint order at frame {frame_index}"
        );
    }
}
