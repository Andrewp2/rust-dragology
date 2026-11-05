use super::*;
use dragology::{vary, vary_constrained};
mod face;

pub(super) fn entries() -> Vec<Entry> {
    vec![
        Entry::new(
            "bezier",
            "bezier",
            "Drag the endpoints (red) or control points (yellow) orrrrr the curve (??). – Orion Reed",
            || demo(Bezier),
        ),
        Entry::new(
            "rectangle",
            "rectangle",
            "Your basic move-and-resize rectangle. Implementation is a little funky though: Every resize handle controls all four parameters (x, y, w, h), but with appropriate pin constraints to control its effects.",
            || demo(Resizable),
        ),
        Entry::new(
            "dragon",
            "dragon",
            "Adapted from g9's famous example. Nice performance stress test.",
            || demo(Dragon),
        ),
        Entry::new("scene-3d", "scene-3d", "", || demo(Scene3d)),
        Entry::new(
            "face-creator",
            "face-creator",
            "Drag eyes to move/space them. Drag the mouth curve or endpoints. Drag the face outline to reshape. Features push each other.",
            || demo(face::Face),
        ),
    ]
}

fn cubic(points: [Point; 4], t: f64) -> Point {
    let u = 1. - t;
    (points[0].to_vec2() * u.powi(3)
        + points[1].to_vec2() * (3. * u * u * t)
        + points[2].to_vec2() * (3. * u * t * t)
        + points[3].to_vec2() * t.powi(3))
    .to_point()
}
fn cubic_path(points: [Point; 4]) -> BezPath {
    let mut path = BezPath::new();
    path.move_to(points[0]);
    path.curve_to(points[1], points[2], points[3]);
    path
}
struct Bezier;
impl Example for Bezier {
    type State = [Point; 4];
    fn initial(&self) -> Self::State {
        [
            Point::new(50., 50.),
            Point::new(200., 50.),
            Point::new(100., 200.),
            Point::new(300., 200.),
        ]
    }
    fn scene(&self, s: &Self::State) -> Scene {
        self.scene_dragging(s, None)
    }
    fn scene_dragging(&self, s: &Self::State, dragged: Option<&str>) -> Scene {
        let mut scene = Scene::new(400., 250.);
        scene.push(Object::line("guide-0", s[0], s[1], rgb(0xcccccc), 1.).dash(vec![4., 4.]));
        scene.push(Object::line("guide-1", s[3], s[2], rgb(0xcccccc), 1.).dash(vec![4., 4.]));
        scene.push(
            Object::path("curve", cubic_path(*s), rgb(0x1e40af), 6.).line_cap(LineCap::Round),
        );
        for i in 1..10 {
            let id = format!("curve-{i}");
            let active = dragged == Some(id.as_str());
            scene.push(Object::circle(
                id,
                cubic(*s, i as f64 / 10.),
                if active { 8. } else { 12. },
                if active { [37, 99, 235, 77] } else { [0; 4] },
            ));
        }
        for i in [1, 2, 0, 3] {
            let id = format!("point-{i}");
            let active = dragged == Some(id.as_str());
            let endpoint = i == 0 || i == 3;
            scene.push(stroked_circle(
                id,
                s[i],
                if endpoint { 12. } else { 8. },
                rgb(if endpoint {
                    if active { 0xdc2626 } else { 0xef4444 }
                } else {
                    if active { 0xf59e0b } else { 0xfbbf24 }
                }),
                rgb(if endpoint { 0x7f1d1d } else { 0x92400e }),
                2.,
            ));
        }
        for i in 0..4 {
            scene.push(
                Object::text(
                    format!("label-{i}"),
                    s[i] + Vec2::new(0., -if i == 0 || i == 3 { 17. } else { 13. }),
                    format!("P{i}"),
                    12.,
                    rgb(0x666666),
                )
                .text_anchor(TextAnchor::Middle)
                .text_baseline(TextBaseline::Alphabetic),
            );
        }
        source_cursors(scene)
    }
    fn locate(&self, s: &Self::State, id: &str) -> Option<Affine> {
        let p = if let Some(i) = index(id, "point-") {
            *s.get(i)?
        } else {
            cubic(*s, index(id, "curve-")? as f64 / 10.)
        };
        Some(Affine::translate(p.to_vec2()))
    }
    fn drag(
        &self,
        s: &Self::State,
        id: &str,
        _: Modifiers,
    ) -> Option<DragSpec<'static, Self::State>> {
        let base = *s;
        if let Some(i) = index(id, "point-") {
            let p = *s.get(i)?;
            return Some(vary([p.x, p.y], move |[x, y]| {
                let mut next = base;
                next[i] = Point::new(x, y);
                next
            }));
        }
        index(id, "curve-")?;
        // Translating both controls together pins their relative displacement.
        Some(vary([0., 0.], move |[dx, dy]| {
            let mut next = base;
            next[1] += Vec2::new(dx, dy);
            next[2] += Vec2::new(dx, dy);
            next
        }))
    }
}

