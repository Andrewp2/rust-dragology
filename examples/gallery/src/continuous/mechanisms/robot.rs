use super::*;

const LENGTHS: [f64; 4] = [130., 110., 80., 36.];
const WIDTHS: [f64; 3] = [26., 22., 18.];
#[derive(Clone, PartialEq)]
pub(super) struct ArmState {
    base: f64,
    angles: [f64; 4],
    balls: [f64; 4],
    bin: Vec<usize>,
    held: Option<usize>,
    claw_down: bool,
}
pub(super) struct Robot;
fn angles(s: &ArmState) -> [f64; 4] {
    let mut a = s.angles;
    if s.claw_down {
        a[3] = 90. - a[0] - a[1] - a[2];
    }
    a
}
fn joints(s: &ArmState) -> [Point; 5] {
    let mut points = [Point::new(s.base, 330.); 5];
    let mut angle = 0.;
    for (i, a) in angles(s).iter().enumerate() {
        angle += a;
        points[i + 1] = points[i] + polar(LENGTHS[i], angle.to_radians());
    }
    points
}
fn arm_transform(s: &ArmState, i: usize) -> Affine {
    Affine::translate(joints(s)[i].to_vec2())
        * Affine::rotate(angles(s)[..=i].iter().sum::<f64>().to_radians())
}
fn arm_components(s: &ArmState, i: usize) -> Vec<TransformComponent> {
    let mut result = vec![TransformComponent::Translate(Vec2::new(s.base, 330.))];
    for (joint, angle) in angles(s).into_iter().enumerate().take(i + 1) {
        result.push(TransformComponent::Rotate {
            radians: angle.to_radians(),
            center: Point::ORIGIN,
        });
        if joint < i {
            result.push(TransformComponent::Translate(Vec2::new(LENGTHS[joint], 0.)));
        }
    }
    result
}
fn ball_position(s: &ArmState, i: usize) -> Point {
    if s.held == Some(i) {
        return joints(s)[4];
    }
    if let Some(index) = s.bin.iter().position(|b| *b == i) {
        return Point::new(501. + 29. * index as f64, 351.);
    }
    Point::new(s.balls[i], 357.)
}
fn grabbable(s: &ArmState) -> Option<usize> {
    if s.held.is_some() {
        return None;
    }
    let tip = joints(s)[4];
    (0..4)
        .filter(|i| ball_position(s, *i).distance(tip) < 20.)
        .min_by(|a, b| {
            ball_position(s, *a)
                .distance(tip)
                .total_cmp(&ball_position(s, *b).distance(tip))
        })
}
fn settle(s: &ArmState) -> ArmState {
    let mut next = s.clone();
    next.angles = angles(s);
    let tip = joints(s)[4];
    if let Some(i) = s.held {
        if tip.x > 480. && tip.x < 610. {
            next.held = None;
            next.bin.push(i);
        } else if tip.y > 331. {
            next.held = None;
            next.balls[i] = tip.x.clamp(13., 627.);
        }
    } else if let Some(i) = grabbable(s) {
        next.held = Some(i);
        next.bin.retain(|j| *j != i);
    }
    next
}
fn constraints(s: &ArmState) -> [f64; 12] {
    let a = angles(s);
    let p = joints(s);
    [
        -180. - a[0],
        a[0],
        -150. - a[1],
        a[1] - 150.,
        -150. - a[2],
        a[2] - 150.,
        -120. - a[3],
        a[3] - 120.,
        p[1].y - 358.,
        p[2].y - 358.,
        p[3].y - 358.,
        p[4].y - 364.,
    ]
}
fn line_shape(a: impl Into<Point>, b: impl Into<Point>, color: Color, width: f64) -> Shape {
    Shape::Path {
        path: path([a.into(), b.into()], false),
        fill: None,
        stroke: Some((color, width)),
    }
}
fn vary_arm<const N: usize>(s: &ArmState) -> DragSpec<'static, ArmState> {
    let base = s.clone();
    vary_constrained(
        std::array::from_fn::<_, N, _>(|i| s.angles[i]),
        move |a| {
            let mut next = base.clone();
            next.angles[..N].copy_from_slice(&a);
            next
        },
        constraints,
    )
}
impl Example for Robot {
    type State = ArmState;
    fn reset(&self, s: &Self::State) -> Self::State {
        ArmState {
            claw_down: s.claw_down,
            ..self.initial()
        }
    }
    fn action_transition(&self, _: &Self::State, _: &str) -> Option<dragology::Transition> {
        None
    }
    fn initial(&self) -> ArmState {
        ArmState {
            base: 200.,
            angles: [-70., 80., 60., 20.],
            balls: [50., 290., 410., 450.],
            bin: vec![],
            held: None,
            claw_down: true,
        }
    }
    fn scene(&self, s: &ArmState) -> Scene {
        self.scene_dragging(s, None)
    }
    fn scene_dragging(&self, s: &ArmState, dragged: Option<&str>) -> Scene {
        let mut scene = Scene::new(640., 400.);
        scene.push(
            Object::rect(
                "floor",
                Rect::new(0., 370., 640., 400.),
                0.,
                [231, 229, 228, 255],
            )
            .decorative(),
        );
        scene.push(
            Object::line("rail", (30., 368.), (410., 368.), rgb(0xa8a29e), 4.)
                .line_cap(LineCap::Round),
        );
        let tip = joints(s)[4];
        let bin_lit = dragged.is_some() && s.held.is_some() && tip.x > 480. && tip.x < 610.;
        scene.push(
            Object::rect(
                "bin-background",
                Rect::new(480., 306., 610., 370.),
                0.,
                if bin_lit {
                    [187, 247, 208, 255]
                } else {
                    [245, 245, 244, 255]
                },
            )
            .decorative(),
        );
        scene.push(
            Object::path(
                "bin",
                path(
                    [
                        Point::new(480., 306.),
                        Point::new(480., 370.),
                        Point::new(610., 370.),
                        Point::new(610., 306.),
                    ],
                    false,
                ),
                [120, 113, 108, 255],
                6.,
            )
            .line_cap(LineCap::Round)
            .line_join(LineJoin::Round),
        );
        scene.push(
            Object::text(
                "count",
                (545., 391.),
                format!("{} / 4", s.bin.len()),
                14.,
                rgb(0x78716c),
            )
            .text_anchor(TextAnchor::Middle)
            .text_baseline(TextBaseline::Alphabetic),
        );
        let base_shape = path(
            [
                Point::new(-36., 36.),
                Point::new(-20., -6.),
                Point::new(20., -6.),
                Point::new(36., 36.),
            ],
            true,
        );
        let mut base_hit = base_shape.clone();
        base_hit.extend(kurbo::stroke(
            base_shape.elements().iter().copied(),
            &kurbo::Stroke::new(1.5).with_join(kurbo::Join::Round),
            &kurbo::StrokeOpts::default(),
            0.01,
        ));
        for x in [-22., 22.] {
            base_hit.extend(kurbo::Circle::new((x, 36.), 6.).to_path(0.01));
        }
        scene.push(
            Object::new(
                "base",
                Affine::translate((s.base, 330.)),
                Some(Hit::Path(base_hit, 0.)),
                vec![
                    Shape::Path {
                        path: base_shape,
                        fill: Some([100, 116, 139, 255]),
                        stroke: Some(([51, 65, 85, 255], 1.5)),
                    },
                    Shape::Circle {
                        center: Point::new(-22., 36.),
                        radius: 6.,
                        fill: [51, 65, 85, 255],
                        stroke: None,
                    },
                    Shape::Circle {
                        center: Point::new(22., 36.),
                        radius: 6.,
                        fill: [51, 65, 85, 255],
                        stroke: None,
                    },
                ],
            )
            .line_join(LineJoin::Round),
        );
        for i in 0..3 {
            let w = WIDTHS[i];
            let bounds = Rect::new(-w / 2., -w / 2., LENGTHS[i] + w / 2., w / 2.);
            scene.push(Object::new(
                format!("segment-{i}"),
                arm_transform(s, i),
                Some(Hit::Rect(bounds)),
                vec![Shape::Rect {
                    bounds,
                    radius: w / 2.,
                    fill: [
                        [245, 158, 11, 255],
                        [251, 191, 36, 255],
                        [252, 211, 77, 255],
                    ][i],
                    stroke: Some(([146, 64, 14, 255], 1.5)),
                }],
            ));
        }
        let claw_shapes = vec![
            line_shape((0., 0.), (16., 0.), [71, 85, 105, 255], 8.),
            line_shape((16., -10.), (16., 10.), [71, 85, 105, 255], 6.),
        ];
        scene.push(
            Object::new(
                "claw",
                arm_transform(s, 3),
                Some(Hit::Circle(Point::new(22., 0.), 22.)),
                claw_shapes,
            )
            .line_cap(LineCap::Round),
        );
        for side in [1_f64, -1.] {
            let mut components = arm_components(s, 3);
            components.push(TransformComponent::Translate(Vec2::new(16., side * 7.)));
            components.push(TransformComponent::Rotate {
                radians: (side * if s.held.is_some() { 14. } else { 38. }).to_radians(),
                center: Point::ORIGIN,
            });
            for (part, length) in [20., 9.].into_iter().enumerate() {
                if part == 1 {
                    components.push(TransformComponent::Translate(Vec2::new(20., 0.)));
                    components.push(TransformComponent::Rotate {
                        radians: (-side * 55.).to_radians(),
                        center: Point::ORIGIN,
                    });
                }
                let mut finger = Object::line(
                    format!("finger-{side}-{part}"),
                    Point::ORIGIN,
                    (length, 0.),
                    [71, 85, 105, 255],
                    5.,
                )
                .transforms(components.clone())
                .line_cap(LineCap::Round);
                finger.parent = Some("claw".into());
                scene.push(finger);
            }
        }
        for i in 0..4 {
            scene.push(Object::new(
                format!("joint-{i}"),
                arm_transform(s, i),
                Some(Hit::Circle(
                    Point::ORIGIN,
                    if i == 3 { 10. } else { WIDTHS[i] / 2. },
                )),
                vec![Shape::Circle {
                    center: Point::ORIGIN,
                    radius: if i == 3 { 6. } else { WIDTHS[i] / 4. },
                    fill: if i == 3 {
                        [51, 65, 85, 255]
                    } else {
                        [146, 64, 14, 255]
                    },
                    stroke: None,
                }],
            ));
        }
        for i in 0..4 {
            let color = [rgb(0xef4444), rgb(0x3b82f6), rgb(0x22c55e), rgb(0xa855f7)][i];
            if dragged.is_some() && grabbable(s) == Some(i) {
                scene.push(circle_outline(
                    format!("target-{i}"),
                    ball_position(s, i),
                    18.,
                    color,
                    2.,
                    &[4., 3.],
                ));
            }
            let mut ball =
                Object::circle(format!("ball-{i}"), ball_position(s, i), 13., color).decorative();
            ball.shapes.push(Shape::Circle {
                center: Point::new(-4., -4.),
                radius: 4.,
                fill: [255, 255, 255, 128],
                stroke: None,
            });
            scene.push(ball);
        }
        {
            let mut again = Object::text("again", (320., 50.), "All tidy!", 22., rgb(0x15803d))
                .text_anchor(TextAnchor::Middle)
                .text_baseline(TextBaseline::Alphabetic);
            let title = crate::text_layout::measure_text("All tidy!", 22., &again.text_style);
            let action =
                crate::text_layout::measure_text("mess it up again", 14., &again.text_style);
            let mut hit = Rect::new(
                -title.advance / 2.,
                -title.ascent,
                title.advance / 2.,
                title.descent,
            )
            .to_path(0.01);
            hit.extend(
                Rect::new(
                    -action.advance / 2.,
                    24. - action.ascent,
                    action.advance / 2.,
                    24. + action.descent,
                )
                .to_path(0.01),
            );
            hit.extend(
                Rect::new(-action.advance / 2., 25.65, action.advance / 2., 26.35).to_path(0.01),
            );
            again.hit = Some(Hit::Path(hit, 0.));
            again.shapes.push(Shape::Text {
                origin: Point::new(0., 24.),
                text: "mess it up again".into(),
                size: 14.,
                color: rgb(0x78716c),
            });
            again.shapes.extend(
                Object::line(
                    "underline",
                    (-action.advance / 2., 26.),
                    (action.advance / 2., 26.),
                    rgb(0x78716c),
                    0.7,
                )
                .shapes,
            );
            // Upstream retains this group and fades its opacity; keeping the
            // identity avoids an emergence-scale effect when the last ball drops.
            if s.bin.len() != 4 {
                again.opacity = 0.;
                again.hit = None;
            }
            scene.push(again);
        }

        for object in &mut scene.objects {
            if object.id == "base" {
                object.cursor = Cursor::ResizeHorizontal;
            } else if object.id == "again" {
                object.cursor = Cursor::Pointer;
                object.click_handler = true;
            }
            let index = index(&object.id, "segment-")
                .or_else(|| index(&object.id, "joint-"))
                .or_else(|| (object.id == "claw").then_some(3));
            if let Some(index) = index {
                *object = object.clone().transforms(arm_components(s, index));
            }
        }
        source_cursors(scene)
    }
    fn locate(&self, s: &ArmState, id: &str) -> Option<Affine> {
        if id == "base" {
            return Some(Affine::translate((s.base, 330.)));
        }
        let i = if id == "claw" {
            3
        } else {
            index(id, "segment-").or_else(|| index(id, "joint-"))?
        };
        (i < 4).then(|| arm_transform(s, i))
    }
    fn drag(&self, s: &ArmState, id: &str, _: Modifiers) -> Option<DragSpec<'static, ArmState>> {
        if id == "base" || id == "joint-0" {
            let base = s.clone();
            return Some(
                vary_constrained(
                    [s.base],
                    move |[x]| ArmState {
                        base: x,
                        ..base.clone()
                    },
                    |s: &ArmState| [70. - s.base, s.base - 370.],
                )
                .on_drop(settle),
            );
        }
        let count = if id == "claw" {
            4
        } else if let Some(i) = index(id, "segment-") {
            i + 1
        } else {
            index(id, "joint-")?
        };
        if count > 4 {
            return None;
        }
        let count = if s.claw_down { count.min(3) } else { count };
        Some(
            match count {
                1 => vary_arm::<1>(s),
                2 => vary_arm::<2>(s),
                3 => vary_arm::<3>(s),
                4 => vary_arm::<4>(s),
                _ => return None,
            }
            .on_drop(settle),
        )
    }
    fn click(&self, s: &ArmState, id: &str, _: Point, _: Modifiers, _: usize) -> Option<ArmState> {
        if id != "again" {
            return None;
        }
        Some(ArmState {
            balls: [50., 290., 410., 450.],
            bin: vec![],
            held: None,
            ..s.clone()
        })
    }
    fn controls(&self, s: &ArmState) -> Vec<Control> {
        vec![Control::toggle(
            "down",
            "Keep claw pointing down",
            s.claw_down,
        )]
    }
    fn action(&self, s: &ArmState, id: &str, value: Value) -> Option<ArmState> {
        let Value::Bool(down) = value else {
            return None;
        };
        (id == "down").then(|| ArmState {
            claw_down: down,
            ..s.clone()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dragology::DragSession;

    #[test]
    fn base_and_tidy_hits_follow_painted_geometry_instead_of_enclosing_rectangles() {
        let mut state = Robot.initial();
        let initial = Robot.scene(&state);
        let hidden = initial.object("again").unwrap();
        assert_eq!(hidden.opacity, 0.);
        assert!(hidden.hit.is_none());
        state.bin = vec![0, 1, 2, 3];
        let scene = Robot.scene(&state);
        let Some(Hit::Path(base, _)) = &scene.object("base").unwrap().hit else {
            panic!("base must use a union path")
        };
        for point in [
            Point::new(0., 0.),
            Point::new(22., 40.),
            Point::new(-20., -6.5),
        ] {
            assert_ne!(
                base.winding(point),
                0,
                "missing painted base area at {point:?}"
            );
        }
        for point in [Point::new(35., 0.), Point::new(0., 40.)] {
            assert_eq!(
                base.winding(point),
                0,
                "empty base corner should not grab at {point:?}"
            );
        }
        let again = scene.object("again").unwrap();
        let Some(Hit::Path(hit, _)) = &again.hit else {
            panic!("text must use the union of its two line bounds")
        };
        let title = crate::text_layout::measure_text("All tidy!", 22., &again.text_style);
        let action = crate::text_layout::measure_text("mess it up again", 14., &again.text_style);
        assert_ne!(hit.winding(Point::new(0., -title.ascent / 2.)), 0);
        assert_ne!(hit.winding(Point::new(0., 24. - action.ascent / 2.)), 0);
        let between_lines = (title.descent + 24. - action.ascent) / 2.;
        assert_eq!(hit.winding(Point::new(0., between_lines)), 0);
        let Shape::Path {
            path: underline, ..
        } = again.shapes.last().unwrap()
        else {
            panic!("underline missing")
        };
        assert!((underline.bounding_box().width() - action.advance).abs() < 1e-9);
    }

    #[test]
    fn inverse_kinematics_can_pick_up_a_ball_without_crossing_the_floor() {
        let state = Robot.initial();
        let press = joints(&state)[4];
        let spec = Robot.drag(&state, "claw", Modifiers::default()).unwrap();
        let next = DragSession::begin(state, press, |s| Robot.locate(s, "claw"), spec)
            .unwrap()
            .finish(Point::new(410., 357.))
            .unwrap();
        assert_eq!(next.held, Some(2));
        assert!(constraints(&next).iter().all(|v| *v <= 1e-8));
    }

    #[test]
    fn fixed_lengths_and_claw_direction() {
        let mut s = Robot.initial();
        for angles in [
            [-70., 80., 60., 20.],
            [-130., 100., 30., -10.],
            [-10., -30., 70., 0.],
        ] {
            s.angles = angles;
            let points = joints(&s);
            for i in 0..4 {
                assert!((points[i + 1].distance(points[i]) - LENGTHS[i]).abs() < 1e-9);
            }
            assert!((points[4].x - points[3].x).abs() < 1e-9);
            assert!((points[4].y - points[3].y - 36.).abs() < 1e-9);
        }
    }
    #[test]
    fn dropping_into_bin_preserves_ball_identity() {
        let mut s = Robot.initial();
        s.held = Some(2);
        let tip = joints(&s)[4];
        s.base += 540. - tip.x;
        let next = settle(&s);
        assert_eq!(next.bin, vec![2]);
        assert_eq!(next.held, None);
        assert_eq!(ball_position(&next, 2), Point::new(501., 351.));
    }
}
