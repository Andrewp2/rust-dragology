//! Exercise reported failures through the same hit testing and frame lifecycle as GPUI.
use dragology_gallery::{Demo, Modifiers, Point, Shape, entries};
use std::time::Duration;

fn open(id: &str, animated: bool) -> Box<dyn Demo> {
    let mut demo = (entries().into_iter().find(|e| e.id == id).unwrap().create)();
    demo.set_animated(animated);
    demo
}
fn position(demo: &dyn Demo, id: &str) -> Point {
    demo.scene().object(id).unwrap().transform * Point::ORIGIN
}
fn move_to(demo: &mut dyn Demo, point: Point) {
    demo.move_to(point, Modifiers::default());
    for _ in 0..20 {
        demo.advance(Duration::from_millis(16));
    }
}
fn release(demo: &mut dyn Demo, point: Point) {
    demo.release(point, Modifiers::default());
    demo.advance(Duration::from_secs(1));
}

#[test]
fn fifteen_tiles_and_blank_chain_through_turns_and_cancel() {
    for animated in [false, true] {
        let mut demo = open("fifteen", animated);
        let original = demo.scene().clone();
        // A numbered tile can enter the blank, but cannot move another tile.
        demo.press(Point::new(25., 125.), Modifiers::default(), 1);
        move_to(&mut *demo, Point::new(25., 175.));
        release(&mut *demo, Point::new(25., 175.));
        assert_eq!(position(&*demo, "tile-7"), Point::new(0., 150.));
        assert_eq!(position(&*demo, "tile-0"), Point::new(0., 100.));
        demo.reset();
        // One held drag follows the blank through multiple rows and columns.
        demo.press(Point::new(12., 188.), Modifiers::default(), 1);
        for cell in [
            (0., 2.),
            (0., 1.),
            (0., 0.),
            (1., 0.),
            (2., 0.),
            (2., 1.),
            (1., 1.),
        ] {
            move_to(
                &mut *demo,
                Point::new(cell.0 * 50. + 12., cell.1 * 50. + 38.),
            );
            assert_eq!(
                position(&*demo, "tile-0"),
                Point::new(cell.0 * 50., cell.1 * 50.)
            );
            let mut positions: Vec<_> = demo
                .scene()
                .objects
                .iter()
                .filter(|o| o.id.starts_with("tile-"))
                .map(|o| {
                    let p = o.transform * Point::ORIGIN;
                    (p.x as i32, p.y as i32)
                })
                .collect();
            positions.sort();
            positions.dedup();
            assert_eq!(positions.len(), 16, "two tiles occupy one cell");
        }
        demo.cancel();
        demo.advance(Duration::from_secs(1));
        assert_eq!(demo.scene(), &original);
        assert!(demo.status().is_empty(), "{}", demo.status());
    }
}

#[test]
fn sokoban_pushes_turns_blocks_walls_and_restores_cancelled_chain() {
    for animated in [false, true] {
        let mut demo = open("sokoban", animated);
        let original = demo.scene().clone();
        demo.press(Point::new(125., 125.), Modifiers::default(), 1);
        move_to(&mut *demo, Point::new(225., 125.));
        assert_eq!(position(&*demo, "player"), Point::new(200., 100.));
        assert_eq!(position(&*demo, "box-0"), Point::new(250., 100.));
        move_to(&mut *demo, Point::new(225., 75.));
        assert_eq!(position(&*demo, "player"), Point::new(200., 50.));
        move_to(&mut *demo, Point::new(225., 25.));
        assert_eq!(position(&*demo, "player"), Point::new(200., 50.));
        demo.cancel();
        demo.advance(Duration::from_secs(1));
        assert_eq!(demo.scene(), &original);
        demo.press(Point::new(125., 125.), Modifiers::default(), 1);
        move_to(&mut *demo, Point::new(175., 125.));
        release(&mut *demo, Point::new(175., 125.));
        assert_eq!(position(&*demo, "player"), Point::new(150., 100.));
        assert_eq!(position(&*demo, "box-0"), Point::new(200., 100.));
        assert!(demo.status().is_empty(), "{}", demo.status());
    }
}

#[test]
fn graph_plus_creates_followed_nodes_and_supports_cancel_and_delete() {
    for animated in [false, true] {
        let mut demo = open("graph", animated);
        let original = demo.scene().clone();
        demo.press(Point::new(40., 32.), Modifiers::default(), 1);
        move_to(&mut *demo, Point::new(200., 40.));
        release(&mut *demo, Point::new(200., 40.));
        assert!(
            position(&*demo, "node-5").distance(Point::new(195., 43.)) < 0.01,
            "{}",
            demo.status()
        );
        assert_eq!(demo.status(), "5 nodes · 10 directed edges");
        demo.press(Point::new(35., 35.), Modifiers::default(), 1);
        move_to(&mut *demo, Point::new(180., 100.));
        demo.cancel();
        demo.advance(Duration::from_secs(1));
        assert!(demo.scene().object("node-6").is_none());
        demo.press(Point::new(195., 43.), Modifiers::default(), 1);
        move_to(&mut *demo, Point::new(35., 35.));
        release(&mut *demo, Point::new(35., 35.));
        assert!(demo.scene().object("node-5").is_none());
        assert_eq!(demo.scene(), &original);
    }
}

#[test]
fn graph_plus_minus_transition_never_draws_a_connecting_diagonal() {
    let mut demo = open("graph", true);
    demo.press(Point::new(80., 100.), Modifiers::default(), 1);
    move_to(&mut *demo, Point::new(100., 120.));
    demo.release(Point::new(100., 120.), Modifiers::default());
    for _ in 0..16 {
        demo.advance(Duration::from_millis(16));
        for object in demo
            .scene()
            .objects
            .iter()
            .filter(|o| o.id == "create" || o.parent.as_deref() == Some("create"))
        {
            for shape in &object.shapes {
                if let Shape::Path { path, .. } = shape {
                    for segment in path.segments() {
                        let kurbo::PathSeg::Line(line) = segment else {
                            panic!("plus stroke became a curve")
                        };
                        assert!(
                            line.p0.x == line.p1.x || line.p0.y == line.p1.y,
                            "unexpected connecting stroke: {line:?}"
                        );
                    }
                }
            }
        }
    }
}
