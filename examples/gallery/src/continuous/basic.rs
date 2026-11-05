use super::*;
use dragology::{closest, vary, vary_constrained};
mod vanilla;

pub(super) fn entries() -> Vec<Entry> {
    vec![
        Entry::new("angle", "angle", "", || demo(Angle(false))),
        Entry::new("angle-via-transform", "angle-via-transform", "", || {
            demo(Angle(true))
        }),
        Entry::new("stretchy-xy", "stretchy-xy", "", || demo(Stretch(false))),
        Entry::new("stretchy-rot", "stretchy-rot", "", || demo(Stretch(true))),
        Entry::new("clock", "clock", "", || demo(Clock)),
        Entry::new("constrained-slider", "constrained-slider", "", || {
            demo(ConstrainedPoint(Region::Slider))
        }),
        Entry::new("point-in-circle", "point-in-circle", "", || {
            demo(ConstrainedPoint(Region::Disk))
        }),
        Entry::new("point-in-polygon", "point-in-polygon", "", || {
            demo(ConstrainedPoint(Region::Polygon))
        }),
        Entry::new(
            "point-on-circle-via-constraint",
            "point-on-circle-via-constraint",
            "This demo parameterizes a point's position using x and y coordinates and then constrains it to be on the circle using an equal constraint. (You could alternatively parameterize it with an angle, in which case you wouldn't need constraints)",
            || demo(ConstrainedPoint(Region::Circle)),
        ),
        Entry::new(
            "point-on-curve",
            "point-on-curve",
            "Point constrained to a Cassini oval (a=1.1, c=1) using an equal constraint on the product of distances to two foci. (The visible curve is plotted using a polar parameterization, which kind of ruins the fun of using the constraint solver here.)\n\nNote a strange asymmetry when dragging from the upper concavity to the lower concavity and back again.",
            || demo(ConstrainedPoint(Region::Cassini)),
        ),
        Entry::new("orbiting-planet", "orbiting-planet", "", || {
            demo(Orbits(false))
        }),
        Entry::new(
            "orbiting-planet-with-background",
            "orbiting-planet-with-background",
            "",
            || demo(Orbits(true)),
        ),
        Entry::new("multi-circle-points", "multi-circle-points", "", || {
            demo(Circles(false))
        }),
        Entry::new(
            "multi-circle-points-vanilla",
            "multi-circle-points-vanilla",
            "From-scratch React reimplementation of multi-circle-points, without using Dragology.",
            || Box::new(vanilla::VanillaCircles::new()),
        ),
    ]
}

struct Angle(bool);
impl Example for Angle {
    type State = f64;
    fn initial(&self) -> f64 {
        0.
    }
    fn scene(&self, angle: &f64) -> Scene {
        let mut scene = Scene::new(250., 250.);
        let center = Point::new(100., 100.);
        let end = center + polar(100., angle.to_radians());
        let line = if self.0 {
            Object::line("arm", Point::ORIGIN, (100., 0.), rgb(0), 4.).transforms(vec![
                TransformComponent::Translate(center.to_vec2()),
                TransformComponent::Rotate {
                    radians: angle.to_radians(),
                    center: Point::ORIGIN,
                },
            ])
        } else {
            Object::line("arm", center, end, rgb(0), 4.)
        };
        if self.0 {
            scene.push(line.clone());
        }
        let mut knob = Object::circle("knob", end, 20., rgb(0));
        knob.transform = self.locate(angle, "knob").unwrap();
        if self.0 {
            knob = knob.transforms(vec![
                TransformComponent::Translate(center.to_vec2()),
                TransformComponent::Rotate {
                    radians: angle.to_radians(),
                    center: Point::ORIGIN,
                },
                TransformComponent::Translate(Vec2::new(100., 0.)),
            ]);
        }
        scene.push(knob);
        if !self.0 {
            scene.push(line);
        }
        source_cursors(scene)
    }
    fn locate(&self, angle: &f64, id: &str) -> Option<Affine> {
        if id != "knob" {
            return None;
        }
        Some(if self.0 {
            Affine::translate((100., 100.))
                * Affine::rotate(angle.to_radians())
                * Affine::translate((100., 0.))
        } else {
            Affine::translate((Point::new(100., 100.) + polar(100., angle.to_radians())).to_vec2())
        })
    }
    fn drag(&self, state: &f64, id: &str, _: Modifiers) -> Option<DragSpec<'static, f64>> {
        (id == "knob").then(|| vary([*state], |[angle]| angle))
    }
}

