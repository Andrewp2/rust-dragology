use super::*;
use dragology::{ClosestOptions, closest_with};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq)]
struct Commit {
    id: usize,
    parent: Option<usize>,
    message: String,
    seq: usize,
    hash: String,
    ghost: bool,
}
#[derive(Clone, Debug, PartialEq)]
pub struct State {
    commits: BTreeMap<usize, Commit>,
    refs: BTreeMap<String, usize>,
    next: usize,
    next_seq: usize,
}
pub struct GitGraph;

fn fnv(text: &str) -> String {
    let hash = text.encode_utf16().fold(0x811c9dc5u32, |h, c| {
        (h ^ u32::from(c)).wrapping_mul(0x01000193)
    });
    format!("{hash:08x}")[..7].to_string()
}
fn hash_of(s: &mut State, id: usize) -> String {
    let c = s.commits[&id].clone();
    if c.ghost {
        return c.hash;
    }
    let parent = c.parent.map(|p| hash_of(s, p)).unwrap_or_default();
    let hash = fnv(&format!("{}\n{}\n{}", parent, c.message, c.seq));
    s.commits.get_mut(&id).unwrap().hash = hash.clone();
    hash
}
fn finalize(s: &mut State) {
    for id in s.commits.keys().copied().collect::<Vec<_>>() {
        hash_of(s, id);
    }
    let mut live = BTreeSet::new();
    for &tip in s.refs.values() {
        let mut cursor = Some(tip);
        while let Some(id) = cursor {
            if !live.insert(id) {
                break;
            }
            cursor = s.commits[&id].parent;
        }
    }
    for c in s.commits.values_mut() {
        c.ghost |= !live.contains(&c.id);
    }
}
fn children(s: &State, id: Option<usize>, ghosts: bool) -> Vec<usize> {
    let mut kids: Vec<_> = s
        .commits
        .values()
        .filter(|c| c.parent == id && (ghosts || !c.ghost))
        .collect();
    kids.sort_by_key(|c| c.seq);
    kids.into_iter().map(|c| c.id).collect()
}
fn descendants(s: &State, id: usize) -> Vec<usize> {
    let mut out = vec![id];
    let mut i = 0;
    while i < out.len() {
        out.extend(children(s, Some(out[i]), false));
        i += 1;
    }
    out
}
fn clean(s: &State) -> State {
    let mut next = s.clone();
    next.commits.retain(|_, c| !c.ghost);
    next
}
fn reparent(s: &State, id: usize, parent: usize) -> State {
    let mut n = s.clone();
    let subtree = descendants(s, id);
    let copies: BTreeMap<_, _> = subtree
        .iter()
        .enumerate()
        .map(|(i, id)| (*id, n.next + i))
        .collect();
    n.next += subtree.len();
    for child in &subtree {
        let mut ghost = n.commits[child].clone();
        ghost.id = copies[child];
        if *child != id {
            ghost.parent = ghost.parent.and_then(|p| copies.get(&p).copied());
        }
        ghost.ghost = true;
        n.commits.insert(ghost.id, ghost);
    }
    n.commits.get_mut(&id).unwrap().parent = Some(parent);
    for child in subtree {
        n.commits.get_mut(&child).unwrap().seq = n.next_seq;
        n.next_seq += 1;
    }
    finalize(&mut n);
    n
}
fn linear_run(s: &State, id: usize) -> Vec<usize> {
    let mut first = id;
    while let Some(p) = s.commits[&first].parent {
        if children(s, Some(p), false).len() != 1 {
            break;
        }
        first = p;
    }
    let mut run = vec![first];
    loop {
        let kids = children(s, Some(*run.last().unwrap()), false);
        if kids.len() != 1 {
            break;
        }
        run.push(kids[0]);
    }
    run
}
fn reorder(s: &State, run: &[usize], new_order: &[usize]) -> State {
    let mut n = s.clone();
    let parent = s.commits[&run[0]].parent;
    let kids = children(s, run.last().copied(), false);
    for (i, id) in new_order.iter().enumerate() {
        n.commits.get_mut(id).unwrap().parent = if i == 0 {
            parent
        } else {
            Some(new_order[i - 1])
        };
    }
    for kid in kids {
        n.commits.get_mut(&kid).unwrap().parent = new_order.last().copied();
    }
    finalize(&mut n);
    n
}
fn cherry(s: &State, id: usize, branch: &str) -> State {
    let mut n = s.clone();
    let copy_id = n.next;
    n.next += 1;
    let mut copy = n.commits[&id].clone();
    copy.id = copy_id;
    for child in children(s, Some(id), false) {
        n.commits.get_mut(&child).unwrap().parent = Some(copy_id);
    }
    for tip in n.refs.values_mut() {
        if *tip == id {
            *tip = copy_id;
        }
    }
    n.commits.insert(copy_id, copy);
    let target = s.refs[branch];
    let original = n.commits.get_mut(&id).unwrap();
    original.parent = Some(target);
    original.seq = n.next_seq;
    n.next_seq += 1;
    // If the target was the original commit's descendant, its ancestry now
    // reaches the preserved copy. The picked commit is therefore still acyclic.
    n.refs.insert(branch.to_string(), id);
    finalize(&mut n);
    n
}
fn positions(s: &State) -> BTreeMap<usize, (usize, usize)> {
    fn width(s: &State, id: usize) -> usize {
        children(s, Some(id), true)
            .into_iter()
            .map(|k| width(s, k))
            .sum::<usize>()
            .max(1)
    }
    fn visit(
        s: &State,
        id: usize,
        lane: usize,
        row: &mut usize,
        out: &mut BTreeMap<usize, (usize, usize)>,
    ) {
        out.insert(id, (*row, lane));
        *row += 1;
        let kids = children(s, Some(id), true);
        let mut lanes = Vec::new();
        let mut at = lane;
        for kid in &kids {
            lanes.push(at);
            at += width(s, *kid);
        }
        for (kid, lane) in kids.into_iter().zip(lanes).rev() {
            visit(s, kid, lane, row, out);
        }
    }
    let mut out = BTreeMap::new();
    let mut row = 0;
    let mut lane = 0;
    for root in children(s, None, true) {
        visit(s, root, lane, &mut row, &mut out);
        lane += width(s, root);
    }
    out
}
impl Example for GitGraph {
    type State = State;
    fn initial(&self) -> State {
        let mut s = State {
            commits: BTreeMap::new(),
            refs: BTreeMap::new(),
            next: 8,
            next_seq: 8,
        };
        for (id, (message, parent)) in [
            ("Initial commit", None),
            ("Add README", Some(0)),
            ("Set up build", Some(1)),
            ("Add CLI entry point", Some(2)),
            ("Add login form", Some(1)),
            ("Wire up auth", Some(4)),
            ("Fix typo in login", Some(5)),
            ("Bump version", Some(3)),
        ]
        .into_iter()
        .enumerate()
        {
            s.commits.insert(
                id,
                Commit {
                    id,
                    parent,
                    message: message.into(),
                    seq: id,
                    hash: String::new(),
                    ghost: false,
                },
            );
        }
        s.refs = BTreeMap::from([
            ("main".into(), 3),
            ("feature".into(), 6),
            ("hotfix".into(), 7),
        ]);
        finalize(&mut s);
        s
    }
    fn scene(&self, s: &State) -> Scene {
        let positions = positions(s);
        let mut scene = Scene::new(480., 440.);
        let lane_color = |lane: usize| {
            hex_color(["3b82f6", "22c55e", "f59e0b", "a855f7", "ec4899", "14b8a6"][lane % 6])
        };
        let mut ordered = s.commits.values().collect::<Vec<_>>();
        ordered.sort_by_key(|c| positions[&c.id].0);
        for c in &ordered {
            if let Some(parent) = c.parent {
                let (row, lane) = positions[&c.id];
                let (pr, pl) = positions[&parent];
                let x = 24. + lane as f64 * 22.;
                let y = 20. + row as f64 * 26.;
                let px = 24. + pl as f64 * 22.;
                let py = 20. + pr as f64 * 26.;
                let mut path = BezPath::new();
                path.move_to((x, y));
                path.line_to((x, py + 26.));
                path.quad_to((x, py), (px, py));
                let mut edge = Object::path(
                    format!("edge-{}", c.id),
                    path,
                    if c.ghost {
                        hex_color("d1d5db")
                    } else {
                        lane_color(lane)
                    },
                    2.,
                );
                if c.ghost {
                    edge = edge.dash(vec![3., 3.]);
                }
                scene.push(edge);
            }
        }
        for c in ordered {
            let (row, lane) = positions[&c.id];
            let x = 24. + lane as f64 * 22.;
            let y = 20. + row as f64 * 26.;
            let color = if c.ghost {
                hex_color("9ca3af")
            } else {
                lane_color(lane)
            };
            let rowid = format!("commit-{}", c.id);
            let mut cursor = 218.;
            let mut chips = Vec::new();
            for branch in ["main", "feature", "hotfix"] {
                if s.refs.get(branch) == Some(&c.id) {
                    let w = branch.len() as f64 * 6.2 + 10.;
                    chips.push((branch, cursor, w));
                    cursor += w + 5.;
                }
            }
            let mut object = Object::new(
                &rowid,
                Affine::translate((x, y)),
                if c.ghost {
                    None
                } else {
                    Some(Hit::Rect(Rect::new(
                        -5.5,
                        -9.,
                        cursor + 2. + c.message.len() as f64 * 6.5 - x,
                        9.,
                    )))
                },
                vec![Shape::Circle {
                    center: Point::ORIGIN,
                    radius: 5.5,
                    fill: if c.ghost { WHITE } else { color },
                    stroke: Some((color, 2.)),
                }],
            )
            .baseline_label(
                &c.message,
                (cursor + 2. - x, 4.),
                12.,
                if c.ghost {
                    hex_color("9ca3af")
                } else {
                    hex_color("111827")
                },
            );
            if c.ghost {
                object.opacity = 0.35;
                let mut strike = BezPath::new();
                strike.move_to((cursor + 2. - x, 0.));
                strike.line_to((cursor + 2. - x + c.message.len() as f64 * 6.2, 0.));
                object.shapes.push(Shape::Path {
                    path: strike,
                    fill: None,
                    stroke: Some((hex_color("9ca3af"), 1.)),
                });
            }
            if c.ghost {
                object = object.dash(vec![2., 2.]);
            }
            scene.push(object);
            let mut hash = baseline_text(
                format!("hash-{}", c.id),
                (160., y + 4.),
                &c.hash,
                11.,
                hex_color("9ca3af"),
            )
            .font_family(FontFamily::Mono);
            hash.parent = Some(rowid.clone());
            if c.ghost {
                hash.opacity = 0.35;
            }
            scene.push(hash);
            for (branch, cx, w) in chips {
                let mut chip = Object::rect(
                    format!("ref-{branch}"),
                    Rect::new(cx, y - 8., cx + w, y + 8.),
                    4.,
                    hex_color(match branch {
                        "main" => "1d4ed8",
                        "feature" => "15803d",
                        "hotfix" => "b45309",
                        _ => "374151",
                    }),
                )
                .baseline_label(
                    branch,
                    ((w - branch.len() as f64 * 5.3) / 2., 12.),
                    10.,
                    WHITE,
                );
                chip.parent = Some(rowid.clone());
                scene.push(chip);
            }
        }
        scene
    }
    fn drag(&self, s: &State, id: &str, m: Modifiers) -> Option<DragSpec<'static, State>> {
        let base = clean(s);
        if let Some(branch) = id.strip_prefix("ref-") {
            if !base.refs.contains_key(branch) {
                return None;
            }
            let candidates = base
                .commits
                .keys()
                .map(|target| {
                    let mut n = base.clone();
                    n.refs.insert(branch.to_string(), *target);
                    finalize(&mut n);
                    n
                })
                .collect::<Vec<_>>();
            return Some(
                closest_with(
                    candidates,
                    ClosestOptions {
                        stickiness: 4.,
                        lock_past: None,
                    },
                )
                .with_floating()
                .when_far(base, 50.),
            );
        }
        let i: usize = id.strip_prefix("commit-")?.parse().ok()?;
        if !base.commits.contains_key(&i) {
            return None;
        }
        let mut targets = if m.alt {
            Vec::new()
        } else {
            vec![base.clone()]
        };
        if m.alt {
            for branch in ["main", "feature", "hotfix"] {
                let tip = base.refs[branch];
                if tip != i {
                    targets.push(cherry(&base, i, branch));
                }
            }
        } else {
            let subtree = descendants(&base, i);
            for &target in base.commits.keys() {
                if !subtree.contains(&target) && base.commits[&i].parent != Some(target) {
                    targets.push(reparent(&base, i, target));
                }
            }
            let run = linear_run(&base, i);
            let rest = run
                .iter()
                .copied()
                .filter(|id| *id != i)
                .collect::<Vec<_>>();
            for slot in 0..run.len() {
                let mut next = rest.clone();
                next.insert(slot, i);
                targets.push(reorder(&base, &run, &next));
            }
        }
        Some(
            closest_with(
                targets,
                ClosestOptions {
                    stickiness: 4.,
                    lock_past: None,
                },
            )
            .with_floating_options(dragology::FloatingOptions {
                ghost_opacity: Some(0.3),
                tether: None,
            })
            .when_far(base, 50.),
        )
    }
    fn status(&self, s: &State) -> String {
        format!(
            "{} live commits · {} old versions · Alt changes to cherry-pick",
            s.commits.values().filter(|c| !c.ghost).count(),
            s.commits.values().filter(|c| c.ghost).count()
        )
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fresh_identity_allocation_does_not_change_the_source_hash_sequence() {
        let s = GitGraph.initial();
        let rebased = reparent(&s, 4, 3);
        assert_eq!(rebased.commits[&4].seq, 8);
        assert_eq!(rebased.commits[&5].seq, 9);
        assert_eq!(rebased.commits[&6].seq, 10);
        let picked = cherry(&s, 4, "main");
        assert_eq!(picked.commits[&4].seq, 8);
    }
    #[test]
    fn rebase_changes_hashes_and_keeps_old_history() {
        let s = GitGraph.initial();
        let n = reparent(&s, 4, 3);
        assert_ne!(s.commits[&4].hash, n.commits[&4].hash);
        assert_eq!(n.commits.values().filter(|c| c.ghost).count(), 3);
        assert_eq!(n.refs["feature"], 6);
        assert_eq!(n.commits[&4].parent, Some(3));
        assert_eq!(positions(&n).len(), n.commits.len());
    }
    #[test]
    fn cherry_pick_preserves_original_descendants() {
        let s = GitGraph.initial();
        let n = cherry(&s, 4, "main");
        assert_eq!(n.refs["main"], 4);
        assert_eq!(n.commits[&5].parent, Some(s.next));
        assert_eq!(positions(&n).len(), n.commits.len());
    }
}
