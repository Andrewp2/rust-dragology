//! Capture-avoiding lambda-calculus rewrites, including the upstream beta waypoint.
use super::tree::fresh_id;
use crate::*;
use dragology::{between, closest};
use std::collections::BTreeSet;
#[derive(Clone, Debug, PartialEq)]
struct Term {
    id: String,
    emerge: Option<String>,
    emerge_mode: EmergeMode,
    hole_outline: bool,
    kind: Kind,
}
#[derive(Clone, Debug, PartialEq)]
enum Kind {
    Var(String),
    Lam {
        param: String,
        body: Box<Term>,
        incoming: Option<Box<Term>>,
    },
    App(Box<Term>, Box<Term>),
    Hole(Box<Term>),
}
fn term(kind: Kind) -> Term {
    Term {
        id: fresh_id("lambda"),
        emerge: None,
        emerge_mode: EmergeMode::Auto,
        hole_outline: true,
        kind,
    }
}
fn var(name: impl Into<String>) -> Term {
    term(Kind::Var(name.into()))
}
fn lam(p: impl Into<String>, body: Term) -> Term {
    term(Kind::Lam {
        param: p.into(),
        body: Box::new(body),
        incoming: None,
    })
}
fn app(f: Term, a: Term) -> Term {
    term(Kind::App(Box::new(f), Box::new(a)))
}
impl Term {
    fn children(&self) -> Vec<&Term> {
        match &self.kind {
            Kind::Var(_) | Kind::Hole(_) => vec![],
            Kind::Lam { body, .. } => vec![body],
            Kind::App(f, a) => vec![f, a],
        }
    }
    fn nodes(&self) -> Vec<&Term> {
        let mut ns = vec![self];
        for c in self.children() {
            ns.extend(c.nodes());
        }
        ns
    }
    fn path(&self, id: &str) -> Option<Vec<&Term>> {
        if self.id == id {
            return Some(vec![self]);
        }
        for c in self.children() {
            if let Some(mut p) = c.path(id) {
                p.insert(0, self);
                return Some(p);
            }
        }
        None
    }
    fn replace(&self, id: &str, new: &Term) -> Term {
        if self.id == id {
            return new.clone();
        }
        let mut n = self.clone();
        n.kind = match &self.kind {
            Kind::Lam {
                param,
                body,
                incoming,
            } => Kind::Lam {
                param: param.clone(),
                body: Box::new(body.replace(id, new)),
                incoming: incoming.clone(),
            },
            Kind::App(f, a) => {
                Kind::App(Box::new(f.replace(id, new)), Box::new(a.replace(id, new)))
            }
            _ => self.kind.clone(),
        };
        n
    }
    fn free(&self) -> BTreeSet<String> {
        match &self.kind {
            Kind::Var(n) => BTreeSet::from([n.clone()]),
            Kind::Hole(_) => BTreeSet::new(),
            Kind::Lam { param, body, .. } => {
                let mut set = body.free();
                set.remove(param);
                set
            }
            Kind::App(f, a) => f.free().union(&a.free()).cloned().collect(),
        }
    }
    fn names(&self) -> BTreeSet<String> {
        self.nodes()
            .iter()
            .filter_map(|n| match &n.kind {
                Kind::Var(n) => Some(n.clone()),
                Kind::Lam { param, .. } => Some(param.clone()),
                _ => None,
            })
            .collect()
    }
    fn same(&self, other: &Term) -> bool {
        match (&self.kind, &other.kind) {
            (Kind::Var(a), Kind::Var(b)) => a == b,
            (
                Kind::Lam {
                    param: a, body: x, ..
                },
                Kind::Lam {
                    param: b, body: y, ..
                },
            ) => a == b && x.same(y),
            (Kind::App(f, a), Kind::App(g, b)) => f.same(g) && a.same(b),
            _ => false,
        }
    }
    fn clone_emerging(&self, origin: Option<&str>) -> Term {
        let mut n = self.clone();
        n.id = fresh_id("lambda-copy");
        n.emerge = Some(origin.unwrap_or(&self.id).to_string());
        n.emerge_mode = if origin.is_none() {
            EmergeMode::Clone
        } else {
            EmergeMode::Auto
        };
        n.kind = match &self.kind {
            Kind::Var(x) => Kind::Var(x.clone()),
            Kind::Lam { param, body, .. } => Kind::Lam {
                param: param.clone(),
                body: Box::new(body.clone_emerging(origin)),
                incoming: None,
            },
            Kind::App(f, a) => Kind::App(
                Box::new(f.clone_emerging(origin)),
                Box::new(a.clone_emerging(origin)),
            ),
            Kind::Hole(t) => Kind::Hole(t.clone()),
        };
        n
    }
    fn clean(&self) -> Term {
        let mut n = self.clone();
        n.emerge = None;
        n.emerge_mode = EmergeMode::Auto;
        n.kind = match &self.kind {
            Kind::Lam { param, body, .. } => Kind::Lam {
                param: param.clone(),
                body: Box::new(body.clean()),
                incoming: None,
            },
            Kind::App(f, a) => Kind::App(Box::new(f.clean()), Box::new(a.clean())),
            _ => self.kind.clone(),
        };
        n
    }
}
/// Annotate a disappearing copy so each node converges onto its counterpart.
fn annotate_merge(copy: &Term, original: &Term) -> Term {
    let mut n = copy.clone();
    n.emerge = Some(original.id.clone());
    n.emerge_mode = EmergeMode::Clone;
    n.kind = match (&copy.kind, &original.kind) {
        (
            Kind::Lam {
                param,
                body,
                incoming,
            },
            Kind::Lam { body: other, .. },
        ) => Kind::Lam {
            param: param.clone(),
            body: Box::new(annotate_merge(body, other)),
            incoming: incoming.clone(),
        },
        (Kind::App(f, a), Kind::App(g, b)) => Kind::App(
            Box::new(annotate_merge(f, g)),
            Box::new(annotate_merge(a, b)),
        ),
        _ => copy.kind.clone(),
    };
    n
}
fn annotate_scale(copy: &Term, origin: &str) -> Term {
    let mut n = copy.clone();
    n.emerge = Some(origin.into());
    n.emerge_mode = EmergeMode::Scale;
    n.kind = match &copy.kind {
        Kind::Lam {
            param,
            body,
            incoming,
        } => Kind::Lam {
            param: param.clone(),
            body: Box::new(annotate_scale(body, origin)),
            incoming: incoming.clone(),
        },
        Kind::App(f, a) => Kind::App(
            Box::new(annotate_scale(f, origin)),
            Box::new(annotate_scale(a, origin)),
        ),
        _ => copy.kind.clone(),
    };
    n
}
fn fresh_name(avoid: &BTreeSet<String>, hint: Option<&str>) -> String {
    if hint.is_none() {
        for s in ["x", "y", "z", "w", "u", "v", "s", "t"] {
            if !avoid.contains(s) {
                return s.into();
            }
        }
    }
    let mut s = hint.unwrap_or("x").to_string();
    while avoid.contains(&s) {
        s.push('\'');
    }
    s
}
fn rename(t: &Term, from: &str, to: &str) -> Term {
    let mut n = t.clone();
    n.kind = match &t.kind {
        Kind::Var(s) if s == from => Kind::Var(to.into()),
        Kind::Lam {
            param,
            body,
            incoming,
        } if param != from => Kind::Lam {
            param: param.clone(),
            body: Box::new(rename(body, from, to)),
            incoming: incoming.clone(),
        },
        Kind::App(f, a) => Kind::App(Box::new(rename(f, from, to)), Box::new(rename(a, from, to))),
        _ => t.kind.clone(),
    };
    n
}
fn substitute_with(
    body: &Term,
    x: &str,
    fv: &BTreeSet<String>,
    replace: &mut impl FnMut(&Term) -> Term,
) -> Term {
    let mut n = body.clone();
    n.kind = match &body.kind {
        Kind::Var(s) if s == x => return replace(body),
        Kind::Lam {
            param,
            body,
            incoming,
        } if param != x => {
            let (p, b) = if fv.contains(param) && body.free().contains(x) {
                let mut avoid = fv.clone();
                avoid.extend(body.names());
                avoid.insert(x.into());
                let p = fresh_name(&avoid, Some(param));
                let b = rename(body, param, &p);
                (p, b)
            } else {
                (param.clone(), *body.clone())
            };
            Kind::Lam {
                param: p,
                body: Box::new(substitute_with(&b, x, fv, replace)),
                incoming: incoming.clone(),
            }
        }
        Kind::App(f, a) => Kind::App(
            Box::new(substitute_with(f, x, fv, replace)),
            Box::new(substitute_with(a, x, fv, replace)),
        ),
        _ => body.kind.clone(),
    };
    n
}
#[cfg(test)]
fn substitute(body: &Term, x: &str, arg: &Term) -> Term {
    let mut first = true;
    substitute_with(body, x, &arg.free(), &mut |_| {
        if first {
            first = false;
            arg.clone()
        } else {
            arg.clone_emerging(None)
        }
    })
}
fn parse(src: &str) -> Result<Term, String> {
    struct Parser {
        xs: Vec<char>,
        p: usize,
    }
    impl Parser {
        fn peek(&mut self) -> Option<char> {
            while self.xs.get(self.p).is_some_and(|x| x.is_whitespace()) {
                self.p += 1
            }
            self.xs.get(self.p).copied()
        }
        fn name(&mut self) -> Result<String, String> {
            self.peek();
            let start = self.p;
            while self
                .xs
                .get(self.p)
                .is_some_and(|x| x.is_ascii_alphanumeric() || *x == '_' || *x == '\'')
            {
                self.p += 1
            }
            if start == self.p {
                Err(format!("Expected variable at character {}", self.p))
            } else {
                Ok(self.xs[start..self.p].iter().collect())
            }
        }
        fn expr(&mut self) -> Result<Term, String> {
            if matches!(self.peek(), Some('λ' | '\\')) {
                self.p += 1;
                let mut ps = vec![];
                while self.peek() != Some('.') {
                    ps.push(self.name()?);
                }
                self.p += 1;
                let mut t = self.expr()?;
                for p in ps.into_iter().rev() {
                    t = lam(p, t)
                }
                return Ok(t);
            }
            let mut result = None;
            loop {
                let atom = match self.peek() {
                    Some('(') => {
                        self.p += 1;
                        let t = self.expr()?;
                        if self.peek() != Some(')') {
                            return Err("Missing closing parenthesis".into());
                        }
                        self.p += 1;
                        t
                    }
                    Some(c) if c.is_ascii_alphanumeric() || c == '_' || c == '\'' => {
                        var(self.name()?)
                    }
                    _ => break,
                };
                result = Some(if let Some(f) = result {
                    app(f, atom)
                } else {
                    atom
                });
            }
            result.ok_or_else(|| "Expected a lambda term".into())
        }
    }
    let mut p = Parser {
        xs: src.chars().collect(),
        p: 0,
    };
    let t = p.expr()?;
    if p.peek().is_some() {
        Err(format!("Unexpected character {}", p.p))
    } else {
        Ok(t)
    }
}
fn print(t: &Term) -> String {
    match &t.kind {
        Kind::Var(n) => n.clone(),
        Kind::Hole(_) => "_".into(),
        Kind::Lam {
            param,
            body,
            incoming,
        } => {
            if let Some(i) = incoming {
                format!("λ{param}:=({}). {}", print(i), print(body))
            } else {
                let mut params = vec![param.as_str()];
                let mut inner = body.as_ref();
                while let Kind::Lam {
                    param,
                    body,
                    incoming: None,
                } = &inner.kind
                {
                    params.push(param);
                    inner = body;
                }
                format!("λ{}. {}", params.join(" "), print(inner))
            }
        }
        Kind::App(f, a) => {
            let f = if matches!(f.kind, Kind::Lam { .. }) {
                format!("({})", print(f))
            } else {
                print(f)
            };
            let a = if matches!(a.kind, Kind::Var(_) | Kind::Hole(_)) {
                print(a)
            } else {
                format!("({})", print(a))
            };
            format!("{f} {a}")
        }
    }
}
#[derive(Clone, PartialEq)]
pub struct LambdaState {
    term: Term,
    preset: usize,
    opts: [bool; 5],
}
pub struct Lambda;
struct Candidate {
    base: Term,
    mid: Option<Term>,
    result: Term,
}
fn binders(path: &[&Term]) -> BTreeSet<String> {
    path.iter()
        .take(path.len().saturating_sub(1))
        .filter_map(|t| {
            if let Kind::Lam { param, .. } = &t.kind {
                Some(param.clone())
            } else {
                None
            }
        })
        .collect()
}
fn candidates(t: &Term, id: &str, opts: [bool; 5]) -> Vec<Candidate> {
    let base = t.clean();
    let Some(path) = base.path(id) else {
        return vec![];
    };
    let dragged = *path.last().unwrap();
    let parent = path.get(path.len().wrapping_sub(2)).copied();
    let grand = path.get(path.len().wrapping_sub(3)).copied();
    let mut out = vec![];
    if opts[0] {
        if let Some(p) = parent {
            if let Kind::App(f, a) = &p.kind {
                if a.id == id {
                    if let Kind::Lam { param, body, .. } = &f.kind {
                        let mut first_copy = None;
                        let mut last_copy = None;
                        let reduced = substitute_with(body, param, &dragged.free(), &mut |occ| {
                            let copy = dragged.clone_emerging(Some(&occ.id));
                            if first_copy.is_none() {
                                first_copy = Some(copy.clone());
                            }
                            last_copy = Some(copy.clone());
                            copy
                        });
                        let incoming = if let Some(first) = &first_copy {
                            annotate_merge(dragged, first)
                        } else {
                            annotate_scale(dragged, &reduced.id)
                        };
                        let mut base_lam = *f.clone();
                        base_lam.emerge = Some(reduced.id.clone());
                        let mut base_app = p.clone();
                        base_app.emerge = Some(reduced.id.clone());
                        base_app.kind =
                            Kind::App(Box::new(base_lam.clone()), Box::new(incoming.clone()));
                        let mut mid_lam = base_lam;
                        mid_lam.kind = Kind::Lam {
                            param: param.clone(),
                            body: Box::new(reduced.clone()),
                            incoming: Some(Box::new(incoming)),
                        };
                        let mut hole = term(Kind::Hole(Box::new(dragged.clone())));
                        hole.hole_outline = last_copy.is_some();
                        hole.emerge = last_copy.map(|t| t.id);
                        let mut mid_app = base_app.clone();
                        mid_app.kind = Kind::App(Box::new(mid_lam), Box::new(hole));
                        out.push(Candidate {
                            base: base.replace(&p.id, &base_app),
                            mid: Some(base.replace(&p.id, &mid_app)),
                            result: base.replace(&p.id, &reduced),
                        });
                    }
                }
            }
        }
    }
    if opts[1] {
        let fv = dragged.free();
        let name = fresh_name(&base.names(), None);
        for i in (0..path.len().saturating_sub(1)).rev() {
            let ancestor = path[i];
            if !binders(&path[i..]).is_disjoint(&fv) {
                break;
            }
            let mut body = ancestor.clone();
            let mut merge_base = base.clone();
            if opts[2] {
                for occ in ancestor.nodes() {
                    if occ.id == id || !occ.same(dragged) {
                        continue;
                    }
                    if !binders(&ancestor.path(&occ.id).unwrap()).is_disjoint(&fv) {
                        continue;
                    }
                    let mut v = var(&name);
                    v.emerge = Some(occ.id.clone());
                    body = body.replace(&occ.id, &v);
                    merge_base = merge_base.replace(&occ.id, &annotate_merge(occ, dragged));
                }
            }
            let mut v = var(&name);
            v.emerge = Some(dragged.id.clone());
            body = body.replace(id, &v);
            let mut l = lam(&name, body);
            l.emerge = Some(ancestor.id.clone());
            let mut a = app(l, dragged.clone());
            a.emerge = Some(ancestor.id.clone());
            out.push(Candidate {
                base: merge_base,
                mid: None,
                result: base.replace(&ancestor.id, &a),
            });
        }
    }
    if opts[3] {
        let name = fresh_name(&base.names(), None);
        let mut v = var(&name);
        v.emerge = Some(id.into());
        let mut a = app(dragged.clone(), v);
        a.emerge = Some(id.into());
        let mut l = lam(name, a);
        l.emerge = Some(id.into());
        out.push(Candidate {
            base: base.clone(),
            mid: None,
            result: base.replace(id, &l),
        });
    }
    if opts[4] {
        if let (Some(p), Some(g)) = (parent, grand) {
            if let (Kind::App(f, a), Kind::Lam { param, .. }) = (&p.kind, &g.kind) {
                if f.id == id
                    && matches!(&a.kind,Kind::Var(n)if n==param)
                    && !dragged.free().contains(param)
                {
                    out.push(Candidate {
                        base: base.clone(),
                        mid: None,
                        result: base.replace(&g.id, dragged),
                    });
                }
            }
        }
    }
    out
}
fn size(t: &Term) -> Vec2 {
    match &t.kind {
        Kind::Var(n) => Vec2::new(n.chars().count() as f64 * 8.4 + 14., 24.),
        Kind::Hole(t) => size(t),
        Kind::Lam { param, body, .. } => {
            let b = size(body);
            Vec2::new(
                18. + (param.chars().count() + 2) as f64 * 8.4 + b.x,
                b.y + 12.,
            )
        }
        Kind::App(f, a) => {
            let f = size(f);
            let a = size(a);
            let row = f.x + a.x + 18.;
            if row > 460. {
                Vec2::new(f.x.max(a.x + 18.) + 12., f.y + a.y + 18.)
            } else {
                Vec2::new(row, f.y.max(a.y) + 12.)
            }
        }
    }
}
thread_local! {
    static HUE_NAMES: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}