struct Stretch(bool);
impl Example for Stretch {
    type State = [f64; 2];
    fn initial(&self) -> Self::State {
        if self.0 { [0., 1.] } else { [1., 1.] }
    }
    fn scene(&self, s: &Self::State) -> Scene {
        let mut scene = Scene::new(250., 250.);
        let mut components = vec![TransformComponent::Translate(Vec2::new(100., 100.))];
        if self.0 {
            components.push(TransformComponent::Rotate {
                radians: s[0].to_radians(),
                center: Point::ORIGIN,
            });
        }
        components.push(TransformComponent::Scale(if self.0 {
            Vec2::new(s[1], 1. / s[1])
        } else {
            Vec2::new(s[0], s[1])
        }));
        let ellipse = Object::circle("ellipse", Point::ORIGIN, 50., [173, 216, 230, 255])
            .transforms(components);
        scene.push(ellipse);
        if !self.0 && s[0] != 0. && s[1] != 0. {
            // Upstream draws this as a separate SVG ellipse, so the stroke
            // stays four screen units wide as either radius changes.
            let outline =
                kurbo::Ellipse::new((100., 100.), (50. * s[0].abs(), 50. * s[1].abs()), 0.)
                    .to_path(0.01);
            scene.push(Object::path("outline", outline, rgb(0), 4.));
        }
        source_cursors(scene)
    }
    fn locate(&self, s: &Self::State, id: &str) -> Option<Affine> {
        if id != "ellipse" {
            return None;
        }
        let scale = if self.0 {
            Affine::rotate(s[0].to_radians()) * Affine::scale_non_uniform(s[1], 1. / s[1])
        } else {
            Affine::scale_non_uniform(s[0], s[1])
        };
        Some(Affine::translate((100., 100.)) * scale)
    }
    fn drag(
        &self,
        s: &Self::State,
        id: &str,
        _: Modifiers,
    ) -> Option<DragSpec<'static, Self::State>> {
        if id != "ellipse" {
            return None;
        }
        Some(vary(*s, |p| p))
    }
}

struct Clock;
impl Clock {
    fn hand(hours: f64, i: usize) -> Affine {
        let (rate, len) = [(TAU / 12., 60.), (TAU, 80.), (TAU * 60., 90.)][i];
        Affine::translate((100., 100.))
            * Affine::rotate(hours * rate - TAU / 4.)
            * Affine::translate((len, 0.))
    }
}
impl Example for Clock {
    type State = f64;
    fn initial(&self) -> f64 {
        4.333333
    }
    fn scene(&self, s: &f64) -> Scene {
        let mut scene = Scene::new(250., 250.);
        for i in 0..3 {
            let (rate, len) = [(TAU / 12., 60.), (TAU, 80.), (TAU * 60., 90.)][i];
            let mut components = vec![
                TransformComponent::Translate(Vec2::new(100., 100.)),
                TransformComponent::Rotate {
                    radians: *s * rate - TAU / 4.,
                    center: Point::ORIGIN,
                },
            ];
            scene.push(
                Object::line(
                    format!("line-{i}"),
                    Point::ORIGIN,
                    (len, 0.),
                    rgb(0),
                    [6., 4., 2.][i],
                )
                .transforms(components.clone()),
            );
            components.push(TransformComponent::Translate(Vec2::new(len, 0.)));
            scene.push(
                Object::circle(format!("hand-{i}"), Point::ORIGIN, 10., rgb(0))
                    .transforms(components),
            );
        }
        source_cursors(scene)
    }
    fn locate(&self, s: &f64, id: &str) -> Option<Affine> {
        index(id, "hand-")
            .filter(|i| *i < 3)
            .map(|i| Self::hand(*s, i))
    }
    fn drag(&self, s: &f64, id: &str, _: Modifiers) -> Option<DragSpec<'static, f64>> {
        index(id, "hand-").map(|_| vary([*s], |[h]| h))
    }
}