struct Resizable;
#[derive(Clone, PartialEq)]
struct Rectangle {
    left: f64,
    top: f64,
    width: f64,
    height: f64,
}
fn rect_anchor(s: &Rectangle, id: &str) -> Option<Point> {
    Some(match id {
        "body" => Point::new(s.left, s.top),
        "top" => Point::new(s.left + s.width / 2., s.top),
        "bottom" => Point::new(s.left + s.width / 2., s.top + s.height),
        "left" => Point::new(s.left, s.top + s.height / 2.),
        "right" => Point::new(s.left + s.width, s.top + s.height / 2.),
        "tl" => Point::new(s.left, s.top),
        "tr" => Point::new(s.left + s.width, s.top),
        "bl" => Point::new(s.left, s.top + s.height),
        "br" => Point::new(s.left + s.width, s.top + s.height),
        _ => return None,
    })
}
impl Example for Resizable {
    type State = Rectangle;
    fn hover_transition(&self, _: &Rectangle) -> Option<dragology::Transition> {
        Some(dragology::Transition {
            duration: std::time::Duration::from_millis(150),
            easing: dragology::Easing::CssEase,
        })
    }
    fn initial(&self) -> Rectangle {
        Rectangle {
            left: 80.,
            top: 60.,
            width: 200.,
            height: 140.,
        }
    }
    fn scene(&self, s: &Rectangle) -> Scene {
        self.scene_interaction(s, None, None)
    }
    fn scene_interaction(
        &self,
        s: &Rectangle,
        _dragged: Option<&str>,
        hovered: Option<&str>,
    ) -> Scene {
        let mut scene = Scene::new(400., 300.);
        let mut body = Object::rect(
            "body",
            Rect::new(s.left, s.top, s.left + s.width, s.top + s.height),
            6.,
            rgb(0x5b9bf5),
        );
        if let Shape::Rect { stroke, .. } = &mut body.shapes[0] {
            *stroke = Some((rgb(0x3b82f6), 2.));
        }
        scene.push(body);
        for id in ["top", "bottom", "left", "right", "tl", "tr", "bl", "br"] {
            let p = rect_anchor(s, id).unwrap();
            let corner = id.len() == 2;
            let hit = if corner {
                Rect::new(-9., -9., 9., 9.)
            } else if id == "top" || id == "bottom" {
                Rect::new(
                    -s.width / 2. + 18.,
                    -7.,
                    (-s.width / 2. + 18.) + (s.width - 36.).max(0.),
                    7.,
                )
            } else {
                Rect::new(
                    -7.,
                    -s.height / 2. + 18.,
                    7.,
                    (-s.height / 2. + 18.) + (s.height - 36.).max(0.),
                )
            };
            let mut handle = Object::new(
                id,
                Affine::translate(p.to_vec2()),
                Some(Hit::Rect(hit)),
                vec![],
            );
            {
                handle.opacity = if hovered == Some(id) { 1. } else { 0. };
                handle.hit_when_transparent = true;
                handle.shapes = if corner {
                    vec![Shape::Circle {
                        center: Point::ORIGIN,
                        radius: 4.,
                        fill: rgb(0x1d4ed8),
                        stroke: None,
                    }]
                } else {
                    let d = if id == "top" || id == "bottom" {
                        Vec2::new(s.width * 0.2, 0.)
                    } else {
                        Vec2::new(0., s.height * 0.2)
                    };
                    Object::line(
                        "feedback",
                        Point::ORIGIN - d,
                        Point::ORIGIN + d,
                        rgb(0x1d4ed8),
                        3.,
                    )
                    .shapes
                };
                handle = handle.line_cap(LineCap::Round);
                if corner {
                    handle.cursor = if id == "tl" || id == "br" {
                        Cursor::ResizeDiagonalNwSe
                    } else {
                        Cursor::ResizeDiagonalNeSw
                    };
                } else {
                    handle.cursor = if id == "top" || id == "bottom" {
                        Cursor::ResizeVertical
                    } else {
                        Cursor::ResizeHorizontal
                    };
                }
            }
            scene.push(handle);
        }
        source_cursors(scene)
    }
    fn locate(&self, s: &Rectangle, id: &str) -> Option<Affine> {
        rect_anchor(s, id).map(|p| Affine::translate(p.to_vec2()))
    }
    fn drag(&self, s: &Rectangle, id: &str, _: Modifiers) -> Option<DragSpec<'static, Rectangle>> {
        rect_anchor(s, id)?;
        let base = s.clone();
        let handle = id.to_owned();
        Some(vary_constrained(
            [0., 0.],
            move |[dx, dy]| {
                let mut next = base.clone();
                if handle == "body" {
                    next.left += dx;
                    next.top += dy;
                } else {
                    if handle == "left" || handle == "tl" || handle == "bl" {
                        next.left += dx;
                        next.width -= dx;
                    }
                    if handle == "right" || handle == "tr" || handle == "br" {
                        next.width += dx;
                    }
                    if handle == "top" || handle == "tl" || handle == "tr" {
                        next.top += dy;
                        next.height -= dy;
                    }
                    if handle == "bottom" || handle == "bl" || handle == "br" {
                        next.height += dy;
                    }
                }
                next
            },
            |s: &Rectangle| {
                [
                    40. - s.width,
                    30. - s.height,
                    -s.left,
                    -s.top,
                    s.left + s.width - 400.,
                    s.top + s.height - 300.,
                ]
            },
        ))
    }
}

