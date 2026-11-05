//! A native port of Dragology's equation manipulation demo, after Graspable Math.
//! Original demo: Dragology contributors, MIT. Algebra operations intentionally
//! follow the upstream expression normal forms and gesture restrictions.
use crate::*;
use dragology::{closest, drop_target, fixed};
use std::collections::HashMap;
mod algebra;
mod layout;
use algebra::*;
use layout::{Role, render};

#[derive(Clone, Debug, PartialEq)]
struct State {
    sides: [Expr; 2],
    preset: usize,
    puff: Option<Puff>,
}
#[derive(Clone, Debug, PartialEq)]
struct Puff {
    expr: Expr,
    sign: Option<i8>,
    at: Point,
    size: f64,
}
impl State {
    fn preset(index: usize) -> Self {
        let (a, b) = parse(EQUATIONS[index]);
        Self {
            sides: [a, b],
            preset: index,
            puff: None,
        }
    }
    fn find(&self, id: u64) -> Option<&Expr> {
        find(&self.sides[0], id).or_else(|| find(&self.sides[1], id))
    }
    fn replace(&self, id: u64, e: Expr) -> Self {
        Self {
            sides: [
                replace(&self.sides[0], id, &e),
                replace(&self.sides[1], id, &e),
            ],
            preset: self.preset,
            puff: None,
        }
    }
    fn side(&self, side: usize, e: Expr) -> Self {
        let mut s = self.clone();
        s.sides[side] = normalize(e);
        s.puff = None;
        s
    }
    fn solved(&self) -> bool {
        (matches!(self.sides[0].kind, Kind::Var(_)) && is_number(&self.sides[1]))
            || (matches!(self.sides[1].kind, Kind::Var(_)) && is_number(&self.sides[0]))
    }
}
struct Math;
pub fn entry() -> Entry {
    Entry::new(
        "graspable-math",
        "Graspable Math",
        "Solve all eleven original equations by moving terms, combining like terms, distributing factors, cancelling fractions, and manipulating powers and logarithms.",
        || demo(Math),
    )
}
const EQUATION_IDS: [&str; 11] = [
    "equation-0",
    "equation-1",
    "equation-2",
    "equation-3",
    "equation-4",
    "equation-5",
    "equation-6",
    "equation-7",
    "equation-8",
    "equation-9",
    "equation-10",
];
const EQUATION_LABELS: [&str; 11] = [
    "2x + 3 = 11",
    "3x − 5 = x + 7",
    "x/4 + 1 = 3",
    "2(x + 1) = 10",
    "(x + 6)/3 = 2x − 3",
    "(x + 3)² = x² + 15",
    "2x³ = 54",
    "x⁵/x² = 8",
    "2ˣ⁺¹ = 32",
    "log₃(x) = 4",
    "log₂(x³) = 6",
];
fn equation_buttons(scene: &mut Scene, selected: usize) -> f64 {
    let mut x = 0.;
    let mut y = 0.;
    for (i, (id, label)) in EQUATION_IDS
        .iter()
        .zip(EQUATION_LABELS)
        .chain(std::iter::once((&"restart", "reset")))
        .enumerate()
    {
        let w = 16.
            + label
                .chars()
                .map(|c| {
                    if "²³⁵ˣ⁺¹₃₂".contains(c) {
                        4.5
                    } else {
                        6.8
                    }
                })
                .sum::<f64>();
        if x > 0. && x + w > 640. {
            x = 0.;
            y += 38.;
        }
        scene
            .inline_control(id, Rect::new(x, y, x + w, y + 30.))
            .font_size(14.)
            .font_family(FontFamily::Serif)
            .foreground(if i == selected {
                [255, 255, 255, 255]
            } else if i == 11 {
                [107, 114, 128, 255]
            } else {
                [55, 65, 81, 255]
            })
            .background(if i == selected {
                [31, 41, 55, 255]
            } else {
                [255, 255, 255, 255]
            })
            .border(if i == selected {
                [31, 41, 55, 255]
            } else {
                [209, 213, 219, 255]
            });
        x += w + 8.;
    }
    y + 42.
}
impl Example for Math {
    type State = State;
    fn initial(&self) -> State {
        State::preset(0)
    }
    fn scene(&self, s: &State) -> Scene {
        render(s).scene
    }
    fn scene_interaction(&self, s: &State, dragged: Option<&str>, hovered: Option<&str>) -> Scene {
        layout::render_interaction(s, dragged, hovered).scene
    }
    fn scene_dragging(&self, s: &State, dragged: Option<&str>) -> Scene {
        layout::render_interaction(s, dragged, None).scene
    }
    fn drag(&self, s: &State, id: &str, _: Modifiers) -> Option<DragSpec<'static, State>> {
        let layout = render(s);
        let role = layout.roles.get(id)?.clone();
        Some(spec(s, &role, &layout.bounds))
    }
    fn controls(&self, s: &State) -> Vec<Control> {
        EQUATION_IDS
            .iter()
            .zip(EQUATION_LABELS)
            .enumerate()
            .map(|(i, (id, label))| Control::action_button(id, label, true, s.preset == i))
            .chain(std::iter::once(Control::action_button(
                "restart", "reset", true, false,
            )))
            .collect()
    }
    fn action_transition(&self, _: &State, _: &str) -> Option<dragology::Transition> {
        None
    }
    fn action(&self, s: &State, id: &str, value: Value) -> Option<State> {
        if let Some(i) = EQUATION_IDS.iter().position(|preset| *preset == id) {
            return Some(State::preset(i));
        }
        match (id, value) {
            ("equation", Value::Index(i)) if i < EQUATIONS.len() => Some(State::preset(i)),
            ("restart", _) => Some(State::preset(s.preset)),
            _ => None,
        }
    }
    fn status(&self, s: &State) -> String {
        if s.solved() {
            "Solved! Pick another equation to continue.".into()
        } else {
            "Drag across = to invert an operation. Drop onto another term or factor to combine. Grab parentheses to move a whole sum.".into()
        }
    }
}
#[derive(Clone, Copy, Debug)]
struct NodeBounds {
    origin: Point,
    bounds: Rect,
    size: f64,
}
fn merge_target(
    s: &State,
    bounds: &HashMap<u64, NodeBounds>,
    target: u64,
    dragged: &Expr,
    sign: Option<i8>,
    merged: State,
) -> DragSpec<'static, State> {
    let b = bounds[&target];
    let mut preview = merged.clone();
    preview.puff = Some(Puff {
        expr: dragged.clone(),
        sign,
        at: b.origin,
        size: bounds.get(&dragged.id).map_or(b.size, |d| d.size),
    });
    let _ = s;
    drop_target(preview, b.bounds).on_drop(move |_| merged.clone())
}
fn set_terms(s: &State, owner: u64, ts: Vec<Term>) -> State {
    let e = s.find(owner).unwrap();
    s.replace(
        owner,
        sum(
            ts,
            if matches!(e.kind, Kind::Sum(_)) {
                owner
            } else {
                new_id()
            },
        ),
    )
}
fn factors_for(s: &State, owner: u64, part: Option<bool>) -> Vec<Expr> {
    let e = s.find(owner).unwrap();
    match (&e.kind, part) {
        (Kind::Frac(a, b), Some(top)) => factors(if top { a } else { b }),
        _ => factors(e),
    }
}
fn set_factors(s: &State, owner: u64, part: Option<bool>, fs: Vec<Expr>) -> State {
    let e = s.find(owner).unwrap();
    let replacement = match (&e.kind, part) {
        (Kind::Frac(a, b), Some(top)) => {
            let current = if top { a } else { b };
            let next = prod(
                fs,
                if matches!(current.kind, Kind::Prod(_)) {
                    current.id
                } else {
                    new_id()
                },
            );
            if top {
                frac(next, (**b).clone(), owner)
            } else {
                frac((**a).clone(), next, owner)
            }
        }
        _ => prod(
            fs,
            if matches!(e.kind, Kind::Prod(_)) {
                owner
            } else {
                new_id()
            },
        ),
    };
    s.replace(owner, replacement)
}
fn set_both(s: &State, owner: u64, top: bool, mine: Vec<Expr>, theirs: Vec<Expr>) -> State {
    let Kind::Frac(a, b) = &s.find(owner).unwrap().kind else {
        unreachable!()
    };
    let (top_fs, bottom_fs) = if top { (mine, theirs) } else { (theirs, mine) };
    s.replace(
        owner,
        frac(
            prod(
                top_fs,
                if matches!(a.kind, Kind::Prod(_)) {
                    a.id
                } else {
                    new_id()
                },
            ),
            prod(
                bottom_fs,
                if matches!(b.kind, Kind::Prod(_)) {
                    b.id
                } else {
                    new_id()
                },
            ),
            owner,
        ),
    )
}
fn reorder_specs<T: Clone>(
    items: &[T],
    ids: &[u64],
    idx: usize,
    mergeable: &[usize],
    bounds: &HashMap<u64, NodeBounds>,
    set: impl Fn(Vec<T>) -> State,
) -> Vec<DragSpec<'static, State>> {
    (0..items.len())
        .map(|j| {
            let mut out = items.to_vec();
            let item = out.remove(idx);
            out.insert(j, item);
            let state = set(out);
            let passed: Vec<_> = (idx.min(j)..=idx.max(j))
                .filter(|k| *k != idx && mergeable.contains(k))
                .map(|k| bounds[&ids[k]].bounds)
                .collect();
            let mut branch = fixed(state);
            if passed.is_empty() {
                return branch;
            }
            let left = passed.iter().map(|b| b.x0).fold(f64::INFINITY, f64::min);
            let right = passed
                .iter()
                .map(|b| b.x1)
                .fold(f64::NEG_INFINITY, f64::max);
            DragSpec::custom(move |ctx, pointer| {
                let mut frame = branch.evaluate(ctx, pointer)?;
                if if j < idx {
                    pointer.x >= left
                } else {
                    pointer.x <= right
                } {
                    frame.gap = f64::INFINITY
                }
                Ok(frame)
            })
        })
        .collect()
}
fn spec(s: &State, role: &Role, bounds: &HashMap<u64, NodeBounds>) -> DragSpec<'static, State> {
    let mut targets = Vec::new();
    match *role {
        Role::Term {
            owner,
            index,
            across,
        } => {
            let ts = terms(s.find(owner).unwrap());
            let t = &ts[index];
            let mut mergeable = Vec::new();
            for (k, u) in ts.iter().enumerate() {
                if k == index {
                    continue;
                }
                if let Some(combined) = combine(t, u) {
                    let mut next = Vec::new();
                    for (i, v) in ts.iter().enumerate() {
                        if i == k {
                            next.extend(combined.clone())
                        } else if i != index {
                            next.push(v.clone())
                        }
                    }
                    mergeable.push(k);
                    targets.push(merge_target(
                        s,
                        bounds,
                        u.expr.id,
                        &t.expr,
                        Some(t.sign),
                        set_terms(s, owner, next),
                    ));
                }
            }
            let ids: Vec<_> = ts.iter().map(|t| t.expr.id).collect();
            targets.extend(reorder_specs(&ts, &ids, index, &mergeable, bounds, |ts| {
                set_terms(s, owner, ts)
            }));
            if let Some(side) = across {
                let mut without = ts.clone();
                without.remove(index);
                let base = set_terms(s, owner, without);
                let there = &s.sides[1 - side];
                let there_ts = if is_num(there, 0.) {
                    Vec::new()
                } else {
                    terms(there)
                };
                for j in 0..=there_ts.len() {
                    let mut next = there_ts.clone();
                    next.insert(
                        j,
                        Term {
                            sign: -t.sign,
                            expr: t.expr.clone(),
                        },
                    );
                    targets.push(fixed(base.side(
                        1 - side,
                        sum(
                            next,
                            if matches!(there.kind, Kind::Sum(_)) {
                                there.id
                            } else {
                                new_id()
                            },
                        ),
                    )));
                }
            }
        }
        Role::Factor {
            owner,
            part,
            index,
            across,
        } => {
            let fs = factors_for(s, owner, part);
            let f = &fs[index];
            let mut mergeable = Vec::new();
            for (k, g) in fs.iter().enumerate() {
                if k == index {
                    continue;
                }
                let merged = distribute(f, g).or_else(|| merge(f, g));
                if let Some(m) = merged {
                    let next = fs
                        .iter()
                        .enumerate()
                        .filter_map(|(i, h)| {
                            if i == index {
                                None
                            } else if i == k {
                                Some(m.clone())
                            } else {
                                Some(h.clone())
                            }
                        })
                        .collect();
                    mergeable.push(k);
                    targets.push(merge_target(
                        s,
                        bounds,
                        g.id,
                        f,
                        None,
                        set_factors(s, owner, part, next),
                    ));
                }
            }
            if let Some(top) = part {
                let opp = factors_for(s, owner, Some(!top));
                for (k, g) in opp.iter().enumerate() {
                    if let Some((fm, gm)) = cancel(f, g) {
                        let mine = fs
                            .iter()
                            .enumerate()
                            .filter_map(|(i, h)| {
                                if i == index {
                                    fm.clone()
                                } else {
                                    Some(h.clone())
                                }
                            })
                            .collect();
                        let theirs = opp
                            .iter()
                            .enumerate()
                            .filter_map(|(i, h)| if i == k { gm.clone() } else { Some(h.clone()) })
                            .collect();
                        targets.push(merge_target(
                            s,
                            bounds,
                            g.id,
                            f,
                            None,
                            set_both(s, owner, top, mine, theirs),
                        ));
                    }
                }
            }
            let ids: Vec<_> = fs.iter().map(|f| f.id).collect();
            targets.extend(reorder_specs(&fs, &ids, index, &mergeable, bounds, |fs| {
                set_factors(s, owner, part, fs)
            }));
            if let Some(side) = across {
                if !is_num(f, 0.) {
                    let mut without = fs.clone();
                    without.remove(index);
                    let base = set_factors(s, owner, part, without);
                    let options = if part == Some(false) {
                        multiplied(&s.sides[1 - side], f)
                    } else {
                        divided(&s.sides[1 - side], f)
                    };
                    for e in options {
                        targets.push(fixed(base.side(1 - side, e)));
                    }
                }
            }
        }
        Role::Exponent {
            owner,
            across,
            log: log_owner,
        } => {
            let Kind::Pow(base, exp) = &s.find(owner).unwrap().kind else {
                unreachable!()
            };
            let n = if let Kind::Num(n) = exp.kind {
                Some(n)
            } else {
                None
            };
            let worked = eval_pow(base, exp).map(|v| num_id(v, base.id)).or_else(|| {
                n.filter(|n| (2. ..=5.).contains(n)).map(|n| {
                    let mut fs = vec![(**base).clone()];
                    fs.extend((1..n as usize).map(|_| fresh(base)));
                    prod(fs, new_id())
                })
            });
            if let Some(worked) = worked {
                targets.push(merge_target(
                    s,
                    bounds,
                    base.id,
                    exp,
                    None,
                    s.replace(owner, worked),
                ));
            }
            if let (Some(side), Some(n)) = (across, n) {
                if n % 2. == 1. {
                    targets.push(fixed(s.side(side, (**base).clone()).side(
                        1 - side,
                        pow(
                            s.sides[1 - side].clone(),
                            frac(num(1.), (**exp).clone(), new_id()),
                            new_id(),
                        ),
                    )));
                }
            }
            if let Some(log_id) = log_owner {
                let Kind::Log(log_base, _) = &s.find(log_id).unwrap().kind else {
                    unreachable!()
                };
                let pulled = prod(
                    vec![
                        (**exp).clone(),
                        log((**log_base).clone(), (**base).clone(), log_id),
                    ],
                    new_id(),
                );
                targets.push(fixed(s.replace(log_id, pulled)));
            }
        }
        Role::PowerBase { owner, side } => {
            let Kind::Pow(base, exp) = &s.find(owner).unwrap().kind else {
                unreachable!()
            };
            targets.push(fixed(s.side(side, (**exp).clone()).side(
                1 - side,
                log((**base).clone(), s.sides[1 - side].clone(), new_id()),
            )));
        }
        Role::LogBase { owner, across } => {
            let Kind::Log(base, arg) = &s.find(owner).unwrap().kind else {
                unreachable!()
            };
            if let Some(v) = eval_log(base, arg) {
                targets.push(merge_target(
                    s,
                    bounds,
                    arg.id,
                    base,
                    None,
                    s.replace(owner, v),
                ));
            }
            if let Some(side) = across {
                targets.push(fixed(s.side(side, (**arg).clone()).side(
                    1 - side,
                    pow((**base).clone(), s.sides[1 - side].clone(), new_id()),
                )));
            }
        }
    }
    closest(targets).when_far(s.clone(), 50.).with_floating()
}

