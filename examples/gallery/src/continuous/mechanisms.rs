use super::*;
use dragology::{between, closest, fixed, vary, vary_constrained};

mod nodes;
mod robot;
mod tree;

pub(super) fn entries() -> Vec<Entry> {
    vec![
        Entry::new(
            "chain-of-links",
            "chain-of-links",
            "Uses vary to move the dragged dot, during to update the rest of the chain, and (subtle part!) withChaining so that the result of the rest-of-chain update is fed back into the next frame. (If you don't do this, you're always simulating drags from the drag-start state; you need to bring in hysteresis somehow!)",
            || demo(Chain),
        ),
        Entry::new(
            "robot-arm",
            "robot-arm",
            "Inverse kinematics with d.vary. Each link is rotated inside the one before it, and grabbing a link varies the angles of every joint between it and the shoulder. Joint limits and \"don't go through the floor\" are constraints. Slide the base along its rail. Let go with the claw near a ball to grab it; let go over the bin (or down by the floor) to drop it.",
            || demo(robot::Robot),
        ),
        Entry::new("nodes-and-noodles", "nodes-and-noodles", "", || {
            demo(nodes::Nodes)
        }),
        Entry::new("sprouting-tree", "sprouting-tree", "", || {
            demo(tree::SproutingTree)
        }),
        Entry::new(
            "gears",
            "gears",
            "Drag edge to spin, center to move. Gear connection logic is WIP.",
            || demo(Gears),
        ),
        Entry::new("scheduler", "scheduler", "", || demo(Scheduler)),
    ]
}

#[derive(Clone, PartialEq)]
struct ChainState {
    points: [Point; 10],
    mode: usize,
}
struct Chain;
fn follow_chain(s: &ChainState, anchor: usize) -> ChainState {
    let mut next = s.clone();
    for i in (0..anchor).rev() {
        let d = next.points[i] - next.points[i + 1];
        next.points[i] = next.points[i + 1]
            + if d.hypot() >= 0.001 {
                d.normalize() * 40.
            } else {
                Vec2::new(-40., 0.)
            };
    }
    for i in anchor + 1..10 {
        let d = next.points[i] - next.points[i - 1];
        next.points[i] = next.points[i - 1]
            + if d.hypot() >= 0.001 {
                d.normalize() * 40.
            } else {
                Vec2::new(40., 0.)
            };
    }
    next
}
impl Example for Chain {
    type State = ChainState;
    fn reset(&self, s: &Self::State) -> Self::State {
        ChainState {
            mode: s.mode,
            ..self.initial()
        }
    }
    fn action_transition(&self, _: &Self::State, _: &str) -> Option<dragology::Transition> {
        None
    }
    fn initial(&self) -> ChainState {
        ChainState {
            points: std::array::from_fn(|i| Point::new(120. + 40. * i as f64, 200.)),
            mode: 0,
        }
    }
    fn scene(&self, s: &ChainState) -> Scene {
        self.scene_dragging(s, None)
    }
    fn scene_dragging(&self, s: &ChainState, dragged: Option<&str>) -> Scene {
        let mut scene = Scene::new(600., 400.);
        for i in 0..9 {
            let object = Object::line(
                format!("link-{i}"),
                s.points[i],
                s.points[i + 1],
                rgb(0x999999),
                2.5,
            )
            .line_cap(LineCap::Round);
            scene.push(object);
        }
        let mut order: Vec<_> = (0..10).collect();
        order.sort_by_key(|i| dragged == Some(format!("dot-{i}").as_str()));
        for i in order {
            let id = format!("dot-{i}");
            let active = dragged == Some(id.as_str());
            let object = stroked_circle(
                id,
                s.points[i],
                8.,
                rgb(0x555555),
                rgb(if active { 0x333333 } else { 0x888888 }),
                if active { 2.5 } else { 1.5 },
            );
            scene.push(object);
        }
        source_cursors(scene)
    }
    fn locate(&self, s: &ChainState, id: &str) -> Option<Affine> {
        Some(Affine::translate(
            s.points.get(index(id, "dot-")?)?.to_vec2(),
        ))
    }
    fn drag(
        &self,
        s: &ChainState,
        id: &str,
        _: Modifiers,
    ) -> Option<DragSpec<'static, ChainState>> {
        let i = index(id, "dot-")?;
        let p = *s.points.get(i)?;
        let start = s.clone();
        let spec = vary([p.x, p.y], move |[x, y]| {
            let mut next = start.clone();
            next.points[i] = Point::new(x, y);
            next
        })
        .during(move |s| follow_chain(s, i));
        Some(match s.mode {
            0 => spec.with_chaining_options(dragology::Chaining {
                dragged_id: None,
                transition: None,
            }),
            1 => spec.with_chaining(),
            _ => spec,
        })
    }
    fn controls(&self, s: &ChainState) -> Vec<Control> {
        vec![Control::Choice {
            id: "mode",
            label: "Mode".into(),
            options: vec![
                "Normal".into(),
                "Forget to disable transitions on chain".into(),
                "Forget to chain at all".into(),
            ],
            selected: s.mode,
        }]
    }
    fn action(&self, s: &ChainState, id: &str, value: Value) -> Option<ChainState> {
        let Value::Index(mode) = value else {
            return None;
        };
        if id != "mode" || mode > 2 {
            return None;
        }
        Some(ChainState { mode, ..s.clone() })
    }
}

