//! Application demos ported from the MIT-licensed Dragology gallery.
//! Copyright (c) 2025 Dragology contributors.
use crate::*;

mod gantt;
mod git;
mod layout;
mod pivot;
mod regex;
mod stock;
mod tableaux;

pub fn entries() -> Vec<Entry> {
    vec![
        Entry::new(
            "regex-nfa-sync",
            "Regex and automata",
            "Edit the regex tree while its Thompson NFA, matching words, and DFA update together.",
            || demo(regex::RegexDemo),
        ),
        Entry::new(
            "gantt",
            "Gantt chart",
            "Slide tasks, resize their ends, move lane grips, and drop a task onto a prerequisite. Click dependency arrows to remove them.",
            || demo(gantt::Gantt),
        ),
        Entry::new(
            "tableaux",
            "Young tableaux",
            "Explore partitions, fillings, row insertion, RSK, and jeu de taquin.",
            || demo(tableaux::Tableaux),
        ),
        Entry::new(
            "layout-editor",
            "Layout editor",
            "Reparent boxes and resize leaves. Change flex direction, gap, padding, and grid columns with the handles.",
            || demo(layout::Layout),
        ),
        Entry::new(
            "git-graph-surgery",
            "Git graph surgery",
            "Rebase and reorder commits, move branch labels, or hold Alt to cherry-pick onto another branch.",
            || demo(git::GitGraph),
        ),
        Entry::new(
            "pivot-table",
            "Pivot table",
            "Move dimension and measure chips between zones. The data is aggregated again for every preview.",
            || demo(pivot::Pivot),
        ),
        Entry::new(
            "stock-and-flow",
            "Stock and flow",
            "Edit stocks, flows, and dependencies while the simulated trajectories and parameter response update.",
            || demo(stock::StockFlow),
        ),
    ]
}

const WHITE: Color = [255; 4];

// Preserve the source SVG baseline explicitly in the shared native text contract.
fn baseline_text(
    id: impl Into<String>,
    origin: impl Into<Point>,
    text: impl Into<String>,
    size: f64,
    color: Color,
) -> Object {
    Object::text(id, origin, text, size, color).text_baseline(TextBaseline::Alphabetic)
}
trait BaselineLabel {
    fn baseline_label(
        self,
        text: impl Into<String>,
        origin: impl Into<Point>,
        size: f64,
        color: Color,
    ) -> Self;
}
impl BaselineLabel for Object {
    fn baseline_label(
        self,
        text: impl Into<String>,
        origin: impl Into<Point>,
        size: f64,
        color: Color,
    ) -> Self {
        self.label(text, origin, size, color)
            .text_baseline(TextBaseline::Alphabetic)
    }
}

fn hex_color(hex: &str) -> Color {
    let value = u32::from_str_radix(hex.trim_start_matches('#'), 16).expect("source hex color");
    [(value >> 16) as u8, (value >> 8) as u8, value as u8, 255]
}
fn dashed_line(
    id: impl Into<String>,
    from: Point,
    to: Point,
    color: Color,
    width: f64,
    dash: f64,
    gap: f64,
) -> Object {
    let delta = to - from;
    let distance = delta.hypot();
    let mut path = BezPath::new();
    if distance > 0. {
        let direction = delta / distance;
        let mut at = 0.;
        while at < distance {
            path.move_to(from + direction * at);
            path.line_to(from + direction * (at + dash).min(distance));
            at += dash + gap;
        }
    }
    Object::path(id, path, color, width)
}
fn arrow_tip(
    id: impl Into<String>,
    tip: Point,
    direction: Vec2,
    length: f64,
    color: Color,
) -> Object {
    let direction = if direction.hypot() > 0. {
        direction / direction.hypot()
    } else {
        Vec2::new(1., 0.)
    };
    let side = Vec2::new(-direction.y, direction.x) * (length * (std::f64::consts::PI / 6.).tan());
    let mut path = BezPath::new();
    path.move_to(tip);
    path.line_to(tip - direction * length + side);
    path.line_to(tip - direction * length - side);
    path.close_path();
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
fn transition(ms: u64) -> dragology::Transition {
    dragology::Transition {
        duration: std::time::Duration::from_millis(ms),
        easing: dragology::Easing::CubicOut,
    }
}