fn hue(name: &str) -> f64 {
    HUE_NAMES.with(|names| {
        let mut names = names.borrow_mut();
        let index = names.iter().position(|n| n == name).unwrap_or_else(|| {
            names.push(name.into());
            names.len() - 1
        });
        [205., 350., 130., 40., 275., 180., 20., 95.][index % 8]
    })
}
fn hsl(h: f64, s: f64, l: f64) -> Color {
    let a = s * l.min(1. - l);
    let component = |n: f64| {
        let k = (n + h / 30.) % 12.;
        (255. * (l - a * (-1.0_f64).max((k - 3.).min(9. - k).min(1.)))).round() as u8
    };
    [component(0.), component(8.), component(4.), 255]
}
fn box_fill(mut c: Color, alpha: f64, opaque: bool) -> Color {
    if opaque {
        for value in c.iter_mut().take(3) {
            *value = (alpha * *value as f64 + (1. - alpha) * 255.).round() as u8;
        }
    } else {
        c[3] = (alpha * 255.).round() as u8;
    }
    c
}
// Upstream assigns hues while laying children out, before their parent binder.
fn register_hues(t: &Term) {
    match &t.kind {
        Kind::Var(n) => {
            hue(n);
        }
        Kind::Lam {
            param,
            body,
            incoming,
        } => {
            register_hues(body);
            if let Some(i) = incoming {
                register_hues(i);
            }
            hue(param);
        }
        Kind::App(f, a) => {
            register_hues(f);
            register_hues(a);
        }
        Kind::Hole(t) => register_hues(t),
    }
}
fn render(
    scene: &mut Scene,
    t: &Term,
    p: Point,
    bound: &BTreeSet<String>,
    parent: Option<&str>,
    dragged: Option<&str>,
) {
    let sz = size(t);
    let is_dragged = dragged == Some(t.id.as_str());
    let mut label = String::new();
    let mut label_x = sz.x / 2.;
    let mut label_color = [31, 41, 55, 255];
    let mut fill = box_fill([120, 110, 100, 255], 0.06, is_dragged);
    let mut stroke = Some(([120, 110, 100, 89], 1.));
    match &t.kind {
        Kind::Var(n) => {
            label = n.clone();
            if bound.contains(n) {
                fill = hsl(hue(n), 0.75, 0.82);
                stroke = Some((hsl(hue(n), 0.5, 0.55), 1.));
            } else {
                fill = [229, 231, 235, 255];
                stroke = Some(([156, 163, 175, 255], 1.));
            }
        }
        Kind::Lam { param, .. } => {
            label = format!("λ{param}.");
            label_x = 6. + label.chars().count() as f64 * 8.4 / 2.;
            fill = box_fill(hsl(hue(param), 0.75, 0.80), 0.3, is_dragged);
            stroke = Some((hsl(hue(param), 0.45, 0.62), 1.));
            label_color = hsl(hue(param), 0.45, 0.35);
        }
        Kind::Hole(_) => {
            fill = [0, 0, 0, 0];
            stroke = t.hole_outline.then_some(([120, 110, 100, 102], 1.));
        }
        _ => {}
    }
    let mut o = Object::rect(
        &t.id,
        Rect::from_origin_size(p, (sz.x, sz.y)),
        if matches!(t.kind, Kind::Var(_)) {
            12.
        } else {
            8.
        },
        fill,
    )
    .text_anchor(TextAnchor::Middle)
    .text_baseline(TextBaseline::Central)
    .font_family(FontFamily::Mono);
    if let Shape::Rect { stroke: s, .. } = &mut o.shapes[0] {
        *s = stroke;
    }
    if !label.is_empty() {
        o = o.label(label, (label_x, sz.y / 2.), 15., label_color);
    }
    if matches!(t.kind, Kind::Var(_)) {
        o = o.italic();
    }
    o.parent = parent.map(str::to_owned);
    o.emerge_from = t.emerge.clone();
    o.emerge_mode = t.emerge_mode;
    if matches!(t.kind, Kind::Hole(_)) {
        o.hit = None;
        o = o.dash(vec![4., 3.]);
    }
    scene.push(o);
    match &t.kind {
        Kind::Lam {
            param,
            body,
            incoming,
        } => {
            let mut bs = bound.clone();
            bs.insert(param.clone());
            let lw = (param.chars().count() + 2) as f64 * 8.4;
            render(
                scene,
                body,
                p + Vec2::new(12. + lw, 6.),
                &bs,
                Some(&t.id),
                dragged,
            );
            if let Some(i) = incoming {
                render(
                    scene,
                    i,
                    p + Vec2::new(14.4, (sz.y - size(i).y) / 2.),
                    bound,
                    Some(&t.id),
                    dragged,
                );
            }
        }
        Kind::App(f, a) => {
            let fs = size(f);
            let as_ = size(a);
            let stacked = fs.x + as_.x + 18. > 460.;
            let h = fs.y.max(as_.y);
            let fp = if stacked {
                Vec2::new(6., 6.)
            } else {
                Vec2::new(6., 6. + (h - fs.y) / 2.)
            };
            let ap = if stacked {
                Vec2::new(24., 12. + fs.y)
            } else {
                Vec2::new(12. + fs.x, 6. + (h - as_.y) / 2.)
            };
            render(scene, f, p + fp, bound, Some(&t.id), dragged);
            render(scene, a, p + ap, bound, Some(&t.id), dragged);
        }
        _ => {}
    }
}
const PRESETS: [(&str, &str); 12] = [
    ("duplicate", "(λx. x x) y"),
    ("flip", "(λx y. y x) a b"),
    ("nested", "(λx. x) ((λy. y) z)"),
    ("K", "(λx y. x) a b"),
    ("if true", "(λb t e. b t e) (λt e. t) a b"),
    ("succ 2", "(λn f x. f (n f x)) (λf x. f (f x))"),
    (
        "1 + 2",
        "(λm n f x. m f (n f x)) (λf x. f x) (λf x. f (f x))",
    ),
    ("2 × 2", "(λm n f. m (n f)) (λf x. f (f x)) (λf x. f (f x))"),
    ("Ω", "(λx. x x) (λx. x x)"),
    ("Ω₃", "(λx. x x x) (λx. x x x)"),
    ("Y g", "(λf. (λx. f (x x)) (λx. f (x x))) g"),
    ("eta", "λx. f x"),
];
const PRESET_IDS: [&str; 12] = [
    "preset-0",
    "preset-1",
    "preset-2",
    "preset-3",
    "preset-4",
    "preset-5",
    "preset-6",
    "preset-7",
    "preset-8",
    "preset-9",
    "preset-10",
    "preset-11",
];
const REWRITE_ROWS: [(usize, &str, &str, &str); 5] = [
    (0, "beta", "β-reduction", "drag an argument into its λ"),
    (
        1,
        "abstract",
        "Abstraction (reverse β)",
        "drag a subterm out of an enclosing box",
    ),
    (
        2,
        "all",
        "…abstracting over all identical occurrences",
        "(copies merge back into the dragged one)",
    ),
    (4, "eta-reduce", "η-reduction", "drag f out of λx. f x"),
    (
        3,
        "eta-expand",
        "η-expansion",
        "drag any subterm slightly right",
    ),
];
fn header(scene: &mut Scene, s: &LambdaState) -> f64 {
    let mut x = 0.;
    let mut y = 0.;
    for (i, (label, _)) in PRESETS.iter().enumerate() {
        let w = 16. + label.chars().count() as f64 * 6.3;
        if x > 0. && x + w > 680. {
            x = 0.;
            y += 30.;
        }
        let selected = i == s.preset;
        scene
            .inline_control(PRESET_IDS[i], Rect::new(x, y, x + w, y + 24.))
            .font_size(12.)
            .foreground(if selected {
                [30, 64, 175, 255]
            } else {
                [55, 65, 81, 255]
            })
            .background(if selected {
                [239, 246, 255, 255]
            } else {
                [255, 255, 255, 255]
            })
            .border(if selected {
                [96, 165, 250, 255]
            } else {
                [209, 213, 219, 255]
            });
        x += w + 6.;
    }
    y += 32.;
    let text = format!("{} (Click the button again to reset.)", Lambda.status(s));
    let mut lines = vec![String::new()];
    for word in text.split_whitespace() {
        if lines.last().unwrap().chars().count() + word.chars().count() + 1 > 110 {
            lines.push(String::new());
        }
        let line = lines.last_mut().unwrap();
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    for (i, line) in lines.iter().enumerate() {
        scene.push(Object::text(
            format!("preset-blurb-{i}"),
            (0., y),
            line,
            12.,
            [75, 85, 99, 255],
        ));
        y += 16.;
    }
    y + 8.
}
impl Example for Lambda {
    type State = LambdaState;
    fn initial(&self) -> LambdaState {
        LambdaState {
            term: parse(PRESETS[0].1).unwrap(),
            preset: 0,
            opts: [true, true, false, false, false],
        }
    }
    fn scene(&self, s: &LambdaState) -> Scene {
        self.scene_dragging(s, None)
    }
    fn scene_dragging(&self, s: &LambdaState, dragged: Option<&str>) -> Scene {
        let mut sc = Scene::new(680., 420.).scrolling();
        let offset = header(&mut sc, s);
        let body_start = sc.objects.len();
        sc.push(
            Object::text(
                "expression",
                (20., 24.),
                print(&s.term),
                13.,
                [107, 114, 128, 255],
            )
            .font_family(FontFamily::Mono)
            .text_baseline(TextBaseline::Alphabetic),
        );
        register_hues(&s.term);
        render(
            &mut sc,
            &s.term,
            Point::new(20., 44.),
            &BTreeSet::new(),
            None,
            dragged,
        );
        super::tree::raise_group(&mut sc, dragged);
        for o in sc.objects.iter_mut().skip(body_start) {
            o.transform = Affine::translate((0., offset)) * o.transform;
        }
        let panel_y = offset + 420. + 16.;
        let mut y = panel_y + 12.;
        sc.push(
            Object::text(
                "rewrites-title",
                (12., y),
                "Rewrites",
                12.,
                [55, 65, 81, 255],
            )
            .font_weight(500),
        );
        y += 24.;
        for (_, id, _, description) in REWRITE_ROWS {
            sc.inline_control(id, Rect::new(12., y, 650., y + 22.))
                .font_size(12.);
            sc.push(Object::text(
                format!("{id}-description"),
                (36., y + 23.),
                description,
                12.,
                [55, 65, 81, 255],
            ));
            y += 46.;
        }
        sc.height = y + 12.;
        sc.objects.insert(
            0,
            Object::rect(
                "rewrite-panel",
                Rect::new(0., panel_y, 680., sc.height),
                4.,
                [249, 250, 251, 255],
            )
            .decorative(),
        );
        sc
    }
    fn drag(
        &self,
        s: &LambdaState,
        id: &str,
        _: Modifiers,
    ) -> Option<DragSpec<'static, LambdaState>> {
        s.term.path(id)?;
        let cs = candidates(&s.term, id, s.opts);
        if cs.is_empty() {
            return Some(between([s.clone()]));
        }
        let mut branches = vec![];
        for c in cs {
            let mut start = s.clone();
            start.term = c.base;
            let mut target = s.clone();
            target.term = c.mid.unwrap_or_else(|| c.result.clone());
            let mid = target.term.clone();
            let result = c.result;
            branches.push(between([start, target]).on_drop(move |s| {
                let mut s = s.clone();
                if s.term == mid {
                    s.term = result.clone();
                }
                s
            }));
        }
        Some(
            closest(branches).with_drop_transition(Some(dragology::Transition {
                duration: std::time::Duration::from_millis(450),
                ..Default::default()
            })),
        )
    }
    fn action_transition(&self, _: &LambdaState, _: &str) -> Option<dragology::Transition> {
        None
    }
    fn controls(&self, s: &LambdaState) -> Vec<Control> {
        PRESETS
            .iter()
            .enumerate()
            .map(|(i, (name, _))| Control::action_button(PRESET_IDS[i], *name, true, s.preset == i))
            .chain(
                REWRITE_ROWS
                    .iter()
                    .map(|(i, id, label, _)| Control::toggle(id, *label, s.opts[*i])),
            )
            .collect()
    }
    fn action(&self, s: &LambdaState, id: &str, v: Value) -> Option<LambdaState> {
        let mut n = s.clone();
        if let Some(i) = PRESET_IDS.iter().position(|preset| *preset == id) {
            n.preset = i;
            n.term = parse(PRESETS[i].1).unwrap();
            return Some(n);
        }
        match (id, v) {
            ("preset", Value::Index(i)) if i < PRESETS.len() => {
                n.preset = i;
                n.term = parse(PRESETS[i].1).unwrap();
            }
            (id, Value::Bool(b)) => {
                let i = ["beta", "abstract", "all", "eta-expand", "eta-reduce"]
                    .iter()
                    .position(|x| *x == id)?;
                n.opts[i] = b
            }
            _ => return None,
        }
        Some(n)
    }
    fn status(&self, s: &LambdaState) -> String {
        const BLURBS: [&str; 12] = [
            "Drag y onto either x: a copy splits off for the other one.",
            "Two steps: feed in a, then b.",
            "Two redexes. Drag z into λy first, or drag the whole inner term into λx.",
            "Feed in a, then b. The second λ never uses its parameter, so b is thrown away.",
            "Church booleans: true picks its first argument and discards the second. Reduces to a.",
            "Church numerals: successor of 2 reduces to λf x. f (f (f x)).",
            "Church addition: m f (n f x) applies f n times, then m more times. Reduces to 3.",
            "Church multiplication: 2 (2 f) duplicates (2 f), and each copy unfolds into two f's. Reduces to 4.",
            "Reduces to itself, forever.",
            "Like Ω, but every step makes the term bigger.",
            "The Y combinator unrolls recursion: Y g → g (Y′ g) → g (g (Y′ g)) → … with a new g each time.",
            "Enable η-reduction and drag f out.",
        ];
        BLURBS[s.preset].into()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_source_preset_button_restarts_its_term_and_preserves_rewrite_options() {
        let mut state = Lambda.initial();
        state.opts = [true, false, true, true, true];
        for (i, id) in PRESET_IDS.iter().enumerate() {
            let next = Lambda.action(&state, id, Value::Trigger).unwrap();
            assert_eq!(print(&next.term), print(&parse(PRESETS[i].1).unwrap()));
            assert_eq!(next.opts, state.opts);
            let again = Lambda.action(&next, id, Value::Trigger).unwrap();
            assert_ne!(again.term.id, next.term.id);
            assert!(
                Lambda
                    .scene(&again)
                    .inline_controls
                    .iter()
                    .any(|p| p.id == *id)
            );
        }
    }
    #[test]
    fn unused_beta_argument_disappears_without_a_placeholder_outline() {
        let t = parse("(λx. y) z").unwrap();
        let Kind::App(_, argument) = &t.kind else {
            panic!()
        };
        let c = candidates(&t, &argument.id, [true, false, false, false, false]).remove(0);
        let Kind::App(_, hole) = &c.mid.unwrap().kind else {
            panic!()
        };
        assert!(!hole.hole_outline);
        assert_eq!(print(&c.result), "y");
        assert_eq!(print(&parse("λx. λy. y x").unwrap()), "λx y. y x");
    }
    #[test]
    fn substitution_avoids_capture_and_respects_shadowing() {
        let body = parse("λy. x y").unwrap();
        let result = substitute(&body, "x", &var("y"));
        assert_eq!(result.free(), BTreeSet::from(["y".into()]));
        assert_eq!(print(&result), "λy'. y y'");
        let shadow = parse("λx. x").unwrap();
        assert_eq!(substitute(&shadow, "x", &var("z")), shadow);
    }
    #[test]
    fn beta_waypoint_retains_pointer_identity_then_duplicates() {
        let t = parse("(λx. x x) y").unwrap();
        let Kind::App(_, a) = &t.kind else { panic!() };
        let cs = candidates(&t, &a.id, [true, false, false, false, false]);
        assert_eq!(cs.len(), 1);
        assert_eq!(print(&cs[0].result), "y y");
        let mid = cs[0].mid.as_ref().unwrap();
        let Kind::App(f, _) = &mid.kind else { panic!() };
        let Kind::Lam {
            incoming: Some(i), ..
        } = &f.kind
        else {
            panic!()
        };
        assert_eq!(i.id, a.id);
        let ids: BTreeSet<_> = cs[0].result.nodes().iter().map(|t| &t.id).collect();
        assert_eq!(ids.len(), cs[0].result.nodes().len());
    }
    #[test]
    fn abstraction_never_moves_bound_variable_out_of_binder() {
        let t = parse("λx. f x").unwrap();
        let x = t
            .nodes()
            .into_iter()
            .find(|n| matches!(&n.kind,Kind::Var(s)if s=="x"))
            .unwrap();
        let cs = candidates(&t, &x.id, [false, true, false, false, false]);
        assert_eq!(cs.len(), 1);
        assert!(cs.iter().all(|c| c.result.free() == t.free()));
    }
    #[test]
    fn every_preset_parses_and_prints_round_trip() {
        for (_, s) in PRESETS {
            let t = parse(s).unwrap();
            assert!(t.same(&parse(&print(&t)).unwrap()));
        }
    }
    #[test]
    fn beta_waypoint_merges_argument_as_clone_while_copies_use_auto_emergence() {
        let t = parse("(λx. x x) (f a)").unwrap();
        let Kind::App(_, arg) = &t.kind else { panic!() };
        let c = candidates(&t, &arg.id, [true, false, false, false, false]).remove(0);
        let Kind::App(f, _) = &c.mid.as_ref().unwrap().kind else {
            panic!()
        };
        let Kind::Lam {
            body,
            incoming: Some(incoming),
            ..
        } = &f.kind
        else {
            panic!()
        };
        let copy = &body.children()[0];
        for (node, destination) in incoming.nodes().into_iter().zip(copy.nodes()) {
            assert_eq!(node.emerge_mode, EmergeMode::Clone);
            assert_eq!(node.emerge.as_deref(), Some(destination.id.as_str()));
        }
        for child in body.children() {
            assert!(
                child
                    .nodes()
                    .iter()
                    .all(|n| n.emerge_mode == EmergeMode::Auto)
            );
        }
        let mut state = Lambda.initial();
        state.term = c.mid.unwrap();
        let scene = Lambda.scene(&state);
        assert_eq!(
            scene.object(&arg.id).unwrap().emerge_mode,
            EmergeMode::Clone
        );
    }
    #[test]
    fn abstraction_merges_all_duplicate_subtrees_into_the_grabbed_copy() {
        let t = parse("(f a) (f a)").unwrap();
        let Kind::App(dragged, other) = &t.kind else {
            panic!()
        };
        let c = candidates(&t, &dragged.id, [false, true, true, false, false]).remove(0);
        let disappearing = c.base.path(&other.id).unwrap();
        let disappearing = disappearing.last().unwrap();
        for (copy, original) in disappearing.nodes().into_iter().zip(dragged.nodes()) {
            assert_eq!(copy.emerge_mode, EmergeMode::Clone);
            assert_eq!(copy.emerge.as_deref(), Some(original.id.as_str()));
        }
    }
    #[test]
    fn native_beta_drag_substitutes_and_collapses_the_binder() {
        let mut d = crate::runtime::ModelDemo::new(Lambda);
        d.set_animated(false);
        let object = |d: &crate::runtime::ModelDemo<Lambda>, label: &str| {
            d.scene()
                .objects
                .iter()
                .find(|o| {
                    o.shapes
                        .iter()
                        .any(|s| matches!(s,Shape::Text{text,..}if text==label))
                })
                .unwrap()
                .clone()
        };
        let arg = object(&d, "y");
        let binder = object(&d, "λx.");
        let from = arg.transform * Point::new(8., 12.);
        let height = if let Some(Hit::Rect(r)) = &binder.hit {
            r.height()
        } else {
            panic!()
        };
        let to = binder.transform * Point::new(22.4, height / 2.);
        d.press(from, Modifiers::default(), 1);
        d.move_to(to, Modifiers::default());
        d.advance(std::time::Duration::ZERO);
        d.release(to, Modifiers::default());
        let expression = d.scene().object("expression").unwrap();
        assert!(
            expression
                .shapes
                .iter()
                .any(|s| matches!(s,Shape::Text{text,..}if text=="y y")),
            "{}",
            d.status()
        );
    }
    #[test]
    fn eta_reduction_checks_free_variables() {
        let t = parse("λx. x x").unwrap();
        let Kind::Lam { body, .. } = &t.kind else {
            panic!()
        };
        let Kind::App(f, _) = &body.kind else {
            panic!()
        };
        assert!(candidates(&t, &f.id, [false, false, false, false, true]).is_empty());
    }
}
