//! The same model, layout and drag rules drive all three native examples.
use dragology::{DragSpec, Rect, Vec2, closest};
use dragology_scene::{Canvas, Diagram, Item, Quad};

pub const TITLE: &str = "Tower of Hanoi";
pub const WIDTH: f64 = 720.;
pub const HEIGHT: f64 = 360.;
pub const PAPER: [u8; 4] = [246, 244, 235, 255];
const DISKS: [[u8; 4]; 4] = [
    [221, 98, 61, 255],
    [237, 174, 64, 255],
    [65, 148, 138, 255],
    [47, 78, 100, 255],
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Id {
    Paper,
    Platform(usize),
    Foot(usize),
    Peg(usize),
    PegLight(usize),
    Disk(usize),
    GuideLeft,
    GuideRight,
    Goal,
}

#[derive(Clone, Default)]
pub struct Board {
    pegs: [usize; 4],
    landing: Option<usize>,
    moves: usize,
}

impl Board {
    pub fn status(&self) -> String {
        if self.pegs == [2; 4] {
            format!("Solved in {} moves · R to restart", self.moves)
        } else {
            format!(
                "{} moves · Move the stack to the right peg · R to restart",
                self.moves
            )
        }
    }
}
pub struct Hanoi;

fn selected(board: &Board, peg: usize) -> bool {
    board.landing.is_some_and(|disk| board.pegs[disk] == peg)
}

fn disk_rect(board: &Board, disk: usize) -> Rect {
    let peg = board.pegs[disk];
    let level = board.pegs[disk + 1..].iter().filter(|&&p| p == peg).count();
    let half = 33. + 17. * disk as f64;
    let x = 144. + 216. * peg as f64;
    let bottom = 280. - 32. * level as f64 - if selected(board, peg) { 7. } else { 0. };
    Rect::new(x - half, bottom - 30., x + half, bottom)
}

impl Diagram for Hanoi {
    type Id = Id;
    type State = Board;
    fn layout(&self, board: &Board) -> Vec<Item<Id>> {
        let mut items = Vec::new();
        let mut add = |id, bounds, radius, color| {
            items.push(Item {
                id,
                quad: Quad {
                    bounds,
                    radius,
                    color,
                },
            })
        };
        add(Id::Paper, Rect::new(0., 0., WIDTH, HEIGHT), 0., PAPER);
        for peg in 0..3 {
            let x = 144. + 216. * peg as f64;
            let active = selected(board, peg);
            let rise = if active { 7. } else { 0. };
            let wood = if active {
                DISKS[board.landing.unwrap()]
            } else {
                [169, 143, 110, 255]
            };
            add(
                Id::Foot(peg),
                Rect::new(x - 89., 290., x + 89., 304.),
                7.,
                [81, 62, 45, 20],
            );
            add(
                Id::Peg(peg),
                Rect::new(x - 5., 92. - rise, x + 5., 290. - rise),
                5.,
                wood,
            );
            add(
                Id::PegLight(peg),
                Rect::new(x - 3., 96. - rise, x - 1., 276. - rise),
                1.,
                [255, 255, 255, 75],
            );
            let half = if active { 94. } else { 88. };
            add(
                Id::Platform(peg),
                Rect::new(x - half, 280. - rise, x + half, 296. - rise),
                7.,
                [97, 75, 59, 255],
            );
        }
        // An inset in the right pedestal marks the goal and fills on completion.
        let solved = board.pegs == [2; 4];
        let goal_color = if solved {
            DISKS[2]
        } else {
            [191, 167, 136, 255]
        };
        let rise = if selected(board, 2) { 7. } else { 0. };
        add(
            Id::Goal,
            Rect::new(550., 286. - rise, 602., 289. - rise),
            1.5,
            goal_color,
        );
        if let Some(disk) = board.landing {
            let b = disk_rect(board, disk);
            let color = DISKS[disk];
            add(
                Id::GuideLeft,
                Rect::new(b.x0 - 8., b.y1 - 8., b.x0 - 5., b.y1),
                1.5,
                color,
            );
            add(
                Id::GuideRight,
                Rect::new(b.x1 + 5., b.y1 - 8., b.x1 + 8., b.y1),
                1.5,
                color,
            );
        }
        for disk in (0..4).rev() {
            add(Id::Disk(disk), disk_rect(board, disk), 12., DISKS[disk]);
        }
        items
    }

    fn paint(&self, item: &Item<Id>, dragging: bool) -> Vec<Quad> {
        let q = item.quad;
        let Id::Disk(_) = item.id else { return vec![q] };
        let b = q.bounds;
        let mut quads = Vec::with_capacity(9);
        let offset = if dragging { 13. } else { 3. };
        // Several translucent silhouettes soften the offset shadow on every backend.
        for spread in (0..4).rev() {
            let spread = f64::from(spread) * if dragging { 1.6 } else { 0.7 };
            quads.push(Quad {
                bounds: (b + Vec2::new(0., offset)).inflate(spread, spread),
                radius: q.radius + spread,
                color: [58, 44, 33, q.color[3] / 32],
            });
        }
        let [r, g, b_color, a] = q.color;
        quads.push(Quad {
            color: [
                (u16::from(r) * 4 / 5) as u8,
                (u16::from(g) * 4 / 5) as u8,
                (u16::from(b_color) * 4 / 5) as u8,
                a,
            ],
            ..q
        });
        quads.push(Quad {
            bounds: Rect::new(b.x0, b.y0, b.x1, b.y1 - 5.),
            ..q
        });
        quads.push(Quad {
            bounds: Rect::new(b.x0 + 10., b.y0 + 4., b.x1 - 10., b.y0 + 6.),
            radius: 1.,
            color: [255, 255, 255, a / 4],
        });
        quads
    }

    fn drag(&self, board: &Board, id: &Id) -> Option<DragSpec<'static, Board>> {
        let &Id::Disk(disk) = id else { return None };
        let smaller = &board.pegs[..disk];
        if smaller.contains(&board.pegs[disk]) {
            return None;
        }
        let origin = board.pegs[disk];
        let targets = (0..3).filter(|p| !smaller.contains(p)).map(|peg| {
            let mut target = board.clone();
            target.pegs[disk] = peg;
            target.landing = Some(disk);
            target
        });
        Some(
            closest(targets)
                .with_floating()
                .with_snap_radius(26.)
                .on_drop(move |s| Board {
                    landing: None,
                    moves: s.moves + usize::from(s.pegs[disk] != origin),
                    ..s.clone()
                }),
        )
    }
}

