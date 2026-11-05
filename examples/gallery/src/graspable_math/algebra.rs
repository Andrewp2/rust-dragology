//! Expression normalization and algebra from the upstream Graspable Math demo.
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT_ID: AtomicU64 = AtomicU64::new(1);
pub fn new_id() -> u64 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

#[derive(Clone, Debug, PartialEq)]
pub struct Expr {
    pub id: u64,
    pub kind: Kind,
}
#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    Num(f64),
    Var(String),
    Sum(Vec<Term>),
    Prod(Vec<Expr>),
    Frac(Box<Expr>, Box<Expr>),
    Pow(Box<Expr>, Box<Expr>),
    Log(Box<Expr>, Box<Expr>),
}
#[derive(Clone, Debug, PartialEq)]
pub struct Term {
    pub sign: i8,
    pub expr: Expr,
}
pub fn positive(expr: Expr) -> Term {
    Term { sign: 1, expr }
}
pub fn num(n: f64) -> Expr {
    num_id(n, new_id())
}
pub fn num_id(n: f64, id: u64) -> Expr {
    Expr {
        id,
        kind: Kind::Num(n),
    }
}
pub fn is_num(e: &Expr, n: f64) -> bool {
    matches!(&e.kind,Kind::Num(v) if *v==n)
}
pub fn terms(e: &Expr) -> Vec<Term> {
    match &e.kind {
        Kind::Sum(ts) => ts.clone(),
        _ => vec![positive(e.clone())],
    }
}
pub fn factors(e: &Expr) -> Vec<Expr> {
    match &e.kind {
        Kind::Prod(fs) => fs.clone(),
        _ => vec![e.clone()],
    }
}
pub fn sum(ts: Vec<Term>, id: u64) -> Expr {
    let mut out = Vec::new();
    for t in ts {
        match t.expr.kind {
            Kind::Sum(us) => out.extend(us.into_iter().map(|u| Term {
                sign: t.sign * u.sign,
                expr: u.expr,
            })),
            _ => out.push(t),
        }
    }
    if out.is_empty() {
        num(0.)
    } else if out.len() == 1 && out[0].sign == 1 {
        out.remove(0).expr
    } else {
        Expr {
            id,
            kind: Kind::Sum(out),
        }
    }
}
pub fn prod(fs: Vec<Expr>, id: u64) -> Expr {
    let mut out = Vec::new();
    for f in fs {
        match f.kind {
            Kind::Prod(gs) => out.extend(gs),
            _ => out.push(f),
        }
    }
    if out.is_empty() {
        num(1.)
    } else if out.len() == 1 {
        out.remove(0)
    } else {
        Expr {
            id,
            kind: Kind::Prod(out),
        }
    }
}
pub fn frac(a: Expr, b: Expr, id: u64) -> Expr {
    if is_num(&b, 1.) {
        a
    } else {
        Expr {
            id,
            kind: Kind::Frac(Box::new(a), Box::new(b)),
        }
    }
}
pub fn pow(a: Expr, b: Expr, id: u64) -> Expr {
    if is_num(&b, 1.) {
        a
    } else {
        Expr {
            id,
            kind: Kind::Pow(Box::new(a), Box::new(b)),
        }
    }
}
pub fn log(a: Expr, b: Expr, id: u64) -> Expr {
    Expr {
        id,
        kind: Kind::Log(Box::new(a), Box::new(b)),
    }
}
pub fn normalize(e: Expr) -> Expr {
    let id = e.id;
    match e.kind {
        Kind::Sum(ts) => sum(
            ts.into_iter()
                .map(|t| Term {
                    sign: t.sign,
                    expr: normalize(t.expr),
                })
                .collect(),
            id,
        ),
        Kind::Prod(fs) => prod(fs.into_iter().map(normalize).collect(), id),
        Kind::Frac(a, b) => frac(normalize(*a), normalize(*b), id),
        Kind::Pow(a, b) => pow(normalize(*a), normalize(*b), id),
        Kind::Log(a, b) => log(normalize(*a), normalize(*b), id),
        _ => e,
    }
}
pub fn replace(e: &Expr, id: u64, r: &Expr) -> Expr {
    if e.id == id {
        return r.clone();
    }
    let kind = match &e.kind {
        Kind::Sum(ts) => Kind::Sum(
            ts.iter()
                .map(|t| Term {
                    sign: t.sign,
                    expr: replace(&t.expr, id, r),
                })
                .collect(),
        ),
        Kind::Prod(fs) => Kind::Prod(fs.iter().map(|f| replace(f, id, r)).collect()),
        Kind::Frac(a, b) => Kind::Frac(Box::new(replace(a, id, r)), Box::new(replace(b, id, r))),
        Kind::Pow(a, b) => Kind::Pow(Box::new(replace(a, id, r)), Box::new(replace(b, id, r))),
        Kind::Log(a, b) => Kind::Log(Box::new(replace(a, id, r)), Box::new(replace(b, id, r))),
        _ => e.kind.clone(),
    };
    normalize(Expr { id: e.id, kind })
}
pub fn find(e: &Expr, id: u64) -> Option<&Expr> {
    if e.id == id {
        return Some(e);
    }
    match &e.kind {
        Kind::Sum(ts) => ts.iter().find_map(|t| find(&t.expr, id)),
        Kind::Prod(fs) => fs.iter().find_map(|f| find(f, id)),
        Kind::Frac(a, b) | Kind::Pow(a, b) | Kind::Log(a, b) => find(a, id).or_else(|| find(b, id)),
        _ => None,
    }
}
pub fn fresh(e: &Expr) -> Expr {
    let kind = match &e.kind {
        Kind::Sum(ts) => Kind::Sum(
            ts.iter()
                .map(|t| Term {
                    sign: t.sign,
                    expr: fresh(&t.expr),
                })
                .collect(),
        ),
        Kind::Prod(fs) => Kind::Prod(fs.iter().map(fresh).collect()),
        Kind::Frac(a, b) => Kind::Frac(Box::new(fresh(a)), Box::new(fresh(b))),
        Kind::Pow(a, b) => Kind::Pow(Box::new(fresh(a)), Box::new(fresh(b))),
        Kind::Log(a, b) => Kind::Log(Box::new(fresh(a)), Box::new(fresh(b))),
        _ => e.kind.clone(),
    };
    Expr { id: new_id(), kind }
}
pub fn key(e: &Expr) -> String {
    match &e.kind {
        Kind::Num(v) => v.to_string(),
        Kind::Var(v) => v.clone(),
        Kind::Sum(ts) => format!(
            "({})",
            ts.iter()
                .map(|t| format!("{}{}", if t.sign < 0 { "-" } else { "+" }, key(&t.expr)))
                .collect::<String>()
        ),
        Kind::Prod(fs) => {
            let mut ks: Vec<_> = fs.iter().map(key).collect();
            ks.sort();
            format!("[{}]", ks.join("*"))
        }
        Kind::Frac(a, b) => format!("{{{}/{}}}", key(a), key(b)),
        Kind::Pow(a, b) => format!("<{}^{}>", key(a), key(b)),
        Kind::Log(a, b) => format!("log<{}>({})", key(a), key(b)),
    }
}
fn coefficient(e: &Expr) -> Option<(f64, Option<u64>, Vec<Expr>)> {
    let fs = factors(e);
    if fs
        .iter()
        .any(|f| matches!(f.kind, Kind::Sum(_) | Kind::Frac(..)))
    {
        return None;
    }
    let mut c = 1.;
    let mut id = None;
    let mut rest = Vec::new();
    for f in fs {
        if let Kind::Num(v) = f.kind {
            c *= v;
            id = id.or(Some(f.id))
        } else {
            rest.push(f)
        }
    }
    Some((c, id, rest))
}
pub fn combine(t: &Term, u: &Term) -> Option<Vec<Term>> {
    let (a, _, ar) = coefficient(&t.expr)?;
    let (b, bid, br) = coefficient(&u.expr)?;
    let mut ak: Vec<_> = ar.iter().map(key).collect();
    let mut bk: Vec<_> = br.iter().map(key).collect();
    ak.sort();
    bk.sort();
    if ak != bk {
        return None;
    }
    let c = t.sign as f64 * a + u.sign as f64 * b;
    if c == 0. {
        return Some(Vec::new());
    }
    let mut fs = br;
    if fs.is_empty() || c.abs() != 1. {
        fs.insert(0, num_id(c.abs(), bid.unwrap_or_else(new_id)))
    }
    Some(vec![Term {
        sign: if c < 0. { -1 } else { 1 },
        expr: prod(
            fs,
            if matches!(u.expr.kind, Kind::Prod(_)) {
                u.expr.id
            } else {
                new_id()
            },
        ),
    }])
}
fn power(e: &Expr) -> Option<(Expr, f64)> {
    match &e.kind {
        Kind::Num(_) => None,
        Kind::Pow(a, b) => {
            if let Kind::Num(n) = b.kind {
                Some(((**a).clone(), n))
            } else {
                None
            }
        }
        _ => Some((e.clone(), 1.)),
    }
}
pub fn merge(f: &Expr, g: &Expr) -> Option<Expr> {
    if let (Kind::Num(a), Kind::Num(b)) = (&f.kind, &g.kind) {
        return Some(num_id(a * b, g.id));
    }
    let (a, n) = power(f)?;
    let (b, m) = power(g)?;
    if key(&a) != key(&b) {
        return None;
    }
    Some(if let Kind::Pow(base, exp) = &g.kind {
        pow((**base).clone(), num_id(n + m, exp.id), g.id)
    } else {
        pow(g.clone(), num(n + m), new_id())
    })
}
fn multiply_factor(f: Expr, e: Expr) -> Expr {
    if is_num(&f, 1.) {
        return e;
    }
    if is_num(&e, 1.) {
        return f;
    }
    let id = if matches!(e.kind, Kind::Prod(_)) {
        e.id
    } else {
        new_id()
    };
    let mut fs = factors(&e);
    for g in &mut fs {
        if let Some(m) = merge(&f, g) {
            *g = m;
            return prod(fs, id);
        }
    }
    let at = if matches!(f.kind, Kind::Num(_)) {
        0
    } else {
        fs.iter()
            .take_while(|g| matches!(g.kind, Kind::Num(_)))
            .count()
    };
    fs.insert(at, f);
    prod(fs, id)
}
fn multiply_into(f: Expr, e: Expr) -> Expr {
    match e.kind {
        Kind::Frac(a, b) => frac(multiply_into(f, *a), *b, e.id),
        Kind::Sum(_) => prod(vec![f, e], new_id()),
        _ => factors(&f)
            .into_iter()
            .fold(e, |acc, g| multiply_factor(g, acc)),
    }
}
pub fn distribute(f: &Expr, g: &Expr) -> Option<Expr> {
    let Kind::Sum(us) = &g.kind else { return None };
    let mut out = Vec::new();
    for (i, t) in terms(f).into_iter().enumerate() {
        for u in us {
            out.push(Term {
                sign: t.sign * u.sign,
                expr: multiply_into(
                    fresh(&t.expr),
                    if i == 0 {
                        u.expr.clone()
                    } else {
                        fresh(&u.expr)
                    },
                ),
            })
        }
    }
    Some(sum(out, g.id))
}
pub fn gcd(mut a: f64, mut b: f64) -> f64 {
    while b != 0. {
        let r = a % b;
        a = b;
        b = r;
    }
    a.abs()
}
pub fn cancel(f: &Expr, g: &Expr) -> Option<(Option<Expr>, Option<Expr>)> {
    if let (Kind::Num(a), Kind::Num(b)) = (&f.kind, &g.kind) {
        let k = gcd(*a, *b);
        if k <= 1. {
            return None;
        }
        return Some((
            if *a == k { None } else { Some(num(a / k)) },
            if *b == k {
                None
            } else {
                Some(num_id(b / k, g.id))
            },
        ));
    }
    let (a, n) = power(f)?;
    let (b, m) = power(g)?;
    if key(&a) != key(&b) || n.min(m) == 0. {
        return None;
    }
    let k = n.min(m);
    fn lower(e: &Expr, n: f64) -> Option<Expr> {
        if n == 0. {
            None
        } else if let Kind::Pow(a, b) = &e.kind {
            Some(pow((**a).clone(), num_id(n, b.id), e.id))
        } else {
            Some(e.clone())
        }
    }
    Some((lower(f, n - k).as_ref().map(fresh), lower(g, m - k)))
}
pub fn eval_pow(base: &Expr, exp: &Expr) -> Option<f64> {
    let Kind::Num(b) = base.kind else { return None };
    match &exp.kind {
        Kind::Num(n) => {
            let v = b.powf(*n);
            (v.is_finite() && v <= 1e6).then_some(v)
        }
        Kind::Frac(a, d) if is_num(a, 1.) => {
            if let Kind::Num(n) = d.kind {
                let r = b.powf(1. / n).round();
                (r.powf(n) == b).then_some(r)
            } else {
                None
            }
        }
        _ => None,
    }
}
pub fn eval_log(b: &Expr, a: &Expr) -> Option<Expr> {
    if is_num(a, 1.) {
        return Some(num_id(0., a.id));
    }
    if let Kind::Pow(base, e) = &a.kind {
        if key(base) == key(b) {
            return Some((**e).clone());
        }
    }
    if key(a) == key(b) {
        return Some(num_id(1., a.id));
    }
    let (Kind::Num(bv), Kind::Num(av)) = (&b.kind, &a.kind) else {
        return None;
    };
    if *bv < 2. {
        return None;
    }
    let k = av.log(*bv).round();
    (k >= 0. && bv.powf(k) == *av).then(|| num_id(k, a.id))
}
pub fn divided(e: &Expr, f: &Expr) -> Vec<Expr> {
    match &e.kind {
        Kind::Sum(ts) if ts.len() == 1 => divided(&ts[0].expr, f)
            .into_iter()
            .map(|x| {
                sum(
                    vec![Term {
                        sign: ts[0].sign,
                        expr: x,
                    }],
                    e.id,
                )
            })
            .collect(),
        Kind::Frac(a, b) => {
            let fs = factors(b);
            (0..=fs.len())
                .map(|j| {
                    let mut out = fs.clone();
                    out.insert(j, f.clone());
                    frac(
                        (**a).clone(),
                        prod(
                            out,
                            if matches!(b.kind, Kind::Prod(_)) {
                                b.id
                            } else {
                                new_id()
                            },
                        ),
                        e.id,
                    )
                })
                .collect()
        }
        _ => vec![frac(e.clone(), f.clone(), new_id())],
    }
}
pub fn multiplied(e: &Expr, f: &Expr) -> Vec<Expr> {
    match &e.kind {
        Kind::Sum(ts) if ts.len() == 1 => multiplied(&ts[0].expr, f)
            .into_iter()
            .map(|x| {
                sum(
                    vec![Term {
                        sign: ts[0].sign,
                        expr: x,
                    }],
                    e.id,
                )
            })
            .collect(),
        Kind::Frac(a, b) => {
            let fs = factors(a);
            (0..=fs.len())
                .map(|j| {
                    let mut out = fs.clone();
                    out.insert(j, f.clone());
                    frac(
                        prod(
                            out,
                            if matches!(a.kind, Kind::Prod(_)) {
                                a.id
                            } else {
                                new_id()
                            },
                        ),
                        (**b).clone(),
                        e.id,
                    )
                })
                .collect()
        }
        Kind::Sum(_) => vec![
            prod(vec![f.clone(), e.clone()], new_id()),
            prod(vec![e.clone(), f.clone()], new_id()),
        ],
        _ => {
            let fs = factors(e);
            (0..=fs.len())
                .map(|j| {
                    let mut out = fs.clone();
                    out.insert(j, f.clone());
                    prod(
                        out,
                        if matches!(e.kind, Kind::Prod(_)) {
                            e.id
                        } else {
                            new_id()
                        },
                    )
                })
                .collect()
        }
    }
}
pub fn is_number(e: &Expr) -> bool {
    match &e.kind {
        Kind::Num(_) => true,
        Kind::Sum(ts) => ts.len() == 1 && is_number(&ts[0].expr),
        Kind::Frac(a, b) => {
            matches!((&a.kind,&b.kind),(Kind::Num(x),Kind::Num(y)) if gcd(*x,*y)==1.)
        }
        _ => false,
    }
}