#[derive(Clone, PartialEq)]
struct DragonState {
    from: Point,
    to: Point,
    square: f64,
    tilt: f64,
    levels: usize,
    tilt_enabled: bool,
}
struct Dragon;
fn dragon_segment(s: &DragonState, index: usize) -> (Point, Point) {
    let mut a = s.from;
    let mut b = s.to;
    let mut direction = -1.;
    for level in (0..s.levels).rev() {
        let offset = b - a;
        let rotated =
            (Affine::rotate(TAU / 4. + s.tilt.to_radians()) * offset.to_point()).to_vec2();
        let mid = a + (b - a) * 0.5 + rotated * (s.square * direction);
        if index & (1 << level) == 0 {
            b = mid;
            direction = -1.;
        } else {
            a = mid;
            direction = 1.;
        }
    }
    (a, b)
}
fn segment_transform(a: Point, b: Point) -> Affine {
    let d = b - a;
    Affine::new([d.x, d.y, -d.y, d.x, a.x, a.y])
}
impl Example for Dragon {
    type State = DragonState;
    fn reset(&self, s: &Self::State) -> Self::State {
        DragonState {
            levels: s.levels,
            tilt_enabled: s.tilt_enabled,
            ..self.initial()
        }
    }
    fn action_transition(&self, _: &Self::State, _: &str) -> Option<dragology::Transition> {
        None
    }
    fn initial(&self) -> DragonState {
        DragonState {
            from: Point::new(300., 150.),
            to: Point::new(40., 60.),
            square: 0.4,
            tilt: 0.,
            levels: 9,
            tilt_enabled: true,
        }
    }
    fn scene(&self, s: &DragonState) -> Scene {
        let mut scene = Scene::new(400., 280.);
        let mut lines = BezPath::new();
        for i in 0..1 << s.levels {
            let (a, b) = dragon_segment(s, i);
            lines.move_to(a);
            lines.line_to(b);
            scene.push(Object::new(
                format!("segment-{i}"),
                segment_transform(a, b),
                Some(Hit::Path(
                    path([Point::ORIGIN, Point::new(1., 0.)], false),
                    4. / a.distance(b),
                )),
                Vec::new(),
            ));
        }
        scene.objects.insert(
            0,
            Object::path("dragon", lines, rgb(0), 4.).line_cap(LineCap::Round),
        );
        scene.push(Object::circle("from", s.from, 8., rgb(0xff0000)));
        scene.push(Object::circle("to", s.to, 8., rgb(0x0000ff)));
        source_cursors(scene)
    }
    fn locate(&self, s: &DragonState, id: &str) -> Option<Affine> {
        match id {
            "from" => Some(Affine::translate(s.from.to_vec2())),
            "to" => Some(Affine::translate(s.to.to_vec2())),
            _ => {
                let (a, b) = dragon_segment(s, index(id, "segment-")?);
                Some(segment_transform(a, b))
            }
        }
    }
    fn drag(
        &self,
        s: &DragonState,
        id: &str,
        _: Modifiers,
    ) -> Option<DragSpec<'static, DragonState>> {
        let base = s.clone();
        if id == "from" || id == "to" {
            let from = id == "from";
            let p = if from { s.from } else { s.to };
            return Some(vary([p.x, p.y], move |[x, y]| {
                let mut next = base.clone();
                if from {
                    next.from = Point::new(x, y);
                } else {
                    next.to = Point::new(x, y);
                }
                next
            }));
        }
        index(id, "segment-")?;
        Some(if s.tilt_enabled {
            vary_constrained(
                [s.square, s.tilt],
                move |[square, tilt]| DragonState {
                    square,
                    tilt,
                    ..base.clone()
                },
                |s: &DragonState| [-0.8 - s.square, s.square - 0.8],
            )
        } else {
            vary_constrained(
                [s.square],
                move |[square]| DragonState {
                    square,
                    ..base.clone()
                },
                |s: &DragonState| [-0.8 - s.square, s.square - 0.8],
            )
        })
    }
    fn controls(&self, s: &DragonState) -> Vec<Control> {
        vec![
            slider("levels", "Levels", s.levels as f64, 1., 13., 1.),
            Control::toggle("tilt", "Control tilt with drag", s.tilt_enabled),
        ]
    }
    fn action(&self, s: &DragonState, id: &str, value: Value) -> Option<DragonState> {
        let mut next = s.clone();
        match (id, value) {
            ("levels", Value::Number(v)) => next.levels = v.round().clamp(1., 13.) as usize,
            ("tilt", Value::Bool(v)) => next.tilt_enabled = v,
            _ => return None,
        }
        Some(next)
    }
}

