//! Stock-and-flow dynamics and presets from the Dragology contributors.
//! The JSON schema intentionally matches the upstream demo's saved models.
use super::*;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
const STOCK_TOP: f64 = 64.;
const PRESET_IDS: [&str; 9] = [
    "stock-p0", "stock-p1", "stock-p2", "stock-p3", "stock-p4", "stock-p5", "stock-p6", "stock-p7",
    "stock-p8",
];
use std::{
    cell::RefCell,
    collections::{BTreeMap, VecDeque},
    sync::OnceLock,
};
const STEPS: usize = 400;
const SHAPES: [&str; 6] = [
    "linear",
    "threshold",
    "gap",
    "saturating",
    "crowding",
    "repression",
];
#[derive(Clone, PartialEq, Serialize, Deserialize)]
struct Param {
    label: String,
    value: f64,
    min: f64,
    max: f64,
}
#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
struct Position {
    x: f64,
    y: f64,
}
impl Position {
    fn point(self) -> Point {
        Point::new(self.x, self.y)
    }
}
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
enum End {
    Id(String),
    Cloud(Position),
}
#[derive(Clone, PartialEq, Serialize, Deserialize)]
struct Stock {
    name: String,
    symbol: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    unit: Option<String>,
    x: f64,
    y: f64,
    color: String,
    init: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    volume: Option<String>,
    scale: [f64; 2],
}
#[derive(Clone, PartialEq, Serialize, Deserialize)]
struct Flow {
    name: String,
    from: End,
    to: End,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    rate: Option<String>,
}
#[derive(Clone, PartialEq, Serialize, Deserialize)]
struct Link {
    from: String,
    to: End,
    shape: String,
    params: Vec<String>,
}
#[derive(Clone, PartialEq, Serialize, Deserialize)]
struct Model {
    stocks: IndexMap<String, Stock>,
    flows: IndexMap<String, Flow>,
    links: IndexMap<String, Link>,
    params: IndexMap<String, Param>,
    #[serde(rename = "T")]
    duration: f64,
    #[serde(rename = "Trange")]
    trange: [f64; 2],
    #[serde(rename = "timeUnit")]
    time_unit: String,
    cursor: f64,
    sweep: Option<String>,
}
#[derive(Deserialize)]
struct Preset {
    label: String,
    model: Model,
}
fn presets() -> &'static Vec<Preset> {
    static PRESETS: OnceLock<Vec<Preset>> = OnceLock::new();
    PRESETS.get_or_init(|| {
        serde_json::from_str(include_str!("stock/presets.json"))
            .expect("bundled stock-flow presets")
    })
}
#[derive(Clone, PartialEq)]
pub struct State {
    model: Model,
    preset: usize,
    selected: String,
    param: Option<String>,
    creating: Option<(String, Point)>,
    history: Vec<Model>,
    pending_original: Option<Box<Model>>,
}
pub struct StockFlow;
fn factor(shape: &str, x: f64, p: &[f64]) -> f64 {
    let a = p.first().copied().unwrap_or(1.);
    match shape {
        "linear" => x,
        "threshold" => (x - a).max(0.),
        "gap" => a - x,
        "saturating" => {
            let xp = x.max(0.);
            if xp + a > 0. { xp / (a + xp) } else { 0. }
        }
        "crowding" => {
            if a > 0. {
                (1. - x / a).max(0.)
            } else {
                0.
            }
        }
        "repression" => {
            if a <= 0. {
                if x > 0. { 0. } else { 1. }
            } else {
                1. / (1. + (x.max(0.) / a).powf(p.get(1).copied().unwrap_or(2.)))
            }
        }
        _ => 0.,
    }
}
#[derive(Clone)]
struct Run {
    values: BTreeMap<String, Vec<f64>>,
    rates: BTreeMap<String, Vec<f64>>,
    diverged: bool,
}
struct CompiledFlow {
    from: Option<usize>,
    to: Option<usize>,
    rate: f64,
    factors: Vec<(usize, String, Vec<f64>)>,
}
fn integrate(m: &Model, override_param: Option<(&str, f64)>) -> Run {
    let pv = |id: &str| {
        override_param
            .filter(|(pid, _)| *pid == id)
            .map(|(_, v)| v)
            .unwrap_or_else(|| m.params.get(id).map(|p| p.value).unwrap_or(0.))
    };
    let ids = m.stocks.keys().cloned().collect::<Vec<_>>();
    let index = ids
        .iter()
        .enumerate()
        .map(|(i, id)| (id.as_str(), i))
        .collect::<BTreeMap<_, _>>();
    let endpoint = |e: &End| {
        if let End::Id(id) = e {
            index.get(id.as_str()).copied()
        } else {
            None
        }
    };
    let mut xs = ids
        .iter()
        .map(|id| pv(&m.stocks[id].init))
        .collect::<Vec<_>>();
    let volumes = ids
        .iter()
        .map(|id| m.stocks[id].volume.as_ref().map(|p| pv(p)).unwrap_or(1.))
        .collect::<Vec<_>>();
    let flowids = m.flows.keys().cloned().collect::<Vec<_>>();
    let flows = flowids
        .iter()
        .map(|id| {
            let f = &m.flows[id];
            CompiledFlow {
                from: endpoint(&f.from),
                to: endpoint(&f.to),
                rate: f.rate.as_ref().map(|id| pv(id)).unwrap_or(1.),
                factors: m
                    .links
                    .values()
                    .filter(|l| l.to == End::Id(id.clone()))
                    .filter_map(|l| {
                        index.get(l.from.as_str()).map(|i| {
                            (
                                *i,
                                l.shape.clone(),
                                l.params.iter().map(|p| pv(p)).collect(),
                            )
                        })
                    })
                    .collect(),
            }
        })
        .collect::<Vec<_>>();
    let rates = |xs: &[f64]| {
        flows
            .iter()
            .map(|f| {
                f.factors.iter().fold(f.rate, |rate, (i, shape, p)| {
                    rate * factor(shape, xs[*i], p)
                })
            })
            .collect::<Vec<_>>()
    };
    let deriv = |xs: &[f64]| {
        let r = rates(xs);
        let mut out = vec![0.; xs.len()];
        for (f, r) in flows.iter().zip(r) {
            if let Some(i) = f.from {
                out[i] -= r / volumes[i]
            }
            if let Some(i) = f.to {
                out[i] += r / volumes[i]
            }
        }
        out
    };
    let mut values = xs.iter().map(|v| vec![*v]).collect::<Vec<_>>();
    let mut flowvalues = rates(&xs).into_iter().map(|r| vec![r]).collect::<Vec<_>>();
    let dt = m.duration / STEPS as f64;
    let mut diverged = false;
    for _ in 0..STEPS {
        let k1 = deriv(&xs);
        let k2 = deriv(
            &xs.iter()
                .zip(&k1)
                .map(|(x, k)| x + (dt / 2.) * k)
                .collect::<Vec<_>>(),
        );
        let k3 = deriv(
            &xs.iter()
                .zip(&k2)
                .map(|(x, k)| x + (dt / 2.) * k)
                .collect::<Vec<_>>(),
        );
        let k4 = deriv(
            &xs.iter()
                .zip(&k3)
                .map(|(x, k)| x + dt * k)
                .collect::<Vec<_>>(),
        );
        let next = xs
            .iter()
            .enumerate()
            .map(|(i, x)| x + (dt / 6.) * (k1[i] + 2. * k2[i] + 2. * k3[i] + k4[i]))
            .collect::<Vec<_>>();
        if next.iter().any(|x| !x.is_finite() || x.abs() > 1e12) {
            diverged = true;
            break;
        }
        xs = next;
        for (i, x) in xs.iter().enumerate() {
            values[i].push(*x)
        }
        for (i, r) in rates(&xs).iter().enumerate() {
            flowvalues[i].push(*r)
        }
    }
    Run {
        values: ids.into_iter().zip(values).collect(),
        rates: flowids.into_iter().zip(flowvalues).collect(),
        diverged,
    }
}
fn dynamics_key(m: &Model, omit: Option<&str>) -> String {
    serde_json::to_string(&(
        m.stocks
            .iter()
            .map(|(id, s)| (id, &s.init, &s.volume))
            .collect::<Vec<_>>(),
        m.flows
            .iter()
            .map(|(id, f)| {
                (
                    id,
                    match &f.from {
                        End::Id(id) => Some(id),
                        _ => None,
                    },
                    match &f.to {
                        End::Id(id) => Some(id),
                        _ => None,
                    },
                    &f.rate,
                )
            })
            .collect::<Vec<_>>(),
        m.links
            .values()
            .map(|l| (&l.from, &l.to, &l.shape, &l.params))
            .collect::<Vec<_>>(),
        m.params
            .iter()
            .map(|(id, p)| {
                (
                    id,
                    if omit == Some(id) {
                        None
                    } else {
                        Some(p.value)
                    },
                )
            })
            .collect::<Vec<_>>(),
        m.duration,
    ))
    .unwrap()
}
type Bands = BTreeMap<String, Vec<Option<[f64; 2]>>>;
thread_local! {static RUN_CACHE:RefCell<VecDeque<(String,Run)>>=const{RefCell::new(VecDeque::new())};static SWEEP_CACHE:RefCell<VecDeque<(String,Bands)>>=const{RefCell::new(VecDeque::new())};}
fn simulate(m: &Model) -> Run {
    let key = dynamics_key(m, None);
    RUN_CACHE.with(|cache| {
        let mut c = cache.borrow_mut();
        if let Some((_, run)) = c.iter().find(|(k, _)| *k == key) {
            return run.clone();
        }
        let run = integrate(m, None);
        if c.len() >= 16 {
            c.pop_front();
        }
        c.push_back((key, run.clone()));
        run
    })
}
fn sweep(m: &Model, pid: &str) -> Bands {
    let p = &m.params[pid];
    let key = format!("{pid}:{}:{}:{}", p.min, p.max, dynamics_key(m, Some(pid)));
    SWEEP_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some((_, bands)) = cache.iter().find(|(k, _)| *k == key) {
            return bands.clone();
        }
        let mut bands: Bands = m.stocks.keys().map(|id| (id.clone(), vec![])).collect();
        for i in 0..41 {
            let run = integrate(m, Some((pid, p.min + (p.max - p.min) * i as f64 / 40.)));
            for (id, vs) in &run.values {
                let tail = &vs[(vs.len() as f64 * 0.8).floor() as usize..];
                let band = if run.diverged {
                    None
                } else {
                    Some([
                        tail.iter().copied().fold(f64::INFINITY, f64::min),
                        tail.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                    ])
                };
                bands.get_mut(id).unwrap().push(band);
            }
        }
        if cache.len() >= 4 {
            cache.pop_front();
        }
        cache.push_back((key, bands.clone()));
        bands
    })
}
fn fresh<T>(map: &IndexMap<String, T>, prefix: &str) -> String {
    (0..)
        .map(|i| format!("{prefix}{i}"))
        .find(|id| !map.contains_key(id))
        .unwrap()
}
fn add_param(m: &mut Model, label: &str, value: f64, min: f64, max: f64) -> String {
    let id = fresh(&m.params, "p");
    m.params.insert(
        id.clone(),
        Param {
            label: label.into(),
            value,
            min,
            max,
        },
    );
    id
}
fn endpoint(m: &Model, e: &End) -> Point {
    match e {
        End::Cloud(p) => p.point(),
        End::Id(id) => m
            .stocks
            .get(id)
            .map(|s| Point::new(s.x, s.y))
            .unwrap_or(Point::ORIGIN),
    }
}
fn boundary(stock: &Stock, toward: Point, pad: f64) -> Point {
    let origin = Point::new(stock.x, stock.y);
    let d = toward - origin;
    if d.hypot() < 1e-6 {
        return origin;
    }
    let t = ((52. + pad) / d.x.abs().max(1e-9))
        .min((25. + pad) / d.y.abs().max(1e-9))
        .min(1.);
    origin + d * t
}
fn flow_geometry(m: &Model, id: &str) -> (Point, Point, Point, Point, Point) {
    let f = &m.flows[id];
    let a = endpoint(m, &f.from);
    let b = endpoint(m, &f.to);
    let d = b - a;
    let dir = if d.hypot() > 1e-6 {
        d / d.hypot()
    } else {
        Vec2::new(1., 0.)
    };
    let p = match &f.from {
        End::Cloud(_) => a + dir * 13.,
        End::Id(id) => boundary(&m.stocks[id], b, 0.),
    };
    let q = match &f.to {
        End::Cloud(_) => b - dir * 15.,
        End::Id(id) => boundary(&m.stocks[id], a, 2.),
    };
    let from_cloud = matches!(f.from, End::Cloud(_));
    let to_cloud = matches!(f.to, End::Cloud(_));
    let ratio = if from_cloud == to_cloud {
        0.5
    } else if to_cloud {
        0.6
    } else {
        0.4
    };
    (
        p,
        q,
        p + (q - p) * ratio,
        if from_cloud { a } else { p },
        if to_cloud { b } else { q },
    )
}
fn center(m: &Model, id: &str) -> Point {
    flow_geometry(m, id).2
}
fn link_geometry(m: &Model, id: &str) -> (Point, Point, Point, Point) {
    let l = &m.links[id];
    let stock = &m.stocks[&l.from];
    let target = match &l.to {
        End::Id(id) => center(m, id),
        End::Cloud(p) => p.point(),
    };
    let start = boundary(stock, target, 0.);
    let chord = target - start;
    let own = match &l.to {
        End::Id(id) => {
            let f = &m.flows[id];
            f.from == End::Id(l.from.clone()) || f.to == End::Id(l.from.clone())
        }
        _ => false,
    };
    let mut side = if chord.hypot() > 1e-6 {
        Vec2::new(-chord.y, chord.x) / chord.hypot()
    } else {
        Vec2::new(0., -1.)
    };
    if own
        && if chord.y.abs() > chord.x.abs() {
            side.x > 0.
        } else {
            side.y > 0.
        }
    {
        side = -side;
    }
    let twin = m
        .links
        .iter()
        .filter(|(_, other)| other.from == l.from && other.to == l.to)
        .position(|(other, _)| other == id)
        .unwrap_or(0);
    let bow = (twin + 1) as f64;
    let ctrl = start.midpoint(target)
        + side
            * if own {
                bow * (0.45 * chord.hypot()).max(30.)
            } else {
                -0.3 * bow * chord.hypot()
            };
    let d = target - ctrl;
    let end = if matches!(l.to, End::Id(_)) && d.hypot() > 1e-6 {
        target - d / d.hypot() * 13.
    } else {
        target
    };
    let mid = Point::new(
        start.x * 0.25 + ctrl.x * 0.5 + end.x * 0.25,
        start.y * 0.25 + ctrl.y * 0.5 + end.y * 0.25,
    );
    (start, ctrl, end, mid)
}
fn chart_row(m: &Model, id: &str) -> (f64, f64) {
    let row = m.stocks.keys().position(|key| key == id).unwrap_or(0);
    let n = m.stocks.len().max(1) as f64;
    let h = (224. - 8. * (n - 1.)) / n;
    (366. + row as f64 * (h + 8.), h)
}
fn hex(s: &str) -> Color {
    let s = s.trim_start_matches('#');
    if s.len() == 6 {
        let v = u32::from_str_radix(s, 16).unwrap_or(0x3e74be);
        [(v >> 16) as u8, (v >> 8) as u8, v as u8, 255]
    } else {
        BLUE
    }
}
fn nice_ceil(v: f64) -> f64 {
    if v <= 0. {
        return 0.;
    }
    let p = 10f64.powf(v.log10().floor());
    [1., 2., 2.5, 5., 10.]
        .into_iter()
        .map(|x| x * p)
        .find(|x| *x >= v * (1. - 1e-9))
        .unwrap_or(10. * p)
}
fn range(values: &[f64]) -> [f64; 2] {
    let lo = -nice_ceil(-values.iter().copied().fold(0., f64::min));
    let hi = nice_ceil(values.iter().copied().fold(0., f64::max));
    [lo, if hi - lo < 1e-12 { lo + 1. } else { hi }]
}
fn delete_stock(m: &mut Model, id: &str) {
    if let Some(s) = m.stocks.shift_remove(id) {
        for f in m.flows.values_mut() {
            if f.from == End::Id(id.into()) {
                f.from = End::Cloud(Position {
                    x: s.x - 20.,
                    y: s.y,
                })
            }
            if f.to == End::Id(id.into()) {
                f.to = End::Cloud(Position {
                    x: s.x + 20.,
                    y: s.y,
                })
            }
        }
        m.links.retain(|_, l| l.from != id);
    }
    tidy(m);
}
fn tidy(m: &mut Model) {
    for f in m.flows.values_mut() {
        for e in [&mut f.from, &mut f.to] {
            if let End::Id(id) = e {
                if !m.stocks.contains_key(id) {
                    *e = End::Cloud(Position { x: 0., y: 0. });
                }
            }
        }
    }

    m.links.retain(|_, l| {
        m.stocks.contains_key(&l.from) && matches!(&l.to,End::Id(id)if m.flows.contains_key(id))
    });
    let mut used = std::collections::BTreeSet::new();
    for stock in m.stocks.values() {
        used.insert(stock.init.clone());
        if let Some(p) = &stock.volume {
            used.insert(p.clone());
        }
    }
    for flow in m.flows.values() {
        if let Some(p) = &flow.rate {
            used.insert(p.clone());
        }
    }
    for link in m.links.values() {
        used.extend(link.params.iter().cloned());
    }
    m.params.retain(|id, p| {
        p.value = p.value.clamp(p.min, p.max);
        used.contains(id)
    });
    if m.sweep
        .as_ref()
        .is_some_and(|id| !m.params.contains_key(id))
    {
        m.sweep = None;
    }
}
fn set_shape(m: &mut Model, id: &str, shape: usize) {
    let stock_id = m.links[id].from.clone();
    let run = simulate(m);
    let values = &run.values[&stock_id];
    let at = ((m.cursor * STEPS as f64).round() as usize).min(values.len() - 1);
    let current = values[at].abs();
    let value = if current > 0. {
        current
    } else {
        m.stocks[&stock_id].scale[1] / 2.
    };
    let old = m.links[id].params.clone();
    for p in old {
        if !m
            .links
            .iter()
            .any(|(lid, l)| lid != id && l.params.contains(&p))
        {
            m.params.shift_remove(&p);
        }
    }
    let count = [0, 1, 1, 1, 1, 2][shape];
    let mut params = Vec::new();
    for i in 0..count {
        let base = if i == 1 {
            "n"
        } else if shape <= 2 {
            "θ"
        } else {
            "K"
        };
        let label = (1..)
            .map(|i| format!("{base}_{i}"))
            .find(|name| !m.params.values().any(|p| p.label == *name))
            .unwrap();
        let pid = if i == 1 {
            add_param(m, &label, 2., 1., 6.)
        } else {
            add_param(m, &label, value, 0., nice_ceil(2. * value))
        };
        params.push(pid);
    }
    let l = m.links.get_mut(id).unwrap();
    l.shape = SHAPES[shape].into();
    l.params = params;
    tidy(m);
}
fn fit_model(m: &mut Model) {
    let run = simulate(m);
    let bands = m
        .sweep
        .as_ref()
        .filter(|id| m.params.contains_key(*id))
        .map(|id| sweep(m, id));
    for (id, stock) in &mut m.stocks {
        let mut values = run.values[id].clone();
        if let Some(bands) = &bands {
            if let Some(bands) = bands.get(id) {
                values.extend(bands.iter().flatten().flat_map(|b| b.iter().copied()));
            }
        }
        stock.scale = range(&values);
    }
}
fn factor_formula(m: &Model, l: &Link) -> String {
    let x = &m.stocks[&l.from].symbol;
    factor_formula_with_symbol(m, l, x)
}
fn factor_formula_with_symbol(m: &Model, l: &Link, x: &str) -> String {
    let a = l
        .params
        .first()
        .map(|id| m.params[id].label.as_str())
        .unwrap_or("?");
    let b = l
        .params
        .get(1)
        .map(|id| m.params[id].label.as_str())
        .unwrap_or("?");
    match l.shape.as_str() {
        "linear" => x.into(),
        "threshold" => format!("({x}−{a})⁺"),
        "gap" => format!("({a}−{x})"),
        "saturating" => format!("{x}/({a}+{x})"),
        "crowding" => format!("(1−{x}/{a})"),
        "repression" => format!("1/(1+({x}/{a})^{b})"),
        _ => "?".into(),
    }
}
fn flow_formula(m: &Model, id: &str) -> String {
    flow_formula_with_stock_tags(m, id, false)
}
fn flow_formula_with_stock_tags(m: &Model, id: &str, tagged: bool) -> String {
    let flow = &m.flows[id];
    let mut parts = flow
        .rate
        .as_ref()
        .map(|id| vec![m.params[id].label.clone()])
        .unwrap_or_default();
    parts.extend(
        m.links
            .values()
            .filter(|l| l.to == End::Id(id.into()))
            .map(|l| {
                if tagged {
                    factor_formula_with_symbol(m, l, &format!("⟦{}⟧", l.from))
                } else {
                    factor_formula(m, l)
                }
            }),
    );
    if parts.is_empty() {
        "1".into()
    } else {
        parts
            .into_iter()
            .reduce(|acc, part| {
                if part.starts_with("1/") {
                    acc + &part[1..]
                } else {
                    acc + "·" + &part
                }
            })
            .unwrap()
    }
}
fn finish(s: &State) -> State {
    let mut t = s.clone();
    t.creating = None;
    tidy(&mut t.model);
    t
}
fn validate(m: &Model) -> Result<(), String> {
    if !m.duration.is_finite() || m.duration <= 0. || !m.cursor.is_finite() {
        return Err("Duration must be positive and cursor finite.".into());
    }
    for (id, p) in &m.params {
        if !p.value.is_finite() || !p.min.is_finite() || !p.max.is_finite() || p.min > p.max {
            return Err(format!("Parameter {id} has an invalid range."));
        }
    }
    let param = |id: &str| m.params.contains_key(id);
    for (id, s) in &m.stocks {
        if !param(&s.init)
            || s.volume
                .as_ref()
                .is_some_and(|p| !param(p) || m.params[p].value <= 0.)
            || !s.x.is_finite()
            || !s.y.is_finite()
        {
            return Err(format!(
                "Stock {id} has an invalid initial value, volume, or position."
            ));
        }
    }
    for (id, f) in &m.flows {
        if f.rate.as_ref().is_some_and(|p| !param(p)) {
            return Err(format!("Flow {id} refers to a missing parameter."));
        }
        for end in [&f.from, &f.to] {
            match end {
                End::Id(s) if !m.stocks.contains_key(s) => {
                    return Err(format!("Flow {id} refers to a missing stock."));
                }
                End::Cloud(p) if !p.x.is_finite() || !p.y.is_finite() => {
                    return Err(format!("Flow {id} has a nonfinite cloud."));
                }
                _ => {}
            }
        }
    }
    for (id, l) in &m.links {
        let Some(i) = SHAPES.iter().position(|s| *s == l.shape) else {
            return Err(format!("Link {id} has an unknown factor."));
        };
        if !m.stocks.contains_key(&l.from)
            || !matches!(&l.to,End::Id(id)if m.flows.contains_key(id))
            || l.params.len() != [0, 1, 1, 1, 1, 2][i]
            || l.params.iter().any(|p| !param(p))
        {
            return Err(format!(
                "Link {id} has missing stocks, flows, or factor parameters."
            ));
        }
    }
    Ok(())
}
fn param_order(m: &Model) -> Vec<String> {
    let mut out = vec![];
    let mut add = |p: &str| {
        if m.params.contains_key(p) && !out.iter().any(|x| x == p) {
            out.push(p.to_string());
        }
    };
    for s in m.stocks.values() {
        add(&s.init);
        if let Some(p) = &s.volume {
            add(p);
        }
    }
    for (fid, f) in &m.flows {
        if let Some(p) = &f.rate {
            add(p);
        }
        for l in m.links.values().filter(|l| l.to == End::Id(fid.clone())) {
            for p in &l.params {
                add(p);
            }
        }
    }
    for p in m.params.keys() {
        add(p);
    }
    out
}
fn knob_position(m: &Model, pid: &str) -> Point {
    let order = param_order(m);
    let i = order.iter().position(|p| p == pid).unwrap();
    let row = (280. / order.len().max(1) as f64).min(22.);
    let p = &m.params[pid];
    Point::new(
        516. + if p.max > p.min {
            100. * (p.value - p.min) / (p.max - p.min)
        } else {
            0.
        },
        44. + i as f64 * row,
    )
}
fn stock_fmt(v: f64) -> String {
    if !v.is_finite() {
        return "—".into();
    }
    if v == 0. {
        return "0".into();
    }
    let a = v.abs();
    if a >= 1e6 || a < 1e-3 {
        return format!("{v:.1e}");
    }
    if a >= 100. {
        return ((v + 0.5).floor() as i64).to_string();
    }
    let digits = (2. - a.log10().floor()).max(0.) as usize;
    let text = format!("{v:.digits$}");
    if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        text
    }
}
fn stroked(mut o: Object, color: Color, width: f64) -> Object {
    for shape in &mut o.shapes {
        match shape {
            Shape::Circle { stroke, .. } | Shape::Rect { stroke, .. } => {
                *stroke = Some((color, width))
            }
            _ => {}
        }
    }
    o
}
fn stock_label(
    id: impl Into<String>,
    p: impl Into<Point>,
    text: impl Into<String>,
    size: f64,
    color: Color,
) -> Object {
    baseline_text(id, p, text, size, color)
}
fn stock_center(
    id: impl Into<String>,
    p: impl Into<Point>,
    text: impl Into<String>,
    size: f64,
    color: Color,
) -> Object {
    stock_label(id, p, text, size, color).text_anchor(TextAnchor::Middle)
}
fn filled(id: impl Into<String>, path: BezPath, color: Color) -> Object {
    Object::new(
        id,
        Affine::IDENTITY,
        None,
        vec![Shape::Path {
            path,
            fill: Some(color),
            stroke: None,
        }],
    )
}
#[derive(Clone)]
struct MathRun {
    text: String,
    size: f64,
    dy: f64,
    color: Color,
    mono: bool,
    script: bool,
}
fn math_runs(text: &str, size: f64, color: Color) -> Vec<MathRun> {
    let mut out = vec![];
    let mut plain = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if matches!(c, '_' | '^') && chars.peek().is_some_and(char::is_ascii_alphanumeric) {
            if !plain.is_empty() {
                out.push(MathRun {
                    text: std::mem::take(&mut plain),
                    size,
                    dy: 0.,
                    color,
                    mono: false,
                    script: false,
                });
            }
            let mut text = String::new();
            while chars.peek().is_some_and(char::is_ascii_alphanumeric) {
                text.push(chars.next().unwrap());
            }
            out.push(MathRun {
                text,
                size: size * 0.75,
                dy: if c == '_' { size * 0.2 } else { -size * 0.4 },
                color,
                mono: false,
                script: true,
            });
        } else {
            plain.push(c);
        }
    }
    if !plain.is_empty() {
        out.push(MathRun {
            text: plain,
            size,
            dy: 0.,
            color,
            mono: false,
            script: false,
        });
    }
    out
}
fn math_style(run: &MathRun) -> TextStyle {
    TextStyle {
        family: if run.mono {
            FontFamily::Mono
        } else if run.script {
            FontFamily::Sans
        } else {
            FontFamily::Serif
        },
        baseline: TextBaseline::Alphabetic,
        italic: !run.mono && !run.script,
        ..Default::default()
    }
}
fn math_width(text: &str, size: f64) -> f64 {
    math_runs(text, size, INK)
        .iter()
        .map(|r| crate::text_layout::measure_text(&r.text, r.size, &math_style(r)).advance)
        .sum()
}
fn equation_runs(text: &str, m: &Model, size: f64, color: Color) -> Vec<MathRun> {
    let mut remaining = text;
    let mut runs = vec![];
    while let Some((before, rest)) = remaining.split_once('⟦') {
        runs.extend(math_runs(before, size, color));
        let (id, after) = rest
            .split_once('⟧')
            .expect("generated stock marker is closed");
        let stock = &m.stocks[id];
        runs.extend(math_runs(&stock.symbol, size, hex(&stock.color)));
        remaining = after;
    }
    runs.extend(math_runs(remaining, size, color));
    runs
}
fn expand_math(scene: &mut Scene, m: &Model) {
    let mut added = vec![];
    for object in &mut scene.objects {
        let tank = m.stocks.keys().any(|id| object.id == format!("stock-{id}"));
        let rich = object.id.starts_with("parameter-label-")
            || object.id.starts_with("flow-formula-")
            || object.id.starts_with("chart-symbol-")
            || object.id.starts_with("equation-");
        if !tank && !rich {
            continue;
        }
        let mut kept = vec![];
        for (shape_index, shape) in std::mem::take(&mut object.shapes).into_iter().enumerate() {
            let Shape::Text {
                origin,
                text,
                size,
                color,
            } = shape
            else {
                kept.push(shape);
                continue;
            };
            if tank && !text.contains(" = ") {
                kept.push(Shape::Text {
                    origin,
                    text,
                    size,
                    color,
                });
                continue;
            }
            let runs = if tank {
                let (symbol, value) = text.split_once(" = ").unwrap();
                let mut rs = math_runs(symbol, 14., color);
                rs.push(MathRun {
                    text: format!(" = {value}"),
                    size: 12.,
                    dy: 0.,
                    color,
                    mono: true,
                    script: false,
                });
                rs
            } else if object.id.starts_with("flow-formula-") {
                let (formula, rate) = text.rsplit_once("  ").unwrap_or((&text, ""));
                let mut rs = math_runs(formula, size, color);
                rs.push(MathRun {
                    text: format!(" {rate}"),
                    size: 9.,
                    dy: 0.,
                    color: hex("94a3b8"),
                    mono: true,
                    script: false,
                });
                rs
            } else if object.id.starts_with("equation-term-") {
                equation_runs(&text, m, size, color)
            } else if let Some(id) = object.id.strip_prefix("equation-numerator-") {
                let mut rs = math_runs("d", size, hex("334155"));
                rs.extend(math_runs(&m.stocks[id].symbol, size, color));
                rs
            } else {
                math_runs(&text, size, color)
            };
            let widths: Vec<_> = runs
                .iter()
                .map(|r| crate::text_layout::measure_text(&r.text, r.size, &math_style(r)).advance)
                .collect();
            let width = widths.iter().sum::<f64>();
            let mut x = origin.x
                - match object.text_style.anchor {
                    TextAnchor::Start => 0.,
                    TextAnchor::Middle => width / 2.,
                    TextAnchor::End => width,
                };
            for (i, (run, w)) in runs.into_iter().zip(widths).enumerate() {
                let style = math_style(&run);
                let mut o = Object::text(
                    format!("{}-run-{shape_index}-{i}", object.id),
                    (0., 0.),
                    run.text,
                    run.size,
                    run.color,
                )
                .text_style(style);
                o.text_style.baseline = object.text_style.baseline;
                o.transform = object.transform * Affine::translate((x, origin.y + run.dy));
                o.parent = Some(object.id.clone());
                added.push(o);
                x += w;
            }
        }
        object.shapes = kept;
    }
    scene.objects.extend(added);
}
fn circle_target(state: State, point: Point, radius: f64) -> DragSpec<'static, State> {
    let mut spec = dragology::fixed(state);
    DragSpec::custom(move |ctx, p| {
        let mut result = spec.evaluate(ctx, p)?;
        result.gap = if p.distance(point) <= radius {
            0.
        } else {
            f64::INFINITY
        };
        Ok(result)
    })
}
fn trash_target(state: State) -> DragSpec<'static, State> {
    dragology::drop_target(state, Rect::new(78., STOCK_TOP + 4., 110., STOCK_TOP + 36.))
}
fn move_end(s: &State, fid: &str, from: bool) -> DragSpec<'static, State> {
    let flow = &s.model.flows[fid];
    let other = if from { &flow.to } else { &flow.from };
    let (_, _, _, a, b) = flow_geometry(&s.model, fid);
    let point = if from { a } else { b };
    let set = |end: End| {
        let mut t = s.clone();
        let f = t.model.flows.get_mut(fid).unwrap();
        if from {
            f.from = end;
        } else {
            f.to = end;
        }
        t
    };
    let mut targets = vec![];
    for (id, stock) in &s.model.stocks {
        if *other == End::Id(id.clone()) {
            continue;
        }
        targets.push(dragology::drop_target(
            set(End::Id(id.clone())),
            Rect::new(
                stock.x - 52.,
                STOCK_TOP + stock.y - 25.,
                stock.x + 52.,
                STOCK_TOP + stock.y + 25.,
            ),
        ));
    }
    let mut deleted = s.clone();
    deleted.model.flows.shift_remove(fid);
    tidy(&mut deleted.model);
    targets.push(trash_target(deleted));
    let base = s.clone();
    let fid = fid.to_string();
    let free = dragology::vary([point.x, point.y], move |[x, y]| {
        let mut t = base.clone();
        let f = t.model.flows.get_mut(&fid).unwrap();
        let end = End::Cloud(Position { x, y });
        if from {
            f.from = end;
        } else {
            f.to = end;
        }
        t
    });
    dragology::closest(targets).when_far(free, 4.)
}
fn move_link(s: &State, lid: &str) -> DragSpec<'static, State> {
    let point = link_geometry(&s.model, lid).2;
    let mut targets = vec![];
    for fid in s.model.flows.keys() {
        let mut t = s.clone();
        t.model.links.get_mut(lid).unwrap().to = End::Id(fid.clone());
        targets.push(circle_target(
            t,
            center(&s.model, fid) + Vec2::new(0., STOCK_TOP),
            18.,
        ));
    }
    let base = s.clone();
    let lid = lid.to_string();
    let free = dragology::vary([point.x, point.y], move |[x, y]| {
        let mut t = base.clone();
        t.model.links.get_mut(&lid).unwrap().to = End::Cloud(Position { x, y });
        t
    });
    if targets.is_empty() {
        free.on_drop(finish)
    } else {
        dragology::closest(targets)
            .when_far(free, 4.)
            .on_drop(finish)
    }
}
fn vary_parameter(s: &State, pid: &str) -> DragSpec<'static, State> {
    let p = &s.model.params[pid];
    let (value, min, max) = (p.value, p.min, p.max);
    let base = s.clone();
    let update_id = pid.to_string();
    let check_id = pid.to_string();
    dragology::vary_constrained(
        [value],
        move |[value]| {
            let mut t = base.clone();
            t.model.params.get_mut(&update_id).unwrap().value = value;
            t
        },
        move |t| {
            let v = t.model.params[&check_id].value;
            [min - v, v - max]
        },
    )
}
impl Example for StockFlow {
    type State = State;
    fn initial(&self) -> State {
        let mut model = presets()[0].model.clone();
        fit_model(&mut model);
        let param = None;
        State {
            model,
            preset: 0,
            selected: String::new(),
            param,
            creating: None,
            history: Vec::new(),
            pending_original: None,
        }
    }
    fn anchor(&self, _: &State, id: &str, local: Point) -> Point {
        if id == "new-stock" {
            Point::new(52., 25.)
        } else if id.starts_with("new-") {
            Point::ORIGIN
        } else {
            local
        }
    }
    fn begin_drag(&self, s: &State, id: &str, _: Modifiers) -> Option<(State, String)> {
        let mut t = s.clone();
        t.pending_original = Some(Box::new(s.model.clone()));
        t.creating = None;
        if id == "new-stock" {
            let sid = fresh(&t.model.stocks, "stock");
            let symbol = "xyzuvwpqrsabcdefgh"
                .chars()
                .map(|c| c.to_string())
                .chain((1..).map(|i| format!("x_{i}")))
                .find(|x| !t.model.stocks.values().any(|s| s.symbol == *x))
                .unwrap();
            let init = add_param(&mut t.model, &format!("{symbol}_0"), 1., 0., 2.);
            let color = [
                "#ea580c", "#2563eb", "#16a34a", "#9333ea", "#db2777", "#0891b2", "#ca8a04",
            ][t.model.stocks.len() % 7]
                .into();
            t.model.stocks.insert(
                sid.clone(),
                Stock {
                    name: format!("Stock {symbol}"),
                    symbol,
                    unit: None,
                    x: 60.,
                    y: 33.,
                    color,
                    init,
                    volume: None,
                    scale: [0., 2.],
                },
            );
            return Some((t, format!("stock-{sid}")));
        }
        if let Some(sid) = id.strip_prefix("new-link-") {
            let stock = t.model.stocks.get(sid)?;
            let point = Position {
                x: stock.x,
                y: stock.y - 35.,
            };
            let lid = fresh(&t.model.links, "link");
            t.model.links.insert(
                lid.clone(),
                Link {
                    from: sid.into(),
                    to: End::Cloud(point),
                    shape: "linear".into(),
                    params: vec![],
                },
            );
            return Some((t, format!("link-end-{lid}")));
        }
        for (prefix, outgoing) in [("new-in-", false), ("new-out-", true)] {
            if let Some(sid) = id.strip_prefix(prefix) {
                let stock = t.model.stocks.get(sid)?;
                let point = Position {
                    x: stock.x + if outgoing { 52. } else { -52. },
                    y: stock.y,
                };
                let volume = stock
                    .volume
                    .as_ref()
                    .map(|p| t.model.params[p].value)
                    .unwrap_or(1.);
                let k = 3. / t.model.duration
                    * volume
                    * if outgoing { 1. } else { stock.scale[1] / 2. };
                let k = if k != 0. { k } else { 1. };
                let label = (1..)
                    .map(|i| format!("k_{i}"))
                    .find(|x| !t.model.params.values().any(|p| p.label == *x))
                    .unwrap();
                let rate = add_param(&mut t.model, &label, k, 0., nice_ceil(4. * k));
                let fid = fresh(&t.model.flows, "flow");
                let end = End::Cloud(point);
                let stock = End::Id(sid.into());
                t.model.flows.insert(
                    fid.clone(),
                    Flow {
                        name: if outgoing { "outflow" } else { "inflow" }.into(),
                        from: if outgoing { stock.clone() } else { end.clone() },
                        to: if outgoing { end } else { stock },
                        rate: Some(rate),
                    },
                );
                if outgoing {
                    let lid = fresh(&t.model.links, "link");
                    t.model.links.insert(
                        lid,
                        Link {
                            from: sid.into(),
                            to: End::Id(fid.clone()),
                            shape: "linear".into(),
                            params: vec![],
                        },
                    );
                }
                return Some((
                    t,
                    format!("end-{}-{fid}", if outgoing { "to" } else { "from" }),
                ));
            }
        }
        None
    }
    fn scene(&self, s: &State) -> Scene {
        self.scene_interaction(s, None, None)
    }
    fn scene_dragging(&self, s: &State, dragged: Option<&str>) -> Scene {
        self.scene_interaction(s, dragged, None)
    }
    fn scene_interaction(&self, s: &State, dragged: Option<&str>, hovered: Option<&str>) -> Scene {
        let m = &s.model;
        let run = simulate(m);
        let mut scene = Scene::new(680., 640.);
        let ink = hex("334155");
        let muted = hex("64748b");
        let faint = hex("94a3b8");
        let highlight = hex("d97706");
        let hovered_flow = hovered.and_then(|id| {
            m.flows.keys().find(|fid| {
                id == format!("flow-{fid}")
                    || id == format!("flow-label-{fid}")
                    || id == format!("flow-formula-{fid}")
                    || id.starts_with(&format!("equation-term-{fid}-"))
            })
        });
        scene.push(
            stroked(
                Object::rect("diagram", Rect::new(0., 0., 450., 320.), 6., hex("fcfcfd")),
                hex("e5e7eb"),
                1.,
            )
            .decorative(),
        );
        scene.push(
            stroked(
                Object::rect("new-stock", Rect::new(8., 8., 72., 32.), 4., WHITE),
                faint,
                1.,
            )
            .dash(vec![3., 2.])
            .label("+ stock", (32., 16.), 11., muted)
            .text_anchor(TextAnchor::Middle)
            .text_baseline(TextBaseline::Alphabetic),
        );
        let trashing = dragged.is_some_and(|id| {
            id.starts_with("stock-") || id.starts_with("flow-") || id.starts_with("end-")
        });
        let trash_color = if trashing { hex("ef4444") } else { faint };
        scene.push(
            stroked(
                Object::rect(
                    "trash",
                    Rect::new(78., 4., 110., 36.),
                    5.,
                    if trashing { hex("fee2e2") } else { WHITE },
                ),
                if trashing { trash_color } else { hex("e2e8f0") },
                1.,
            )
            .decorative(),
        );
        let mut bin = BezPath::new();
        for (a, b) in [
            ((87., 14.), (101., 14.)),
            ((91., 14.), (91., 11.5)),
            ((91., 11.5), (97., 11.5)),
            ((97., 11.5), (97., 14.)),
            ((88.5, 14.), (89.5, 29.)),
            ((89.5, 29.), (98.5, 29.)),
            ((98.5, 29.), (99.5, 14.)),
            ((92., 17.5), (92., 26.)),
            ((96., 17.5), (96., 26.)),
        ] {
            bin.move_to(a);
            bin.line_to(b);
        }
        scene.push(
            Object::path("trash-icon", bin, trash_color, 1.4)
                .line_cap(LineCap::Round)
                .line_join(LineJoin::Round),
        );
        for (id, f) in &m.flows {
            let (from, to, p, from_handle, to_handle) = flow_geometry(m, id);
            let d = to - from;
            let dir = if d.hypot() > 1e-6 {
                d.normalize()
            } else {
                Vec2::new(1., 0.)
            };
            let color = if hovered_flow == Some(id) {
                highlight
            } else {
                muted
            };
            scene.push(Object::line(
                format!("flow-path-{id}"),
                from,
                to - dir * 9.,
                color,
                7.,
            ));
            scene.push(Object::line(
                format!("flow-inner-{id}"),
                from,
                to - dir * 9.,
                WHITE,
                3.5,
            ));
            let side = Vec2::new(-dir.y, dir.x) * 6.;
            let mut head = BezPath::new();
            head.move_to(to);
            head.line_to(to - dir * 9. + side);
            head.line_to(to - dir * 9. - side);
            head.close_path();
            scene.push(filled(format!("flow-head-{id}"), head, color));
            let mut valve = BezPath::new();
            valve.move_to((-6., -9.));
            valve.line_to((6., -9.));
            valve.line_to((-6., 9.));
            valve.line_to((6., 9.));
            valve.close_path();
            // Keep the original native local anchor for solver compatibility, while drawing the source valve.
            let mut v = Object::new(
                format!("flow-{id}"),
                Affine::translate((p.x - 12., p.y - 10.)),
                Some(Hit::Circle(Point::new(12., 10.), 11.)),
                vec![Shape::Path {
                    path: Affine::translate((12., 10.))
                        * Affine::rotate(dir.y.atan2(dir.x))
                        * valve,
                    fill: Some(if hovered_flow == Some(id) {
                        hex("fef3c7")
                    } else {
                        WHITE
                    }),
                    stroke: Some((
                        if hovered_flow == Some(id) {
                            highlight
                        } else {
                            ink
                        },
                        1.5,
                    )),
                }],
            );
            v.stroke_style.join = LineJoin::Round;
            scene.push(v);
            let horizontal = dir.x.abs() >= 0.6 * dir.y.abs();
            let formula = flow_formula(m, id);
            let fx = if horizontal {
                p.x
            } else {
                p.x + 14. + formula.replace(['_', '^'], "").len() as f64 * 3.
            };
            let nx = if horizontal {
                p.x
            } else {
                p.x + 14. + f.name.chars().count() as f64 * 2.6
            };
            let rate = run
                .rates
                .get(id)
                .and_then(|r| {
                    r.get(
                        ((m.cursor * STEPS as f64).round() as usize).min(r.len().saturating_sub(1)),
                    )
                })
                .copied()
                .unwrap_or(0.);
            let mut formula_label = stock_center(
                format!("flow-formula-{id}"),
                (fx, p.y + if horizontal { 21. } else { -4. }),
                format!("{formula}  {}", stock_fmt(rate)),
                11.5,
                if hovered_flow == Some(id) {
                    highlight
                } else {
                    ink
                },
            )
            .font_family(FontFamily::Serif)
            .italic();
            formula_label.hit = Some(Hit::Rect(Rect::new(-40., -13., 80., 4.)));
            scene.push(formula_label);
            let mut name = stock_center(
                format!("flow-label-{id}"),
                (nx, p.y + if horizontal { 32. } else { 9. }),
                &f.name,
                9.5,
                muted,
            );
            name.hit = Some(Hit::Rect(Rect::new(-40., -10., 40., 3.)));
            scene.push(name);
            for (which, point, end) in [("from", from_handle, &f.from), ("to", to_handle, &f.to)] {
                let key = format!("end-{which}-{id}");
                let mut o = if matches!(end, End::Cloud(_)) {
                    let cloud = BezPath::from_svg(
                        "M-9,6 A5,5 0 0 1 -10,-2 A6,6 0 0 1 -2,-8 A6,6 0 0 1 8,-5 A5,5 0 0 1 9,6 Z",
                    )
                    .unwrap();
                    Object::new(
                        &key,
                        Affine::translate(point.to_vec2()),
                        Some(Hit::Circle(Point::ORIGIN, 13.)),
                        vec![Shape::Path {
                            path: cloud,
                            fill: Some(WHITE),
                            stroke: Some((faint, 1.5)),
                        }],
                    )
                } else {
                    Object::circle(&key, point, 7., muted).hit_when_transparent()
                };
                if !matches!(end, End::Cloud(_)) {
                    o.opacity = if hovered == Some(key.as_str()) {
                        0.5
                    } else {
                        0.
                    };
                }
                scene.push(o);
            }
        }
        for (id, l) in &m.links {
            let Some(stock) = m.stocks.get(&l.from) else {
                continue;
            };
            let color = hex(&stock.color);
            let (from, control, to, mid) = link_geometry(m, id);
            let delta = to - control;
            let dir = if delta.hypot() > 1e-6 {
                delta.normalize()
            } else {
                Vec2::ZERO
            };
            let mut path = BezPath::new();
            path.move_to(from);
            path.quad_to(control, to - dir * 5.);
            let mut line =
                Object::path(format!("link-path-{id}"), path, color, 1.5).dash(vec![4., 3.]);
            line.opacity = 0.8;
            scene.push(line);
            let mut head = BezPath::new();
            head.move_to((0., 0.));
            head.line_to((-7., -4.));
            head.line_to((-7., 4.));
            head.close_path();
            scene.push(Object::new(
                format!("link-end-{id}"),
                Affine::translate(to.to_vec2()),
                Some(Hit::Circle(Point::ORIGIN, 7.)),
                vec![Shape::Path {
                    path: Affine::rotate(dir.y.atan2(dir.x)) * head,
                    fill: Some(color),
                    stroke: None,
                }],
            ));
            if matches!(l.to, End::Id(_)) {
                scene.push(
                    stroked(
                        Object::rect(
                            format!("link-{id}"),
                            Rect::new(mid.x - 13., mid.y - 9., mid.x + 13., mid.y + 9.),
                            4.,
                            WHITE,
                        ),
                        color,
                        1.,
                    )
                    .tooltip(format!(
                        "{}: {}\n(click for the next shape, shift-click for the previous)",
                        l.shape,
                        factor_formula(m, l)
                    )),
                );
                let (params, scale) = match l.shape.as_str() {
                    "threshold" => (vec![0.4], 0.6),
                    "gap" => (vec![0.6], 1.),
                    "saturating" => (vec![0.15], 1.),
                    "crowding" => (vec![0.8], 1.),
                    "repression" => (vec![0.4, 3.], 1.),
                    _ => (vec![], 1.),
                };
                let mut glyph = BezPath::new();
                for i in 0..=20 {
                    let p = Point::new(
                        mid.x - 10. + i as f64,
                        mid.y + 6.
                            - 12.
                                * (factor(&l.shape, i as f64 / 20., &params) / scale)
                                    .clamp(-0.4, 1.),
                    );
                    if i == 0 {
                        glyph.move_to(p)
                    } else {
                        glyph.line_to(p)
                    }
                }
                scene.push(
                    Object::path(format!("factor-glyph-{id}"), glyph, color, 1.5)
                        .line_join(LineJoin::Round),
                );
            }
        }
        for (id, stock) in &m.stocks {
            let values = &run.values[id];
            let index = ((m.cursor * STEPS as f64).round() as usize).min(values.len() - 1);
            let color = hex(&stock.color);
            let value = values[index];
            let mut tank = stroked(
                Object::rect(
                    format!("stock-{id}"),
                    Rect::new(stock.x - 52., stock.y - 25., stock.x + 52., stock.y + 25.),
                    4.,
                    WHITE,
                ),
                color,
                2.,
            );
            let level =
                ((value - stock.scale[0]) / (stock.scale[1] - stock.scale[0])).clamp(0., 1.) * 46.;
            let mut tint = color;
            tint[3] = 38;
            tank.shapes.push(Shape::Rect {
                bounds: Rect::new(2., 48. - level, 102., 48.),
                radius: 0.,
                fill: tint,
                stroke: None,
            });
            tank = tank
                .label(&stock.name, (52., 18.), 11., ink)
                .label(
                    format!("{} = {}", stock.symbol, stock_fmt(value)),
                    (52., 38.),
                    12.,
                    color,
                )
                .text_anchor(TextAnchor::Middle)
                .text_baseline(TextBaseline::Alphabetic);
            scene.push(tank);
            for (side, delta) in [
                ("in", Vec2::new(-52., 0.)),
                ("out", Vec2::new(52., 0.)),
                ("link", Vec2::new(0., -25.)),
            ] {
                let key = format!("new-{side}-{id}");
                let p = s
                    .creating
                    .as_ref()
                    .filter(|(k, _)| *k == key)
                    .map(|(_, p)| *p)
                    .unwrap_or(Point::new(stock.x, stock.y) + delta);
                let mut port = stroked(
                    Object::circle(&key, p, if side == "link" { 6. } else { 7. }, WHITE),
                    if side == "link" { color } else { muted },
                    1.2,
                );
                if side == "link" {
                    port = port.dash(vec![2., 1.5]);
                    port.shapes.push(Shape::Circle {
                        center: Point::ORIGIN,
                        radius: 1.8,
                        fill: color,
                        stroke: None,
                    });
                } else {
                    let mut arrow = BezPath::new();
                    arrow.move_to((-3.5, 0.));
                    arrow.line_to((3.5, 0.));
                    arrow.move_to((0.5, -3.));
                    arrow.line_to((3.5, 0.));
                    arrow.line_to((0.5, 3.));
                    port.shapes.push(Shape::Path {
                        path: arrow,
                        fill: None,
                        stroke: Some((muted, 1.3)),
                    });
                }
                port.opacity = if hovered == Some(key.as_str()) {
                    1.
                } else if hovered == Some(format!("stock-{id}").as_str()) {
                    0.8
                } else {
                    0.3
                };
                scene.push(port);
            }
        }
        if let Some((id, p)) = &s.creating {
            if id == "new-stock" {
                scene.objects.retain(|o| o.id != "new-stock");
                let mut o = Object::circle(id, *p, 0., [0; 4]);
                o.opacity = 0.;
                scene.push(o);
            }
        }
        scene.push(
            stock_label("parameters-heading", (470., 18.), "PARAMETERS", 11., muted)
                .font_weight(600),
        );
        let order = param_order(m);
        let rh = (280. / order.len().max(1) as f64).min(22.);
        for (i, id) in order.iter().enumerate() {
            let p = &m.params[id];
            let y = 44. + i as f64 * rh;
            let color = m
                .stocks
                .values()
                .find(|s| s.init == *id)
                .map(|s| hex(&s.color))
                .unwrap_or(ink);
            if m.sweep.as_ref() == Some(id) {
                scene.push(
                    Object::rect(
                        format!("parameter-selected-{id}"),
                        Rect::new(464., y - rh / 2., 676., y + rh / 2.),
                        4.,
                        hex("fef3c7"),
                    )
                    .decorative(),
                );
            }
            let mut label = Object::text(
                format!("parameter-label-{id}"),
                (470., y),
                &p.label,
                13.,
                color,
            )
            .text_baseline(TextBaseline::Central)
            .font_family(FontFamily::Serif)
            .italic();
            label.hit = Some(Hit::Rect(Rect::new(-4., -rh / 2., 42., rh / 2.)));
            scene.push(label);
            scene.push(
                Object::line(
                    format!("parameter-track-{id}"),
                    (516., y),
                    (616., y),
                    hex("cbd5e1"),
                    2.,
                )
                .line_cap(LineCap::Round),
            );
            scene.push(stroked(
                Object::circle(format!("parameter-{id}"), knob_position(m, id), 5.5, WHITE),
                color,
                1.5,
            ));
            let mut value = Object::text(
                format!("parameter-value-{id}"),
                (626., y),
                stock_fmt(p.value),
                10.5,
                ink,
            )
            .text_baseline(TextBaseline::Central)
            .font_family(FontFamily::Mono);
            value.hit = Some(Hit::Rect(Rect::new(0., -rh / 2., 54., rh / 2.)));
            scene.push(value);
        }
        scene.push(
            stock_label("time-heading", (48., 336.), "OVER TIME", 11., muted).font_weight(600),
        );
        for (id, stock) in &m.stocks {
            let values = &run.values[id];
            let (y, h) = chart_row(m, id);
            let project = |v: f64, clipped: bool| {
                let f = (v - stock.scale[0]) / (stock.scale[1] - stock.scale[0]);
                y + h - if clipped { f.clamp(-0.03, 1.03) } else { f } * h
            };
            let color = hex(&stock.color);
            scene.push(
                Object::rect(
                    format!("chart-background-{id}"),
                    Rect::new(48., y, 318., y + h),
                    0.,
                    hex("f8fafc"),
                )
                .decorative(),
            );
            if stock.scale[0] < 0. {
                scene.push(Object::line(
                    format!("chart-zero-{id}"),
                    (48., project(0., true)),
                    (318., project(0., true)),
                    hex("e2e8f0"),
                    1.,
                ));
            }
            scene.push(
                stock_label(
                    format!("chart-hi-{id}"),
                    (44., y + 8.),
                    stock_fmt(stock.scale[1]),
                    9.,
                    faint,
                )
                .text_anchor(TextAnchor::End)
                .font_family(FontFamily::Mono),
            );
            scene.push(
                stock_label(
                    format!("chart-lo-{id}"),
                    (44., y + h),
                    stock_fmt(stock.scale[0]),
                    9.,
                    faint,
                )
                .text_anchor(TextAnchor::End)
                .font_family(FontFamily::Mono),
            );
            scene.push(
                stock_label(
                    format!("chart-symbol-{id}"),
                    (4., y + h / 2. + 5.),
                    &stock.symbol,
                    15.,
                    color,
                )
                .font_family(FontFamily::Serif)
                .italic(),
            );
            scene.push(
                stock_label(
                    format!("chart-unit-{id}"),
                    (314., y + 11.),
                    stock.unit.as_deref().unwrap_or(""),
                    9.,
                    faint,
                )
                .text_anchor(TextAnchor::End),
            );
            let mut path = BezPath::new();
            for i in (0..values.len())
                .step_by(4)
                .chain((values.len() % 4 != 1).then_some(values.len() - 1))
            {
                let p = Point::new(
                    48. + 270. * i as f64 / STEPS as f64,
                    project(values[i], true),
                );
                if path.elements().is_empty() {
                    path.move_to(p)
                } else {
                    path.line_to(p)
                }
            }
            scene.push(
                Object::path(format!("chart-line-{id}"), path, color, 2.)
                    .line_join(LineJoin::Round),
            );
            let ci = ((m.cursor * STEPS as f64).round() as usize).min(values.len() - 1);
            scene.push(
                Object::circle(
                    format!("chart-dot-{id}"),
                    (48. + 270. * m.cursor, project(values[ci], true)),
                    3.,
                    color,
                )
                .decorative(),
            );
            scene.push(stroked(
                Object::circle(
                    format!("initial-{id}"),
                    (48., project(m.params[&stock.init].value, false)),
                    5.,
                    WHITE,
                ),
                color,
                2.,
            ));
            if run.diverged {
                scene.push(
                    stock_label(
                        format!("diverged-{id}"),
                        (314., y + h - 5.),
                        "diverged",
                        9.,
                        hex("dc2626"),
                    )
                    .text_anchor(TextAnchor::End),
                );
            }
        }
        let cx = 48. + 270. * m.cursor;
        let mut cursor = Object::new(
            "cursor",
            Affine::translate((cx, 0.)),
            Some(Hit::Rect(Rect::new(-5., 356., 5., 590.))),
            vec![Shape::Path {
                path: {
                    let mut p = BezPath::new();
                    p.move_to((0., 362.));
                    p.line_to((0., 590.));
                    p
                },
                fill: None,
                stroke: Some(([51, 65, 85, 128], 1.)),
            }],
        );
        let mut triangle = BezPath::new();
        triangle.move_to((-5., 356.));
        triangle.line_to((5., 356.));
        triangle.line_to((0., 363.));
        triangle.close_path();
        cursor.shapes.push(Shape::Path {
            path: triangle,
            fill: Some(ink),
            stroke: None,
        });
        if m.stocks.is_empty() {
            cursor.opacity = 0.;
        }
        scene.push(cursor);
        scene.push(
            stock_center(
                "cursor-time",
                (cx.clamp(78., 288.), 353.),
                format!("t = {} {}", stock_fmt(m.cursor * m.duration), m.time_unit),
                9.5,
                ink,
            )
            .font_family(FontFamily::Mono),
        );
        scene.push(
            stock_label("time-zero", (48., 603.), "0", 9., faint).font_family(FontFamily::Mono),
        );
        scene.push(
            stock_label(
                "time-max",
                (318., 603.),
                format!("{} {}", stock_fmt(m.duration), m.time_unit),
                9.,
                faint,
            )
            .font_family(FontFamily::Mono)
            .text_anchor(TextAnchor::End),
        );
        scene.push(stock_label("run-for", (48., 628.), "run for", 10., muted));
        scene.push(
            Object::line("t-track", (100., 624.), (244., 624.), hex("cbd5e1"), 2.)
                .line_cap(LineCap::Round),
        );
        let tx = 100. + (m.duration / m.trange[0]).ln() / (m.trange[1] / m.trange[0]).ln() * 144.;
        scene.push(stroked(
            Object::circle("t-knob", (tx, 624.), 5.5, WHITE),
            ink,
            1.5,
        ));
        scene.push(
            stock_label(
                "run-length",
                (254., 628.),
                format!("{} {}", stock_fmt(m.duration), m.time_unit),
                10.5,
                ink,
            )
            .font_family(FontFamily::Mono),
        );
        let pid = m.sweep.as_ref().filter(|id| m.params.contains_key(*id));
        scene.push(
            stock_label(
                "sweep-heading",
                (394., 336.),
                pid.map(|id| format!("IN THE LONG RUN VS {}", m.params[id].label))
                    .unwrap_or_else(|| "IN THE LONG RUN".into()),
                11.,
                muted,
            )
            .font_weight(600),
        );
        if let Some(pid) = pid {
            let bands = sweep(m, pid);
            let p = &m.params[pid];
            scene.push(
                Object::rect("sweep-area", Rect::new(394., 366., 664., 590.), 0., [0; 4])
                    .hit_when_transparent(),
            );
            for (id, stock) in &m.stocks {
                let (y, h) = chart_row(m, id);
                let project = |v: f64| {
                    y + h
                        - ((v - stock.scale[0]) / (stock.scale[1] - stock.scale[0]))
                            .clamp(-0.03, 1.03)
                            * h
                };
                let color = hex(&stock.color);
                scene.push(
                    Object::rect(
                        format!("sweep-bg-{id}"),
                        Rect::new(394., y, 664., y + h),
                        0.,
                        hex("f8fafc"),
                    )
                    .decorative(),
                );
                let mut runs: Vec<Vec<(Point, Point)>> = vec![vec![]];
                for (i, band) in bands[id].iter().enumerate() {
                    if let Some([lo, hi]) = band {
                        let x = 394. + 270. * i as f64 / 40.;
                        runs.last_mut()
                            .unwrap()
                            .push((Point::new(x, project(*lo)), Point::new(x, project(*hi))));
                    } else if !runs.last().unwrap().is_empty() {
                        runs.push(vec![]);
                    }
                }
                for (k, points) in runs.iter().filter(|p| p.len() > 1).enumerate() {
                    let mut polygon = BezPath::new();
                    polygon.move_to(points[0].1);
                    for p in points.iter().skip(1) {
                        polygon.line_to(p.1);
                    }
                    for p in points.iter().rev() {
                        polygon.line_to(p.0);
                    }
                    polygon.close_path();
                    let mut tint = color;
                    tint[3] = 51;
                    scene.push(filled(format!("band-{id}-{k}"), polygon, tint));
                    for (high, suffix) in [(false, "low"), (true, "high")] {
                        let mut path = BezPath::new();
                        for (i, p) in points.iter().enumerate() {
                            let p = if high { p.1 } else { p.0 };
                            if i == 0 {
                                path.move_to(p)
                            } else {
                                path.line_to(p)
                            }
                        }
                        scene.push(Object::path(
                            format!("sweep-{suffix}-{id}-{k}"),
                            path,
                            color,
                            1.5,
                        ));
                    }
                }
                if !run.diverged {
                    let values = &run.values[id];
                    let tail = &values[(values.len() as f64 * 0.8).floor() as usize..];
                    let lo = tail.iter().copied().fold(f64::INFINITY, f64::min);
                    let hi = tail.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                    let x = 394.
                        + 270.
                            * if p.max > p.min {
                                (p.value - p.min) / (p.max - p.min)
                            } else {
                                0.
                            };
                    scene.push(Object::line(
                        format!("tail-range-{id}"),
                        (x, project(lo)),
                        (x, project(hi)),
                        color,
                        3.,
                    ));
                    for (label, v) in [("lo", lo), ("hi", hi)] {
                        scene.push(
                            Object::circle(
                                format!("tail-{label}-{id}"),
                                (x, project(v)),
                                3.,
                                color,
                            )
                            .decorative(),
                        );
                    }
                }
            }
            let fraction = if p.max > p.min {
                (p.value - p.min) / (p.max - p.min)
            } else {
                0.
            };
            scene.push(
                Object::line(
                    "sweep-cursor",
                    (394. + 270. * fraction, 366.),
                    (394. + 270. * fraction, 590.),
                    [51, 65, 85, 128],
                    1.,
                )
                .dash(vec![2., 2.]),
            );
            scene.push(
                stock_label("sweep-min", (394., 603.), stock_fmt(p.min), 9., faint)
                    .font_family(FontFamily::Mono),
            );
            scene.push(
                stock_label("sweep-max", (664., 603.), stock_fmt(p.max), 9., faint)
                    .font_family(FontFamily::Mono)
                    .text_anchor(TextAnchor::End),
            );
            scene.push(stock_center(
                "sweep-note",
                (529., 628.),
                "range of the last 20% of each run",
                10.,
                faint,
            ));
        } else {
            scene.push(stock_center(
                "sweep-hint",
                (529., 474.),
                "Click a parameter’s name",
                11.,
                faint,
            ));
        }
        if pid.is_none() && !m.stocks.is_empty() {
            scene.push(stock_center(
                "sweep-hint-second",
                (529., 489.),
                "to see how the long run depends on it.",
                11.,
                faint,
            ));
            for (id, _) in &m.stocks {
                let (y, h) = chart_row(m, id);
                let bg = Object::rect(
                    format!("sweep-bg-empty-{id}"),
                    Rect::new(394., y, 664., y + h),
                    0.,
                    hex("f8fafc"),
                )
                .decorative();
                scene.objects.insert(0, bg);
            }
        }
        let ey = 664.;
        scene.push(
            stock_label("equations-heading", (0., ey), "EQUATIONS", 11., muted).font_weight(600),
        );
        let mut eq_y = ey + 28.;
        for (id, stock) in &m.stocks {
            let mut x = 0.;
            if let Some(pid) = &stock.volume {
                let label = format!("{} ·", m.params[pid].label);
                let w = math_width(&label, 15.);
                scene.push(
                    stock_label(format!("equation-volume-{id}"), (x, eq_y), label, 15., ink)
                        .font_family(FontFamily::Serif)
                        .italic(),
                );
                x += w + 5.;
            }
            let numerator = format!("d{}", stock.symbol);
            let w = math_width(&numerator, 15.).max(math_width("dt", 15.)) + 4.;
            scene.push(
                stock_center(
                    format!("equation-numerator-{id}"),
                    (x + w / 2., eq_y - 6.),
                    numerator,
                    15.,
                    hex(&stock.color),
                )
                .font_family(FontFamily::Serif)
                .italic(),
            );
            scene.push(Object::line(
                format!("fraction-{id}"),
                (x, eq_y - 2.),
                (x + w, eq_y - 2.),
                ink,
                1.,
            ));
            scene.push(
                stock_center(
                    format!("equation-denominator-{id}"),
                    (x + w / 2., eq_y + 13.),
                    "dt",
                    15.,
                    ink,
                )
                .font_family(FontFamily::Serif)
                .italic(),
            );
            x += w + 8.;
            scene.push(stock_label(
                format!("equals-{id}"),
                (x, eq_y),
                "=",
                15.,
                ink,
            ));
            x += 18.;
            let mut any = false;
            for (fid, f) in &m.flows {
                for (sign, include) in [
                    ("+", f.to == End::Id(id.clone())),
                    ("−", f.from == End::Id(id.clone())),
                ] {
                    if include {
                        let text = format!(
                            "{}{}",
                            if !any && sign == "+" { "" } else { sign },
                            flow_formula(m, fid)
                        );
                        let width = math_width(&text, 15.);
                        let tagged = format!(
                            "{}{}",
                            if !any && sign == "+" { "" } else { sign },
                            flow_formula_with_stock_tags(m, fid, true)
                        );
                        if hovered_flow == Some(fid) {
                            scene.push(
                                Object::rect(
                                    format!("equation-highlight-{fid}-{id}-{sign}"),
                                    Rect::new(x - 2., eq_y - 16., x + width + 2., eq_y + 4.),
                                    4.,
                                    hex("fef3c7"),
                                )
                                .decorative(),
                            );
                        }
                        let mut term = stock_label(
                            format!("equation-term-{fid}-{id}-{sign}"),
                            (x, eq_y),
                            &tagged,
                            15.,
                            ink,
                        )
                        .font_family(FontFamily::Serif)
                        .italic();
                        term.hit = Some(Hit::Rect(Rect::new(0., -17., width, 3.)));
                        scene.push(term);
                        x += width + 8.;
                        any = true;
                    }
                }
            }
            if !any {
                scene.push(stock_label(
                    format!("equation-zero-{id}"),
                    (x, eq_y),
                    "0",
                    16.,
                    ink,
                ));
            }
            eq_y += 42.;
        }
        expand_math(&mut scene, m);
        for o in &mut scene.objects {
            if o.id.starts_with("stock-")
                || o.id.starts_with("flow-label-")
                || o.id.starts_with("parameter-label-")
                || o.id.starts_with("parameter-value-")
                || m.links.keys().any(|id| o.id == format!("link-{id}"))
                || o.id == "sweep-area"
            {
                o.click_handler = true;
            }
            o.transform = Affine::translate((0., STOCK_TOP)) * o.transform;
        }
        scene.height = STOCK_TOP + eq_y + 12.;
        scene.presentation = Presentation::Scroll;
        scene.inline_control("undo", Rect::new(0., 0., 66., 26.));
        let mut x = 74.;
        let mut y = 0.;
        for (i, preset) in presets().iter().enumerate() {
            let w = preset.label.chars().count() as f64 * 6. + 20.;
            if x + w > 680. {
                x = 0.;
                y += 28.;
            }
            scene.inline_control(PRESET_IDS[i], Rect::new(x, y, x + w, y + 26.));
            x += w + 6.;
        }
        scene
    }
    fn locate(&self, s: &State, id: &str) -> Option<Affine> {
        let m = &s.model;
        let translate = |p: Point| Some(Affine::translate(p.to_vec2() + Vec2::new(0., STOCK_TOP)));
        if id.starts_with("new-") {
            if let Some((key, p)) = &s.creating {
                if key == id {
                    return translate(*p);
                }
            }
        }
        if id == "new-stock" {
            return translate(Point::new(8., 8.));
        }
        if let Some(sid) = id.strip_prefix("stock-") {
            let stock = m.stocks.get(sid)?;
            return translate(Point::new(stock.x - 52., stock.y - 25.));
        }
        for (prefix, offset) in [
            ("new-in-", Vec2::new(-52., 0.)),
            ("new-out-", Vec2::new(52., 0.)),
            ("new-link-", Vec2::new(0., -25.)),
        ] {
            if let Some(sid) = id.strip_prefix(prefix) {
                let stock = m.stocks.get(sid)?;
                return translate(Point::new(stock.x, stock.y) + offset);
            }
        }
        if let Some(fid) = id.strip_prefix("flow-") {
            m.flows.get(fid)?;
            return translate(center(m, fid) - Vec2::new(12., 10.));
        }
        for prefix in ["end-from-", "end-to-"] {
            if let Some(fid) = id.strip_prefix(prefix) {
                m.flows.get(fid)?;
                let (_, _, _, from, to) = flow_geometry(m, fid);
                return translate(if prefix == "end-from-" { from } else { to });
            }
        }
        if let Some(lid) = id.strip_prefix("link-end-") {
            m.links.get(lid)?;
            return translate(link_geometry(m, lid).2);
        }
        if let Some(lid) = id.strip_prefix("link-") {
            m.links.get(lid)?;
            return translate(link_geometry(m, lid).3 - Vec2::new(13., 9.));
        }
        if let Some(pid) = id.strip_prefix("parameter-") {
            m.params.get(pid)?;
            return translate(knob_position(m, pid));
        }
        if let Some(sid) = id.strip_prefix("initial-") {
            let stock = m.stocks.get(sid)?;
            let (y, h) = chart_row(m, sid);
            let initial = m.params[&stock.init].value;
            return translate(Point::new(
                48.,
                y + h - (initial - stock.scale[0]) / (stock.scale[1] - stock.scale[0]) * h,
            ));
        }
        if id == "cursor" {
            return translate(Point::new(48. + 270. * m.cursor, 0.));
        }
        if id == "t-knob" {
            return translate(Point::new(
                100. + (m.duration / m.trange[0]).ln() / (m.trange[1] / m.trange[0]).ln() * 144.,
                624.,
            ));
        }
        self.scene(s).object(id).map(|o| o.transform)
    }
    fn drag(&self, s: &State, id: &str, _: Modifiers) -> Option<DragSpec<'static, State>> {
        let spec = (|| {
            if id == "t-knob" {
                let base = s.clone();
                let [lo, hi] = s.model.trange;
                return Some(dragology::vary_constrained(
                    [s.model.duration],
                    move |[duration]| {
                        let mut t = base.clone();
                        t.model.duration = duration;
                        t
                    },
                    move |t| [lo - t.model.duration, t.model.duration - hi],
                ));
            }
            if let Some(pid) = id.strip_prefix("parameter-") {
                s.model.params.get(pid)?;
                return Some(vary_parameter(s, pid));
            }
            if id == "cursor" {
                let base = s.clone();
                return Some(dragology::vary_constrained(
                    [s.model.cursor],
                    move |[cursor]| State {
                        model: Model {
                            cursor,
                            ..base.model.clone()
                        },
                        ..base.clone()
                    },
                    |t| [-t.model.cursor, t.model.cursor - 1.],
                ));
            }
            if let Some(sid) = id.strip_prefix("initial-") {
                return Some(vary_parameter(s, &s.model.stocks.get(sid)?.init));
            }
            if let Some(sid) = id.strip_prefix("stock-") {
                let stock = s.model.stocks.get(sid)?;
                let initial = [stock.x, stock.y];
                let mut deleted = s.clone();
                delete_stock(&mut deleted.model, sid);
                let base = s.clone();
                let sid = sid.to_string();
                let check = sid.clone();
                let free = dragology::vary_constrained(
                    initial,
                    move |[x, y]| {
                        let mut t = base.clone();
                        let stock = t.model.stocks.get_mut(&sid).unwrap();
                        stock.x = x;
                        stock.y = y;
                        t
                    },
                    move |t| {
                        let p = &t.model.stocks[&check];
                        [52. - p.x, p.x - 398., 25. - p.y, p.y - 295.]
                    },
                );
                return Some(trash_target(deleted).when_far(free, 50.));
            }
            if let Some(lid) = id.strip_prefix("link-end-") {
                s.model.links.get(lid)?;
                return Some(move_link(s, lid));
            }
            for (prefix, from) in [("end-from-", true), ("end-to-", false)] {
                if let Some(fid) = id.strip_prefix(prefix) {
                    s.model.flows.get(fid)?;
                    return Some(move_end(s, fid, from));
                }
            }
            if let Some(fid) = id.strip_prefix("flow-") {
                let fid = fid.to_string();
                s.model.flows.get(&fid)?;
                let base = s.clone();
                let movable = matches!(base.model.flows[&fid].from, End::Cloud(_))
                    || matches!(base.model.flows[&fid].to, End::Cloud(_));
                if !movable {
                    let mut removed = s.clone();
                    removed.model.flows.shift_remove(&fid);
                    tidy(&mut removed.model);
                    return Some(
                        dragology::drop_target(
                            removed,
                            Rect::new(78., STOCK_TOP + 4., 110., STOCK_TOP + 36.),
                        )
                        .when_far(s.clone(), 4.),
                    );
                }
                let mut removed = s.clone();
                removed.model.flows.shift_remove(&fid);
                tidy(&mut removed.model);
                return Some(
                    dragology::drop_target(
                        removed,
                        Rect::new(78., STOCK_TOP + 4., 110., STOCK_TOP + 36.),
                    )
                    .when_far(
                        dragology::vary([0., 0.], move |[dx, dy]| {
                            let mut t = base.clone();
                            let f = t.model.flows.get_mut(&fid).unwrap();
                            for e in [&mut f.from, &mut f.to] {
                                if let End::Cloud(p) = e {
                                    p.x += dx;
                                    p.y += dy;
                                }
                            }
                            t.creating = Some((format!("flow-{fid}"), center(&t.model, &fid)));
                            t
                        }),
                        0.,
                    )
                    .on_drop(|s| {
                        let mut t = s.clone();
                        if let Some((id, p)) = &s.creating {
                            if p.x >= 78. && p.x <= 110. && p.y >= 4. && p.y <= 36. {
                                if let Some(id) = id.strip_prefix("flow-") {
                                    t.model.flows.shift_remove(id);
                                }
                            }
                        }
                        finish(&t)
                    }),
                );
            }
            None
        })();
        let previous = s.pending_original.as_deref().unwrap_or(&s.model).clone();
        spec.map(|spec| {
            spec.on_drop(move |next| {
                let mut next = next.clone();
                next.pending_original = None;
                tidy(&mut next.model);
                fit_model(&mut next.model);
                if next.model != previous {
                    next.history.push(previous.clone());
                    if next.history.len() > 100 {
                        next.history.remove(0);
                    }
                }
                next
            })
        })
    }

    fn click(
        &self,
        s: &State,
        id: &str,
        point: Point,
        modifiers: Modifiers,
        count: usize,
    ) -> Option<State> {
        let point = point - Vec2::new(0., STOCK_TOP);
        let mut t = s.clone();
        if let Some(pid) = id.strip_prefix("parameter-value-") {
            if count > 1 {
                t.param = Some(pid.into());
                return Some(t);
            }
            return None;
        }
        if let Some(fid) = id.strip_prefix("flow-label-") {
            if count > 1 {
                t.selected = format!("flow-{fid}");
                return Some(t);
            }
            return None;
        }
        if let Some(pid) = id.strip_prefix("parameter-label-") {
            if count > 1 {
                t.param = Some(pid.into());
                return Some(t);
            }
            t.model.sweep = if t.model.sweep.as_deref() == Some(pid) {
                None
            } else {
                Some(pid.into())
            };
            fit_model(&mut t.model);
            t.history.push(s.model.clone());
            if t.history.len() > 100 {
                t.history.remove(0);
            }
            return Some(t);
        }
        if id == "sweep-area" {
            let pid = t.model.sweep.as_ref()?;
            let p = t.model.params.get_mut(pid)?;
            p.value = p.min + (p.max - p.min) * ((point.x - 394.) / 270.).clamp(0., 1.);
            fit_model(&mut t.model);
            t.history.push(s.model.clone());
            if t.history.len() > 100 {
                t.history.remove(0);
            }
            return Some(t);
        }
        if let Some(lid) = id.strip_prefix("link-") {
            if t.model.links.contains_key(lid) {
                {
                    let i = SHAPES
                        .iter()
                        .position(|x| *x == t.model.links[lid].shape)
                        .unwrap_or(0);
                    set_shape(
                        &mut t.model,
                        lid,
                        (i + if modifiers.shift { 5 } else { 1 }) % 6,
                    );
                    fit_model(&mut t.model);
                    t.history.push(s.model.clone());
                    if t.history.len() > 100 {
                        t.history.remove(0);
                    }
                }
                t.selected = id.into();
                return Some(t);
            }
        }
        if count > 1 && (id.starts_with("stock-") || id.starts_with("flow-")) {
            t.selected = id.into();
            return Some(t);
        }
        None
    }
    fn controls(&self, s: &State) -> Vec<Control> {
        let mut c = vec![Control::action_button(
            "undo",
            "↶ undo",
            !s.history.is_empty(),
            false,
        )];
        for (i, p) in presets().iter().enumerate() {
            c.push(Control::button(PRESET_IDS[i], &p.label));
        }
        if let Some(pid) = s.param.as_ref().filter(|p| s.model.params.contains_key(*p)) {
            let p = &s.model.params[pid];
            c.push(Control::Text {
                id: "parameter-name",
                label: "Rename".into(),
                value: p.label.clone(),
            });
            c.push(Control::Text {
                id: "parameter-value",
                label: format!("{} =", p.label),
                value: p.value.to_string(),
            });
        }
        if let Some(id) = s
            .selected
            .strip_prefix("stock-")
            .filter(|id| s.model.stocks.contains_key(*id))
        {
            let stock = &s.model.stocks[id];
            c.push(Control::Text {
                id: "name",
                label: "Rename stock".into(),
                value: stock.name.clone(),
            });
            c.push(Control::Text {
                id: "symbol",
                label: "Rename symbol".into(),
                value: stock.symbol.clone(),
            });
        }
        if let Some(id) = s
            .selected
            .strip_prefix("flow-")
            .filter(|id| s.model.flows.contains_key(*id))
        {
            c.push(Control::Text {
                id: "name",
                label: "Rename flow".into(),
                value: s.model.flows[id].name.clone(),
            });
        }
        c
    }
    fn action(&self, s: &State, id: &str, value: Value) -> Option<State> {
        if let Some(i) = PRESET_IDS.iter().position(|p| *p == id) {
            return self.action(s, "preset", Value::Index(i));
        }
        let mut t = s.clone();
        if id == "undo" {
            let previous = t.history.pop()?;
            t.model = previous;
            t.creating = None;
            return Some(t);
        }
        let ids = t.model.params.keys().cloned().collect::<Vec<_>>();
        match (id, value) {
            ("preset", Value::Index(i)) if i < presets().len() => {
                t.model = presets()[i].model.clone();
                t.preset = i;
                t.selected.clear();
                t.param = None;
            }
            ("duration", Value::Number(v)) => {
                t.model.duration = v.clamp(t.model.trange[0].max(0.001), t.model.trange[1])
            }
            ("parameter", Value::Index(i)) => t.param = ids.get(i).cloned(),
            ("value", Value::Number(v)) => {
                let pid = t.param.as_ref()?;
                let p = t.model.params.get_mut(pid)?;
                p.value = v.clamp(p.min, p.max);
            }
            ("parameter-value", Value::Text(text)) => {
                let value = text.trim().parse::<f64>().ok()?;
                if !value.is_finite() {
                    return None;
                }
                let p = t.model.params.get_mut(t.param.as_ref()?)?;
                p.value = value;
                p.min = p.min.min(value);
                p.max = p.max.max(value);
            }
            ("parameter-name", Value::Text(text)) => {
                t.model.params.get_mut(t.param.as_ref()?)?.label = text
            }
            ("parameter-range", Value::Text(text)) => {
                let ns = text
                    .split(',')
                    .map(str::trim)
                    .map(str::parse::<f64>)
                    .collect::<Result<Vec<_>, _>>()
                    .ok()?;
                if ns.len() != 2 || !ns[0].is_finite() || !ns[1].is_finite() || ns[0] >= ns[1] {
                    return None;
                }
                let p = t.model.params.get_mut(t.param.as_ref()?)?;
                p.min = ns[0];
                p.max = ns[1];
                p.value = p.value.clamp(p.min, p.max);
            }
            ("new-parameter", _) => {
                let pid = add_param(&mut t.model, "k", 1., 0., 10.);
                t.param = Some(pid);
            }
            ("sweep", Value::Bool(enabled)) => {
                t.model.sweep = if enabled { t.param.clone() } else { None }
            }
            ("name", Value::Text(text)) => {
                if let Some(id) = t.selected.strip_prefix("stock-") {
                    t.model.stocks.get_mut(id)?.name = text
                } else if let Some(id) = t.selected.strip_prefix("flow-") {
                    t.model.flows.get_mut(id)?.name = text
                } else {
                    return None;
                }
            }
            ("symbol", Value::Text(text)) => {
                t.model
                    .stocks
                    .get_mut(t.selected.strip_prefix("stock-")?)?
                    .symbol = text
            }
            ("unit", Value::Text(text)) => {
                t.model
                    .stocks
                    .get_mut(t.selected.strip_prefix("stock-")?)?
                    .unit = if text.is_empty() { None } else { Some(text) }
            }
            ("initial-param", Value::Index(i)) => {
                t.model
                    .stocks
                    .get_mut(t.selected.strip_prefix("stock-")?)?
                    .init = ids.get(i)?.clone()
            }
            ("volume", Value::Index(i)) => {
                t.model
                    .stocks
                    .get_mut(t.selected.strip_prefix("stock-")?)?
                    .volume = if i == 0 {
                    None
                } else {
                    Some(ids.get(i - 1)?.clone())
                }
            }
            ("rate", Value::Index(i)) => {
                t.model
                    .flows
                    .get_mut(t.selected.strip_prefix("flow-")?)?
                    .rate = if i == 0 {
                    None
                } else {
                    Some(ids.get(i - 1)?.clone())
                }
            }
            ("shape", Value::Index(i)) if i < 6 => {
                let lid = t.selected.strip_prefix("link-")?.to_string();
                set_shape(&mut t.model, &lid, i);
            }
            ("factor-p0", Value::Index(i)) | ("factor-p1", Value::Index(i)) => {
                let link = t.model.links.get_mut(t.selected.strip_prefix("link-")?)?;
                let index = usize::from(id == "factor-p1");
                *link.params.get_mut(index)? = ids.get(i)?.clone();
            }
            ("delete", _) => {
                if let Some(id) = t.selected.strip_prefix("stock-") {
                    delete_stock(&mut t.model, id)
                } else if let Some(id) = t.selected.strip_prefix("flow-") {
                    t.model.flows.shift_remove(id);
                } else if let Some(id) = t.selected.strip_prefix("link-") {
                    t.model.links.shift_remove(id);
                }
                t.selected.clear();
                tidy(&mut t.model);
            }
            _ => return None,
        }
        if t.model != s.model {
            fit_model(&mut t.model);
            t.history.push(s.model.clone());
            if t.history.len() > 100 {
                t.history.remove(0);
            }
        }
        Some(t)
    }
    fn key(&self, s: &State, key: &str, modifiers: Modifiers) -> Option<State> {
        if key == "z" && modifiers.control {
            return self.action(s, "undo", Value::Trigger);
        }
        if key == "delete" || key == "backspace" {
            self.action(s, "delete", Value::Trigger)
        } else {
            None
        }
    }
    fn status(&self, s: &State) -> String {
        let run = simulate(&s.model);
        format!(
            "{} stocks · {} flows · {} dependencies · t = {:.2} {}{}",
            s.model.stocks.len(),
            s.model.flows.len(),
            s.model.links.len(),
            s.model.cursor * s.model.duration,
            s.model.time_unit,
            if run.diverged {
                " · integration diverged: reduce rates or run duration"
            } else {
                " · RK4, 400 steps"
            }
        )
    }
    fn can_import(&self) -> bool {
        true
    }
    fn export(&self, s: &State) -> Option<(String, String)> {
        Some((
            "stock-and-flow.json".into(),
            serde_json::to_string_pretty(&s.model).ok()?,
        ))
    }
    fn import(&self, s: &State, text: &str) -> Result<State, String> {
        let mut json: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
        let object = json.as_object_mut().ok_or("not a JSON object")?;
        let duration = object
            .get("T")
            .and_then(|v| v.as_f64())
            .ok_or("missing T")?;
        object
            .entry("Trange")
            .or_insert(serde_json::json!([duration.min(1.), duration.max(100.)]));
        object.entry("timeUnit").or_insert(serde_json::json!(""));
        if !object.get("cursor").is_some_and(|v| v.is_number()) {
            object.insert("cursor".into(), serde_json::json!(1.));
        }
        if !object.get("sweep").is_some_and(|v| v.is_string()) {
            object.insert("sweep".into(), serde_json::Value::Null);
        }
        let mut model: Model = serde_json::from_value(json).map_err(|e| e.to_string())?;
        tidy(&mut model);
        validate(&model)?;
        fit_model(&mut model);
        let param = None;
        let mut history = s.history.clone();
        history.push(s.model.clone());
        if history.len() > 100 {
            history.remove(0);
        }
        Ok(State {
            model,
            param,
            selected: String::new(),
            creating: None,
            pending_original: None,
            history,
            ..s.clone()
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_endpoints_exclude_self_and_palette_trash_cancels_creation() {
        let mut app = demo(StockFlow);
        app.set_animated(false);
        app.action("preset", Value::Index(3));
        let m = presets()[3].model.clone();
        let dir =
            (endpoint(&m, &End::Id("I".into())) - endpoint(&m, &End::Id("S".into()))).normalize();
        let start =
            app.scene().object("end-from-infection").unwrap().transform * Point::ORIGIN + dir * 5.;
        let target = endpoint(&m, &End::Id("I".into())) + Vec2::new(0., STOCK_TOP) + dir * 5.;
        app.press(start, Modifiers::default(), 1);
        app.move_to(target, Modifiers::default());
        app.advance(std::time::Duration::ZERO);
        app.release(target, Modifiers::default());
        let model: Model = serde_json::from_str(&app.export().unwrap().1).unwrap();
        assert!(
            matches!(model.flows["infection"].from, End::Cloud(_)),
            "opposite stock cannot also become the source"
        );
        app.action("undo", Value::Trigger);
        let before = app.export().unwrap().1;
        let palette = app.scene().object("new-stock").unwrap().transform * Point::new(32., 12.);
        let trash = Point::new(94., STOCK_TOP + 20.);
        app.press(palette, Modifiers::default(), 1);
        app.move_to(trash, Modifiers::default());
        app.advance(std::time::Duration::ZERO);
        app.release(trash, Modifiers::default());
        assert_eq!(
            app.export().unwrap().1,
            before,
            "palette-to-trash leaves no stock or orphan initial parameter"
        );
    }
    #[test]
    fn simulation_sweeps_and_formulas_match_pinned_upstream() {
        let fixtures: serde_json::Value =
            serde_json::from_str(include_str!("stock/upstream-fixtures.json")).unwrap();
        let close = |actual: f64, expected: f64| {
            assert!(
                (actual - expected).abs() <= 1e-10 * expected.abs().max(1.),
                "{actual} differs from source {expected}"
            );
        };
        for f in fixtures.as_array().unwrap() {
            let mut m = presets()[f["preset"].as_u64().unwrap() as usize]
                .model
                .clone();
            if f["changed"] == true {
                for p in m.params.values_mut() {
                    p.value = p.min + 0.63 * (p.max - p.min);
                }
            }
            fit_model(&mut m);
            let mut expected: Model = serde_json::from_value(f["model"].clone()).unwrap();
            fit_model(&mut expected);
            assert_eq!(
                serde_json::to_value(&m).unwrap(),
                serde_json::to_value(&expected).unwrap()
            );
            let run = integrate(&m, None);
            assert_eq!(run.diverged, f["diverged"].as_bool().unwrap());
            for (name, values) in [("values", &run.values), ("rates", &run.rates)] {
                for (id, samples) in f[name].as_object().unwrap() {
                    for pair in samples.as_array().unwrap() {
                        close(
                            values[id][pair[0].as_u64().unwrap() as usize],
                            pair[1].as_f64().unwrap(),
                        );
                    }
                }
            }
            for (id, formula) in f["formulas"].as_object().unwrap() {
                assert_eq!(flow_formula(&m, id), formula.as_str().unwrap());
            }
            if let Some(pid) = &m.sweep {
                let bands = sweep(&m, pid);
                for (id, source) in f["bands"].as_object().unwrap() {
                    for (actual, expected) in bands[id].iter().zip(source.as_array().unwrap()) {
                        if let Some([low, high]) = actual {
                            close(*low, expected[0].as_f64().unwrap());
                            close(*high, expected[1].as_f64().unwrap());
                        } else {
                            assert!(expected.is_null());
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn source_import_defaults_order_and_logarithmic_time_drag() {
        let state = StockFlow.initial();
        let mut json = serde_json::to_value(&state.model).unwrap();
        for key in ["Trange", "timeUnit", "cursor", "sweep"] {
            json.as_object_mut().unwrap().remove(key);
        }
        let loaded = StockFlow.import(&state, &json.to_string()).unwrap();
        assert_eq!(loaded.model.cursor, 1.);
        assert_eq!(loaded.model.time_unit, "");
        assert_eq!(
            loaded.model.trange,
            [
                loaded.model.duration.min(1.),
                loaded.model.duration.max(100.)
            ]
        );
        let first_stock = loaded.model.stocks.values().next().unwrap();
        assert_eq!(param_order(&loaded.model)[0], first_stock.init);
        let mut app = demo(StockFlow);
        app.set_animated(false);
        let origin = app.scene().object("t-knob").unwrap().transform * Point::ORIGIN;
        app.press(origin, Modifiers::default(), 1);
        app.move_to(origin + Vec2::new(20., 0.), Modifiers::default());
        app.advance(std::time::Duration::ZERO);
        app.release(origin + Vec2::new(20., 0.), Modifiers::default());
        let (_, saved) = app.export().unwrap();
        let model: Model = serde_json::from_str(&saved).unwrap();
        let expected =
            state.model.duration * (state.model.trange[1] / state.model.trange[0]).powf(20. / 144.);
        // The source's numerical vary solver promises the projected knob position.
        let pixel_error = 144. * (model.duration / expected).ln()
            / (state.model.trange[1] / state.model.trange[0]).ln();
        assert!(pixel_error.abs() < 0.01, "log slider error {pixel_error}px");
    }
    #[test]
    fn new_outflow_and_undo_follow_a_real_drag() {
        let mut app = demo(StockFlow);
        app.set_animated(false);
        let original = app.export().unwrap().1;
        let start = app.scene().object("new-out-G").unwrap().transform * Point::ORIGIN;
        let end = Point::new(445., 310.);
        app.press(start, Modifiers::default(), 1);
        app.move_to(end, Modifiers::default());
        app.advance(std::time::Duration::ZERO);
        app.release(end, Modifiers::default());
        let model: Model = serde_json::from_str(&app.export().unwrap().1).unwrap();
        let flow = model.flows.get("flow0").expect("created outflow");
        assert!(flow.from == End::Id("G".into()));
        assert!(matches!(flow.to, End::Cloud(_)));
        assert!(
            model
                .links
                .values()
                .any(|l| l.from == "G" && l.to == End::Id("flow0".into()) && l.shape == "linear")
        );
        let k = model.params[flow.rate.as_ref().unwrap()].value;
        assert!((k - 3. / 12. * 15000.).abs() < 1e-9);
        assert!((integrate(&model, None).rates["flow0"][0] - k * 1.5).abs() < 1e-9);
        app.action("undo", Value::Trigger);
        let restored: Model = serde_json::from_str(&app.export().unwrap().1).unwrap();
        let before: Model = serde_json::from_str(&original).unwrap();
        assert!(restored == before);
    }
    #[test]
    fn endpoint_excludes_self_and_deleting_cleans_parameters() {
        let mut m = presets()[3].model.clone();
        let rate = m.flows["infection"].rate.clone().unwrap();
        m.flows.shift_remove("infection");
        tidy(&mut m);
        assert!(!m.params.contains_key(&rate));
        assert!(
            m.links
                .values()
                .all(|l| l.to != End::Id("infection".into()))
        );
    }
    #[test]
    fn geometry_lookup_matches_drawn_handles_and_valve_solver_follows_pointer() {
        let example = StockFlow;
        let s = example.initial();
        for object in example.scene(&s).objects {
            if object.hit.is_some() && example.drag(&s, &object.id, Modifiers::default()).is_some()
            {
                let located = example.locate(&s, &object.id).unwrap();
                assert!(
                    (located * Point::ORIGIN).distance(object.transform * Point::ORIGIN) < 1e-9,
                    "{}",
                    object.id
                );
            }
        }
        let press = center(&s.model, "input");
        let spec = example
            .drag(&s, "flow-input", Modifiers::default())
            .unwrap();
        let mut drag =
            dragology::DragSession::begin(s, press, |s| StockFlow.locate(s, "flow-input"), spec)
                .unwrap();
        let pointer = press + Vec2::new(-12., 18.);
        let frame = drag.frame(pointer).unwrap();
        assert!(frame.grab_position.unwrap().distance(pointer) < 0.1);
    }
    #[test]
    fn exact_parameter_entry_expands_range_and_undo_restores_it() {
        let mut s = StockFlow.initial();
        s.param = Some("p0".into());
        let edited = StockFlow
            .action(&s, "parameter-value", Value::Text("25000".into()))
            .unwrap();
        assert_eq!(edited.model.params["p0"].value, 25000.);
        assert_eq!(edited.model.params["p0"].max, 25000.);
        let restored = StockFlow.action(&edited, "undo", Value::Trigger).unwrap();
        assert!(restored.model == s.model);
        assert!(
            StockFlow
                .action(&s, "parameter-value", Value::Text("NaN".into()))
                .is_none()
        );
    }
    #[test]
    fn all_nine_presets_validate_and_round_trip() {
        assert_eq!(presets().len(), 9);
        for p in presets() {
            validate(&p.model).unwrap();
            let data = serde_json::to_string(&p.model).unwrap();
            let parsed: Model = serde_json::from_str(&data).unwrap();
            assert!(parsed == p.model);
            assert!(!integrate(&p.model, None).diverged, "{}", p.label);
        }
    }
    #[test]
    fn sir_conserves_population() {
        let m = &presets()[3].model;
        let run = integrate(m, None);
        for i in 0..=STEPS {
            let total: f64 = run.values.values().map(|v| v[i]).sum();
            assert!((total - 1.).abs() < 1e-10);
        }
    }
    #[test]
    fn rk4_matches_damped_oscillator() {
        let m = &presets()[7].model;
        let run = integrate(m, None);
        let omega = (1f64 - 0.1f64.powi(2)).sqrt();
        for i in [0, 20, 100, 200, 400] {
            let t = m.duration * i as f64 / STEPS as f64;
            let expected = (-0.1 * t).exp() * ((omega * t).cos() + 0.1 / omega * (omega * t).sin());
            assert!((run.values["x"][i] - expected).abs() < 2e-5);
        }
    }
    #[test]
    fn stock_deletion_preserves_clouds_and_rejects_bad_import() {
        let mut m = presets()[3].model.clone();
        delete_stock(&mut m, "I");
        tidy(&mut m);
        assert!(matches!(m.flows["infection"].to, End::Cloud(_)));
        assert!(m.links.values().all(|l| l.from != "I"));
        validate(&m).unwrap();
        m.links.insert(
            "bad".into(),
            Link {
                from: "S".into(),
                to: End::Id("infection".into()),
                shape: "repression".into(),
                params: vec![],
            },
        );
        assert!(validate(&m).is_err());
    }
    #[test]
    fn nonlinear_factors_obey_threshold_and_capacity() {
        assert_eq!(factor("threshold", 0.5, &[1.]), 0.);
        assert_eq!(factor("crowding", 2., &[1.]), 0.);
        assert!((factor("repression", 2., &[2., 3.]) - 0.5).abs() < 1e-12);
        assert_eq!(factor("gap", 3., &[2.]), -1.);
    }
}