#[derive(Clone, PartialEq)]
struct Gear {
    center: Point,
    angle: f64,
    teeth: usize,
}
#[derive(Clone, PartialEq)]
struct GearState {
    gears: [Gear; 5],
}
struct Gears;
fn gear_radius(g: &Gear) -> f64 {
    g.teeth as f64 * 12. / TAU
}
fn connected(a: &Gear, b: &Gear) -> bool {
    (a.center.distance(b.center) - gear_radius(a) - gear_radius(b)).abs() < 2.5
}
fn gear_transform(g: &Gear) -> Affine {
    Affine::translate(g.center.to_vec2()) * Affine::rotate(g.angle.to_radians())
}
fn dock_constraints(s: &GearState, i: usize, other: usize) -> [f64; 4] {
    let a = &s.gears[i];
    let b = &s.gears[other];
    let distance = a.center.distance(b.center) - (gear_radius(a) + gear_radius(b));
    let alpha = (b.center - a.center).atan2().to_degrees();
    let phase =
        (a.angle - alpha) * a.teeth as f64 + (b.angle - (alpha + 180.)) * b.teeth as f64 - (-180.);
    // Source equal(a,b) expands to [a-b,b-a], in these original units.
    [distance, -distance, phase, -phase]
}
fn propagate_absolute(mut s: GearState, from: usize) -> GearState {
    let mut visited = [false; 5];
    visited[from] = true;
    let mut queue = vec![from];
    let mut i = 0;
    while i < queue.len() {
        let p = queue[i];
        i += 1;
        for c in 0..5 {
            if !visited[c] && connected(&s.gears[p], &s.gears[c]) {
                let alpha = (s.gears[c].center - s.gears[p].center).atan2().to_degrees();
                s.gears[c].angle = alpha + 180.
                    - 180. / s.gears[c].teeth as f64
                    - (s.gears[p].angle - alpha) * s.gears[p].teeth as f64
                        / s.gears[c].teeth as f64;
                visited[c] = true;
                queue.push(c);
            }
        }
    }
    s
}
fn propagate_delta(initial: &GearState, current: &GearState, from: usize) -> GearState {
    let mut s = current.clone();
    let delta = s.gears[from].angle - initial.gears[from].angle;
    let mut visited = [false; 5];
    visited[from] = true;
    let mut queue = vec![(from, 1.)];
    let mut i = 0;
    while i < queue.len() {
        let (p, ratio) = queue[i];
        i += 1;
        for c in 0..5 {
            if !visited[c] && connected(&s.gears[p], &s.gears[c]) {
                let ratio = -ratio * s.gears[p].teeth as f64 / s.gears[c].teeth as f64;
                s.gears[c].angle = initial.gears[c].angle + delta * ratio;
                visited[c] = true;
                queue.push((c, ratio));
            }
        }
    }
    s
}
fn gear_outline(g: &Gear) -> BezPath {
    let r = gear_radius(g);
    path(
        (0..g.teeth).flat_map(|tooth| {
            [
                (-0.325, r - 4.),
                (-0.175, r + 4.),
                (0.175, r + 4.),
                (0.325, r - 4.),
            ]
            .map(move |(offset, r)| {
                Point::ORIGIN + polar(r, (tooth as f64 + offset) * TAU / g.teeth as f64)
            })
        }),
        true,
    )
}
impl Example for Gears {
    type State = GearState;
    fn anchor(&self, _: &GearState, id: &str, local: Point) -> Point {
        if id.starts_with("hub-") {
            Point::ORIGIN
        } else {
            local
        }
    }
    fn initial(&self) -> GearState {
        let teeth = [22, 14, 28, 18, 12];
        let mut center = Point::new(140., 180.);
        let mut previous = 0.;
        let gears = std::array::from_fn(|i| {
            let r = teeth[i] as f64 * 12. / TAU;
            if i > 0 {
                center += polar(previous + r, if i % 2 == 0 { -0.4 } else { 0.4 });
            }
            previous = r;
            Gear {
                center,
                angle: 0.,
                teeth: teeth[i],
            }
        });
        propagate_absolute(GearState { gears }, 0)
    }
    fn scene(&self, s: &GearState) -> Scene {
        let mut scene = Scene::new(500., 360.);
        for (i, g) in s.gears.iter().enumerate() {
            let radius = gear_radius(g);
            let color = [
                rgb(0xe74c3c),
                rgb(0xf39c12),
                rgb(0x27ae60),
                rgb(0x3498db),
                rgb(0x9b59b6),
            ][i];
            let darker = [
                rgb(0xc0392b),
                rgb(0xd35400),
                rgb(0x1e8449),
                rgb(0x21618c),
                rgb(0x6c3483),
            ][i];
            let body = Object::new(
                format!("gear-{i}"),
                gear_transform(g),
                Some(Hit::Path(gear_outline(g), 1.2)),
                vec![
                    Shape::Path {
                        path: gear_outline(g),
                        fill: Some(color),
                        stroke: Some((rgb(0x222222), 1.2)),
                    },
                    Shape::Circle {
                        center: Point::ORIGIN,
                        radius: radius * 0.45,
                        fill: [darker[0], darker[1], darker[2], 255],
                        stroke: Some((rgb(0x222222), 1.)),
                    },
                    Shape::Rect {
                        bounds: Rect::new(-2., -radius * 0.45, 2., radius - 4.),
                        radius: 0.,
                        fill: [34, 34, 34, 179],
                        stroke: None,
                    },
                ],
            );
            let components = vec![
                TransformComponent::Translate(g.center.to_vec2()),
                TransformComponent::Rotate {
                    radians: g.angle.to_radians(),
                    center: Point::ORIGIN,
                },
            ];
            scene.push(body.transforms(components.clone()));
            scene.push(
                Object::new(
                    format!("hub-{i}"),
                    gear_transform(g),
                    Some(Hit::Circle(Point::ORIGIN, (radius * 0.32).max(10.))),
                    vec![],
                )
                .transforms(components),
            );
        }
        source_cursors(scene)
    }
    fn locate(&self, s: &GearState, id: &str) -> Option<Affine> {
        let i = index(id, "gear-").or_else(|| index(id, "hub-"))?;
        Some(gear_transform(s.gears.get(i)?))
    }
    fn drag(&self, s: &GearState, id: &str, _: Modifiers) -> Option<DragSpec<'static, GearState>> {
        if let Some(i) = index(id, "gear-") {
            let base = s.clone();
            let initial = s.clone();
            return Some(
                vary([s.gears.get(i)?.angle], move |[a]| {
                    let mut next = base.clone();
                    next.gears[i].angle = a;
                    next
                })
                .during(move |s| propagate_delta(&initial, s, i)),
            );
        }
        let i = index(id, "hub-")?;
        let g = s.gears.get(i)?;
        let mut branches = Vec::new();
        for other in 0..5 {
            if other == i {
                continue;
            }
            let base = s.clone();
            branches.push(vary_constrained(
                [g.center.x, g.center.y, g.angle],
                move |[x, y, angle]| {
                    let mut next = base.clone();
                    next.gears[i].center = Point::new(x, y);
                    next.gears[i].angle = angle;
                    next
                },
                move |candidate| dock_constraints(candidate, i, other),
            ));
        }
        let base = s.clone();
        let p = g.center;
        let free = vary([p.x, p.y], move |[x, y]| {
            let mut next = base.clone();
            next.gears[i].center = Point::new(x, y);
            next
        });
        Some(closest(branches).when_far(free, 10.).with_floating())
    }
}