#[derive(Clone, PartialEq)]
struct Boxes {
    positions: [(f64, f64); 5],
    az: f64,
    el: f64,
    fov: f64,
    distance: f64,
}
const BOX_SIZES: [f64; 5] = [0.65, 0.45, 0.75, 0.55, 0.85];
struct Scene3d;
fn project(s: &Boxes, x: f64, y: f64, z: f64) -> (Point, f64) {
    let az = s.az.to_radians();
    let el = s.el.to_radians();
    let rx = x * az.cos() - z * az.sin();
    let rz = x * az.sin() + z * az.cos();
    let depth = y * el.sin() + rz * el.cos();
    let f = 250. / (s.fov.to_radians() / 2.).tan() / (s.distance - depth);
    (
        Point::new(250. + rx * f, 252. + (-y * el.cos() + rz * el.sin()) * f),
        depth,
    )
}
fn round_local(p: Vec2) -> Point {
    Point::new((p.x * 100.).round() / 100., (p.y * 100.).round() / 100.)
}
fn box_center(s: &Boxes, i: usize) -> Point {
    project(s, s.positions[i].0, BOX_SIZES[i], s.positions[i].1).0
}
fn tint(mut color: Color, factor: f64) -> Color {
    for c in &mut color[..3] {
        *c = (f64::from(*c) * factor).clamp(0., 255.).round() as u8;
    }
    color
}
impl Example for Scene3d {
    type State = Boxes;
    fn reset(&self, s: &Self::State) -> Self::State {
        Boxes {
            az: s.az,
            el: s.el,
            fov: s.fov,
            distance: s.distance,
            ..self.initial()
        }
    }
    fn action_transition(&self, _: &Self::State, _: &str) -> Option<dragology::Transition> {
        None
    }
    fn initial(&self) -> Boxes {
        Boxes {
            positions: [(-2., -1.5), (1.5, -2.), (-1., 2.), (2.5, 0.5), (0.2, 0.5)],
            az: 35.,
            el: 28.,
            fov: 60.,
            distance: 10.,
        }
    }
    fn scene(&self, s: &Boxes) -> Scene {
        let mut scene = Scene::new(500., 450.);
        scene.push(
            Object::rect(
                "background",
                Rect::new(0., 0., 500., 450.),
                0.,
                [26, 26, 46, 255],
            )
            .decorative(),
        );
        let ground = path(
            [(-4., -4.), (4., -4.), (4., 4.), (-4., 4.)].map(|(x, z)| project(s, x, 0., z).0),
            true,
        );
        scene.push(Object::new(
            "ground",
            Affine::IDENTITY,
            None,
            vec![Shape::Path {
                path: ground.clone(),
                fill: Some([255, 255, 255, 8]),
                stroke: None,
            }],
        ));
        for i in -4..=4 {
            let v = f64::from(i);
            scene.push(Object::line(
                format!("grid-x-{i}"),
                project(s, v, 0., -4.).0,
                project(s, v, 0., 4.).0,
                [255, 255, 255, 31],
                0.5,
            ));
            scene.push(Object::line(
                format!("grid-z-{i}"),
                project(s, -4., 0., v).0,
                project(s, 4., 0., v).0,
                [255, 255, 255, 31],
                0.5,
            ));
        }
        scene.push(Object::path(
            "ground-edge",
            ground,
            [255, 255, 255, 71],
            1.2,
        ));
        let mut order = [0, 1, 2, 3, 4];
        order.sort_by(|a, b| {
            project(s, s.positions[*a].0, 0., s.positions[*a].1)
                .1
                .total_cmp(&project(s, s.positions[*b].0, 0., s.positions[*b].1).1)
        });
        let camera_x = s.distance * s.el.to_radians().cos() * s.az.to_radians().sin();
        let camera_z = s.distance * s.el.to_radians().cos() * s.az.to_radians().cos();
        for i in order {
            let (x, z) = s.positions[i];
            let r = BOX_SIZES[i];
            let h = 2. * r;
            let center = box_center(s, i);
            let p = |dx, dy, dz| project(s, x + dx, dy, z + dz);
            let sx = if camera_x > x { r } else { -r };
            let sz = if camera_z > z { r } else { -r };
            let mut faces = [
                ([p(-r, h, -r), p(r, h, -r), p(r, h, r), p(-r, h, r)], 1.3),
                (
                    [p(sx, 0., -r), p(sx, 0., r), p(sx, h, r), p(sx, h, -r)],
                    0.65,
                ),
                (
                    [p(-r, 0., sz), p(r, 0., sz), p(r, h, sz), p(-r, h, sz)],
                    0.85,
                ),
            ];
            faces.sort_by(|a, b| {
                a.0.iter()
                    .map(|p| p.1)
                    .sum::<f64>()
                    .total_cmp(&b.0.iter().map(|p| p.1).sum::<f64>())
            });
            let shadow = path(
                [(-r, -r), (r, -r), (r, r), (-r, r)]
                    .map(|(dx, dz)| round_local(p(dx, 0., dz).0 - center)),
                true,
            );
            let mut shapes = vec![Shape::Path {
                path: shadow.clone(),
                fill: Some([0, 0, 0, 102]),
                stroke: None,
            }];
            let mut hit = shadow;
            for (points, brightness) in faces {
                let local: Vec<_> = points
                    .iter()
                    .map(|(point, _)| round_local(*point - center))
                    .collect();
                let face = path(local, true);
                hit.extend(face.elements().iter().copied());
                shapes.push(Shape::Path {
                    path: face,
                    fill: Some(tint(
                        [
                            rgb(0xe05060),
                            rgb(0x5090e0),
                            rgb(0x50d080),
                            rgb(0xe0c040),
                            rgb(0xc050e0),
                        ][i],
                        brightness,
                    )),
                    stroke: Some(([0, 0, 0, 115], 0.8)),
                });
            }
            scene.push(Object::new(
                format!("box-{i}"),
                Affine::translate(center.to_vec2()),
                Some(Hit::Path(hit, 0.8)),
                shapes,
            ));
        }
        source_cursors(scene)
    }
    fn locate(&self, s: &Boxes, id: &str) -> Option<Affine> {
        index(id, "box-")
            .filter(|i| *i < 5)
            .map(|i| Affine::translate(box_center(s, i).to_vec2()))
    }
    fn drag(&self, s: &Boxes, id: &str, _: Modifiers) -> Option<DragSpec<'static, Boxes>> {
        let i = index(id, "box-")?;
        let base = s.clone();
        let (x, z) = s.positions[i];
        let r = BOX_SIZES[i];
        Some(vary_constrained(
            [x, z],
            move |[x, z]| {
                let mut next = base.clone();
                next.positions[i] = (x, z);
                next
            },
            move |s: &Boxes| {
                let (x, z) = s.positions[i];
                [-4. + r - x, x - (4. - r), -4. + r - z, z - (4. - r)]
            },
        ))
    }
    fn controls(&self, s: &Boxes) -> Vec<Control> {
        vec![
            slider("az", "Azimuth", s.az, -180., 180., 1.),
            slider("el", "Elevation", s.el, 0., 90., 1.),
            slider("fov", "FOV", s.fov, 0., 180., 1.),
            slider("distance", "Distance", s.distance, 0., 30., 1.),
        ]
    }
    fn action(&self, s: &Boxes, id: &str, value: Value) -> Option<Boxes> {
        let Value::Number(v) = value else { return None };
        let mut next = s.clone();
        match id {
            "az" => next.az = v,
            "el" => next.el = v,
            "fov" => next.fov = v,
            "distance" => next.distance = v,
            _ => return None,
        }
        Some(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dragology::DragSession;

    #[test]
    fn resize_handles_are_invisible_until_hover_and_use_css_fade() {
        let mut d = demo(Resizable);
        let m = Modifiers::default();
        assert_eq!(d.scene().object("top").unwrap().opacity, 0.);
        d.move_to(Point::new(180., 60.), m);
        d.advance(std::time::Duration::from_millis(75));
        let opacity = d.scene().object("top").unwrap().opacity;
        assert!(
            (opacity - 0.8024033876).abs() < 1e-6,
            "CSS ease opacity {opacity}"
        );
        d.advance(std::time::Duration::from_millis(75));
        assert_eq!(d.scene().object("top").unwrap().opacity, 1.);
        d.move_to(Point::new(350., 250.), m);
        d.advance(std::time::Duration::from_millis(150));
        assert_eq!(d.scene().object("top").unwrap().opacity, 0.);
    }
    #[test]
    fn resizing_pins_opposite_edges_and_preserves_canvas_constraints() {
        for id in ["top", "bottom", "left", "right", "tl", "tr", "bl", "br"] {
            let s = Resizable.initial();
            let spec = Resizable.drag(&s, id, Modifiers::default()).unwrap();
            let press = rect_anchor(&s, id).unwrap();
            let next = DragSession::begin(s.clone(), press, move |s| Resizable.locate(s, id), spec)
                .unwrap()
                .finish(press + Vec2::new(-170., -130.))
                .unwrap();
            assert!(
                next.left >= -1e-8
                    && next.top >= -1e-8
                    && next.width >= 40. - 1e-8
                    && next.height >= 30. - 1e-8
            );
            assert!(next.left + next.width <= 400. + 1e-8 && next.top + next.height <= 300. + 1e-8);
            if matches!(id, "left" | "tl" | "bl") {
                assert!((next.left + next.width - s.left - s.width).abs() < 1e-9);
            } else {
                assert_eq!(next.left, s.left);
            }
            if matches!(id, "top" | "tl" | "tr") {
                assert!((next.top + next.height - s.top - s.height).abs() < 1e-9);
            } else {
                assert_eq!(next.top, s.top);
            }
        }
    }
    #[test]
    fn bezier_curve_drag_preserves_control_displacement() {
        let state = Bezier.initial();
        let press = cubic(state, 0.5);
        let spec = Bezier
            .drag(&state, "curve-5", Modifiers::default())
            .unwrap();
        let next = DragSession::begin(state, press, |s| Bezier.locate(s, "curve-5"), spec)
            .unwrap()
            .finish(press + Vec2::new(30., -20.))
            .unwrap();
        assert_eq!(next[0], state[0]);
        assert_eq!(next[3], state[3]);
        assert!(((next[2] - next[1]) - (state[2] - state[1])).hypot() < 1e-9);
        assert!(cubic(next, 0.5).distance(press + Vec2::new(30., -20.)) < 0.01);
    }
    #[test]
    fn dragon_segments_form_one_continuous_path() {
        let mut state = Dragon.initial();
        for levels in [1, 5, 9, 13] {
            state.levels = levels;
            assert_eq!(dragon_segment(&state, 0).0, state.from);
            assert_eq!(dragon_segment(&state, (1 << levels) - 1).1, state.to);
            for i in 1..1 << levels {
                assert_eq!(dragon_segment(&state, i - 1).1, dragon_segment(&state, i).0);
            }
        }
    }
}
