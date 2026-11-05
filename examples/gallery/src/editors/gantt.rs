use super::*;
use dragology::{between, closest, drop_target, vary_constrained};

#[derive(Clone, Debug, PartialEq)]
pub struct Task {
    name: &'static str,
    lag: f64,
    duration: f64,
    lane: usize,
}
#[derive(Clone, Debug, PartialEq)]
pub struct State {
    tasks: Vec<Task>,
    deps: Vec<(usize, usize)>,
}
pub struct Gantt;
const DAY: f64 = 13.;
const LEFT: f64 = 44.;
const TOP: f64 = 22.;
const LANE: f64 = 44.;

fn order(s: &State) -> Vec<usize> {
    let mut result = Vec::new();
    while result.len() < s.tasks.len() {
        let Some(i) = (0..s.tasks.len()).find(|i| {
            !result.contains(i)
                && s.deps
                    .iter()
                    .filter(|(_, b)| b == i)
                    .all(|(a, _)| result.contains(a))
        }) else {
            break;
        };
        result.push(i);
    }
    result
}
fn schedule(s: &State) -> Vec<(f64, f64, f64)> {
    let mut out = vec![(0., 0., 0.); s.tasks.len()];
    for i in order(s) {
        let earliest = s
            .deps
            .iter()
            .filter(|(_, b)| *b == i)
            .map(|(a, _)| out[*a].2)
            .fold(0., f64::max);
        let start = earliest + s.tasks[i].lag;
        out[i] = (earliest, start, start + s.tasks[i].duration);
    }
    out
}
fn descendants(s: &State, id: usize) -> Vec<usize> {
    let mut result = vec![id];
    let mut k = 0;
    while k < result.len() {
        for &(a, b) in &s.deps {
            if a == result[k] && !result.contains(&b) {
                result.push(b);
            }
        }
        k += 1;
    }
    result
}
fn with_deps(s: &State, deps: Vec<(usize, usize)>) -> State {
    let before = schedule(s);
    let mut next = s.clone();
    next.deps = deps;
    // Match upstream's single schedule snapshot before assigning any new lags.
    let after = schedule(&next);
    for i in 0..next.tasks.len() {
        next.tasks[i].lag = (before[i].1 - after[i].0).max(0.);
    }
    next
}
fn rect(s: &State, i: usize) -> Rect {
    let times = schedule(s);
    let x = LEFT + DAY * times[i].1;
    let y = TOP + LANE * s.tasks[i].lane as f64 + LANE / 2. - 12.;
    Rect::new(x, y, x + DAY * s.tasks[i].duration, y + 24.)
}
impl Example for Gantt {
    type State = State;
    fn initial(&self) -> State {
        State {
            tasks: [
                ("Plan", 0., 4., 0),
                ("Design", 0., 6., 1),
                ("Infra", 0., 5., 2),
                ("Build", 0., 8., 0),
                ("Docs", 1., 4., 3),
                ("Test", 0., 4., 1),
                ("Ship", 0., 2., 2),
            ]
            .into_iter()
            .map(|(name, lag, duration, lane)| Task {
                name,
                lag,
                duration,
                lane,
            })
            .collect(),
            deps: vec![
                (0, 1),
                (0, 2),
                (1, 3),
                (2, 3),
                (1, 4),
                (3, 5),
                (5, 6),
                (4, 6),
            ],
        }
    }
    fn scene(&self, s: &State) -> Scene {
        self.scene_dragging(s, None)
    }
    fn scene_dragging(&self, s: &State, dragged: Option<&str>) -> Scene {
        let mut scene = Scene::new(574., 216.);
        let times = schedule(s);
        let finish = times.iter().map(|t| t.2).fold(0., f64::max);
        let mut latest = vec![finish; s.tasks.len()];
        for i in order(s).into_iter().rev() {
            latest[i] = s
                .deps
                .iter()
                .filter(|(a, _)| *a == i)
                .map(|(_, b)| latest[*b] - s.tasks[*b].duration)
                .fold(finish, f64::min);
        }
        let critical = |i: usize| latest[i] - times[i].2 < 1e-6;
        let dragging = dragged
            .and_then(|id| id.strip_prefix("bar-"))
            .and_then(|id| id.parse::<usize>().ok());
        let valid = dragging.map(|i| {
            let excluded = descendants(s, i);
            (0..s.tasks.len())
                .filter(|j| !excluded.contains(j) && !s.deps.contains(&(*j, i)))
                .collect::<Vec<_>>()
        });
        for day in (0..=40).step_by(2) {
            let x = LEFT + day as f64 * DAY;
            scene.push(Object::line(
                format!("grid-{day}"),
                (x, TOP),
                (x, TOP + 4. * LANE),
                if day % 10 == 0 {
                    hex_color("d1d5db")
                } else {
                    hex_color("f1f5f9")
                },
                1.,
            ));
            if day % 10 == 0 {
                scene.push(baseline_text(
                    format!("day-{day}"),
                    (x - 13., TOP - 8.),
                    format!("day {day}"),
                    9.,
                    hex_color("9ca3af"),
                ));
            }
        }
        for (lane, name) in ["Ana", "Ben", "Cam", "Dee"].iter().enumerate() {
            let y = TOP + LANE * (lane as f64 + 0.5);
            scene.push(Object::line(
                format!("lane-{lane}"),
                (LEFT, y + LANE / 2.),
                (LEFT + 40. * DAY, y + LANE / 2.),
                hex_color("e5e7eb"),
                1.,
            ));
            scene.push(baseline_text(
                format!("lane-name-{lane}"),
                (LEFT - 8. - name.len() as f64 * 6., y + 4.),
                *name,
                11.,
                hex_color("6b7280"),
            ));
        }
        scene.push(dashed_line(
            "project-end",
            Point::new(LEFT + finish * DAY, TOP),
            Point::new(LEFT + finish * DAY, TOP + 4. * LANE),
            hex_color("ef4444"),
            1.5,
            4.,
            3.,
        ));
        scene.push(baseline_text(
            "project-end-label",
            (LEFT + finish * DAY - 27., TOP + 4. * LANE + 12.),
            format!("done: day {}", finish.round()),
            9.,
            hex_color("ef4444"),
        ));
        for &(a, b) in &s.deps {
            let ra = rect(s, a);
            let rb = rect(s, b);
            let from = Point::new(ra.x1, ra.center().y);
            let to = Point::new(rb.x0, rb.center().y);
            let is_critical = critical(a) && critical(b) && (times[a].2 - times[b].1).abs() < 1e-6;
            let color = if is_critical {
                hex_color("ef4444")
            } else {
                hex_color("94a3b8")
            };
            let middle = from.x + 6f64.max((to.x - from.x) / 2.);
            let mut path = BezPath::new();
            path.move_to(from);
            path.curve_to(
                Point::new(middle, from.y),
                Point::new(middle, to.y),
                Point::new(to.x - 6., to.y),
            );
            let mut edge = Object::path(
                format!("dep-{a}-{b}"),
                path.clone(),
                color,
                if is_critical { 2.5 } else { 1.5 },
            );
            edge.hit = Some(Hit::Path(path, 10.));
            edge.click_handler = true;
            scene.push(edge);
            scene.push(arrow_tip(
                format!("dep-head-{a}-{b}"),
                to,
                Vec2::new(1., 0.),
                7.,
                color,
            ));
        }
        for (i, t) in s.tasks.iter().enumerate() {
            let r = rect(s, i);
            let color = if critical(i) {
                hex_color("ef4444")
            } else {
                hex_color("3b82f6")
            };
            let target = valid.as_ref().is_some_and(|v| v.contains(&i));
            let dim = valid.is_some() && !target && dragging != Some(i);
            let start = scene.objects.len();
            if t.lag > 0. {
                scene.push(dashed_line(
                    format!("lag-{i}"),
                    Point::new(LEFT + times[i].0 * DAY, r.center().y),
                    Point::new(r.x0, r.center().y),
                    color,
                    3.,
                    2.,
                    3.,
                ));
                scene.push(Object::line(
                    format!("lag-start-{i}"),
                    (LEFT + times[i].0 * DAY, r.y0 + 4.),
                    (LEFT + times[i].0 * DAY, r.y1 - 4.),
                    color,
                    2.,
                ));
            }
            let mut bar = Object::rect(format!("bar-{i}"), r, 5., color).baseline_label(
                t.name,
                (14., 16.),
                11.,
                WHITE,
            );
            if let Shape::Rect { stroke, .. } = &mut bar.shapes[0] {
                *stroke = Some((
                    if target { hex_color("111827") } else { WHITE },
                    if target { 2. } else { 1. },
                ));
            }
            if target {
                bar = bar.dash(vec![4., 3.]);
            }
            scene.push(bar);
            if !critical(i) {
                scene.push(baseline_text(
                    format!("slack-{i}"),
                    (r.x1 + 4., r.y0 + 16.),
                    format!("+{}", (latest[i] - times[i].2).round()),
                    9.,
                    hex_color("9ca3af"),
                ));
            }
            let mut grip = Object::rect(
                format!("grip-{i}"),
                Rect::new(r.x0, r.y0, r.x0 + 10., r.y1),
                5.,
                [0, 0, 0, 38],
            );
            grip.parent = Some(format!("bar-{i}"));
            for y in [8., 12., 16.] {
                let mut path = BezPath::new();
                path.move_to((3., y));
                path.line_to((7., y));
                grip.shapes.push(Shape::Path {
                    path,
                    fill: None,
                    stroke: Some(([255, 255, 255, 179], 1.)),
                });
            }
            scene.push(grip);
            let mut end = Object::rect(
                format!("end-{i}"),
                Rect::new(r.x1 - 8., r.y0, r.x1, r.y1),
                4.,
                [0, 0, 0, 38],
            );
            end.parent = Some(format!("bar-{i}"));
            scene.push(end);
            if dim {
                for object in &mut scene.objects[start..] {
                    object.opacity = 0.4;
                }
            }
        }
        scene
    }
    fn drag(&self, s: &State, id: &str, _: Modifiers) -> Option<DragSpec<'static, State>> {
        let (kind, index) = id.split_once('-')?;
        let i: usize = index.parse().ok()?;
        if i >= s.tasks.len() {
            return None;
        }
        if kind == "grip" {
            return Some(
                between((0..4).map(|lane| {
                    let mut n = s.clone();
                    n.tasks[i].lane = lane;
                    n
                }))
                .with_snap_radius_options(
                    12.,
                    dragology::SnapOptions {
                        transition: Some(dragology::Transition::default()),
                        chain: false,
                    },
                ),
            );
        }
        if kind != "bar" && kind != "end" {
            return None;
        }
        let duration = kind == "end";
        let base = s.clone();
        let slide = vary_constrained(
            [if duration {
                s.tasks[i].duration
            } else {
                s.tasks[i].lag
            }],
            move |[value]| {
                let mut n = base.clone();
                if duration {
                    n.tasks[i].duration = value;
                } else {
                    n.tasks[i].lag = value;
                }
                n
            },
            move |state| {
                [
                    if duration {
                        1. - state.tasks[i].duration
                    } else {
                        -state.tasks[i].lag
                    },
                    schedule(state)[i].2 - 40.,
                ]
            },
        );
        if duration {
            return Some(slide);
        }
        let excluded = descendants(s, i);
        let candidates: Vec<_> = (0..s.tasks.len())
            .filter(|j| !excluded.contains(j) && !s.deps.contains(&(*j, i)))
            .map(|j| {
                let mut deps = s.deps.clone();
                deps.push((j, i));
                drop_target(with_deps(s, deps), rect(s, j))
            })
            .collect();
        if candidates.is_empty() {
            Some(slide)
        } else {
            Some(
                closest(candidates)
                    .with_floating_options(dragology::FloatingOptions {
                        ghost_opacity: Some(0.35),
                        tether: None,
                    })
                    .when_far(slide, 50.)
                    .with_branch_transition(Some(transition(120))),
            )
        }
    }
    fn click(&self, s: &State, id: &str, _: Point, _: Modifiers, _: usize) -> Option<State> {
        let parts: Vec<_> = id.split('-').collect();
        if parts.len() != 3 || parts[0] != "dep" {
            return None;
        }
        let pair = (parts[1].parse().ok()?, parts[2].parse().ok()?);
        Some(with_deps(
            s,
            s.deps.iter().copied().filter(|p| *p != pair).collect(),
        ))
    }
    fn status(&self, s: &State) -> String {
        format!(
            "{} dependencies · red tasks are on the critical path",
            s.deps.len()
        )
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn duration_gesture_obeys_source_bounds_and_reschedules_dependents() {
        let initial = Gantt.initial();
        let press = Gantt.locate(&initial, "end-0").unwrap() * Point::ORIGIN;
        for (delta, expected) in [(1000., 40.), (-1000., 1.)] {
            let spec = Gantt.drag(&initial, "end-0", Modifiers::default()).unwrap();
            let mut drag = dragology::DragSession::begin(
                initial.clone(),
                press,
                |s| Gantt.locate(s, "end-0"),
                spec,
            )
            .unwrap();
            let frame = drag.frame(press + Vec2::new(delta, 0.)).unwrap();
            let state = &frame.drop_state;
            // Evaluate the numerical constraint in screen pixels, not against
            // the exact bits produced by an analytic clamp.
            assert!((state.tasks[0].duration - expected).abs() * DAY < 0.02);
            let times = schedule(state);
            assert!((times[1].1 - times[0].2).abs() < 1e-9);
            assert!((times[2].1 - times[0].2).abs() < 1e-9);
        }
    }
    #[test]
    fn dependency_edit_uses_the_upstream_snapshot_lag_rule() {
        let s = Gantt.initial();
        let edited = with_deps(
            &s,
            s.deps
                .iter()
                .copied()
                .filter(|dep| *dep != (0, 1))
                .collect(),
        );
        assert_eq!(edited.tasks[1].lag, 4.);
        assert_eq!(edited.tasks[3].lag, 1.);
        assert_eq!(schedule(&edited)[3].1, 11.);
    }
    #[test]
    fn downstream_follows_and_dependency_removal_preserves_schedule() {
        let s = Gantt.initial();
        let mut moved = s.clone();
        moved.tasks[0].lag = 3.;
        let a = schedule(&s);
        let b = schedule(&moved);
        for i in 0..s.tasks.len() {
            assert!((b[i].1 - a[i].1 - 3.).abs() < 1e-9);
        }
        let removed = with_deps(
            &s,
            s.deps.iter().copied().filter(|p| *p != (0, 1)).collect(),
        );
        assert_eq!(schedule(&removed)[1].1, a[1].1);
        assert_eq!(order(&s).len(), s.tasks.len());
    }
}