#[derive(Clone, PartialEq)]
struct Meeting {
    start: f64,
    end: f64,
    room: usize,
}
#[derive(Clone, PartialEq)]
struct Schedule {
    meetings: [Meeting; 5],
}
struct Scheduler;
fn meeting_y(m: &Meeting) -> f64 {
    10. + 20. + m.room as f64 * 48.
}
fn schedule_position(s: &Schedule, id: &str) -> Option<Point> {
    let (kind, i) = id.split_once('-')?;
    let m = s.meetings.get(i.parse::<usize>().ok()?)?;
    Some(Point::new(
        10. + match kind {
            "bar" => m.start,
            "left" => m.start + 7.,
            "right" => m.end - 7.,
            "node" => 290. + (m.start + m.end) / 2.,
            _ => return None,
        },
        meeting_y(m) - if kind == "bar" { 3. } else { 0. },
    ))
}
fn overlaps(a: &Meeting, b: &Meeting) -> bool {
    a.start < b.end && b.start < a.end
}
impl Example for Scheduler {
    type State = Schedule;
    fn initial(&self) -> Schedule {
        Schedule {
            meetings: [
                (20., 100., 0),
                (60., 160., 1),
                (10., 70., 2),
                (120., 200., 0),
                (80., 140., 2),
            ]
            .map(|(start, end, room)| Meeting { start, end, room }),
        }
    }
    fn scene(&self, s: &Schedule) -> Scene {
        self.scene_dragging(s, None)
    }
    fn scene_dragging(&self, s: &Schedule, dragged: Option<&str>) -> Scene {
        let mut scene = Scene::new(560., 212.);
        for i in 0..5 {
            for j in 0..i {
                if overlaps(&s.meetings[i], &s.meetings[j]) {
                    let a = schedule_position(s, &format!("node-{i}")).unwrap();
                    let b = schedule_position(s, &format!("node-{j}")).unwrap();
                    if s.meetings[i].room == s.meetings[j].room {
                        scene.push(Object::line(
                            format!("conflict-{j}-{i}"),
                            a,
                            b,
                            [239, 68, 68, 153],
                            6.,
                        ));
                    }
                    scene.push(Object::line(
                        format!("edge-{j}-{i}"),
                        a,
                        b,
                        rgb(0x94a3b8),
                        1.5,
                    ));
                }
            }
        }
        for room in 0..4 {
            let y = 30. + room as f64 * 48.;
            scene.push(Object::line(
                format!("track-{room}"),
                (10., y),
                (260., y),
                rgb(0xd1d5db),
                1.5,
            ));
            scene.push(
                Object::text(
                    format!("room-{room}"),
                    (260., y + 9.),
                    format!("room {room}"),
                    10.,
                    rgb(0xd1d5db),
                )
                .text_anchor(TextAnchor::End)
                .text_baseline(TextBaseline::Alphabetic),
            );
        }
        let mut order: Vec<_> = (0..5).collect();
        order.sort_by_key(|i| dragged == Some(format!("bar-{i}").as_str()));
        for i in order {
            let m = &s.meetings[i];
            let c = [
                rgb(0x3b82f6),
                rgb(0x06b6d4),
                rgb(0x22c55e),
                rgb(0xf59e0b),
                rgb(0x8b5cf6),
            ][i];
            let origin = schedule_position(s, &format!("bar-{i}")).unwrap();
            scene.push(Object::rect(
                format!("bar-{i}"),
                Rect::from_origin_size(origin, (m.end - m.start, 6.)),
                3.,
                c,
            ));
            for end in ["left", "right"] {
                let id = format!("{end}-{i}");
                scene.push(Object::circle(
                    &id,
                    schedule_position(s, &id).unwrap(),
                    7.,
                    c,
                ));
            }
            let id = format!("node-{i}");
            scene.push(stroked_circle(
                &id,
                schedule_position(s, &id).unwrap(),
                10.,
                c,
                [255; 4],
                2.,
            ));
        }
        source_cursors(scene)
    }
    fn locate(&self, s: &Schedule, id: &str) -> Option<Affine> {
        Some(Affine::translate(schedule_position(s, id)?.to_vec2()))
    }
    fn drag(&self, s: &Schedule, id: &str, _: Modifiers) -> Option<DragSpec<'static, Schedule>> {
        let (kind, i) = id.split_once('-')?;
        let i = i.parse::<usize>().ok()?;
        let meeting = s.meetings.get(i)?;
        match kind {
            "bar" | "node" => Some(
                between((0..4).map(|room| {
                    let mut next = s.clone();
                    next.meetings[i].room = room;
                    next
                }))
                .with_snap_radius_options(
                    10.,
                    dragology::SnapOptions {
                        transition: Some(dragology::Transition::default()),
                        chain: false,
                    },
                ),
            ),
            "left" | "right" => {
                let start = s.clone();
                let left = kind == "left";
                let initial = if left { meeting.start } else { meeting.end };
                Some(vary_constrained(
                    [initial],
                    move |[value]| {
                        let mut next = start.clone();
                        if left {
                            next.meetings[i].start = value;
                        } else {
                            next.meetings[i].end = value;
                        }
                        next
                    },
                    move |s: &Schedule| {
                        let m = &s.meetings[i];
                        [-m.start, 28. - (m.end - m.start), m.end - 250.]
                    },
                ))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chain_lengths_survive_coincident_and_displaced_links() {
        for anchor in 0..10 {
            let mut state = Chain.initial();
            state.points[anchor] = Point::new(300., 300.);
            if anchor > 0 {
                state.points[anchor - 1] = state.points[anchor];
            }
            let next = follow_chain(&state, anchor);
            assert_eq!(next.points[anchor], state.points[anchor]);
            for pair in next.points.windows(2) {
                assert!((pair[0].distance(pair[1]) - 40.).abs() < 1e-9);
            }
        }
    }
    #[test]
    fn meshed_gears_preserve_tooth_phase_and_ratio() {
        let initial = Gears.initial();
        let mut rotated = initial.clone();
        rotated.gears[0].angle += 37.;
        let rotated = propagate_delta(&initial, &rotated, 0);
        for i in 1..5 {
            let a = &initial.gears[i - 1];
            let b = &initial.gears[i];
            let alpha = (b.center - a.center).atan2().to_degrees();
            assert!(
                ((a.angle - alpha) * a.teeth as f64
                    + (b.angle - alpha - 180.) * b.teeth as f64
                    + 180.)
                    .abs()
                    < 1e-8
            );
            let phase = (rotated.gears[i - 1].angle - a.angle) * a.teeth as f64
                + (rotated.gears[i].angle - b.angle) * b.teeth as f64;
            assert!(phase.abs() < 1e-8);
        }
    }
    #[test]
    fn gear_docking_uses_source_distance_and_unnormalized_tooth_phase_equalities() {
        let mut state = Gears.initial();
        // A horizontal22-tooth/14-tooth pair from the source equations:
        // 22*a +14*(0-180) = -180, hence a =2340/22 degrees.
        state.gears[0].center = Point::ORIGIN;
        state.gears[0].angle = 2340. / 22.;
        state.gears[1].center = Point::new(36. * 12. / TAU, 0.);
        state.gears[1].angle = 0.;
        assert!(
            dock_constraints(&state, 0, 1)
                .iter()
                .all(|v| v.abs() < 1e-9)
        );
        state.gears[0].center.x -= 3.;
        state.gears[0].angle += 1.;
        for (actual, expected) in dock_constraints(&state, 0, 1)
            .into_iter()
            .zip([3., -3., 22., -22.])
        {
            assert!((actual - expected).abs() < 1e-9);
        }
    }
}