#[derive(Clone, Copy)]
enum Region {
    Slider,
    Disk,
    Polygon,
    Circle,
    Cassini,
}
struct ConstrainedPoint(Region);
fn vertices() -> [Point; 5] {
    std::array::from_fn(|i| Point::new(150., 150.) + polar(100., i as f64 * TAU / 5. - TAU / 4.))
}
impl Example for ConstrainedPoint {
    type State = Point;
    fn initial(&self) -> Point {
        match self.0 {
            Region::Slider => Point::new(150., 50.),
            Region::Circle => Point::new(250., 150.),
            Region::Cassini => Point::new(200. + 22100_f64.sqrt(), 150.),
            _ => Point::new(150., 150.),
        }
    }
    fn scene(&self, s: &Point) -> Scene {
        let mut scene = match self.0 {
            Region::Slider => Scene::new(300., 100.),
            Region::Cassini => Scene::new(400., 300.),
            _ => Scene::new(300., 300.),
        };
        match self.0 {
            Region::Slider => {
                scene.push(
                    Object::line("track", (20., 50.), (280., 50.), rgb(0xcccccc), 4.)
                        .line_cap(LineCap::Round),
                );
                scene.push(
                    Object::line("range", (72., 50.), (228., 50.), rgb(0x44aa99), 4.)
                        .line_cap(LineCap::Round),
                );
                for (i, (x, y, label, size, color)) in [
                    (20., 75., "0", 12., rgb(0x999999)),
                    (280., 75., "100", 12., rgb(0x999999)),
                    (72., 35., "20", 11., rgb(0x44aa99)),
                    (228., 35., "80", 11., rgb(0x44aa99)),
                ]
                .into_iter()
                .enumerate()
                {
                    scene.push(
                        Object::text(format!("tick-{i}"), (x, y), label, size, color)
                            .text_anchor(TextAnchor::Middle)
                            .text_baseline(TextBaseline::Alphabetic),
                    );
                }
            }
            Region::Disk | Region::Circle => scene.push(circle_outline(
                "boundary",
                Point::new(150., 150.),
                100.,
                rgb(0xcccccc),
                2.,
                &[6., 4.],
            )),
            Region::Polygon => scene.push(dashed_path(
                "boundary",
                path(vertices(), true),
                rgb(0xcccccc),
                2.,
                &[6., 4.],
            )),
            Region::Cassini => {
                let points = (0..200).map(|i| {
                    let a = i as f64 * TAU / 200.;
                    let r = (10000. * (2. * a).cos()
                        + (110_f64.powi(4) - 100_f64.powi(4) * (2. * a).sin().powi(2)).sqrt())
                    .max(0.)
                    .sqrt();
                    Point::new(200., 150.) + polar(r, a)
                });
                scene.push(dashed_path(
                    "boundary",
                    path(points, true),
                    rgb(0xcccccc),
                    2.,
                    &[6., 4.],
                ));
            }
        }
        scene.push(Object::circle(
            "point",
            *s,
            if matches!(self.0, Region::Slider) {
                12.
            } else {
                14.
            },
            rgb(0),
        ));
        if matches!(self.0, Region::Slider) {
            scene.push(
                Object::text(
                    "value",
                    (s.x, 90.),
                    format!("{:.0}", ((s.x - 20.) / 2.6 + 0.5).floor()),
                    13.,
                    rgb(0),
                )
                .text_anchor(TextAnchor::Middle)
                .text_baseline(TextBaseline::Alphabetic),
            );
        }
        source_cursors(scene)
    }
    fn locate(&self, s: &Point, id: &str) -> Option<Affine> {
        (id == "point").then(|| Affine::translate(s.to_vec2()))
    }
    fn drag(&self, s: &Point, id: &str, _: Modifiers) -> Option<DragSpec<'static, Point>> {
        if id != "point" {
            return None;
        }
        let initial = [s.x, s.y];
        Some(match self.0 {
            Region::Slider => vary_constrained(
                [s.x],
                |[x]| Point::new(x, 50.),
                |p: &Point| [72. - p.x, p.x - 228.],
            ),
            Region::Disk => vary_constrained(
                initial,
                |[x, y]| Point::new(x, y),
                |p: &Point| [p.distance_squared(Point::new(150., 150.)) - 10000.],
            ),
            Region::Circle => vary_constrained(
                initial,
                |[x, y]| Point::new(x, y),
                |p: &Point| {
                    let residual = p.distance_squared(Point::new(150., 150.)) - 10000.;
                    [residual, -residual]
                },
            ),
            Region::Polygon => vary_constrained(
                initial,
                |[x, y]| Point::new(x, y),
                |p: &Point| {
                    let v = vertices();
                    std::array::from_fn::<_, 5, _>(|i| -(v[(i + 1) % 5] - v[i]).cross(*p - v[i]))
                },
            ),
            Region::Cassini => vary_constrained(
                initial,
                |[x, y]| Point::new(x, y),
                |p: &Point| {
                    let residual = p.distance(Point::new(100., 150.))
                        * p.distance(Point::new(300., 150.))
                        - 12100.;
                    [residual, -residual]
                },
            ),
        })
    }
}

