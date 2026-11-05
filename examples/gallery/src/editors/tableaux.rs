use super::*;

#[derive(Clone, serde::Deserialize)]
struct ArticleRun {
    text: String,
    weight: u16,
    italic: bool,
    mono: bool,
    color: String,
}
impl ArticleRun {
    fn style(&self) -> TextStyle {
        TextStyle {
            weight: self.weight,
            italic: self.italic,
            family: if self.mono {
                FontFamily::Mono
            } else {
                FontFamily::Sans
            },
            ..Default::default()
        }
    }
    fn same_style(&self, other: &Self) -> bool {
        self.weight == other.weight
            && self.italic == other.italic
            && self.mono == other.mono
            && self.color == other.color
    }
}

// The source is a 65ch article, with inline emphasis and a numbered list.
// Wrap words using the same native font metrics used to draw them; preserve
// styled portions inside words rather than introducing spaces around JSX tags.
fn article_lines(runs: &[ArticleRun], size: f64, max_width: f64) -> Vec<Vec<ArticleRun>> {
    let mut words: Vec<Vec<ArticleRun>> = vec![];
    let mut word = vec![];
    for run in runs {
        let mut text = String::new();
        for c in run.text.chars() {
            if c.is_whitespace() {
                if !text.is_empty() {
                    word.push(ArticleRun {
                        text: std::mem::take(&mut text),
                        ..run.clone()
                    });
                }
                if !word.is_empty() {
                    words.push(std::mem::take(&mut word));
                }
            } else {
                text.push(c);
            }
        }
        if !text.is_empty() {
            word.push(ArticleRun {
                text,
                ..run.clone()
            });
        }
    }
    if !word.is_empty() {
        words.push(word);
    }
    let mut lines = vec![];
    let mut line: Vec<ArticleRun> = vec![];
    let mut width = 0.;
    for word in words {
        let word_width: f64 = word
            .iter()
            .map(|r| crate::text_layout::measure_text(&r.text, size, &r.style()).advance)
            .sum();
        let gap = line
            .last()
            .map(|r| crate::text_layout::measure_text(" ", size, &r.style()).advance)
            .unwrap_or(0.);
        if !line.is_empty() && width + gap + word_width > max_width {
            lines.push(std::mem::take(&mut line));
            width = 0.;
        }
        if let Some(last) = line.last_mut() {
            last.text.push(' ');
            width += gap;
        }
        for run in word {
            if let Some(last) = line.last_mut().filter(|last| last.same_style(&run)) {
                last.text.push_str(&run.text);
            } else {
                line.push(run);
            }
        }
        width += word_width;
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}
use dragology::{between, closest};
#[cfg(test)]
use std::collections::BTreeSet;
const S: f64 = 36.;
#[derive(Clone, PartialEq)]
struct Cell {
    r: usize,
    c: usize,
    value: usize,
}
type Rows = Vec<Vec<(usize, usize)>>;
#[derive(Clone, PartialEq)]
pub struct State {
    section: usize,
    shape: Vec<Cell>,
    fill: Vec<Cell>,
    found: Vec<String>,
    rows: Rows,
    queue: Vec<(usize, usize)>,
    held: Option<(usize, usize, usize)>,
    message: String,
    word: Vec<usize>,
    k: usize,
    mu: Vec<usize>,
    skew: Vec<Cell>,
    hole: Option<(usize, usize)>,
}
pub struct Tableaux;
fn row_insert(rows: &mut Rows, mut item: (usize, usize)) -> (usize, usize) {
    for (r, row) in rows.iter_mut().enumerate() {
        if let Some(c) = row.iter().position(|x| x.1 > item.1) {
            std::mem::swap(&mut row[c], &mut item)
        } else {
            row.push(item);
            return (r, row.len() - 1);
        }
    }
    rows.push(vec![item]);
    (rows.len() - 1, 0)
}
fn rsk(word: &[usize]) -> (Rows, Rows) {
    let mut p = vec![];
    let mut q: Rows = vec![];
    for (i, v) in word.iter().enumerate() {
        let (r, _) = row_insert(&mut p, (*v, *v));
        if r == q.len() {
            q.push(vec![])
        }
        q[r].push((i + 1, i + 1));
    }
    (p, q)
}
fn lis(word: &[usize]) -> Vec<usize> {
    let mut paths: Vec<Vec<usize>> = vec![];
    for (i, v) in word.iter().enumerate() {
        let mut p = (0..i)
            .filter(|j| word[*j] < *v)
            .map(|j| paths[j].clone())
            .max_by_key(Vec::len)
            .unwrap_or_default();
        p.push(*v);
        paths.push(p)
    }
    paths.into_iter().max_by_key(Vec::len).unwrap_or_default()
}
fn lengths(cells: &[Cell]) -> Vec<usize> {
    let mut result = vec![0; cells.iter().map(|c| c.r + 1).max().unwrap_or(0)];
    for c in cells {
        result[c.r] = result[c.r].max(c.c + 1)
    }
    result
}
fn corners(lens: &[usize]) -> Vec<(usize, usize)> {
    (0..=lens.len())
        .filter(|r| *r == 0 || lens[r - 1] > *lens.get(*r).unwrap_or(&0))
        .map(|r| (r, *lens.get(r).unwrap_or(&0)))
        .collect()
}
fn fillkey(cells: &[Cell]) -> String {
    let mut cells = cells.to_vec();
    cells.sort_by_key(|c| (c.r, c.c));
    cells
        .iter()
        .map(|c| c.value.to_string())
        .collect::<Vec<_>>()
        .join(",")
}
fn violations(cells: &[Cell], hole: Option<(usize, usize)>) -> Vec<(usize, usize, bool)> {
    let mut v = vec![];
    for a in cells {
        for (dr, dc) in [(0, 1), (1, 0)] {
            let mut next = (a.r + dr, a.c + dc);
            if hole == Some(next) {
                next = (a.r + 2 * dr, a.c + 2 * dc)
            }
            if let Some(b) = cells.iter().find(|b| (b.r, b.c) == next) {
                if b.value < a.value || (dr == 1 && b.value == a.value) {
                    v.push((a.r, a.c, dr == 1))
                }
            }
        }
    }
    v
}
fn slide(s: &State, i: usize) -> Option<State> {
    let (r, c) = s.hole?;
    let b = &s.skew[i];
    if b.r.abs_diff(r) + b.c.abs_diff(c) != 1 {
        return None;
    }
    let mut t = s.clone();
    t.hole = Some((b.r, b.c));
    t.skew[i].r = r;
    t.skew[i].c = c;
    if violations(&t.skew, t.hole).is_empty() {
        Some(t)
    } else {
        None
    }
}
fn outer(s: &State) -> bool {
    s.hole.is_some_and(|(r, c)| {
        !s.skew
            .iter()
            .any(|b| (b.r, b.c) == (r + 1, c) || (b.r, b.c) == (r, c + 1))
    })
}
fn reading(s: &State) -> Vec<usize> {
    let mut boxes = s.skew.clone();
    boxes.sort_by_key(|b| (std::cmp::Reverse(b.r), b.c));
    boxes.iter().map(|b| b.value).collect()
}
fn tile(
    scene: &mut Scene,
    id: impl Into<String>,
    r: usize,
    c: usize,
    x: f64,
    y: f64,
    value: impl ToString,
    color: Color,
) {
    let mut o = Object::rect(
        id,
        Rect::new(
            x + c as f64 * S,
            y + r as f64 * S,
            x + (c + 1) as f64 * S,
            y + (r + 1) as f64 * S,
        ),
        0.,
        color,
    )
    .label(
        value.to_string(),
        (S / 2., S / 2.),
        17.,
        hex_color("111111"),
    )
    .text_anchor(TextAnchor::Middle)
    .text_baseline(TextBaseline::Central);
    if let Shape::Rect { stroke, .. } = &mut o.shapes[0] {
        *stroke = Some((hex_color("333333"), 1.5));
    }
    scene.push(o);
}
fn draw_rows(scene: &mut Scene, rows: &Rows, prefix: &str, x: f64, y: f64, color: Color) {
    for (r, row) in rows.iter().enumerate() {
        for (c, (id, v)) in row.iter().enumerate() {
            tile(scene, format!("{prefix}-{id}"), r, c, x, y, v, color)
        }
    }
}
fn caption(scene: &mut Scene, id: &str, x: f64, y: f64, text: impl Into<String>, color: Color) {
    scene.push(baseline_text(id, (x, y), text, 13., color));
}
fn figure_scene(s: &State, section: usize, dragged: Option<&str>) -> Scene {
    let mut scene = Scene::new(492., 250.);
    let blue = hex_color("dbeafe");
    let amber = hex_color("fef3c7");
    let gray = hex_color("f3f4f6");
    let ink = hex_color("374151");
    match section {
        0 => {
            scene.width = 260.;
            scene.height = 204.;
            let lens = lengths(&s.shape);
            if let Some(i) = dragged
                .and_then(|id| id.strip_prefix("shape-"))
                .and_then(|x| x.parse::<usize>().ok())
            {
                let mut without = s.shape.clone();
                without.remove(i);
                for (r, c) in corners(&lengths(&without)) {
                    if (r, c) == (s.shape[i].r, s.shape[i].c) {
                        continue;
                    }
                    let mut o = Object::rect(
                        format!("target-{r}-{c}"),
                        Rect::new(
                            10. + c as f64 * S,
                            10. + r as f64 * S,
                            10. + (c + 1) as f64 * S,
                            10. + (r + 1) as f64 * S,
                        ),
                        0.,
                        [0; 4],
                    )
                    .dash(vec![4., 3.])
                    .decorative();
                    if let Shape::Rect { stroke, .. } = &mut o.shapes[0] {
                        *stroke = Some((hex_color("9ca3af"), 1.));
                    }
                    scene.push(o);
                }
            }
            for (i, b) in s.shape.iter().enumerate() {
                let removable =
                    b.c + 1 == lens[b.r] && lens.get(b.r + 1).copied().unwrap_or(0) < lens[b.r];
                tile(
                    &mut scene,
                    format!("shape-{i}"),
                    b.r,
                    b.c,
                    10.,
                    10.,
                    "",
                    if removable { amber } else { gray },
                );
            }
            caption(
                &mut scene,
                "shape-caption",
                10.,
                196.,
                format!(
                    "λ = ({}) — a partition of {}",
                    lens.iter()
                        .map(usize::to_string)
                        .collect::<Vec<_>>()
                        .join(", "),
                    s.shape.len()
                ),
                ink,
            );
        }
        1 => {
            scene.width = 402.;
            scene.height = 234.;
            let bad = violations(&s.fill, None);
            let current = fillkey(&s.fill);
            let complete = s.found.len() == 16;
            for (i, b) in s.fill.iter().enumerate() {
                tile(
                    &mut scene,
                    format!("fill-{i}"),
                    b.r,
                    b.c,
                    10.,
                    10.,
                    b.value,
                    blue,
                );
            }
            for (i, (r, c, down)) in bad.iter().enumerate() {
                let x = 10. + *c as f64 * S;
                let y = 10. + *r as f64 * S;
                scene.push(
                    Object::line(
                        format!("viol-{i}"),
                        if *down {
                            (x + 2., y + S)
                        } else {
                            (x + S, y + 2.)
                        },
                        if *down {
                            (x + S - 2., y + S)
                        } else {
                            (x + S, y + S - 2.)
                        },
                        hex_color("dc2626"),
                        5.,
                    )
                    .line_cap(LineCap::Round),
                );
            }
            caption(
                &mut scene,
                "fill-caption",
                10.,
                138.,
                if bad.is_empty() {
                    "✓ a standard Young tableau".into()
                } else {
                    format!(
                        "✗ {} broken rule{}",
                        bad.len(),
                        if bad.len() == 1 { "" } else { "s" }
                    )
                },
                hex_color(if bad.is_empty() { "15803d" } else { "dc2626" }),
            );
            for k in 0..16 {
                let key = s.found.get(k);
                let values = key
                    .map(|s| s.split(',').collect::<Vec<_>>())
                    .unwrap_or_else(|| vec![""; 6]);
                let is_current = key.is_some_and(|k| *k == current) && bad.is_empty();
                for (i, (r, c)) in [(0, 0), (0, 1), (0, 2), (1, 0), (1, 1), (2, 0)]
                    .iter()
                    .enumerate()
                {
                    let x = 198. + (k % 4) as f64 * 51. + *c as f64 * 13.;
                    let y = 10. + (k / 4) as f64 * 51. + *r as f64 * 13.;
                    let color = if key.is_none() {
                        WHITE
                    } else if complete {
                        hex_color(if is_current { "fde68a" } else { "fef3c7" })
                    } else {
                        hex_color(if is_current { "bfdbfe" } else { "dbeafe" })
                    };
                    let mut o = Object::rect(
                        format!("slot-{k}-{i}"),
                        Rect::new(x, y, x + 13., y + 13.),
                        0.,
                        color,
                    )
                    .label(values[i], (6.5, 6.5), 8., hex_color("111111"))
                    .text_anchor(TextAnchor::Middle)
                    .text_baseline(TextBaseline::Central)
                    .decorative();
                    if let Shape::Rect { stroke, .. } = &mut o.shapes[0] {
                        *stroke = Some((
                            hex_color(if key.is_none() {
                                "d1d5db"
                            } else if complete {
                                "b45309"
                            } else {
                                "333333"
                            }),
                            0.8,
                        ));
                    }
                    if key.is_none() {
                        o = o.dash(vec![2., 2.]);
                    }
                    scene.push(o);
                }
            }
            caption(
                &mut scene,
                "fill-count",
                198.,
                224.,
                if complete {
                    "🎉 All 16 standard Young tableaux of shape (3, 2, 1)!".into()
                } else {
                    format!("{} of 16 found", s.found.len())
                },
                hex_color(if complete { "b45309" } else { "6b7280" }),
            );
        }
        2 => {
            let num = s.rows.len().max(s.held.map(|x| x.2 + 1).unwrap_or(0));
            let qy = num as f64 * S + 44.;
            scene.height = qy + 110.;
            scene.width = 308.;
            draw_rows(&mut scene, &s.rows, "step", 52., 20., blue);
            caption(
                &mut scene,
                "step-tableau",
                52.,
                14.,
                "tableau",
                hex_color("6b7280"),
            );
            if let Some((id, v, r)) = s.held {
                tile(&mut scene, format!("step-{id}"), r, 0, 0., 20., v, amber);
                caption(
                    &mut scene,
                    "step-row",
                    0.,
                    20. + r as f64 * S - 5.,
                    format!("row {}", r + 1),
                    hex_color("92400e"),
                );
                let c = s
                    .rows
                    .get(r)
                    .map(|row| row.iter().position(|x| x.1 > v).unwrap_or(row.len()))
                    .unwrap_or(0);
                let mut o = Object::rect(
                    "step-target",
                    Rect::new(
                        52. + c as f64 * S,
                        20. + r as f64 * S,
                        52. + (c + 1) as f64 * S,
                        20. + (r + 1) as f64 * S,
                    ),
                    2.,
                    [0; 4],
                )
                .decorative();
                if let Shape::Rect { stroke, .. } = &mut o.shapes[0] {
                    *stroke = Some((hex_color("dc2626"), 2.));
                }
                if c >= s.rows.get(r).map(Vec::len).unwrap_or(0) {
                    o = o.dash(vec![4., 3.]);
                } else {
                    o = o.shadow(Shadow {
                        offset: Vec2::ZERO,
                        blur: 3.,
                        color: [220, 38, 38, 85],
                    });
                }
                scene.push(o);
            }
            for (i, (id, v)) in s.queue.iter().enumerate() {
                tile(
                    &mut scene,
                    format!("queue-{id}"),
                    0,
                    0,
                    52. + i as f64 * 42.,
                    20. + qy,
                    v,
                    amber,
                );
                if i > 0 {
                    scene.objects.last_mut().unwrap().opacity = 0.45;
                }
            }
            caption(
                &mut scene,
                "queue-label",
                52.,
                qy + 14.,
                "still to insert",
                hex_color("6b7280"),
            );
            caption(&mut scene, "step-message", 0., qy + 84., &s.message, ink);
        }
        3 => {
            scene.height = 254.;
            caption(
                &mut scene,
                "rsk-label-w",
                10.,
                8.,
                "permutation – reorder it",
                ink,
            );
            for (i, v) in s.word.iter().enumerate() {
                tile(
                    &mut scene,
                    format!("word-{v}"),
                    0,
                    0,
                    10. + i as f64 * 42.,
                    14.,
                    v,
                    if i < s.k { amber } else { WHITE },
                );
            }
            let mut cursor = Object::rect(
                "cursor",
                Rect::new(
                    10. + s.k as f64 * 42. - 4.5,
                    8.,
                    10. + s.k as f64 * 42. - 1.5,
                    56.,
                ),
                0.,
                hex_color("dc2626"),
            );
            cursor.hit = Some(Hit::Rect(Rect::new(-4.5, -2., 7.5, 50.)));
            let mut triangle = BezPath::new();
            triangle.move_to((-4.5, -4.));
            triangle.line_to((7.5, -4.));
            triangle.line_to((1.5, 2.));
            triangle.close_path();
            cursor.shapes.push(Shape::Path {
                path: triangle,
                fill: Some(hex_color("dc2626")),
                stroke: None,
            });
            scene.push(cursor);
            let (p, q) = rsk(&s.word[..s.k]);
            caption(
                &mut scene,
                "rsk-label-p",
                10.,
                88.,
                "P – insertion tableau",
                ink,
            );
            caption(
                &mut scene,
                "rsk-label-q",
                256.,
                88.,
                "Q – recording tableau",
                ink,
            );
            draw_rows(&mut scene, &p, "p", 10., 96., blue);
            draw_rows(&mut scene, &q, "q", 256., 96., hex_color("dcfce7"));
            if s.k > 0 {
                let lis = lis(&s.word[..s.k]);
                caption(
                    &mut scene,
                    "rsk-lis",
                    10.,
                    254.,
                    format!(
                        "longest increasing subsequence: {}  (length {} = length of row 1)",
                        lis.iter()
                            .map(usize::to_string)
                            .collect::<Vec<_>>()
                            .join(" "),
                        lis.len()
                    ),
                    ink,
                );
            }
        }
        _ => {
            scene.width = 452.;
            scene.height = 214.;
            for (r, len) in s.mu.iter().enumerate() {
                for c in 0..*len {
                    scene.push(
                        Object::rect(
                            format!("mu-{r}-{c}"),
                            Rect::new(
                                10. + c as f64 * S,
                                10. + r as f64 * S,
                                10. + (c + 1) as f64 * S,
                                10. + (r + 1) as f64 * S,
                            ),
                            0.,
                            gray,
                        )
                        .decorative(),
                    );
                }
                if s.hole.is_none() && *len > 0 && s.mu.get(r + 1).copied().unwrap_or(0) < *len {
                    tile(
                        &mut scene,
                        format!("open-{r}"),
                        r,
                        len - 1,
                        10.,
                        10.,
                        "+",
                        gray,
                    );
                    let o = scene.objects.last_mut().unwrap();
                    o.stroke_style.dash = vec![4., 3.];
                }
            }
            for (i, b) in s.skew.iter().enumerate() {
                tile(
                    &mut scene,
                    format!("box-{i}"),
                    b.r,
                    b.c,
                    10.,
                    10.,
                    b.value,
                    if slide(s, i).is_some() { amber } else { WHITE },
                );
            }
            if let Some((r, c)) = s.hole {
                tile(
                    &mut scene,
                    "hole",
                    r,
                    c,
                    10.,
                    10.,
                    if outer(s) { "×" } else { "" },
                    hex_color("fde68a"),
                );
                scene.objects.last_mut().unwrap().stroke_style.dash = vec![4., 3.];
            }
            let word = reading(s);
            let (p, _) = rsk(&word);
            caption(
                &mut scene,
                "rw-label",
                226.,
                8.,
                "reading word (rows bottom to top)",
                ink,
            );
            caption(
                &mut scene,
                "rw-word",
                226.,
                26.,
                word.iter()
                    .map(usize::to_string)
                    .collect::<Vec<_>>()
                    .join(" "),
                hex_color("111111"),
            );
            caption(
                &mut scene,
                "rw-p-label",
                226.,
                50.,
                "its insertion tableau P",
                ink,
            );
            draw_rows(&mut scene, &p, "reading", 226., 56., blue);
            let rectified = s.hole.is_none() && s.mu.iter().all(|n| *n == 0);
            if rectified || s.hole.is_none() || outer(s) {
                caption(
                    &mut scene,
                    "jdt-hint",
                    10.,
                    179.2,
                    if rectified {
                        "rectified – a straight tableau!"
                    } else if outer(s) {
                        "at an outer corner – click × to close"
                    } else {
                        "click a + to open a hole"
                    },
                    hex_color(if rectified { "15803d" } else { "374151" }),
                );
            }
        }
    }
    for o in &mut scene.objects {
        if o.id.starts_with("open-") || o.id == "hole" {
            o.click_handler = true;
        }
    }
    scene
}
impl Example for Tableaux {
    type State = State;
    fn initial(&self) -> State {
        let fill = vec![
            Cell {
                r: 0,
                c: 0,
                value: 1,
            },
            Cell {
                r: 0,
                c: 1,
                value: 2,
            },
            Cell {
                r: 0,
                c: 2,
                value: 4,
            },
            Cell {
                r: 1,
                c: 0,
                value: 3,
            },
            Cell {
                r: 1,
                c: 1,
                value: 5,
            },
            Cell {
                r: 2,
                c: 0,
                value: 6,
            },
        ];
        State {
            section: 0,
            shape: vec![(0, 0), (0, 1), (0, 2), (1, 0), (1, 1), (2, 0), (2, 1)]
                .into_iter()
                .enumerate()
                .map(|(i, (r, c))| Cell { r, c, value: i })
                .collect(),
            found: vec![fillkey(&fill)],
            fill,
            rows: vec![
                vec![(1, 1), (2, 2), (3, 2), (4, 5)],
                vec![(5, 3), (6, 4)],
                vec![(7, 6)],
            ],
            queue: vec![(8, 1), (9, 4), (10, 2), (11, 6), (12, 3)],
            held: None,
            message: "Press “insert next number” to begin.".into(),
            word: vec![4, 1, 6, 2, 5, 3],
            k: 6,
            mu: vec![2, 1],
            hole: None,
            skew: vec![
                (0, 2, 1),
                (0, 3, 3),
                (1, 1, 2),
                (1, 2, 5),
                (2, 0, 4),
                (2, 1, 6),
                (2, 2, 8),
                (3, 0, 7),
            ]
            .into_iter()
            .map(|(r, c, value)| Cell { r, c, value })
            .collect(),
        }
    }
    fn scene(&self, s: &State) -> Scene {
        self.scene_dragging(s, None)
    }
    fn scene_dragging(&self, s: &State, id: Option<&str>) -> Scene {
        let article: Vec<serde_json::Value> =
            serde_json::from_str(include_str!("tableaux-article.json")).unwrap();
        let mut page = Scene::new(650., 1.).scrolling();
        let mut y = 0.;
        let mut figure_border = None;
        for (i, b) in article.iter().enumerate() {
            let text = b["text"].as_str().unwrap();
            match b["kind"].as_str().unwrap() {
                "figure" => {
                    y += 16.;
                    let section: usize = text.parse().unwrap();
                    let mut figure = figure_scene(s, section, id);
                    let top = y + if section == 2 { 56. } else { 24. };
                    figure_border = Some((page.objects.len(), y));
                    page.push(
                        Object::rect(
                            format!("figure-border-{section}"),
                            Rect::new(0., y, 4., top + figure.height),
                            0.,
                            hex_color("fde68a"),
                        )
                        .decorative(),
                    );
                    for o in &mut figure.objects {
                        o.transform = Affine::translate((20., top)) * o.transform;
                    }
                    page.objects.extend(figure.objects);
                    let reset = [
                        "reset-shape",
                        "reset-fill",
                        "reset-step",
                        "reset-rsk",
                        "reset-jdt",
                    ][section];
                    if section == 2 {
                        page.inline_control("insert", Rect::new(20., y, 172., y + 28.))
                            .background(hex_color("fde68a"))
                            .font_size(14.);
                        page.inline_control("step", Rect::new(180., y, 287., y + 28.))
                            .background(hex_color("bfdbfe"))
                            .font_size(14.);
                        page.inline_control(reset, Rect::new(295., y, 353., y + 28.))
                            .background(hex_color("e2e8f0"))
                            .font_size(12.);
                    } else {
                        page.inline_control(reset, Rect::new(28., y, 74., y + 20.))
                            .background(hex_color("e2e8f0"))
                            .font_size(12.);
                    }
                    y = top + figure.height + 4.;
                }
                "heading" => {
                    y += 32.;
                    page.push(
                        Object::text(
                            format!("article-{i}"),
                            (0., y),
                            text,
                            20.,
                            hex_color("111827"),
                        )
                        .font_weight(600),
                    );
                    y += 38.;
                }
                kind => {
                    let caption = kind == "caption";
                    let list_item = kind == "list-item";
                    let size = if caption { 13. } else { 15. };
                    let leading = if caption { 19.5 } else { 24.375 };
                    let width = crate::text_layout::measure_text("0", size, &TextStyle::default())
                        .advance
                        * 65.;
                    let runs: Vec<ArticleRun> = serde_json::from_value(b["runs"].clone()).unwrap();
                    let lines =
                        article_lines(&runs, size, width - if list_item { 24. } else { 0. });
                    if list_item {
                        page.push(
                            Object::text(
                                format!("article-{i}-number"),
                                (18., y),
                                format!("{}.", b["number"].as_u64().unwrap()),
                                size,
                                hex_color("1f2937"),
                            )
                            .text_anchor(TextAnchor::End),
                        );
                    }
                    for (j, line) in lines.iter().enumerate() {
                        let mut x = if caption {
                            20.
                        } else if list_item {
                            24.
                        } else {
                            0.
                        };
                        for (k, run) in line.iter().enumerate() {
                            let style = run.style();
                            page.push(
                                Object::text(
                                    format!("article-{i}-{j}-{k}"),
                                    (x, y),
                                    &run.text,
                                    size,
                                    hex_color(&run.color),
                                )
                                .text_style(style.clone()),
                            );
                            x += crate::text_layout::measure_text(&run.text, size, &style).advance;
                        }
                        y += leading;
                    }
                    if caption {
                        if let Some((index, start)) = figure_border.take() {
                            if let Shape::Rect { bounds, .. } = &mut page.objects[index].shapes[0] {
                                bounds.y1 = y - start;
                            }
                        }
                        y += 16.;
                    } else if !list_item || b["number"].as_u64() == Some(3) {
                        y += 12.;
                    }
                }
            }
        }
        page.height = y;
        page
    }
    fn drag(&self, s: &State, id: &str, _: Modifiers) -> Option<DragSpec<'static, State>> {
        if let Some(i) = id
            .strip_prefix("shape-")
            .and_then(|i| i.parse::<usize>().ok())
        {
            let b = &s.shape[i];
            let lens = lengths(&s.shape);
            if b.c + 1 != lens[b.r] || lens.get(b.r + 1).copied().unwrap_or(0) >= lens[b.r] {
                return None;
            }
            let mut base = s.shape.clone();
            base.remove(i);
            let choices = corners(&lengths(&base))
                .into_iter()
                .map(|(r, c)| {
                    let mut t = s.clone();
                    t.shape[i].r = r;
                    t.shape[i].c = c;
                    t
                })
                .collect::<Vec<_>>();
            return Some(closest(choices).with_floating());
        }
        if let Some(i) = id
            .strip_prefix("fill-")
            .and_then(|i| i.parse::<usize>().ok())
        {
            let choices = (0..s.fill.len())
                .map(|j| {
                    let mut t = s.clone();
                    let a = t.fill[i].clone();
                    t.fill[i].r = t.fill[j].r;
                    t.fill[i].c = t.fill[j].c;
                    t.fill[j].r = a.r;
                    t.fill[j].c = a.c;
                    t
                })
                .collect::<Vec<_>>();
            return Some(closest(choices).with_floating().on_drop(|s: &State| {
                let mut t = s.clone();
                if violations(&t.fill, None).is_empty() {
                    let key = fillkey(&t.fill);
                    if !t.found.contains(&key) {
                        t.found.push(key);
                    }
                }
                t
            }));
        }
        if id == "cursor" {
            return Some(closest(
                (0..=s.word.len())
                    .map(|k| State { k, ..s.clone() })
                    .collect::<Vec<_>>(),
            ));
        }
        if let Some(v) = id
            .strip_prefix("word-")
            .and_then(|i| i.parse::<usize>().ok())
        {
            let rest = s
                .word
                .iter()
                .filter(|x| **x != v)
                .copied()
                .collect::<Vec<_>>();
            return Some(
                closest(
                    (0..s.word.len())
                        .map(|i| {
                            let mut t = s.clone();
                            t.word = rest.clone();
                            t.word.insert(i, v);
                            t
                        })
                        .collect::<Vec<_>>(),
                )
                .when_far(s.clone(), 50.)
                .with_floating(),
            );
        }
        if id == "hole" {
            let choices = (0..s.skew.len())
                .filter_map(|i| slide(s, i))
                .map(|t| between([s.clone(), t]))
                .collect::<Vec<_>>();
            if !choices.is_empty() {
                return Some(closest(choices).with_snap_radius_options(
                    10.,
                    dragology::SnapOptions {
                        chain: true,
                        transition: None,
                    },
                ));
            }
        }
        if let Some(i) = id
            .strip_prefix("box-")
            .and_then(|i| i.parse::<usize>().ok())
        {
            return slide(s, i).map(|t| between([s.clone(), t]));
        }
        None
    }
    fn click(&self, s: &State, id: &str, _: Point, _: Modifiers, _: usize) -> Option<State> {
        if let Some(r) = id
            .strip_prefix("open-")
            .and_then(|i| i.parse::<usize>().ok())
        {
            let mut t = s.clone();
            t.mu[r] -= 1;
            t.hole = Some((r, t.mu[r]));
            return Some(t);
        }
        if id == "hole" && outer(s) {
            return Some(State {
                hole: None,
                ..s.clone()
            });
        }
        None
    }
    fn action_transition(&self, _: &State, id: &str) -> Option<dragology::Transition> {
        Some(if id == "step" || id == "insert" {
            dragology::Transition {
                duration: std::time::Duration::from_millis(450),
                easing: dragology::Easing::CubicBezier([0.2, 0.8, 0.2, 1.]),
            }
        } else {
            transition(150)
        })
    }
    fn controls(&self, s: &State) -> Vec<Control> {
        vec![
            Control::button("reset-shape", "reset"),
            Control::button("reset-fill", "reset"),
            Control::button("reset-step", "reset"),
            Control::button("reset-rsk", "reset"),
            Control::button("reset-jdt", "reset"),
            Control::action_button(
                "insert",
                "insert next number",
                s.held.is_none() && !s.queue.is_empty(),
                false,
            ),
            Control::action_button("step", "do this step", s.held.is_some(), false),
        ]
    }
    fn action(&self, s: &State, id: &str, v: Value) -> Option<State> {
        let mut t = s.clone();
        if let Some(section) = [
            "reset-shape",
            "reset-fill",
            "reset-step",
            "reset-rsk",
            "reset-jdt",
        ]
        .iter()
        .position(|x| *x == id)
        {
            t.section = section;
            return self.action(&t, "reset-section", Value::Trigger);
        }
        if id == "insert" {
            return if s.held.is_none() && !s.queue.is_empty() {
                self.action(s, "next", Value::Trigger)
            } else {
                None
            };
        }
        if id == "step" {
            return if s.held.is_some() {
                self.action(s, "next", Value::Trigger)
            } else {
                None
            };
        }
        match (id, v) {
            ("section", Value::Index(i)) if i < 5 => t.section = i,
            ("reset-section", _) => {
                let initial = self.initial();
                match t.section {
                    0 => t.shape = initial.shape,
                    1 => {
                        t.fill = initial.fill;
                        t.found = initial.found;
                    }
                    2 => {
                        t.rows = initial.rows;
                        t.queue = initial.queue;
                        t.held = initial.held;
                        t.message = initial.message;
                    }
                    3 => {
                        t.word = initial.word;
                        t.k = initial.k;
                    }
                    _ => {
                        t.mu = initial.mu;
                        t.skew = initial.skew;
                        t.hole = initial.hole;
                    }
                }
            }
            ("next", _) => {
                if let Some((id, v, r)) = t.held {
                    if r == t.rows.len() {
                        t.rows.push(vec![])
                    }
                    if let Some(c) = t.rows[r].iter().position(|x| x.1 > v) {
                        let bumped = std::mem::replace(&mut t.rows[r][c], (id, v));
                        t.held = Some((bumped.0, bumped.1, r + 1));
                        t.message = format!(
                            "{} is the leftmost entry of row {} bigger than {v}: {v} takes its place and bumps {} down to row {}.",
                            bumped.1,
                            r + 1,
                            bumped.1,
                            r + 2
                        );
                    } else {
                        t.rows[r].push((id, v));
                        t.held = None;
                        t.message = if t.rows[r].len() == 1 {
                            format!("Row {} didn’t exist, so {v} starts a new row. Done.", r + 1)
                        } else {
                            format!(
                                "Nothing in row {} is bigger than {v}, so it goes at the end. Done.",
                                r + 1
                            )
                        };
                    }
                } else if !t.queue.is_empty() {
                    let (id, v) = t.queue.remove(0);
                    t.held = Some((id, v, 0));
                    t.message = format!("Insert {v}. Start at row 1.");
                }
            }
            _ => return None,
        }
        Some(t)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn article_figures_operate_independently_and_keep_discovery_order() {
        let state = Tableaux.initial();
        let page = Tableaux.scene(&state);
        for id in ["shape-2", "fill-0", "word-4", "open-1"] {
            assert!(
                page.object(id).is_some(),
                "missing simultaneous figure {id}"
            );
        }
        let started = Tableaux.action(&state, "insert", Value::Trigger).unwrap();
        let stepped = Tableaux.action(&started, "step", Value::Trigger).unwrap();
        assert_ne!(stepped.rows, state.rows);
        assert_eq!(stepped.word, state.word);
        assert_eq!(
            stepped.shape.iter().map(|c| (c.r, c.c)).collect::<Vec<_>>(),
            state.shape.iter().map(|c| (c.r, c.c)).collect::<Vec<_>>()
        );
        let reset = Tableaux
            .action(&stepped, "reset-fill", Value::Trigger)
            .unwrap();
        assert_eq!(reset.rows, stepped.rows);
        let reset_step = Tableaux
            .action(&reset, "reset-step", Value::Trigger)
            .unwrap();
        assert_eq!(reset_step.rows, state.rows);
        assert_eq!(reset_step.found, state.found);
    }
    #[test]
    fn insertion_preserves_semistandard_and_lis() {
        for word in [
            vec![4, 1, 6, 2, 5, 3],
            vec![1, 1, 2, 2, 1],
            vec![6, 5, 4, 3, 2, 1],
        ] {
            let (p, q) = rsk(&word);
            let cells = p
                .iter()
                .enumerate()
                .flat_map(|(r, row)| {
                    row.iter()
                        .enumerate()
                        .map(move |(c, e)| Cell { r, c, value: e.1 })
                })
                .collect::<Vec<_>>();
            assert!(violations(&cells, None).is_empty());
            assert_eq!(
                p.iter().map(Vec::len).collect::<Vec<_>>(),
                q.iter().map(Vec::len).collect::<Vec<_>>()
            );
            assert_eq!(cells.len(), word.len());
            if word.iter().collect::<BTreeSet<_>>().len() == word.len() {
                assert_eq!(p[0].len(), lis(&word).len());
            }
        }
    }
    #[test]
    fn jeu_de_taquin_preserves_reading_tableau() {
        let mut s = Tableaux.initial();
        s.mu[1] -= 1;
        s.hole = Some((1, 0));
        let expected = rsk(&reading(&s)).0;
        for _ in 0..8 {
            if outer(&s) {
                break;
            }
            let next = (0..s.skew.len())
                .filter_map(|i| slide(&s, i))
                .find(|t| {
                    let (a, b) = s.hole.unwrap();
                    let (c, d) = t.hole.unwrap();
                    c + d > a + b
                })
                .unwrap();
            assert_eq!(rsk(&reading(&next)).0, expected);
            s = next;
        }
        assert!(outer(&s));
    }
}
