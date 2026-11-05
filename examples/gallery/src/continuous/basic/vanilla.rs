//! Direct port of multi-circle-points-vanilla.tsx's imperative event loop.
//! This example deliberately does not use Dragology, just like the original.
use super::*;
use std::time::Duration;

#[derive(Clone, Copy)]
enum Active {
    Circle(usize, Vec2),
    Point(usize),
}
impl Active {
    fn id(self) -> String {
        match self {
            Self::Circle(i, _) => format!("circle-{i}"),
            Self::Point(i) => format!("point-{i}"),
        }
    }
}
pub(super) struct VanillaCircles {
    state: CircleState,
    scene: Scene,
    active: Option<Active>,
    before: Option<CircleState>,
    offsets: [Vec2; 4],
    animated: bool,
    pointer: Option<Point>,
}
impl VanillaCircles {
    pub(super) fn new() -> Self {
        let state = Circles(true).initial();
        let scene = Circles(true).scene(&state);
        Self {
            state,
            scene,
            active: None,
            before: None,
            offsets: [Vec2::ZERO; 4],
            animated: true,
            pointer: None,
        }
    }
    fn rebuild(&mut self) {
        let id = self.active.map(Active::id);
        self.scene = Circles(true).scene_dragging(&self.state, id.as_deref());
        for i in 0..4 {
            if let Some(object) = self
                .scene
                .objects
                .iter_mut()
                .find(|o| o.id == format!("point-{i}"))
            {
                object.transform = Affine::translate(self.offsets[i]) * object.transform;
            }
        }
    }
}
impl Demo for VanillaCircles {
    fn scene(&self) -> &Scene {
        &self.scene
    }
    fn press(&mut self, p: Point, _: Modifiers, _: usize) {
        self.pointer = Some(p);
        let hit = self
            .scene
            .objects
            .iter()
            .rev()
            .find(|o| match &o.hit {
                Some(Hit::Circle(center, r)) => (o.transform.inverse() * p).distance(*center) <= *r,
                _ => false,
            })
            .map(|o| o.id.clone());
        let Some(id) = hit else { return };
        self.before = Some(self.state.clone());
        self.active = if let Some(i) = index(&id, "point-") {
            self.offsets[i] = Vec2::ZERO;
            Some(Active::Point(i))
        } else {
            index(&id, "circle-").map(|i| Active::Circle(i, p - self.state.centers[i]))
        };
        self.rebuild();
    }
    fn move_to(&mut self, p: Point, _: Modifiers) {
        self.pointer = Some(p);
        match self.active {
            Some(Active::Circle(i, offset)) => self.state.centers[i] = p - offset,
            Some(Active::Point(i)) => {
                let mut best = (f64::INFINITY, 0, Vec2::ZERO);
                for (parent, radius) in RADII.iter().enumerate() {
                    let mut d = p - self.state.centers[parent];
                    let limit = radius - 10.;
                    if d.hypot() > limit {
                        d *= limit / d.hypot();
                    }
                    let gap = (self.state.centers[parent] + d).distance(p);
                    if gap < best.0 {
                        best = (gap, parent, d);
                    }
                }
                let (old_parent, old_offset) = self.state.points[i];
                if best.1 != old_parent && self.animated {
                    self.offsets[i] += self.state.centers[old_parent] + old_offset
                        - (self.state.centers[best.1] + best.2);
                }
                self.state.points[i] = (best.1, best.2);
            }
            None => return,
        }
        self.rebuild();
    }
    // The original pointerup only ends the gesture; it does not process a move.
    fn release(&mut self, p: Point, _: Modifiers) {
        self.pointer = Some(p);
        self.active = None;
        self.before = None;
        self.rebuild();
    }
    fn cancel(&mut self) {
        if let Some(s) = self.before.take() {
            self.state = s;
        }
        self.active = None;
        self.offsets = [Vec2::ZERO; 4];
        self.rebuild();
    }
    fn reset(&mut self) {
        self.state = Circles(true).initial();
        self.active = None;
        self.before = None;
        self.offsets = [Vec2::ZERO; 4];
        self.rebuild();
    }
    fn controls(&self) -> Vec<Control> {
        vec![]
    }
    fn action(&mut self, _: &str, _: Value) {}
    fn key(&mut self, _: &str, _: Modifiers) {}
    fn status(&self) -> String {
        String::new()
    }
    fn cursor(&self) -> Cursor {
        if self.active.is_some() {
            Cursor::Grabbing
        } else if self.pointer.is_some_and(|p| {
            self.scene.objects.iter().any(|object| match object.hit {
                Some(Hit::Circle(center, radius)) => {
                    (object.transform.inverse() * p).distance(center) <= radius
                }
                _ => false,
            })
        }) {
            Cursor::Grab
        } else {
            Cursor::Default
        }
    }
    fn advance(&mut self, elapsed: Duration) -> bool {
        if self.offsets.iter().all(|d| *d == Vec2::ZERO) {
            return false;
        }
        let factor = 0.82_f64.powf(elapsed.as_secs_f64() * 1000. / 16.67);
        for d in &mut self.offsets {
            *d *= factor;
            if d.x.abs() < 0.5 && d.y.abs() < 0.5 {
                *d = Vec2::ZERO;
            }
        }
        self.rebuild();
        true
    }
    fn set_animated(&mut self, enabled: bool) {
        self.animated = enabled;
        if !enabled {
            self.offsets = [Vec2::ZERO; 4];
            self.rebuild();
        }
    }
    fn can_import(&self) -> bool {
        false
    }
    fn export(&self) -> Option<(String, String)> {
        None
    }
    fn import(&mut self, _: &str) -> Result<(), String> {
        Err("This demo does not import files.".into())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transfer_retains_visual_position_then_decays_as_in_source() {
        let mut d = VanillaCircles::new();
        let m = Modifiers::default();
        d.press(Point::new(120., 140.), m, 1);
        d.move_to(Point::new(370., 110.), m);
        assert_eq!(d.state.points[0].0, 1);
        assert_eq!(
            d.scene.object("point-0").unwrap().transform * Point::ORIGIN,
            Point::new(120., 140.)
        );
        d.advance(Duration::from_secs_f64(0.01667));
        assert!((d.offsets[0] - Vec2::new(-250., 30.) * 0.82).hypot() < 1e-9);
        d.release(Point::new(999., 999.), m);
        assert_eq!(d.state.points[0].1, Vec2::new(20., -10.));
        for _ in 0..100 {
            d.advance(Duration::from_secs_f64(0.01667));
        }
        assert_eq!(d.offsets[0], Vec2::ZERO);
    }
    #[test]
    fn point_press_discards_grab_offset_but_circle_press_preserves_it() {
        let mut d = VanillaCircles::new();
        let m = Modifiers::default();
        d.press(Point::new(125., 140.), m, 1);
        d.move_to(Point::new(130., 145.), m);
        assert_eq!(d.state.points[0].1, Vec2::new(30., -5.));
        d.release(Point::ORIGIN, m);
        d.press(Point::new(100., 100.), m, 1);
        d.move_to(Point::new(120., 120.), m);
        assert_eq!(d.state.centers[0], Point::new(120., 170.));
        assert_eq!(d.state.points[0].1, Vec2::new(30., -5.));
    }
}