const STARS: [Point; 3] = [
    Point::new(100., 150.),
    Point::new(300., 100.),
    Point::new(250., 280.),
];
#[derive(Clone, PartialEq)]
enum OrbitState {
    Orbit { star: usize, angle: f64 },
    Free(Point),
}
impl OrbitState {
    fn point(&self) -> Point {
        match *self {
            Self::Orbit { star, angle } => STARS[star] + polar(60., angle),
            Self::Free(point) => point,
        }
    }
}
struct Orbits(bool);
impl Example for Orbits {
    type State = OrbitState;
    fn initial(&self) -> OrbitState {
        OrbitState::Orbit { star: 0, angle: 0. }
    }
    fn scene(&self, s: &OrbitState) -> Scene {
        let mut scene = Scene::new(450., 400.);
        for (i, p) in STARS.iter().enumerate() {
            scene.push(circle_outline(
                format!("orbit-{i}"),
                *p,
                60.,
                rgb(0xc0c0c0),
                1.,
                &[4., 4.],
            ));
        }
        for (i, p) in STARS.iter().enumerate() {
            scene.push(
                Object::circle(
                    format!("star-{i}"),
                    *p,
                    14.,
                    [rgb(0xe8b730), rgb(0xe05050), rgb(0x4080e0)][i],
                )
                .label(["A", "B", "C"][i], (0., 0.), 12., [255; 4])
                .text_anchor(TextAnchor::Middle)
                .text_baseline(TextBaseline::Middle)
                .font_weight(700)
                .decorative(),
            );
        }
        scene.push(stroked_circle(
            "planet",
            s.point(),
            8.,
            if matches!(s, OrbitState::Free(_)) {
                rgb(0x888888)
            } else {
                rgb(0x333333)
            },
            rgb(0x666666),
            1.,
        ));
        source_cursors(scene)
    }
    fn locate(&self, s: &OrbitState, id: &str) -> Option<Affine> {
        (id == "planet").then(|| Affine::translate(s.point().to_vec2()))
    }
    fn drag(
        &self,
        s: &OrbitState,
        id: &str,
        _: Modifiers,
    ) -> Option<DragSpec<'static, OrbitState>> {
        if id != "planet" {
            return None;
        }
        let angle = match *s {
            OrbitState::Orbit { angle, .. } => angle,
            _ => 0.,
        };
        let spec = closest(
            (0..3).map(|star| vary([angle], move |[angle]| OrbitState::Orbit { star, angle })),
        );
        let p = s.point();
        Some(if self.0 {
            spec.when_far(
                vary([p.x, p.y], |[x, y]| OrbitState::Free(Point::new(x, y))),
                50.,
            )
        } else {
            spec
        })
    }
}