pub const EQUATIONS: [&str; 11] = [
    "2x + 3 = 11",
    "3x - 5 = x + 7",
    "x/4 + 1 = 3",
    "2(x + 1) = 10",
    "(x + 6)/3 = 2x - 3",
    "(x + 3)^2 = x^2 + 15",
    "2x^3 = 54",
    "x^5/x^2 = 8",
    "2^(x + 1) = 32",
    "log_3(x) = 4",
    "log_2(x^3) = 6",
];
pub fn parse(src: &str) -> (Expr, Expr) {
    struct Parser {
        t: Vec<String>,
        i: usize,
    }
    impl Parser {
        fn peek(&self) -> &str {
            self.t.get(self.i).map_or("", String::as_str)
        }
        fn next(&mut self) -> String {
            let t = self.peek().to_string();
            self.i += 1;
            t
        }
        fn sum(&mut self) -> Expr {
            let sign = if self.peek() == "-" {
                self.next();
                -1
            } else {
                1
            };
            let mut ts = vec![Term {
                sign,
                expr: self.term(),
            }];
            while matches!(self.peek(), "+" | "-") {
                let sign = if self.next() == "-" { -1 } else { 1 };
                ts.push(Term {
                    sign,
                    expr: self.term(),
                })
            }
            sum(ts, new_id())
        }
        fn term(&mut self) -> Expr {
            let mut fs = vec![self.factor()];
            while self.peek() == "*"
                || self.peek() == "("
                || self.peek().starts_with(|c: char| c.is_ascii_alphanumeric())
            {
                if self.peek() == "*" {
                    self.next();
                }
                fs.push(self.factor())
            }
            let mut e = prod(fs, new_id());
            while self.peek() == "/" {
                self.next();
                e = frac(e, self.factor(), new_id())
            }
            e
        }
        fn factor(&mut self) -> Expr {
            let base = self.atom();
            if self.peek() != "^" {
                return base;
            }
            self.next();
            pow(base, self.atom(), new_id())
        }
        fn atom(&mut self) -> Expr {
            let t = self.next();
            if t == "(" {
                let e = self.sum();
                assert_eq!(self.next(), ")");
                return e;
            }
            if t == "log" {
                assert_eq!(self.next(), "_");
                let base = self.atom();
                return log(base, self.atom(), new_id());
            }
            if let Ok(n) = t.parse::<f64>() {
                num(n)
            } else {
                assert!(!t.is_empty());
                Expr {
                    id: new_id(),
                    kind: Kind::Var(t),
                }
            }
        }
    }
    let chars: Vec<_> = src.chars().filter(|c| !c.is_whitespace()).collect();
    let mut t = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let start = i;
        if chars[i].is_ascii_digit() {
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1
            }
        } else if chars[i..].starts_with(&['l', 'o', 'g']) {
            i += 3
        } else {
            i += 1
        }
        t.push(chars[start..i].iter().collect())
    }
    let mut p = Parser { t, i: 0 };
    let a = p.sum();
    assert_eq!(p.next(), "=");
    let b = p.sum();
    assert_eq!(p.peek(), "");
    (a, b)
}