#[cfg(test)]
mod tests {
    use super::*;
    use dragology::DragSession;
    fn value(e: &Expr, x: f64) -> f64 {
        match &e.kind {
            Kind::Num(v) => *v,
            Kind::Var(_) => x,
            Kind::Sum(ts) => ts.iter().map(|t| t.sign as f64 * value(&t.expr, x)).sum(),
            Kind::Prod(fs) => fs.iter().map(|f| value(f, x)).product(),
            Kind::Frac(a, b) => value(a, x) / value(b, x),
            Kind::Pow(a, b) => value(a, x).powf(value(b, x)),
            Kind::Log(a, b) => value(b, x).log(value(a, x)),
        }
    }
    fn balanced(s: &State, x: f64) {
        let a = value(&s.sides[0], x);
        let b = value(&s.sides[1], x);
        assert!(
            a.is_finite() && b.is_finite() && (a - b).abs() < 1e-7 * a.abs().max(b.abs()).max(1.),
            "{} = {} at x={x} ({a} vs {b})",
            key(&s.sides[0]),
            key(&s.sides[1])
        );
    }
    fn gesture(s: State, dragged: u64, point: Point) -> State {
        let id = format!("node-{dragged}");
        let press = Math.locate(&s, &id).unwrap() * Point::new(3., -5.);
        let spec = Math.drag(&s, &id, Modifiers::default()).unwrap();
        DragSession::begin(s, press, move |s| Math.locate(s, &id), spec)
            .unwrap()
            .finish(point)
            .unwrap()
    }
    #[test]
    fn equation_buttons_are_all_reachable_and_selected_button_resets() {
        let mut state = State::preset(0);
        for (i, id) in EQUATION_IDS.iter().enumerate() {
            state = Math.action(&state, id, Value::Trigger).unwrap();
            assert_eq!(state.preset, i);
            let reset = Math.action(&state, id, Value::Trigger).unwrap();
            assert_ne!(state.sides[0].id, reset.sides[0].id);
            assert_eq!(key(&state.sides[0]), key(&reset.sides[0]));
            let scene = Math.scene(&reset);
            assert_eq!(scene.inline_controls.len(), 12);
            assert!(
                scene
                    .inline_controls
                    .iter()
                    .all(|p| p.bounds.width() > 0. && p.bounds.y1 < scene.height)
            );
        }
    }
    #[test]
    fn solved_halo_is_idle_only_and_merge_cloud_keeps_the_dragged_identity() {
        let (lhs, rhs) = parse("x = 4");
        let s = State {
            sides: [lhs, rhs],
            preset: 0,
            puff: None,
        };
        assert!(Math.scene(&s).object("solved-glow").is_some());
        assert!(
            Math.scene_dragging(&s, Some("node-dragged"))
                .object("solved-glow")
                .is_none()
        );
        let s = State::preset(0);
        let ts = terms(&s.sides[0]);
        let dragged = ts[1].expr.clone();
        let mut preview = s.clone();
        preview.puff = Some(Puff {
            expr: dragged.clone(),
            sign: Some(ts[1].sign),
            at: Point::new(100., 128.),
            size: 1.,
        });
        // A real merge removes the dragged expression from both equation sides.
        preview.sides[0] = ts[0].expr.clone();
        let sc = Math.scene_dragging(&preview, Some(&format!("node-{}", dragged.id)));
        let root_id = format!("node-{}", dragged.id);
        let root = sc.object(&root_id).unwrap();
        assert_eq!(root.opacity, 0.);
        assert!(root.hit.is_none());
        let cloud = sc.object(&format!("{root_id}-puff")).unwrap();
        assert_eq!(cloud.parent.as_deref(), Some(root_id.as_str()));
        assert_eq!(cloud.shapes.len(), 13); // Five outline/fill pairs and three wisps, upstream drawPoof.
        assert_eq!(cloud.opacity, 0.75);
        assert!(
            cloud
                .shapes
                .iter()
                .all(|shape| matches!(shape, Shape::Circle { .. }))
        );
    }
    #[test]
    fn compound_merge_preserves_hidden_children_at_the_puff_destination() {
        let (lhs, rhs) = parse("2x + 3x = 10");
        let state = State {
            sides: [lhs, rhs],
            preset: 0,
            puff: None,
        };
        let ts = terms(&state.sides[0]);
        let dragged = &ts[0].expr;
        let before = render(&state);
        let destination = before.bounds[&ts[1].expr.id].origin;
        let mut preview = set_terms(&state, state.sides[0].id, combine(&ts[0], &ts[1]).unwrap());
        preview.puff = Some(Puff {
            expr: dragged.clone(),
            sign: Some(ts[0].sign),
            at: destination,
            size: 1.,
        });
        let scene = Math.scene_dragging(&preview, Some(&format!("node-{}", dragged.id)));
        let root_id = format!("node-{}", dragged.id);
        let old_origin = before.bounds[&dragged.id].origin;
        for child in factors(dragged) {
            let object = scene.object(&format!("node-{}", child.id)).unwrap();
            assert_eq!(object.parent.as_deref(), Some(root_id.as_str()));
            assert_eq!(object.opacity, 0.);
            assert!(object.hit.is_none());
            assert_eq!(
                object.transform * Point::ORIGIN,
                destination + (before.bounds[&child.id].origin - old_origin),
            );
        }
        assert!(scene.object(&format!("{root_id}-puff")).is_some());
        assert_eq!(
            scene
                .objects
                .iter()
                .map(|o| &o.id)
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            scene.objects.len(),
        );
    }
    #[test]
    fn solve_first_equation_with_four_actual_drag_gestures() {
        let mut state = State::preset(0);
        let ts = terms(&state.sides[0]);
        let three = ts[1].expr.clone();
        let eleven = state.sides[1].clone();
        let expected = state.side(0, ts[0].expr.clone()).side(
            1,
            sum(
                vec![
                    positive(eleven.clone()),
                    Term {
                        sign: -1,
                        expr: three.clone(),
                    },
                ],
                new_id(),
            ),
        );
        let target = render(&expected).bounds[&three.id].origin + Vec2::new(3., -5.);
        state = gesture(state, three.id, target);
        assert_eq!(key(&state.sides[1]), "(+11-3)");
        let target = render(&state).bounds[&eleven.id].bounds.center();
        state = gesture(state, three.id, target);
        assert!(is_num(&state.sides[1], 8.));
        let fs = factors(&state.sides[0]);
        let two = fs[0].clone();
        let expected = state
            .side(0, fs[1].clone())
            .side(1, frac(state.sides[1].clone(), two.clone(), new_id()));
        let target = render(&expected).bounds[&two.id].origin + Vec2::new(3., -5.);
        state = gesture(state, two.id, target);
        let Kind::Frac(eight, den) = &state.sides[1].kind else {
            panic!("factor did not divide other side")
        };
        let id = eight.id;
        let target = render(&state).bounds[&den.id].bounds.center();
        state = gesture(state, id, target);
        assert!(state.solved());
        assert!(is_num(&state.sides[1], 4.));
        assert!(state.puff.is_none(), "merge cloud is preview-only");
    }
    #[test]
    fn negative_power_keeps_its_sign_and_even_power_has_no_single_root_move() {
        let (lhs, rhs) = parse("-2^x = -8");
        let state = State {
            sides: [lhs, rhs],
            preset: 0,
            puff: None,
        };
        assert!(
            !render(&state)
                .roles
                .values()
                .any(|r| matches!(r, Role::PowerBase { .. })),
            "taking a log must not discard a leading minus"
        );
        let (lhs, rhs) = parse("x^2 = 4");
        let state = State {
            sides: [lhs, rhs],
            preset: 0,
            puff: None,
        };
        let layout = render(&state);
        let (id, role) = layout
            .roles
            .iter()
            .find(|(_, r)| matches!(r, Role::Exponent { .. }))
            .unwrap();
        let key = id.clone();
        let press = Math.locate(&state, id).unwrap() * Point::ORIGIN;
        let mut session = DragSession::begin(
            state.clone(),
            press,
            move |s| Math.locate(s, &key),
            spec(&state, role, &layout.bounds),
        )
        .unwrap();
        for y in [85., 128., 165.] {
            for x in [300., 350., 400., 450.] {
                let frame = session
                    .frame(Point::new(
                        x,
                        y + equation_buttons(&mut Scene::new(640., 240.), state.preset),
                    ))
                    .unwrap();
                assert!(
                    !matches!(frame.drop_state.sides[0].kind, Kind::Var(_)),
                    "even roots have two solutions and cannot be moved across as one"
                );
            }
        }
    }
    #[test]
    fn every_original_equation_parses_and_draws_unique_nodes() {
        for (i, x) in [4., 6., 8., 4., 3., 1., 3., 2., 4., 81., 4.]
            .into_iter()
            .enumerate()
        {
            let s = State::preset(i);
            balanced(&s, x);
            let layout = render(&s);
            let ids: std::collections::HashSet<_> =
                layout.scene.objects.iter().map(|o| o.id.as_str()).collect();
            assert_eq!(ids.len(), layout.scene.objects.len(), "{}", EQUATIONS[i]);
            assert!(!layout.roles.is_empty());
            for b in layout.bounds.values() {
                assert!(b.origin.is_finite() && b.bounds.width() > 0. && b.bounds.height() > 0.);
            }
        }
    }
    #[test]
    fn candidate_gestures_preserve_all_eleven_equation_solutions() {
        let mut changed = 0;
        for (i, x) in [4., 6., 8., 4., 3., 1., 3., 2., 4., 81., 4.]
            .into_iter()
            .enumerate()
        {
            let s = State::preset(i);
            let layout = render(&s);
            for (id, role) in &layout.roles {
                let id = id.clone();
                let object = layout.scene.object(&id).unwrap();
                let press = object.transform * Point::ORIGIN;
                let key = id.clone();
                let mut session = DragSession::begin(
                    s.clone(),
                    press,
                    move |s| Math.locate(s, &key),
                    spec(&s, role, &layout.bounds),
                )
                .unwrap();
                for py in [55., 85., 115., 128., 150., 175., 205.] {
                    for px in (50..610).step_by(35) {
                        let frame = session
                            .frame(Point::new(
                                px as f64,
                                py + equation_buttons(&mut Scene::new(640., 240.), s.preset),
                            ))
                            .unwrap_or_else(|err| panic!("{} / {id}: {err}", EQUATIONS[i]));
                        balanced(&frame.drop_state, x);
                        changed += usize::from(*frame.drop_state != s);
                    }
                }
            }
        }
        assert!(changed > 50, "gestures must reach actual equation rewrites");
    }
    #[test]
    fn algebra_combines_distributes_cancels_and_evaluates() {
        let (a, _) = parse("3x - x = 0");
        let ts = terms(&a);
        let combined = combine(&ts[0], &ts[1]).unwrap();
        assert_eq!(value(&sum(combined, new_id()), 5.), 10.);
        let (a, _) = parse("2(x + 1) = 0");
        let fs = factors(&a);
        assert_eq!(value(&distribute(&fs[0], &fs[1]).unwrap(), 7.), 16.);
        let (a, b) = parse("x^5 = x^2");
        let (c, d) = cancel(&a, &b).unwrap();
        assert!(d.is_none());
        assert_eq!(value(&c.unwrap(), 2.), 8.);
        let (a, b) = parse("27 = 1/3");
        assert_eq!(eval_pow(&a, &b), Some(3.));
        let (a, b) = parse("2 = 64");
        assert_eq!(value(&eval_log(&a, &b).unwrap(), 0.), 6.);
        let (a, b) = parse("2 = 3");
        assert!(eval_log(&a, &b).is_none());
    }
}
