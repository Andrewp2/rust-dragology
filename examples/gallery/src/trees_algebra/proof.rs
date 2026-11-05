//! Subformula linking: invertible-first backward/forward interaction, as upstream.
use super::tree::{Tree, fresh_id};
use crate::*;
use dragology::{closest, drop_target, fixed};
use std::collections::{BTreeMap, BTreeSet};
type Formula = Tree;
fn leaf(s: &str) -> Formula {
    Tree::leaf(fresh_id("formula"), s)
}
fn bin(op: &str, l: Formula, r: Formula) -> Formula {
    Tree::node(fresh_id("formula"), op, vec![l, r])
}
fn child(f: &Formula, side: usize, new: Formula) -> Formula {
    let mut f = f.clone();
    f.children[side] = new;
    f
}
fn back(h: &Formula, ph: &[usize], c: &Formula, pc: &[usize]) -> Option<Formula> {
    if !pc.is_empty() && c.children.len() == 2 && c.label != "∨" {
        let (side, rest) = (pc[0], &pc[1..]);
        let sub = if c.label == "→" && side == 0 {
            fwd(h, ph, &c.children[0], rest)
        } else {
            back(h, ph, &c.children[side], rest)
        }?;
        return Some(child(c, side, sub));
    }
    if !ph.is_empty() && h.children.len() == 2 && h.label != "→" {
        let (side, rest) = (ph[0], &ph[1..]);
        let sub = back(&h.children[side], rest, c, pc)?;
        return Some(if h.label == "∧" {
            sub
        } else {
            let other = bin("→", h.children[1 - side].clone(), c.clone());
            if side == 0 {
                bin("∧", sub, other)
            } else {
                bin("∧", other, sub)
            }
        });
    }
    if !pc.is_empty() && c.children.len() == 2 {
        let side = pc[0];
        return Some(child(c, side, back(h, ph, &c.children[side], &pc[1..])?));
    }
    if !ph.is_empty() && h.children.len() == 2 {
        if ph[0] == 1 {
            return Some(bin(
                "∧",
                h.children[0].clone(),
                back(&h.children[1], &ph[1..], c, pc)?,
            ));
        }
        return None;
    }
    h.structural_eq(c).then(|| leaf("⊤"))
}
fn fwd(h: &Formula, ph: &[usize], x: &Formula, px: &[usize]) -> Option<Formula> {
    if !px.is_empty() && x.children.len() == 2 && x.label != "→" {
        let side = px[0];
        return Some(child(x, side, fwd(h, ph, &x.children[side], &px[1..])?));
    }
    if !ph.is_empty() && h.children.len() == 2 && h.label != "→" {
        let side = ph[0];
        let sub = fwd(&h.children[side], &ph[1..], x, px)?;
        return Some(if h.label == "∧" {
            sub
        } else if side == 0 {
            bin("∨", sub, h.children[1].clone())
        } else {
            bin("∨", h.children[0].clone(), sub)
        });
    }
    if !px.is_empty() && x.children.len() == 2 {
        let side = px[0];
        let sub = if side == 1 {
            fwd(h, ph, &x.children[1], &px[1..])
        } else {
            back(h, ph, &x.children[0], &px[1..])
        }?;
        return Some(child(x, side, sub));
    }
    if !ph.is_empty() && h.children.len() == 2 {
        let side = ph[0];
        return Some(if side == 1 {
            bin(
                "→",
                h.children[0].clone(),
                fwd(&h.children[1], &ph[1..], x, px)?,
            )
        } else {
            bin(
                "→",
                back(x, px, &h.children[0], &ph[1..])?,
                h.children[1].clone(),
            )
        });
    }
    None
}
fn simplify(f: &Formula) -> Formula {
    if f.children.len() != 2 {
        return f.clone();
    }
    let l = simplify(&f.children[0]);
    let r = simplify(&f.children[1]);
    match f.label.as_str() {
        "∧" => {
            if l.label == "⊤" {
                return r;
            }
            if r.label == "⊤" {
                return l;
            }
            if l.label == "⊥" {
                return l;
            }
            if r.label == "⊥" {
                return r;
            }
        }
        "∨" => {
            if l.label == "⊤" {
                return l;
            }
            if r.label == "⊤" {
                return r;
            }
            if l.label == "⊥" {
                return r;
            }
            if r.label == "⊥" {
                return l;
            }
        }
        "→" => {
            if r.label == "⊤" || l.label == "⊤" {
                return r;
            }
            if l.label == "⊥" {
                return leaf("⊤");
            }
        }
        _ => {}
    }
    let mut n = f.clone();
    n.children = vec![l, r];
    n
}
fn path(f: &Formula, id: &str) -> Option<Vec<usize>> {
    if f.id == id {
        return Some(vec![]);
    }
    for (i, c) in f.children.iter().enumerate() {
        if let Some(mut p) = path(c, id) {
            p.insert(0, i);
            return Some(p);
        }
    }
    None
}
fn at<'a>(mut f: &'a Formula, p: &[usize]) -> &'a Formula {
    for &i in p {
        f = &f.children[i]
    }
    f
}
fn polarity(mut pos: bool, mut f: &Formula, p: &[usize]) -> bool {
    for &i in p {
        if f.label == "→" && i == 0 {
            pos = !pos
        }
        f = &f.children[i]
    }
    pos
}
fn occurrences(f: &Formula) -> Vec<(&Formula, Vec<usize>)> {
    let mut out = vec![(f, vec![])];
    for (i, c) in f.children.iter().enumerate() {
        for (f, mut p) in occurrences(c) {
            p.insert(0, i);
            out.push((f, p));
        }
    }
    out
}
#[derive(Clone, PartialEq)]
struct Sequent {
    id: String,
    hyps: Vec<Formula>,
    goal: Formula,
}
#[derive(Clone, PartialEq)]
pub struct ProofState {
    goals: Vec<Sequent>,
    history: Vec<Vec<Sequent>>,
    preset: usize,
    highlight: Vec<String>,
    continue_id: Option<String>,
    emerge: BTreeMap<String, String>,
}
#[derive(Clone, Debug)]
struct Loc {
    row: usize,
    hyp: Option<usize>,
    path: Vec<usize>,
}
fn root<'a>(s: &'a ProofState, l: &Loc) -> &'a Formula {
    let g = &s.goals[l.row];
    l.hyp.map_or(&g.goal, |i| &g.hyps[i])
}
fn locate(s: &ProofState, id: &str) -> Option<Loc> {
    for (row, g) in s.goals.iter().enumerate() {
        if let Some(path) = path(&g.goal, id) {
            return Some(Loc {
                row,
                hyp: None,
                path,
            });
        }
        for (i, h) in g.hyps.iter().enumerate() {
            if let Some(path) = path(h, id) {
                return Some(Loc {
                    row,
                    hyp: Some(i),
                    path,
                });
            }
        }
    }
    None
}
fn link_targets(s: &ProofState, src: &Loc) -> Vec<Loc> {
    let f = at(root(s, src), &src.path);
    if f.label == "⊤" || f.label == "⊥" {
        return vec![];
    }
    let pos = polarity(src.hyp.is_none(), root(s, src), &src.path);
    let g = &s.goals[src.row];
    let mut out = vec![];
    for hi in std::iter::once(None).chain((0..g.hyps.len()).map(Some)) {
        if hi == src.hyp {
            continue;
        }
        let r = hi.map_or(&g.goal, |i| &g.hyps[i]);
        for (t, path) in occurrences(r) {
            if t.structural_eq(f) && polarity(hi.is_none(), r, &path) != pos {
                out.push(Loc {
                    row: src.row,
                    hyp: hi,
                    path,
                })
            }
        }
    }
    out.sort_by_key(|t| std::cmp::Reverse(t.path.len()));
    out
}
fn freshen(
    f: &Formula,
    seen: &mut BTreeSet<String>,
    emerge: &mut BTreeMap<String, String>,
) -> Formula {
    let mut n = f.clone();
    if seen.contains(&n.id) {
        n.id = fresh_id("formula-copy");
        emerge.insert(n.id.clone(), f.id.clone());
    }
    seen.insert(n.id.clone());
    n.children = f
        .children
        .iter()
        .map(|c| freshen(c, seen, emerge))
        .collect();
    n
}
fn normalized(g: Sequent) -> Option<Sequent> {
    if g.goal.label == "⊤" || g.hyps.iter().any(|h| h.label == "⊥") {
        None
    } else {
        Some(Sequent {
            hyps: g.hyps.into_iter().filter(|h| h.label != "⊤").collect(),
            ..g
        })
    }
}
fn link(s: &ProofState, src: &Loc, dst: &Loc) -> Option<ProofState> {
    if src.row != dst.row || src.hyp == dst.hyp {
        return None;
    }
    let sf = at(root(s, src), &src.path);
    let tf = at(root(s, dst), &dst.path);
    if !sf.structural_eq(tf)
        || polarity(src.hyp.is_none(), root(s, src), &src.path)
            == polarity(dst.hyp.is_none(), root(s, dst), &dst.path)
    {
        return None;
    }
    let g = &s.goals[src.row];
    let (hi, result) = if src.hyp.is_none() || dst.hyp.is_none() {
        let (h, c) = if src.hyp.is_none() {
            (dst, src)
        } else {
            (src, dst)
        };
        (None, back(&g.hyps[h.hyp?], &h.path, &g.goal, &c.path)?)
    } else {
        (
            dst.hyp,
            fwd(&g.hyps[src.hyp?], &src.path, &g.hyps[dst.hyp?], &dst.path)?,
        )
    };
    let mut seen = BTreeSet::new();
    for (row, g) in s.goals.iter().enumerate() {
        for (h, f) in std::iter::once((None, &g.goal))
            .chain(g.hyps.iter().enumerate().map(|(i, h)| (Some(i), h)))
        {
            if row != src.row || h != hi {
                seen.extend(f.nodes().into_iter().map(|n| n.id.clone()));
            }
        }
    }
    let mut n = s.clone();
    let result = freshen(&simplify(&result), &mut seen, &mut n.emerge);
    let result_id = result.id.clone();
    let mut ng = g.clone();
    if let Some(i) = hi {
        ng.hyps[i] = result
    } else {
        ng.goal = result
    }
    let normalized = normalized(ng);
    n.history.push(s.goals.clone());
    n.goals.remove(src.row);
    if let Some(g) = normalized {
        n.goals.insert(src.row, g);
    }
    n.highlight.clear();
    n.continue_id = if locate(&n, &result_id).is_some() {
        Some(result_id)
    } else {
        Some("undo-button".into())
    };
    Some(n)
}
fn parse(src: &str) -> Formula {
    let s = src.replace("->", " → ");
    let mut toks = vec![];
    let mut word = String::new();
    for c in s.chars() {
        if c.is_alphanumeric() || c == '_' || c == '\'' {
            word.push(c)
        } else {
            if !word.is_empty() {
                toks.push(std::mem::take(&mut word));
            }
            if !c.is_whitespace() {
                toks.push(c.to_string())
            }
        }
    }
    if !word.is_empty() {
        toks.push(word)
    }
    fn expr(ts: &[String], p: &mut usize, level: u8) -> Formula {
        if level < 3 {
            let mut left = expr(ts, p, level + 1);
            loop {
                let yes = ts.get(*p).is_some_and(|t| match level {
                    0 => t == "→",
                    1 => t == "|" || t == "∨",
                    2 => t == "&" || t == "∧",
                    _ => false,
                });
                if !yes {
                    break;
                }
                let op = match level {
                    0 => "→",
                    1 => "∨",
                    _ => "∧",
                };
                *p += 1;
                let right = expr(ts, p, if level == 0 { 0 } else { level + 1 });
                left = bin(op, left, right);
                if level == 0 {
                    break;
                }
            }
            left
        } else {
            let t = &ts[*p];
            *p += 1;
            if t == "(" {
                let t = expr(ts, p, 0);
                assert_eq!(ts[*p], ")");
                *p += 1;
                t
            } else {
                leaf(match t.as_str() {
                    "T" => "⊤",
                    "F" => "⊥",
                    _ => t,
                })
            }
        }
    }
    let mut p = 0;
    let f = expr(&toks, &mut p, 0);
    assert_eq!(p, toks.len());
    f
}
fn initial(preset: usize) -> ProofState {
    let (hs, g) = match preset {
        0 => (vec!["p", "p -> q"], "q"),
        1 => (vec!["p & q"], "q & p"),
        2 => (vec!["p | q", "p -> r", "q -> r"], "r"),
        3 => (vec!["p & (q | r)"], "(p & q) | (p & r)"),
        4 => (vec!["p & q -> r"], "p -> q -> r"),
        5 => (vec![], "(p -> q) -> (q -> r) -> p -> r"),
        _ => (vec![], "((p -> q) -> p) -> p"),
    };
    ProofState {
        goals: vec![Sequent {
            id: fresh_id("sequent"),
            hyps: hs.into_iter().map(parse).collect(),
            goal: parse(g),
        }],
        history: vec![],
        preset,
        highlight: vec![],
        continue_id: None,
        emerge: BTreeMap::new(),
    }
}
fn prec(f: &Formula) -> u8 {
    match f.label.as_str() {
        "→" => 1,
        "∨" => 2,
        "∧" => 3,
        _ => 4,
    }
}
fn child_prec(f: &Formula, i: usize) -> u8 {
    if f.label == "→" {
        if i == 0 { 2 } else { 0 }
    } else {
        3
    }
}
fn width(f: &Formula, context: u8) -> f64 {
    let parens = context >= prec(f);
    4. + if parens { 18. } else { 0. }
        + if f.children.len() == 2 {
            width(&f.children[0], child_prec(f, 0))
                + width(&f.children[1], child_prec(f, 1))
                + if f.label == "→" { 36. } else { 30. }
        } else if f.label == "⊤" || f.label == "⊥" {
            18.
        } else {
            15. + f.label.chars().count().saturating_sub(1) as f64 * 11.
        }
}
fn clickable(f: &Formula, hyp: bool) -> bool {
    if hyp {
        f.label == "∧" || f.label == "∨"
    } else {
        f.label == "∧" || f.label == "→"
    }
}
fn math_text(
    id: impl Into<String>,
    at: impl Into<Point>,
    text: impl Into<String>,
    color: Color,
) -> Object {
    Object::text(id, at, text, 22., color)
        .font_family(FontFamily::Serif)
        .text_anchor(TextAnchor::Middle)
        .text_baseline(TextBaseline::Central)
}
fn render_formula(
    sc: &mut Scene,
    s: &ProofState,
    f: &Formula,
    p: Point,
    context: u8,
    hyp: bool,
    isroot: bool,
    parent: Option<&str>,
    dragged: Option<&str>,
    hovered: Option<&str>,
) {
    let w = width(f, context);
    let held = dragged == Some(f.id.as_str());
    let highlighted = s.highlight.contains(&f.id);
    let fill = if held {
        [254, 243, 199, 255]
    } else if highlighted {
        [186, 230, 253, 255]
    } else if isroot && hovered == Some(f.id.as_str()) {
        [231, 229, 228, 255]
    } else if isroot {
        [255, 255, 255, 255]
    } else {
        [0, 0, 0, 0]
    };
    let stroke = if held {
        Some(([245, 158, 11, 255], 1.5))
    } else if highlighted {
        Some(([14, 165, 233, 255], 1.5))
    } else if isroot {
        Some(([214, 211, 209, 255], 1.))
    } else {
        None
    };
    let mut o = Object::new(
        &f.id,
        Affine::translate(p.to_vec2()),
        isroot.then_some(Hit::Rect(Rect::new(0., -16., w, 16.))),
        vec![Shape::Rect {
            bounds: Rect::new(0., -16., w, 16.),
            radius: 7.,
            fill,
            stroke,
        }],
    );
    o.parent = parent.map(str::to_owned);
    o.emerge_from = s.emerge.get(&f.id).cloned().or_else(|| f.emerge.clone());
    sc.push(o);
    let text = |id: String, x: f64, value: &str, color: Color, italic: bool| {
        let mut o = math_text(id, (x, p.y), value, color);
        o.parent = Some(f.id.clone());
        if italic {
            o = o.italic();
        }
        o
    };
    let parens = context >= prec(f);
    let mut x = p.x + 2.;
    // Stable empty slots mirror the source's non-id parenthesis positions.
    sc.push(text(
        format!("{}-open", f.id),
        x + 4.5,
        if parens { "(" } else { "" },
        [154, 154, 148, 255],
        false,
    ));
    if parens {
        x += 9.;
    }
    if f.children.len() == 2 {
        let l = &f.children[0];
        render_formula(
            sc,
            s,
            l,
            Point::new(x, p.y),
            child_prec(f, 0),
            hyp,
            false,
            Some(&f.id),
            dragged,
            hovered,
        );
        x += width(l, child_prec(f, 0));
        let ow = if f.label == "→" { 36. } else { 30. };
        let can = isroot && clickable(f, hyp);
        let bounds = Rect::new(-ow / 2. + 3., -11., ow / 2. - 3., 11.);
        let mut op = Object::new(
            format!("{}-op", f.id),
            Affine::translate((x + ow / 2., p.y)),
            can.then_some(Hit::Rect(bounds)),
            vec![Shape::Rect {
                bounds,
                radius: 6.,
                fill: if can {
                    [232, 230, 255, 255]
                } else {
                    [0, 0, 0, 0]
                },
                stroke: can.then_some(([185, 180, 245, 255], 1.)),
            }],
        )
        .label(
            &f.label,
            (0., 0.),
            22.,
            if can {
                [79, 70, 229, 255]
            } else {
                [92, 91, 87, 255]
            },
        )
        .font_family(FontFamily::Serif)
        .text_anchor(TextAnchor::Middle)
        .text_baseline(TextBaseline::Central);
        if can {
            op = op.clickable();
        }
        op.parent = Some(f.id.clone());
        sc.push(op);
        x += ow;
        render_formula(
            sc,
            s,
            &f.children[1],
            Point::new(x, p.y),
            child_prec(f, 1),
            hyp,
            false,
            Some(&f.id),
            dragged,
            hovered,
        );
        x += width(&f.children[1], child_prec(f, 1));
    } else {
        let atom = f.label != "⊤" && f.label != "⊥";
        let glyphw = if atom {
            15. + f.label.chars().count().saturating_sub(1) as f64 * 11.
        } else {
            18.
        };
        let color = if atom {
            [31, 41, 55, 255]
        } else if f.label == "⊤" {
            [22, 163, 74, 255]
        } else {
            [220, 38, 38, 255]
        };
        sc.push(text(
            format!("{}-text", f.id),
            x + glyphw / 2.,
            &f.label,
            color,
            atom,
        ));
        x += glyphw;
    }
    sc.push(text(
        format!("{}-close", f.id),
        x + 4.5,
        if parens { ")" } else { "" },
        [154, 154, 148, 255],
        false,
    ));
}
pub struct Proof;
impl Example for Proof {
    type State = ProofState;
    fn initial(&self) -> ProofState {
        initial(0)
    }
    fn scene(&self, s: &ProofState) -> Scene {
        self.scene_interaction(s, None, None)
    }
    fn scene_dragging(&self, s: &ProofState, dragged: Option<&str>) -> Scene {
        self.scene_interaction(s, dragged, None)
    }
    fn scene_interaction(
        &self,
        s: &ProofState,
        dragged: Option<&str>,
        hovered: Option<&str>,
    ) -> Scene {
        let mut sc = Scene::new(660., 356.);
        let can_undo = !s.history.is_empty();
        let bounds = Rect::new(-36., -13., 36., 13.);
        sc.push(
            Object::new(
                "undo-button",
                Affine::translate((600., 22.)),
                Some(Hit::Rect(bounds)),
                vec![Shape::Rect {
                    bounds,
                    radius: 13.,
                    fill: if can_undo {
                        [245, 245, 244, 255]
                    } else {
                        [250, 250, 249, 255]
                    },
                    stroke: Some((
                        if can_undo {
                            [214, 211, 209, 255]
                        } else {
                            [231, 229, 228, 255]
                        },
                        1.,
                    )),
                }],
            )
            .label(
                "↶ undo",
                (0., 0.),
                14.,
                if can_undo {
                    [68, 64, 60, 255]
                } else {
                    [196, 192, 187, 255]
                },
            )
            .text_anchor(TextAnchor::Middle)
            .text_baseline(TextBaseline::Central)
            .clickable(),
        );
        if s.goals.is_empty() {
            sc.push(
                math_text("qed", (20., 70.), "∎ Proved!", [22, 163, 74, 255])
                    .text_anchor(TextAnchor::Start),
            );
            if let Some(Shape::Text { size, .. }) =
                sc.objects.last_mut().unwrap().shapes.first_mut()
            {
                *size = 24.;
            }
        }
        for (row, g) in s.goals.iter().enumerate() {
            let y = 70. + row as f64 * 52.;
            let roww = g.hyps.iter().map(|h| width(h, 0)).sum::<f64>()
                + g.hyps.len().saturating_sub(1) as f64 * 16.
                + width(&g.goal, 0)
                + 40.;
            let mut bg = Object::new(
                &g.id,
                Affine::translate((20., y)),
                None,
                vec![Shape::Rect {
                    bounds: Rect::new(-10., -23., roww + 10., 23.),
                    radius: 10.,
                    fill: [245, 245, 244, 255],
                    stroke: None,
                }],
            );
            bg.emerge_from = row.checked_sub(1).map(|i| s.goals[i].id.clone());
            sc.push(bg);
            let mut x = 20.;
            for (i, h) in g.hyps.iter().enumerate() {
                if i > 0 {
                    let mut o = math_text(
                        format!("{}-comma", h.id),
                        (x + 8., y + 4.),
                        ",",
                        [120, 113, 108, 255],
                    );
                    o.parent = Some(g.id.clone());
                    sc.push(o);
                    x += 16.;
                }
                render_formula(
                    &mut sc,
                    s,
                    h,
                    Point::new(x, y),
                    0,
                    true,
                    true,
                    Some(&g.id),
                    dragged,
                    hovered,
                );
                x += width(h, 0);
            }
            let mut turnstile = math_text(
                format!("{}-turnstile", g.id),
                (x + 20., y),
                "⊢",
                [68, 64, 60, 255],
            );
            turnstile.parent = Some(g.id.clone());
            sc.push(turnstile);
            x += 40.;
            render_formula(
                &mut sc,
                s,
                &g.goal,
                Point::new(x, y),
                0,
                false,
                true,
                Some(&g.id),
                dragged,
                hovered,
            );
        }
        super::tree::raise_group(&mut sc, dragged);
        for o in &mut sc.objects {
            o.transform = Affine::translate((0., 90.)) * o.transform;
        }
        sc.push(
            Object::rect(
                "problem-options-panel",
                Rect::new(0., 0., 660., 74.),
                4.,
                [249, 250, 251, 255],
            )
            .decorative(),
        );
        sc.push(
            Object::text(
                "problem-options-heading",
                (12., 12.),
                "Options",
                12.,
                [55, 65, 81, 255],
            )
            .font_weight(500),
        );
        sc.inline_control("preset", Rect::new(12., 36., 648., 62.))
            .select()
            .font_size(12.);
        sc.height += 90.;
        sc
    }
    fn action_transition(&self, _: &ProofState, _: &str) -> Option<dragology::Transition> {
        Some(dragology::Transition {
            duration: std::time::Duration::from_millis(400),
            ..Default::default()
        })
    }
    fn drag(
        &self,
        s: &ProofState,
        id: &str,
        _: Modifiers,
    ) -> Option<DragSpec<'static, ProofState>> {
        if id == "undo-button" {
            return Some(fixed(s.clone()));
        }
        let source = locate(s, id)?;
        if !source.path.is_empty() {
            return None;
        }
        let targets = link_targets(s, &source);
        let scene = self.scene(s);
        let mut branches = vec![];
        let mut highlighted = s.clone();
        highlighted.continue_id = None;
        for t in targets {
            let f = at(root(s, &t), &t.path);
            highlighted.highlight.push(f.id.clone());
            if let Some(n) = link(s, &source, &t) {
                if let Some(o) = scene.object(&f.id) {
                    if let Some(Shape::Rect { bounds, .. }) = o.shapes.first() {
                        let next_id = n
                            .continue_id
                            .clone()
                            .unwrap_or_else(|| "undo-button".into());
                        branches.push(
                            drop_target(n, o.transform.transform_rect_bbox(*bounds))
                                .with_floating()
                                .with_chaining_options(dragology::Chaining {
                                    dragged_id: Some(next_id),
                                    ..Default::default()
                                }),
                        );
                    }
                }
            }
        }
        let original = s.clone();
        let far = fixed(highlighted)
            .on_drop(move |_| original.clone())
            .with_floating();
        Some(if branches.is_empty() {
            far
        } else {
            closest(branches)
                .with_branch_transition(Some(dragology::Transition::default()))
                .when_far(far, 50.)
        })
    }
    fn click(
        &self,
        s: &ProofState,
        id: &str,
        _: Point,
        _: Modifiers,
        _: usize,
    ) -> Option<ProofState> {
        if id == "undo-button" {
            return self.action(s, "undo", Value::Trigger);
        }
        let id = id.strip_suffix("-op")?;
        let l = locate(s, id)?;
        if !l.path.is_empty() {
            return None;
        }
        let g = &s.goals[l.row];
        let f = root(s, &l);
        let mut n = s.clone();
        let mut replacements = vec![];
        let mut first = g.clone();
        match (l.hyp, f.label.as_str()) {
            (None, "∧") => {
                first.goal = f.children[0].clone();
                let mut second = g.clone();
                second.id = fresh_id("sequent");
                second.goal = f.children[1].clone();
                second.hyps = g.hyps.iter().map(|h| h.fresh(&fresh_id("case"))).collect();
                replacements = vec![first, second];
            }
            (None, "→") => {
                first.hyps.push(f.children[0].clone());
                first.goal = f.children[1].clone();
                if let Some(g) = normalized(first) {
                    replacements.push(g);
                }
            }
            (Some(i), "∧") => {
                first.hyps.splice(i..=i, f.children.clone());
                replacements.push(first);
            }
            (Some(i), "∨") => {
                first.hyps[i] = f.children[0].clone();
                let mut second = g.clone();
                second.id = fresh_id("sequent");
                second.hyps = g
                    .hyps
                    .iter()
                    .enumerate()
                    .map(|(j, h)| {
                        if i == j {
                            f.children[1].clone()
                        } else {
                            h.fresh(&fresh_id("case"))
                        }
                    })
                    .collect();
                second.goal = g.goal.fresh(&fresh_id("case-goal"));
                replacements = vec![first, second];
            }
            _ => return None,
        }
        n.history.push(s.goals.clone());
        n.goals.splice(l.row..=l.row, replacements);
        n.highlight.clear();
        n.continue_id = None;
        Some(n)
    }
    fn controls(&self, s: &ProofState) -> Vec<Control> {
        vec![Control::Choice {
            id: "preset",
            label: "Problem".into(),
            options: [
                "Modus ponens: p, p → q ⊢ q",
                "Commutativity: p ∧ q ⊢ q ∧ p",
                "Cases: p ∨ q, p → r, q → r ⊢ r",
                "Distributivity: p ∧ (q ∨ r) ⊢ (p ∧ q) ∨ (p ∧ r)",
                "Currying: (p ∧ q → r) ⊢ p → q → r",
                "Composition: ⊢ (p → q) → (q → r) → p → r",
                "Peirce-ish (not provable!): ⊢ ((p → q) → p) → p",
            ]
            .iter()
            .map(|x| x.to_string())
            .collect(),
            selected: s.preset,
        }]
    }
    fn action(&self, s: &ProofState, id: &str, v: Value) -> Option<ProofState> {
        match (id, v) {
            ("preset", Value::Index(i)) if i < 7 => Some(initial(i)),
            ("undo", _) => {
                let mut n = s.clone();
                n.goals = n.history.pop()?;
                n.highlight.clear();
                n.continue_id = None;
                Some(n)
            }
            _ => None,
        }
    }
    fn key(&self, s: &ProofState, k: &str, m: Modifiers) -> Option<ProofState> {
        if k == "z" && m.control {
            self.action(s, "undo", Value::Trigger)
        } else {
            None
        }
    }
    fn status(&self, s: &ProofState) -> String {
        if s.goals.is_empty() {
            "All proof obligations discharged.".into()
        } else {
            "Drag a hypothesis or goal onto a matching subformula of opposite polarity. Click purple connectives to introduce, split, or case.".into()
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn truth(f: &Formula, bits: u8) -> bool {
        let l = || truth(&f.children[0], bits);
        let r = || truth(&f.children[1], bits);
        match f.label.as_str() {
            "⊤" => true,
            "⊥" => false,
            "∧" => l() && r(),
            "∨" => l() || r(),
            "→" => !l() || r(),
            "p" => bits & 1 != 0,
            "q" => bits & 2 != 0,
            "r" => bits & 4 != 0,
            _ => panic!(),
        }
    }
    #[test]
    fn linking_rules_preserve_entailment() {
        let fs: Vec<_> = [
            "p",
            "q",
            "p&q",
            "p|q",
            "p->q",
            "q->p",
            "(p&q)->r",
            "p&(q|r)",
            "(p->q)->r",
            "p->(q->r)",
            "p|(q->r)",
        ]
        .into_iter()
        .map(parse)
        .collect();
        for h in &fs {
            for c in &fs {
                for (hs, hp) in occurrences(h) {
                    for (cs, cp) in occurrences(c) {
                        if hs.structural_eq(cs) {
                            if polarity(false, h, &hp) != polarity(true, c, &cp) {
                                if let Some(result) = back(h, &hp, c, &cp) {
                                    for bits in 0..8 {
                                        assert!(
                                            !truth(h, bits)
                                                || !truth(&result, bits)
                                                || truth(c, bits),
                                            "backward link is unsound"
                                        );
                                    }
                                }
                            }
                            if polarity(false, h, &hp) != polarity(false, c, &cp) {
                                if let Some(result) = fwd(h, &hp, c, &cp) {
                                    for bits in 0..8 {
                                        assert!(
                                            !truth(h, bits)
                                                || !truth(c, bits)
                                                || truth(&result, bits),
                                            "forward link is unsound"
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn modus_ponens_two_links_close_goal() {
        let s = initial(0);
        let src = Loc {
            row: 0,
            hyp: Some(0),
            path: vec![],
        };
        let target = Loc {
            row: 0,
            hyp: Some(1),
            path: vec![0],
        };
        let n = link(&s, &src, &target).unwrap();
        assert_eq!(n.goals[0].hyps[1].label, "q");
        let n = link(
            &n,
            &Loc {
                row: 0,
                hyp: Some(1),
                path: vec![],
            },
            &Loc {
                row: 0,
                hyp: None,
                path: vec![],
            },
        )
        .unwrap();
        assert!(n.goals.is_empty());
        assert_eq!(n.history.len(), 2);
    }
    #[test]
    fn native_runtime_chains_modus_ponens_in_one_gesture() {
        let mut demo = crate::runtime::ModelDemo::new(Proof);
        demo.set_animated(false);
        fn text_centers(scene: &Scene, label: &str) -> Vec<Point> {
            scene
                .objects
                .iter()
                .filter_map(|o| {
                    o.shapes
                        .iter()
                        .any(|s| matches!(s,Shape::Text{text,..}if text==label))
                        .then_some(o.transform * Point::ORIGIN)
                })
                .collect()
        }
        let ps = text_centers(demo.scene(), "p");
        let goal_id = demo
            .scene()
            .objects
            .iter()
            .filter(|o| {
                o.shapes
                    .iter()
                    .any(|s| matches!(s,Shape::Text{text,..}if text=="q"))
            })
            .last()
            .unwrap()
            .parent
            .clone()
            .unwrap();
        demo.press(ps[0], Modifiers::default(), 1);
        demo.move_to(ps[1], Modifiers::default());
        demo.advance(std::time::Duration::ZERO);
        let goal = demo.scene().object(&goal_id).unwrap().transform * Point::new(8., 0.);
        demo.move_to(goal, Modifiers::default());
        demo.advance(std::time::Duration::ZERO);
        demo.release(goal, Modifiers::default());
        assert!(demo.scene().object("qed").is_some(), "{}", demo.status());
        demo.action("undo", Value::Trigger);
        assert!(demo.scene().object("qed").is_none());
    }
    #[test]
    fn case_splits_keep_ids_unique() {
        let s = initial(2);
        let id = format!("{}-op", s.goals[0].hyps[0].id);
        let n = Proof
            .click(&s, &id, Point::ORIGIN, Modifiers::default(), 1)
            .unwrap();
        let ids: Vec<_> = n
            .goals
            .iter()
            .flat_map(|g| g.hyps.iter().chain(std::iter::once(&g.goal)))
            .flat_map(|f| f.nodes())
            .map(|f| &f.id)
            .collect();
        assert_eq!(ids.len(), ids.iter().collect::<BTreeSet<_>>().len());
    }
}
