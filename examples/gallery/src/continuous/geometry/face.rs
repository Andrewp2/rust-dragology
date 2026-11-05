use super::*;

const CENTER: Point = Point::new(200., 180.);
const MARGIN: f64 = 15.5;

#[derive(Clone, PartialEq)]
pub(super) struct FaceState {
    // eye y / spacing; mouth half-width / baseline; relative control points;
    // face x radius / upper radius / lower radius / upper bulge / lower bulge.
    values: [f64; 13],
    eyes_above: bool,
    expand: bool,
    scale_mouth: bool,
    mickey: bool,
}
pub(super) struct Face;

fn eye(s: &FaceState, right: bool) -> Point {
    Point::new(
        200. + s.values[1] * if right { 1. } else { -1. },
        s.values[0],
    )
}
fn mouth(s: &FaceState) -> [Point; 4] {
    let v = s.values;
    let left = Point::new(200. - v[2], v[3]);
    let right = Point::new(200. + v[2], v[3]);
    [
        left,
        left + Vec2::new(v[4], v[5]),
        right + Vec2::new(v[6], v[7]),
        right,
    ]
}
fn face_segments(s: &FaceState) -> [[Point; 4]; 4] {
    let v = s.values;
    let (rx, top, bottom) = (v[8], v[9], v[10]);
    let m = std::f64::consts::FRAC_1_SQRT_2;
    let tr = Vec2::new(rx * m, -top * m);
    let tr = tr + tr.normalize() * v[11];
    let br = Vec2::new(rx * m, bottom * m);
    let br = br + br.normalize() * v[12];
    let (tx, ty) = ((tr.x - 0.5 * rx) / 0.375, (tr.y + 0.5 * top) / 0.375);
    let (bx, by) = ((br.x - 0.5 * rx) / 0.375, (br.y - 0.5 * bottom) / 0.375);
    [
        [(0., -top), (tx, -top), (rx, ty), (rx, 0.)],
        [(rx, 0.), (rx, by), (bx, bottom), (0., bottom)],
        [(0., bottom), (-bx, bottom), (-rx, by), (-rx, 0.)],
        [(-rx, 0.), (-rx, ty), (-tx, -top), (0., -top)],
    ]
    .map(|points| points.map(|(x, y)| CENTER + Vec2::new(x, y)))
}
fn samples(s: &FaceState) -> Vec<(f64, f64)> {
    let mut points = Vec::with_capacity(64);
    for segment in face_segments(s) {
        for i in 0..16 {
            let d = cubic(segment, i as f64 / 16.) - CENTER;
            points.push((d.atan2(), d.hypot()));
        }
    }
    points.sort_by(|a, b| a.0.total_cmp(&b.0));
    points
}
fn radius(samples: &[(f64, f64)], angle: f64) -> f64 {
    let i1 = samples.partition_point(|(a, _)| *a < angle) % samples.len();
    let i0 = (i1 + samples.len() - 1) % samples.len();
    let mut span = samples[i1].0 - samples[i0].0;
    let mut offset = angle - samples[i0].0;
    if span <= 0. {
        span += TAU;
    }
    if offset < 0. {
        offset += TAU;
    }
    let t = if span > 0. { offset / span } else { 0. };
    samples[i0].1 + t * (samples[i1].1 - samples[i0].1)
}
fn contains(samples: &[(f64, f64)], point: Point) -> f64 {
    let d = point - CENTER;
    d.hypot() - radius(samples, d.atan2()) + MARGIN
}
fn clamp_point(samples: &[(f64, f64)], point: Point) -> Point {
    let d = point - CENTER;
    let length = d.hypot();
    if length == 0. {
        return point;
    }
    let max = radius(samples, d.atan2()) - MARGIN;
    CENTER + d * (max / length).min(1.)
}
fn clamp_features(mut s: FaceState) -> FaceState {
    let boundary = samples(&s);
    let (le, re) = (eye(&s, false), eye(&s, true));
    let (cl, cr) = (clamp_point(&boundary, le), clamp_point(&boundary, re));
    if cl != le || cr != re {
        s.values[0] = cl.y.min(cr.y);
        s.values[1] = (cl.x - 200.).abs().min((cr.x - 200.).abs()).max(17.);
    }
    let (ml, mr) = (mouth(&s)[0], mouth(&s)[3]);
    let (cl, cr) = (clamp_point(&boundary, ml), clamp_point(&boundary, mr));
    if cl != ml || cr != mr {
        s.values[3] = cl.y.max(cr.y);
        s.values[2] = (cl.x - 200.).abs().min((cr.x - 200.).abs()).max(10.);
    }
    s
}
fn expand_features(mut s: FaceState) -> FaceState {
    let mut points = vec![eye(&s, false), eye(&s, true), mouth(&s)[0], mouth(&s)[3]];
    let curve = mouth(&s);
    points.extend((0..=8).map(|i| cubic(curve, i as f64 / 8.)));
    for point in points {
        let boundary = samples(&s);
        let d = point - CENTER;
        let deficit = contains(&boundary, point);
        if deficit > 0. && d.hypot() > 0. {
            let direction = d.normalize();
            s.values[8] += deficit * direction.x.abs();
            s.values[if d.y < 0. { 9 } else { 10 }] += deficit * direction.y.abs();
        }
    }
    s
}
fn push_mouth(mut s: FaceState) -> FaceState {
    for _ in 0..3 {
        let mut amount = 0_f64;
        for i in 0..=8 {
            let p = cubic(mouth(&s), i as f64 / 8.);
            for e in [eye(&s, false), eye(&s, true)] {
                if s.eyes_above {
                    if (p.x - e.x).abs() < 14. {
                        amount = amount.max(e.y + 18. - p.y);
                    }
                } else {
                    amount = amount.max(30. - p.distance(e));
                }
            }
        }
        if amount <= 0. {
            break;
        }
        s.values[3] += amount;
    }
    s
}
fn push_eyes(mut s: FaceState) -> FaceState {
    for _ in 0..3 {
        let mut push = Vec2::ZERO;
        for i in 0..=8 {
            let p = cubic(mouth(&s), i as f64 / 8.);
            for right in [false, true] {
                let e = eye(&s, right);
                let d = e - p;
                let length = d.hypot();
                if length > 0. && length < 30. {
                    let delta = d * ((30. - length) / length);
                    push.x += delta.x * if right { 1. } else { -1. };
                    push.y += delta.y;
                }
            }
        }
        if push.hypot() < 0.1 {
            break;
        }
        s.values[1] = (s.values[1] + push.x * 0.5).max(17.);
        s.values[0] += push.y * 0.5;
    }
    s
}
fn finalize(mut s: FaceState, expand: bool) -> FaceState {
    if s.eyes_above {
        for i in 0..=8 {
            let p = cubic(mouth(&s), i as f64 / 8.);
            if (p.x - eye(&s, true).x).abs() < 14. || (p.x - eye(&s, false).x).abs() < 14. {
                s.values[0] = s.values[0].min(p.y - 18.);
            }
        }
    }
    if expand && s.expand {
        s = expand_features(s);
    }
    clamp_features(s)
}
fn constraints(s: &FaceState, face: bool) -> [f64; 48] {
    let mut result = [0.; 48];
    let v = s.values;
    if face {
        result[..3].copy_from_slice(&[40. - v[8], 40. - v[9], 40. - v[10]]);
        if !s.mickey {
            let mut angles = Vec::with_capacity(32);
            for segment in face_segments(s) {
                for i in 0..8 {
                    angles.push((cubic(segment, i as f64 / 8.) - CENTER).atan2());
                }
            }
            for i in 0..32 {
                let mut diff = angles[(i + 1) % 32] - angles[i];
                if diff < -std::f64::consts::PI {
                    diff += TAU;
                }
                result[3 + i] = TAU / 32. * 0.25 - diff;
            }
        }
    } else {
        result[..4].copy_from_slice(&[
            17. - v[1],
            10. - v[2],
            Vec2::new(v[4], v[5]).hypot() - 100.,
            Vec2::new(v[6], v[7]).hypot() - 100.,
        ]);
        let curve = mouth(s);
        for i in 0..8 {
            result[4 + i] = cubic(curve, i as f64 / 8.).x - cubic(curve, (i + 1) as f64 / 8.).x;
        }
        if !s.expand {
            let boundary = samples(s);
            result[12] = contains(&boundary, eye(s, false));
            result[13] = contains(&boundary, eye(s, true));
            result[14] = contains(&boundary, curve[0]);
            result[15] = contains(&boundary, curve[3]);
            for i in 0..=8 {
                result[16 + i] = contains(&boundary, cubic(curve, i as f64 / 8.));
            }
        }
    }
    result
}
fn perimeter_id(id: &str) -> Option<(usize, usize)> {
    let rest = id.strip_prefix("face-")?;
    let (a, b) = rest.split_once('-')?;
    Some((a.parse().ok()?, b.parse().ok()?))
}
fn vary_face<const N: usize>(
    s: &FaceState,
    parameters: Vec<usize>,
    face: bool,
) -> DragSpec<'static, FaceState> {
    let initial = std::array::from_fn(|i| s.values[parameters[i]]);
    let base = s.clone();
    vary_constrained::<_, N, 48>(
        initial,
        move |values| {
            let mut next = base.clone();
            for (i, value) in parameters.iter().zip(values) {
                next.values[*i] = value;
            }
            next
        },
        move |s: &FaceState| constraints(s, face),
    )
}
impl Example for Face {
    type State = FaceState;
    fn reset(&self, s: &Self::State) -> Self::State {
        FaceState {
            eyes_above: s.eyes_above,
            expand: s.expand,
            scale_mouth: s.scale_mouth,
            mickey: s.mickey,
            ..self.initial()
        }
    }
    fn action_transition(&self, _: &Self::State, _: &str) -> Option<dragology::Transition> {
        None
    }
    fn initial(&self) -> FaceState {
        FaceState {
            values: [
                155., 40., 50., 220., 20., 30., -20., 30., 120., 120., 120., 0., 0.,
            ],
            eyes_above: true,
            expand: true,
            scale_mouth: true,
            mickey: false,
        }
    }
    fn scene(&self, s: &FaceState) -> Scene {
        self.scene_dragging(s, None)
    }
    fn scene_dragging(&self, s: &FaceState, dragged: Option<&str>) -> Scene {
        let mut scene = Scene::new(400., 350.);
        let segments = face_segments(s);
        let mut outline = BezPath::new();
        outline.move_to(segments[0][0]);
        for p in segments {
            outline.curve_to(p[1], p[2], p[3]);
        }
        outline.close_path();
        scene.push(Object::new(
            "outline",
            Affine::IDENTITY,
            None,
            vec![Shape::Path {
                path: outline,
                fill: Some([255, 224, 178, 255]),
                stroke: Some(([230, 167, 86, 255], 3.)),
            }],
        ));
        for (segment, p) in segments.iter().enumerate() {
            for i in 0..8 {
                let id = format!("face-{segment}-{i}");
                let active = dragged == Some(id.as_str());
                scene.push(Object::circle(
                    id,
                    cubic(*p, i as f64 / 8.),
                    if active {
                        6.
                    } else {
                        (s.values[8].max(s.values[9]).max(s.values[10]) * 0.13).max(16.)
                    },
                    if active { [230, 167, 86, 102] } else { [0; 4] },
                ));
            }
        }
        scene.push(Object::circle(
            "left-eye",
            eye(s, false),
            12.,
            rgb(0x333333),
        ));
        scene.push(Object::circle(
            "right-eye",
            eye(s, true),
            12.,
            rgb(0x333333),
        ));
        scene.push(
            Object::path("mouth", cubic_path(mouth(s)), rgb(0xc0392b), 4.).line_cap(LineCap::Round),
        );
        for i in 1..12 {
            let active = dragged == Some(format!("mouth-{i}").as_str());
            scene.push(Object::circle(
                format!("mouth-{i}"),
                cubic(mouth(s), i as f64 / 12.),
                if active { 8. } else { 14. },
                if active { [192, 57, 43, 102] } else { [0; 4] },
            ));
        }
        for (id, point) in [("mouth-left", mouth(s)[0]), ("mouth-right", mouth(s)[3])] {
            scene.push(stroked_circle(
                id,
                point,
                6.,
                rgb(if dragged == Some(id) {
                    0xe74c3c
                } else {
                    0xc0392b
                }),
                rgb(0xc0392b),
                1.5,
            ));
        }
        source_cursors(scene)
    }
    fn locate(&self, s: &FaceState, id: &str) -> Option<Affine> {
        let p = match id {
            "left-eye" => eye(s, false),
            "right-eye" => eye(s, true),
            "mouth-left" => mouth(s)[0],
            "mouth-right" => mouth(s)[3],
            _ => {
                if let Some((segment, i)) = perimeter_id(id) {
                    cubic(*face_segments(s).get(segment)?, i as f64 / 8.)
                } else {
                    cubic(mouth(s), index(id, "mouth-")? as f64 / 12.)
                }
            }
        };
        Some(Affine::translate(p.to_vec2()))
    }
    fn drag(&self, s: &FaceState, id: &str, _: Modifiers) -> Option<DragSpec<'static, FaceState>> {
        let face = perimeter_id(id);
        let eye = id.ends_with("eye");
        let endpoint = id == "mouth-left" || id == "mouth-right";
        let parameters: Vec<usize> = if let Some((segment, i)) = face {
            let choices = [[9, 11, 8], [8, 12, 10], [10, 12, 8], [8, 11, 9]];
            let t = i as f64 / 8.;
            vec![
                choices.get(segment)?[if t < 0.35 {
                    0
                } else if t > 0.65 {
                    2
                } else {
                    1
                }],
            ]
        } else if eye {
            vec![0, 1]
        } else if endpoint {
            vec![2, 3]
        } else {
            let t = index(id, "mouth-")? as f64 / 12.;
            if t < 0.4 {
                vec![4, 5]
            } else if t > 0.6 {
                vec![6, 7]
            } else {
                vec![4, 5, 6, 7]
            }
        };
        let before = s.clone();
        let face_drag = face.is_some();
        let spec = match parameters.len() {
            1 => vary_face::<1>(s, parameters, face_drag),
            2 => vary_face::<2>(s, parameters, face_drag),
            4 => vary_face::<4>(s, parameters, face_drag),
            _ => return None,
        };
        Some(spec.during(move |s| {
            let mut next = s.clone();
            if face.is_some() {
                next = push_eyes(push_mouth(next));
            } else if eye {
                next = push_mouth(next);
                if next.values[3] != before.values[3] {
                    let floor = 180. + before.values[10] - MARGIN;
                    let space = floor - before.values[3];
                    let scale = if space > 0. {
                        (floor - next.values[3]) / space
                    } else {
                        1.
                    };
                    next.values[5] = before.values[5] * scale;
                    next.values[7] = before.values[7] * scale;
                }
            } else {
                next = push_eyes(next);
                if endpoint && next.scale_mouth {
                    let scale = next.values[2] / before.values[2];
                    let floor = 180. + before.values[10] - MARGIN;
                    let space = floor - before.values[3];
                    let vertical = if space > 0. {
                        (floor - next.values[3]) / space
                    } else {
                        1.
                    };
                    next.values[4] = before.values[4] * scale;
                    next.values[6] = before.values[6] * scale;
                    next.values[5] = before.values[5] * vertical;
                    next.values[7] = before.values[7] * vertical;
                }
            }
            finalize(next, face.is_none())
        }))
    }
    fn controls(&self, s: &FaceState) -> Vec<Control> {
        vec![
            Control::toggle("above", "Keep eyes above mouth", s.eyes_above),
            Control::toggle("expand", "Features move face", s.expand),
            Control::toggle("scale", "Scale mouth with endpoints", s.scale_mouth),
            Control::toggle("mickey", "Enable Mickey mode", s.mickey),
        ]
    }
    fn action(&self, s: &FaceState, id: &str, value: Value) -> Option<FaceState> {
        let Value::Bool(value) = value else {
            return None;
        };
        let mut next = s.clone();
        match id {
            "above" => next.eyes_above = value,
            "expand" => next.expand = value,
            "scale" => next.scale_mouth = value,
            "mickey" => next.mickey = value,
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
    fn resizing_face_clamps_features_without_expanding_it_back() {
        let state = Face.initial();
        let id = "face-0-0";
        let press = Face.locate(&state, id).unwrap() * Point::ORIGIN;
        let spec = Face.drag(&state, id, Modifiers::default()).unwrap();
        let next = DragSession::begin(state, press, move |s| Face.locate(s, id), spec)
            .unwrap()
            .finish(Point::new(200., 120.))
            .unwrap();
        assert!((next.values[9] - 60.).abs() < 0.1);
        let boundary = samples(&next);
        for p in [
            eye(&next, false),
            eye(&next, true),
            mouth(&next)[0],
            mouth(&next)[3],
        ] {
            assert!(contains(&boundary, p) < 1e-6);
        }
    }
}
