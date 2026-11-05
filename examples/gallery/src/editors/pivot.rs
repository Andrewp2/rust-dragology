use super::*;
use dragology::closest;

const FIELDS: [&str; 5] = ["region", "product", "quarter", "sales", "units"];
const DOMAINS: [&[&str]; 3] = [
    &["North", "South", "West"],
    &["Widgets", "Gadgets", "Gizmos"],
    &["Q1", "Q2", "Q3", "Q4"],
];
#[derive(Clone, Debug, PartialEq)]
pub struct State {
    zones: [Vec<usize>; 4],
    in_table: bool,
}
pub struct Pivot;
#[derive(Clone)]
struct Record {
    dims: [usize; 3],
    sales: u32,
    units: u32,
}
fn records() -> Vec<Record> {
    let mut seed: f64 = 12345.;
    let mut rand = || -> f64 {
        seed = (seed * 1103515245. + 12345.) % 2147483648.;
        seed / 2147483648.
    };
    let mut out = Vec::new();
    for a in 0..3 {
        for b in 0..3 {
            for c in 0..4 {
                let units = 5 + (rand() * 60.).floor() as u32;
                let price = 8 + (rand() * 12.).floor() as u32;
                out.push(Record {
                    dims: [a, b, c],
                    sales: units * price,
                    units,
                });
            }
        }
    }
    out
}
fn tuples(dims: &[usize]) -> Vec<Vec<usize>> {
    let mut out = vec![vec![]];
    for &dim in dims {
        out = out
            .into_iter()
            .flat_map(|t| {
                (0..DOMAINS[dim].len()).map(move |v| {
                    let mut n = t.clone();
                    n.push(v);
                    n
                })
            })
            .collect();
    }
    out
}
fn aggregate(
    records: &[Record],
    rd: &[usize],
    rt: &[usize],
    cd: &[usize],
    ct: &[usize],
    measure: usize,
) -> u32 {
    records
        .iter()
        .filter(|r| {
            rd.iter()
                .zip(rt)
                .chain(cd.iter().zip(ct))
                .all(|(&d, &v)| r.dims[d] == v)
        })
        .map(|r| match measure {
            3 => r.sales,
            4 => r.units,
            _ => 1,
        })
        .sum()
}
fn group_id(rd: &[usize], rt: &[usize], cd: &[usize], ct: &[usize], measure: usize) -> String {
    let mut group = rd
        .iter()
        .zip(rt)
        .chain(cd.iter().zip(ct))
        .map(|(&d, &v)| (d, v))
        .collect::<Vec<_>>();
    group.sort();
    format!(
        "cell-{}-{}",
        group
            .iter()
            .map(|(d, v)| format!("{}-{}", FIELDS[*d], DOMAINS[*d][*v]))
            .collect::<Vec<_>>()
            .join("-"),
        measure_name(measure)
    )
}
fn measure_name(m: usize) -> &'static str {
    if m < 5 { FIELDS[m] } else { "count" }
}
fn chips_width(n: usize) -> f64 {
    n.max(1) as f64 * 64. + n.saturating_sub(1) as f64 * 4.
}
struct Geometry {
    corner: f64,
    row_width: f64,
    header_row: f64,
    measure_y: f64,
    row_field_y: f64,
    header_h: f64,
    cell: f64,
    show_measure: bool,
    zones: [Option<f64>; 4],
}
fn geometry(s: &State) -> Geometry {
    let header_row = if s.in_table { 26. } else { 18. };
    let row_columns = s.zones[1].len().max(1);
    let col_levels = if s.in_table {
        s.zones[2].len().max(1)
    } else {
        s.zones[2].len()
    };
    let measures = s.zones[3].len().max(1);
    let show_measure = s.in_table || measures > 1 || s.zones[2].is_empty();
    let measure_y = col_levels as f64 * header_row;
    let row_field_y = measure_y + if show_measure { header_row } else { 0. };
    let corner = (row_columns as f64 * 76.).max(if s.in_table {
        chips_width(s.zones[3].len()) + 8.
    } else {
        0.
    });
    let columns = tuples(&s.zones[2]).len() * measures;
    let mut zones = [None; 4];
    let mut x = 0.;
    for z in 0..if s.in_table { 1 } else { 4 } {
        zones[z] = Some(x);
        x += (chips_width(s.zones[z].len()) + 12.).max(70.) + 12.;
    }
    Geometry {
        corner,
        row_width: corner / row_columns as f64,
        header_row,
        measure_y,
        row_field_y,
        header_h: row_field_y + if s.in_table { header_row } else { 0. },
        cell: ((720. - corner) / columns as f64).clamp(8., 64.),
        show_measure,
        zones,
    }
}
fn chip_pos(s: &State, g: &Geometry, zone: usize, i: usize) -> Point {
    if let Some(x) = g.zones[zone] {
        return Point::new(x + 6. + i as f64 * 68., 22.);
    }
    match zone {
        2 => Point::new(g.corner - 68., 68. + i as f64 * g.header_row + 2.),
        3 => Point::new(
            g.corner - 4. - chips_width(s.zones[3].len()) + i as f64 * 68.,
            68. + g.measure_y + 2.,
        ),
        _ => Point::new(i as f64 * g.row_width + 4., 68. + g.row_field_y + 2.),
    }
}
fn header_cells(
    dims: &[usize],
    tuples: &[Vec<usize>],
) -> Vec<(usize, String, String, usize, usize)> {
    let mut result = Vec::new();
    for level in 0..dims.len() {
        let mut start = 0;
        while start < tuples.len() {
            let prefix = &tuples[start][..=level];
            let mut end = start;
            while end < tuples.len() && tuples[end][..=level] == *prefix {
                end += 1;
            }
            let id = dims[..=level]
                .iter()
                .zip(prefix)
                .map(|(d, v)| format!("{}-{}", FIELDS[*d], DOMAINS[*d][*v]))
                .collect::<Vec<_>>()
                .join("-");
            result.push((
                level,
                id,
                DOMAINS[dims[level]][prefix[level]].into(),
                start,
                end - start,
            ));
            start = end;
        }
    }
    result
}
fn table_cell(
    id: impl Into<String>,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    color: Color,
    text: Option<(&str, f64)>,
    align: usize,
) -> Object {
    let mut o = Object::rect(id, Rect::new(x, y, x + w, y + h), 0., color).decorative();
    if let Shape::Rect { stroke, .. } = &mut o.shapes[0] {
        *stroke = Some((hex_color("d1d5db"), 1.));
    }
    if let Some((text, size)) = text {
        let (tx, anchor) = match align {
            1 => (w / 2., TextAnchor::Middle),
            2 => (w - 5., TextAnchor::End),
            _ => (6., TextAnchor::Start),
        };
        o = o
            .label(text, (tx, 9.), size, hex_color("374151"))
            .text_anchor(anchor)
            .text_baseline(TextBaseline::Central);
    }
    o
}
fn number(v: u32) -> String {
    let text = v.to_string();
    let mut out = String::new();
    for (i, c) in text.chars().enumerate() {
        if i > 0 && (text.len() - i) % 3 == 0 {
            out.push(',')
        }
        out.push(c)
    }
    out
}
impl Example for Pivot {
    type State = State;
    fn initial(&self) -> State {
        State {
            zones: [vec![4], vec![0, 1], vec![2], vec![3]],
            in_table: false,
        }
    }
    fn scene(&self, s: &State) -> Scene {
        self.scene_dragging(s, None)
    }
    fn scene_dragging(&self, s: &State, dragged: Option<&str>) -> Scene {
        let mut scene = Scene::new(720., 800.);
        let g = geometry(s);
        let rd = &s.zones[1];
        let cd = &s.zones[2];
        let rt = tuples(rd);
        let ct = tuples(cd);
        let measures = if s.zones[3].is_empty() {
            vec![5]
        } else {
            s.zones[3].clone()
        };
        let records = records();
        let leafcols = ct.len() * measures.len();
        for (z, name) in ["Fields", "Rows", "Columns", "Values"].iter().enumerate() {
            if let Some(x) = g.zones[z] {
                scene.push(baseline_text(
                    format!("zone-title-{z}"),
                    (x, 11.),
                    *name,
                    11.,
                    hex_color("6b7280"),
                ));
                let mut zone = Object::rect(
                    format!("zone-{z}"),
                    Rect::new(
                        x,
                        16.,
                        x + (chips_width(s.zones[z].len()) + 12.).max(70.),
                        50.,
                    ),
                    5.,
                    hex_color(if z == 0 { "fafafa" } else { "f3f4f6" }),
                )
                .decorative();
                if let Shape::Rect { stroke, .. } = &mut zone.shapes[0] {
                    *stroke = Some((hex_color("d1d5db"), 1.));
                }
                if z == 0 {
                    zone = zone.dash(vec![4., 3.]);
                }
                scene.push(zone);
            }
        }
        scene.push(table_cell(
            "corner",
            0.,
            68.,
            g.corner
                + if s.in_table {
                    leafcols as f64 * g.cell
                } else {
                    0.
                },
            g.header_h,
            hex_color("f9fafb"),
            None,
            0,
        ));
        if !s.in_table {
            for (i, d) in rd.iter().enumerate() {
                scene.push(baseline_text(
                    format!("corner-{}", FIELDS[*d]),
                    (i as f64 * g.row_width + 6., 68. + g.header_h - 9. + 3.),
                    FIELDS[*d],
                    10.,
                    hex_color("6b7280"),
                ));
            }
        }
        let header_color = |level: usize| {
            hex_color(if level == 0 {
                "e5e7eb"
            } else if level == 1 {
                "eef0f3"
            } else {
                "f5f6f8"
            })
        };
        for (level, id, label, start, span) in header_cells(cd, &ct) {
            let w = span as f64 * measures.len() as f64 * g.cell;
            let mut c = table_cell(
                format!("ch-{id}"),
                g.corner + start as f64 * measures.len() as f64 * g.cell,
                68. + level as f64 * g.header_row,
                w,
                g.header_row,
                header_color(level),
                if w >= 32. {
                    Some((label.as_str(), 10.))
                } else {
                    None
                },
                1,
            );
            if let Some(Shape::Text { origin, .. }) = c.shapes.get_mut(1) {
                origin.y += (g.header_row - 18.) / 2.;
            }
            scene.push(c);
        }
        if g.show_measure {
            for (ci, tuple) in ct.iter().enumerate() {
                for (mi, m) in measures.iter().enumerate() {
                    let path = cd
                        .iter()
                        .zip(tuple)
                        .map(|(d, v)| format!("{}-{}", FIELDS[*d], DOMAINS[*d][*v]))
                        .collect::<Vec<_>>()
                        .join("-");
                    let mut c = table_cell(
                        format!("ch-{path}-m-{}", measure_name(*m)),
                        g.corner + (ci * measures.len() + mi) as f64 * g.cell,
                        68. + g.measure_y,
                        g.cell,
                        g.header_row,
                        header_color(cd.len()),
                        if g.cell >= 32. {
                            Some((measure_name(*m), 10.))
                        } else {
                            None
                        },
                        1,
                    );
                    if let Some(Shape::Text { origin, .. }) = c.shapes.get_mut(1) {
                        origin.y += (g.header_row - 18.) / 2.;
                    }
                    scene.push(c);
                }
            }
        }
        if rd.is_empty() {
            scene.push(table_cell(
                "rh-all",
                0.,
                68. + g.header_h,
                g.row_width,
                18.,
                header_color(0),
                Some(("All", 10.)),
                0,
            ));
        }
        for (level, id, label, start, span) in header_cells(rd, &rt) {
            scene.push(table_cell(
                format!("rh-{id}"),
                level as f64 * g.row_width,
                68. + g.header_h + start as f64 * 18.,
                g.row_width,
                span as f64 * 18.,
                header_color(level),
                Some((&label, 10.)),
                0,
            ));
        }
        let mut max = [0u32; 6];
        for r in &rt {
            for c in &ct {
                for m in &measures {
                    max[*m] = max[*m].max(aggregate(&records, rd, r, cd, c, *m));
                }
            }
        }
        for (ri, r) in rt.iter().enumerate() {
            for (ci, c) in ct.iter().enumerate() {
                for (mi, m) in measures.iter().enumerate() {
                    let value = aggregate(&records, rd, r, cd, c, *m);
                    let t = if max[*m] > 0 {
                        value as f64 / max[*m] as f64
                    } else {
                        0.
                    };
                    let base = match m {
                        3 => [59., 130., 246.],
                        4 => [249., 115., 22.],
                        _ => [107., 114., 128.],
                    };
                    let color = [
                        (255. + (base[0] - 255.) * t * 0.55).round() as u8,
                        (255. + (base[1] - 255.) * t * 0.55).round() as u8,
                        (255. + (base[2] - 255.) * t * 0.55).round() as u8,
                        255,
                    ];
                    let label = number(value);
                    let mut cell = table_cell(
                        group_id(rd, r, cd, c, *m),
                        g.corner + (ci * measures.len() + mi) as f64 * g.cell,
                        68. + g.header_h + ri as f64 * 18.,
                        g.cell,
                        18.,
                        color,
                        if g.cell >= 32. {
                            Some((&label, 10.))
                        } else {
                            None
                        },
                        2,
                    );
                    if let Shape::Rect { stroke, .. } = &mut cell.shapes[0] {
                        *stroke = Some((hex_color("e5e7eb"), 1.));
                    }
                    scene.push(cell);
                }
            }
        }
        if s.in_table {
            for z in [2, 3, 1] {
                if s.zones[z].is_empty() {
                    let p = chip_pos(s, &g, z, 0);
                    let mut slot = Object::rect(
                        format!("slot-{z}"),
                        Rect::new(p.x, p.y, p.x + 64., p.y + 22.),
                        11.,
                        WHITE,
                    )
                    .decorative()
                    .baseline_label(
                        ["fields", "rows", "columns", "values"][z],
                        (8., 14.),
                        10.,
                        hex_color("9ca3af"),
                    );
                    if let Shape::Rect { stroke, .. } = &mut slot.shapes[0] {
                        *stroke = Some((hex_color("9ca3af"), 1.));
                    }
                    scene.push(slot.dash(vec![3., 2.]).italic());
                }
            }
        }
        for (z, fields) in s.zones.iter().enumerate() {
            for (i, f) in fields.iter().enumerate() {
                let p = chip_pos(s, &g, z, i);
                let id = format!("chip-{}", FIELDS[*f]);
                let mut chip = Object::rect(
                    &id,
                    Rect::new(p.x, p.y, p.x + 64., p.y + 22.),
                    11.,
                    hex_color(if *f < 3 { "bfdbfe" } else { "fed7aa" }),
                )
                .baseline_label(
                    FIELDS[*f],
                    ((64. - FIELDS[*f].len() as f64 * 5.7) / 2., 14.5),
                    11.,
                    hex_color("1f2937"),
                );
                if let Shape::Rect { stroke, .. } = &mut chip.shapes[0] {
                    *stroke = Some((
                        hex_color(if *f < 3 { "3b82f6" } else { "f97316" }),
                        if dragged == Some(id.as_str()) { 2. } else { 1. },
                    ));
                }
                scene.push(chip);
            }
        }
        scene
    }
    fn drag(&self, s: &State, id: &str, _: Modifiers) -> Option<DragSpec<'static, State>> {
        let field = FIELDS
            .iter()
            .position(|field| Some(*field) == id.strip_prefix("chip-"))?;
        if field >= 5 {
            return None;
        }
        let mut base = s.clone();
        for fields in &mut base.zones {
            fields.retain(|f| *f != field);
        }
        let zones: &[usize] = if field < 3 { &[0, 1, 2] } else { &[0, 3] };
        let mut targets = vec![s.clone()];
        for &zone in zones {
            for slot in 0..=base.zones[zone].len() {
                let mut n = base.clone();
                n.zones[zone].insert(slot, field);
                targets.push(n);
            }
        }
        Some(
            closest(targets)
                .when_far_with(s.clone(), 40., 56.)
                .with_branch_transition(Some(transition(150)))
                .with_floating_options(dragology::FloatingOptions {
                    ghost_opacity: Some(0.3),
                    tether: None,
                }),
        )
    }
    fn controls(&self, s: &State) -> Vec<Control> {
        vec![Control::toggle(
            "in-table",
            "Chips in table headers",
            s.in_table,
        )]
    }
    fn action(&self, s: &State, id: &str, value: Value) -> Option<State> {
        let Value::Bool(value) = value else {
            return None;
        };
        if id != "in-table" {
            return None;
        }
        let mut n = s.clone();
        n.in_table = value;
        Some(n)
    }
    fn status(&self, s: &State) -> String {
        format!(
            "36 records · {} row groups × {} column groups",
            tuples(&s.zones[1]).len(),
            tuples(&s.zones[2]).len()
        )
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn moving_dimensions_preserves_data_and_cell_identity() {
        let data = records();
        let total: u32 = data.iter().map(|r| r.sales).sum();
        let a = tuples(&[0, 1])
            .iter()
            .flat_map(|r| tuples(&[2]).into_iter().map(move |c| (r.clone(), c)))
            .map(|(r, c)| aggregate(&data, &[0, 1], &r, &[2], &c, 3))
            .sum::<u32>();
        assert_eq!(a, total);
        assert_eq!(
            group_id(&[0, 1], &[1, 2], &[2], &[3], 3),
            group_id(&[2], &[3], &[1, 0], &[2, 1], 3)
        );
        assert_eq!(data.len(), 36);
    }
}