pub fn board() -> Canvas<Hanoi> {
    let mut canvas = Canvas::new(Hanoi, Board::default(), Vec2::new(WIDTH, HEIGHT));
    canvas.set_animated(!std::env::args().any(|arg| arg == "--reduced-motion"));
    canvas
}

#[cfg(test)]
mod tests {
    use super::*;
    use dragology_scene::{Point, Pointer};
    use std::time::Duration;

    #[test]
    fn only_top_disks_move_and_larger_disks_cannot_land_on_smaller_ones() {
        let mut canvas = board();
        let top = disk_rect(canvas.model(), 0).center();
        assert!(!canvas.handle(Pointer::Press(disk_rect(canvas.model(), 1).center())));
        canvas.handle(Pointer::Press(top));
        let hovering = Point::new(360., 110.);
        canvas.handle(Pointer::Move(hovering));
        assert!(matches!(canvas.items().last().unwrap().id, Id::Disk(0)));
        assert_eq!(
            canvas.items().last().unwrap().quad.bounds.center(),
            hovering
        );
        canvas.handle(Pointer::Release(Point::new(360., 265.)));
        assert_eq!(canvas.model().pegs, [1, 0, 0, 0]);
        assert!(canvas.model().landing.is_none());
        assert_eq!(canvas.model().moves, 1);
        canvas.advance(Duration::from_secs(1));
        canvas.handle(Pointer::Press(disk_rect(canvas.model(), 1).center()));
        canvas.handle(Pointer::Release(Point::new(360., 265.)));
        assert_ne!(canvas.model().pegs[1], 1);
    }

    #[test]
    fn snapping_and_cancellation_keep_moves_and_model_consistent() {
        let mut canvas = board();
        canvas.handle(Pointer::Press(disk_rect(canvas.model(), 0).center()));
        canvas.handle(Pointer::Move(Point::new(365., 260.)));
        let snapped = canvas.items().last().unwrap().quad.bounds.center();
        assert_eq!(snapped.x, 360.);
        assert_ne!(snapped, Point::new(365., 260.));
        canvas.handle(Pointer::Cancel);
        assert_eq!(canvas.model().pegs, [0; 4]);
        assert_eq!(canvas.model().moves, 0);
        canvas.advance(Duration::from_secs(1));
        assert_eq!(canvas.items(), Hanoi.layout(canvas.model()));
    }
}