#[derive(Clone, PartialEq)]
struct CircleState {
    centers: [Point; 3],
    points: [(usize, Vec2); 4],
}
const RADII: [f64; 3] = [70., 60., 80.];
struct Circles(bool);
impl Example for Circles {
    type State = CircleState;
    fn initial(&self) -> CircleState {
        CircleState {
            centers: [
                Point::new(100., 150.),
                Point::new(350., 120.),
                Point::new(250., 250.),
            ],
            points: [
                (0, Vec2::new(20., -10.)),
                (0, Vec2::new(-30., 20.)),
                (1, Vec2::new(0., 15.)),
                (2, Vec2::new(25., -25.)),
            ],
        }
    }
    fn scene(&self, s: &CircleState) -> Scene {
        self.scene_dragging(s, None)
    }
    fn scene_dragging(&self, s: &CircleState, dragged: Option<&str>) -> Scene {
        let mut scene = Scene::new(600., 350.);
        let colors = [rgb(0xe57373), rgb(0x64b5f6), rgb(0x81c784)];
        let mut circles = (0..3).collect::<Vec<_>>();
        if !self.0 {
            circles.sort_by_key(|i| usize::from(dragged == Some(format!("circle-{i}").as_str())));
        }
        for i in circles {
            let c = colors[i];
            let id = format!("circle-{i}");
            let active = dragged == Some(id.as_str());
            let mut circle = Object::circle(id, s.centers[i], RADII[i], [c[0], c[1], c[2], 32]);
            let outline = circle_outline(
                "outline",
                Point::ORIGIN,
                RADII[i],
                c,
                if active { 3. } else { 2. },
                if active { &[] } else { &[6., 4.] },
            );
            circle.shapes.extend(outline.shapes);
            scene.push(circle);
        }
        let mut points = (0..4).collect::<Vec<_>>();
        points.sort_by_key(|i| usize::from(dragged == Some(format!("point-{i}").as_str())));
        for i in points {
            let (parent, offset) = s.points[i];
            scene.push(stroked_circle(
                format!("point-{i}"),
                s.centers[parent] + offset,
                10.,
                colors[parent],
                [255; 4],
                2.,
            ));
        }
        source_cursors(scene)
    }
    fn locate(&self, s: &CircleState, id: &str) -> Option<Affine> {
        let p = if let Some(i) = index(id, "circle-") {
            *s.centers.get(i)?
        } else {
            let i = index(id, "point-")?;
            let (parent, offset) = s.points.get(i)?;
            s.centers[*parent] + *offset
        };
        Some(Affine::translate(p.to_vec2()))
    }
    fn drag(
        &self,
        s: &CircleState,
        id: &str,
        _: Modifiers,
    ) -> Option<DragSpec<'static, CircleState>> {
        if let Some(i) = index(id, "circle-") {
            let start = s.clone();
            let p = *s.centers.get(i)?;
            return Some(vary([p.x, p.y], move |[x, y]| {
                let mut next = start.clone();
                next.centers[i] = Point::new(x, y);
                next
            }));
        }
        let i = index(id, "point-")?;
        s.points.get(i)?;
        let start = s.clone();
        Some(closest((0..3).map(move |parent| {
            let mut base = start.clone();
            let offset = if base.points[i].0 == parent {
                base.points[i].1
            } else {
                Vec2::ZERO
            };
            base.points[i] = (parent, offset);
            let radius = RADII[parent] - 10.;
            vary_constrained(
                [offset.x, offset.y],
                move |[x, y]| {
                    let mut next = base.clone();
                    next.points[i].1 = Vec2::new(x, y);
                    next
                },
                move |next: &CircleState| [next.points[i].1.hypot2() - radius * radius],
            )
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dragology::DragSession;

    #[test]
    fn negative_stretch_keeps_the_outline_around_the_reflected_ellipse() {
        for scales in [[-2., 0.5], [2., -0.5], [-2., -0.5]] {
            let scene = Stretch(false).scene(&scales);
            let outline = scene.objects.iter().find(|o| o.id == "outline").unwrap();
            let Shape::Path { path, stroke, .. } = &outline.shapes[0] else {
                panic!("ellipse outline missing")
            };
            let bounds = path.bounding_box();
            // The path approximates the exact ellipse to 0.01 scene units.
            assert!(bounds.origin().distance(Point::new(0., 75.)) < 0.01);
            assert!(Point::new(bounds.x1, bounds.y1).distance(Point::new(200., 125.)) < 0.01);
            assert_eq!(*stroke, Some((rgb(0), 4.)));
        }
    }

    fn move_point(region: Region, target: Point) -> Point {
        let example = ConstrainedPoint(region);
        let state = example.initial();
        let spec = example.drag(&state, "point", Modifiers::default()).unwrap();
        DragSession::begin(
            state,
            state,
            |p: &Point| Some(Affine::translate(p.to_vec2())),
            spec,
        )
        .unwrap()
        .finish(target)
        .unwrap()
    }

    #[test]
    fn constrained_drag_reaches_boundary_and_stays_feasible() {
        let center = Point::new(150., 150.);
        let disk = move_point(Region::Disk, Point::new(290., 230.));
        assert!((disk.distance(center) - 100.).abs() < 0.1);
        let polygon = move_point(Region::Polygon, Point::new(290., 230.));
        let v = vertices();
        for i in 0..5 {
            assert!((v[(i + 1) % 5] - v[i]).cross(polygon - v[i]) >= -1e-6);
        }
        assert!(polygon.distance(center) > 70.);
        let circle = move_point(Region::Circle, Point::new(150., 250.));
        assert!((circle.distance(center) - 100.).abs() < 0.001);
        assert!(circle.y > 225., "circle stuck at {circle:?}");
        let cassini = move_point(Region::Cassini, Point::new(200., 200.));
        assert!(
            (cassini.distance(Point::new(100., 150.)) * cassini.distance(Point::new(300., 150.))
                - 12100.)
                .abs()
                < 0.001
        );
        assert!(cassini.x < 260., "Cassini oval stuck at {cassini:?}");
        assert!((move_point(Region::Slider, Point::new(0., 0.)).x - 72.).abs() < 0.001);
    }

    #[test]
    fn declarative_points_transfer_to_the_nearest_circle() {
        {
            let example = Circles(false);
            let state = example.initial();
            let spec = example
                .drag(&state, "point-0", Modifiers::default())
                .unwrap();
            let press = example.locate(&state, "point-0").unwrap() * Point::ORIGIN;
            let moved =
                DragSession::begin(state, press, move |s| example.locate(s, "point-0"), spec)
                    .unwrap()
                    .finish(Point::new(370., 110.))
                    .unwrap();
            assert_eq!(moved.points[0].0, 1);
            assert!((moved.points[0].1 - Vec2::new(20., -10.)).hypot() < 0.1);
        }
    }
}
