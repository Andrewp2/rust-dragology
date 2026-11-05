use super::tree::*;
#[path = "editor_layout.rs"]
mod editor_layout;
use crate::*;
use dragology::{ClosestOptions, between, closest, closest_with, fixed};
use std::collections::BTreeSet;
fn rule_sets() -> Vec<(&'static str, Vec<Rule>)> {
    vec![
        (
            "Identity",
            vec![rule("(+ (0) #A)", "A"), rule("(+ #A (0))", "A")],
        ),
        ("Identity (reverse)", vec![rule("#A", "(+ (0) A)")]),
        (
            "Commutativity",
            vec![rule("(+ #A #B)", "(+ B A)"), rule("(× #A #B)", "(× B A)")],
        ),
        (
            "Associativity — Pull up op",
            vec![
                rule("(+2 #(+1 A B) C)", "(+1 A (+2 B C))"),
                rule("(+1 A #(+2 B C))", "(+2 (+1 A B) C)"),
                rule("(×2 #(×1 A B) C)", "(×1 A (×2 B C))"),
                rule("(×1 A #(×2 B C))", "(×2 (×1 A B) C)"),
            ],
        ),
        (
            "Associativity — Pull down op",
            vec![
                rule("#(+1 A (+2 B C))", "(+2 (+1 A B) C)"),
                rule("#(+2 (+1 A B) C)", "(+1 A (+2 B C))"),
                rule("#(×1 A (×2 B C))", "(×2 (×1 A B) C)"),
                rule("#(×2 (×1 A B) C)", "(×1 A (×2 B C))"),
            ],
        ),
        (
            "Associativity — Pull up operand",
            vec![
                rule("(+2 (+1 #A B) C)", "(+1 A (+2 B C))"),
                rule("(+1 A (+2 B #C))", "(+2 (+1 A B) C)"),
                rule("(×2 (×1 #A B) C)", "(×1 A (×2 B C))"),
                rule("(×1 A (×2 B #C))", "(×2 (×1 A B) C)"),
            ],
        ),
        (
            "Associativity — Pull down operand",
            vec![
                rule("(+1 #A (+2 B C))", "(+2 (+1 A B) C)"),
                rule("(+2 (+1 A B) #C)", "(+1 A (+2 B C))"),
                rule("(×1 #A (×2 B C))", "(×2 (×1 A B) C)"),
                rule("(×2 (×1 A B) #C)", "(×1 A (×2 B C))"),
            ],
        ),
        (
            "Distributivity — Distribute (drag +)",
            vec![
                rule("(× A #(+ B C))", "(+ (× A B) (× A C))"),
                rule("(× #(+ B C) A)", "(+ (× B A) (× C A))"),
            ],
        ),
        (
            "Distributivity — Distribute (drag operand)",
            vec![
                rule("(× #A (+ B C))", "(+ (× A B) (× A C))"),
                rule("(× (+ B C) #A)", "(+ (× B A) (× C A))"),
            ],
        ),
        (
            "Distributivity — Factor (drag +)",
            vec![
                rule("#(+ (× A B) (× A C))", "(× A (+ B C))"),
                rule("#(+ (× B A) (× C A))", "(× A (+ B C))"),
            ],
        ),
        (
            "Distributivity — Factor (drag operand)",
            vec![
                rule("(+ (× #A B) (× A C))", "(× A (+ B C))"),
                rule("(+ (× A B) (× #A C))", "(× A (+ B C))"),
                rule("(+ (× B #A) (× C A))", "(× A (+ B C))"),
                rule("(+ (× B A) (× C #A))", "(× A (+ B C))"),
            ],
        ),
        (
            "Associativity — Pull op sideways",
            vec![
                rule("(+2 #(+1 A B) C)", "(+2 A #(+1 B C))"),
                rule("(+1 A #(+2 B C))", "(+1 #(+2 A B) C)"),
                rule("(×2 #(×1 A B) C)", "(×2 A #(×1 B C))"),
                rule("(×1 A #(×2 B C))", "(×1 #(×2 A B) C)"),
            ],
        ),
    ]
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Algebra {
    Tree,
    Builder,
    Macro,
}
#[derive(Clone, PartialEq)]
pub struct AlgebraState {
    mode: Algebra,
    trees: Vec<Tree>,
    palette: Vec<Tree>,
    brushes: Vec<Tree>,
    buckets: [bool; 3],
    order: Vec<usize>,
    variadic: BTreeSet<String>,
    expanded: bool,
    recording: Option<Tree>,
    rules: Vec<Rule>,
    selected_rule: usize,
    enabled: [bool; 12],
    emerge: bool,
    force_scale: bool,
    voids: Vec<Tree>,
    voided: Option<Tree>,
    notice: String,
}
fn op_arity(label: &str) -> usize {
    match label {
        "+" | "×" | "→" => 2,
        "-" => 1,
        _ => 0,
    }
}
fn well_formed(t: &Tree) -> bool {
    t.children.len() == op_arity(&t.label) && t.children.iter().all(well_formed)
}
fn atom(id: &str, label: &str) -> Tree {
    Tree::leaf(id, label)
}
fn expression() -> Tree {
    Tree::node(
        "root",
        "+",
        vec![
            Tree::node(
                "r1",
                "+",
                vec![
                    Tree::node(
                        "r11",
                        "+",
                        vec![
                            atom("a", "⛅"),
                            Tree::node("neg", "-", vec![atom("b", "🍄")]),
                        ],
                    ),
                    atom("c", "🍄"),
                ],
            ),
            Tree::node(
                "r2",
                "+",
                vec![
                    Tree::node("r21", "×", vec![atom("d", "🎲"), atom("e", "🦠")]),
                    Tree::node("r22", "×", vec![atom("f", "🎲"), atom("g", "🐝")]),
                ],
            ),
        ],
    )
}
fn macro_expression() -> Tree {
    Tree::node(
        "root",
        "+",
        vec![
            Tree::node("left", "+", vec![atom("sun", "⛅"), atom("mushroom", "🍄")]),
            atom("dice", "🎲"),
        ],
    )
}
fn brush_list(macro_mode: bool) -> Vec<Tree> {
    if macro_mode {
        return ["+", "×", "-", "0", "⛅", "🍄", "🎲", "🦠", "🐝"]
            .iter()
            .enumerate()
            .map(|(i, s)| atom(&format!("brush-{i}"), s))
            .collect();
    }
    [
        "→", "+", "×", "-", "→", "+", "×", "-", "⛅", "🍄", "🎲", "🧊", "🪨", "🐝", "🌕", "🌘",
    ]
    .iter()
    .enumerate()
    .map(|(i, s)| atom(&format!("brush-{i}"), s))
    .collect()
}
fn remove_from(
    trees: &[Tree],
    id: &str,
    holes: bool,
    variadic: &BTreeSet<String>,
) -> Option<(Vec<Tree>, Tree)> {
    for (i, t) in trees.iter().enumerate() {
        if t.id == id {
            let mut xs = trees.to_vec();
            return Some((
                {
                    xs.remove(i);
                    xs
                },
                t.clone(),
            ));
        }
        if let Some(sub) = t.find(id) {
            let mut xs = trees.to_vec();
            let item = sub.clone();
            let parent = t
                .nodes()
                .into_iter()
                .find(|n| n.children.iter().any(|c| c.id == id))?;
            xs[i] = if holes && !variadic.contains(&parent.id) {
                t.replace(id, &atom(&fresh_id("hole"), "◯"))
            } else {
                t.remove(id)?.0
            };
            return Some((xs, item));
        }
    }
    None
}
fn insert_into(t: &Tree, item: &Tree, variadic: &BTreeSet<String>, free: bool) -> Vec<Tree> {
    let mut out = vec![];
    if t.label == "◯" && !free {
        out.push(item.clone());
    }
    if op_arity(&t.label) > 0 && (free || variadic.contains(&t.id)) {
        for i in 0..=t.children.len() {
            let mut n = t.clone();
            n.children.insert(i, item.clone());
            out.push(n);
        }
    }
    for (i, c) in t.children.iter().enumerate() {
        for sub in insert_into(c, item, variadic, free) {
            let mut n = t.clone();
            n.children[i] = sub;
            out.push(n)
        }
    }
    out
}
fn edit_targets(s: &AlgebraState, item: &Tree) -> Vec<AlgebraState> {
    let mut out = vec![];
    let free = s.mode == Algebra::Macro;
    for (i, t) in s.trees.iter().enumerate() {
        for n in insert_into(t, item, &s.variadic, free) {
            let mut a = s.clone();
            a.trees[i] = n;
            out.push(a)
        }
    }
    if s.trees.is_empty() {
        for i in 0..=s.trees.len() {
            let mut a = s.clone();
            a.trees.insert(i, item.clone());
            out.push(a)
        }
    }
    if free || s.expanded {
        for i in 0..=s.palette.len() {
            let mut a = s.clone();
            a.palette.insert(i, item.clone());
            out.push(a)
        }
    }

    out
}
fn rule_description(rule: &Rule) -> String {
    fn text(p: &Pattern, vars: &mut std::collections::BTreeMap<String, String>) -> String {
        let contents = if let Some(label) = &p.label {
            format!(
                "({} {})",
                label,
                p.children
                    .iter()
                    .map(|c| text(c, vars))
                    .collect::<Vec<_>>()
                    .join(" ")
            )
        } else {
            let i = vars.len();
            vars.entry(p.key.clone())
                .or_insert_with(|| ((b'A' + (i % 26) as u8) as char).to_string())
                .clone()
        };
        format!("{}{contents}", if p.trigger { "#" } else { "" })
    }
    let mut vars = std::collections::BTreeMap::new();
    format!(
        "{} → {}",
        text(&rule.from, &mut vars),
        text(&rule.to, &mut vars)
    )
}
fn rule_sample(scene: &mut Scene, rule: &Rule, at: Point, prefix: &str) {
    fn trigger(p: &Pattern) -> Option<&str> {
        if p.trigger {
            Some(&p.key)
        } else {
            p.children.iter().find_map(trigger)
        }
    }
    fn draw(
        scene: &mut Scene,
        p: &Pattern,
        top: bool,
        trigger: Option<&str>,
        mut x: f64,
        y: f64,
        prefix: &str,
        serial: &mut usize,
    ) -> f64 {
        let start = x;
        let first = scene.objects.len();
        fn glyph(
            scene: &mut Scene,
            s: &str,
            x: &mut f64,
            y: f64,
            color: Color,
            bold: bool,
            prefix: &str,
            serial: &mut usize,
        ) {
            let mut o = Object::text(format!("{prefix}-{}", *serial), (*x, y), s, 12., color);
            *serial += 1;
            if bold {
                o = o.font_weight(700);
            }
            scene.push(o);
            *x += s.chars().count() as f64 * 7.2;
        }
        if p.label.is_none() {
            glyph(
                scene,
                &p.key,
                &mut x,
                y,
                [55, 65, 81, 255],
                false,
                prefix,
                serial,
            );
        } else if p.key == "0" {
            glyph(
                scene,
                "0",
                &mut x,
                y,
                [37, 99, 235, 255],
                true,
                prefix,
                serial,
            );
        } else {
            if !top {
                glyph(
                    scene,
                    "(",
                    &mut x,
                    y,
                    [55, 65, 81, 255],
                    false,
                    prefix,
                    serial,
                );
            }
            for (i, c) in p.children.iter().enumerate() {
                if i > 0 {
                    let (op, color) = match p.key.as_str() {
                        "+" | "+1" => (" + ", [220, 38, 38, 255]),
                        "+2" => (" + ", [22, 163, 74, 255]),
                        "×" | "×1" => (" × ", [147, 51, 234, 255]),
                        "×2" => (" × ", [234, 88, 12, 255]),
                        _ => (" ", [55, 65, 81, 255]),
                    };
                    glyph(scene, op, &mut x, y, color, true, prefix, serial);
                }
                x = draw(scene, c, false, trigger, x, y, prefix, serial);
            }
            if !top {
                glyph(
                    scene,
                    ")",
                    &mut x,
                    y,
                    [55, 65, 81, 255],
                    false,
                    prefix,
                    serial,
                );
            }
        }
        if trigger == Some(p.key.as_str()) {
            scene.objects.insert(
                first,
                Object::rect(
                    format!("{prefix}-highlight-{}", *serial),
                    Rect::new(start - 2., y - 2., x + 2., y + 15.),
                    2.,
                    [253, 230, 138, 255],
                )
                .decorative(),
            );
            *serial += 1;
        }
        x
    }
    let trigger = trigger(&rule.from);
    let mut serial = 0;
    let x = draw(
        scene,
        &rule.from,
        true,
        trigger,
        at.x,
        at.y,
        prefix,
        &mut serial,
    );
    scene.push(Object::text(
        format!("{prefix}-arrow"),
        (x + 4., at.y),
        "→",
        12.,
        [55, 65, 81, 255],
    ));
    draw(
        scene,
        &rule.to,
        true,
        trigger,
        x + 24.,
        at.y,
        prefix,
        &mut serial,
    );
}
fn source_controls(scene: &mut Scene, s: &AlgebraState) {
    match s.mode {
        Algebra::Tree => {
            scene.width = 640.;
            scene.presentation = Presentation::Scroll;
            scene.push(
                Object::text(
                    "rules-heading",
                    (336., 12.),
                    "Rewrite rules",
                    12.,
                    [55, 65, 81, 255],
                )
                .font_weight(500),
            );
            let mut y = 40.;
            for (i, (name, rules)) in rule_sets().iter().enumerate() {
                let subtitle = name.split_once(" — ").map(|(_, b)| b);
                scene
                    .inline_control(RULE_IDS[i], Rect::new(336., y, 628., y + 22.))
                    .font_size(12.);
                if let Some(subtitle) = subtitle {
                    scene.push(Object::text(
                        format!("rule-subtitle-{i}"),
                        (360., y + 24.),
                        subtitle,
                        12.,
                        [55, 65, 81, 255],
                    ));
                }
                let sample_y = y + if i == 11 {
                    scene.push(
                        Object::text(
                            "sideways-warning",
                            (360., y + 40.),
                            "(Conflicts with commutativity!)",
                            12.,
                            [239, 68, 68, 255],
                        )
                        .italic(),
                    );
                    60.
                } else if subtitle.is_some() {
                    42.
                } else {
                    24.
                };
                rule_sample(
                    scene,
                    &rules[0],
                    Point::new(360., sample_y),
                    &format!("rule-sample-{i}"),
                );
                y = sample_y + 24.;
            }
            y += 16.;
            scene.push(
                Object::text(
                    "animation-heading",
                    (336., y),
                    "Animation",
                    12.,
                    [55, 65, 81, 255],
                )
                .font_weight(500),
            );
            y += 24.;
            scene
                .inline_control("emerge", Rect::new(336., y, 628., y + 40.))
                .font_size(12.);
            y += 48.;
            scene
                .inline_control("force-scale", Rect::new(336., y, 628., y + 40.))
                .font_size(12.);
            scene.height = 350f64.max(y + 52.);
            scene.objects.insert(
                0,
                Object::rect(
                    "rewrite-options-panel",
                    Rect::new(324., 0., 640., scene.height),
                    4.,
                    [249, 250, 251, 255],
                )
                .decorative(),
            );
        }
        Algebra::Macro => {
            scene.width = 704.;
            scene.presentation = Presentation::Scroll;
            scene.push(
                Object::text(
                    "macro-options-heading",
                    (424., 12.),
                    "Options",
                    12.,
                    [55, 65, 81, 255],
                )
                .font_weight(500),
            );
            let mut y = 36.;
            if s.recording.is_some() {
                scene.push(
                    Object::circle("recording-light", (428., 42.), 4., [147, 51, 234, 255])
                        .decorative(),
                );
                scene.push(
                    Object::text(
                        "recording-label",
                        (440., 36.),
                        "Recording...",
                        12.,
                        [126, 34, 206, 255],
                    )
                    .font_weight(500),
                );
                for (i, line) in [
                    "Freely rearrange the tree. Move nodes",
                    "between operators, add from the toolkit,",
                    "or park in the gutter. When done, stop",
                    "recording to derive a rule.",
                ]
                .iter()
                .enumerate()
                {
                    scene.push(Object::text(
                        format!("macro-instructions-{i}"),
                        (424., 60. + i as f64 * 16.),
                        *line,
                        12.,
                        [107, 114, 128, 255],
                    ));
                }
                y = 136.;
                scene
                    .inline_control("record", Rect::new(424., y, 688., y + 30.))
                    .font_size(12.)
                    .background([147, 51, 234, 255])
                    .foreground([255; 4])
                    .border([147, 51, 234, 255]);
                y += 38.;
                scene
                    .inline_control("cancel", Rect::new(424., y, 688., y + 30.))
                    .font_size(12.);
                y += 46.;
            } else {
                if !s.rules.is_empty() {
                    scene.push(
                        Object::text(
                            "recorded-rules-title",
                            (424., y),
                            format!("Recorded rules ({})", s.rules.len()),
                            12.,
                            [55, 65, 81, 255],
                        )
                        .font_weight(500),
                    );
                    y += 24.;
                    for (i, rule) in s.rules.iter().enumerate() {
                        let label = rule_description(rule);
                        scene.push(
                            Object::text(
                                format!("recorded-rule-{i}"),
                                (424., y),
                                label,
                                12.,
                                [126, 34, 206, 255],
                            )
                            .font_family(FontFamily::Mono),
                        );
                        let mut delete = Object::text(
                            format!("delete-rule-{i}"),
                            (680., y),
                            "×",
                            12.,
                            [248, 113, 113, 255],
                        )
                        .cursor(Cursor::Pointer)
                        .clickable();
                        delete.hit = Some(Hit::Rect(Rect::new(-4., -2., 12., 18.)));
                        scene.push(delete);
                        y += 24.;
                    }
                    y += 12.;
                }
                scene
                    .inline_control("record", Rect::new(424., y, 688., y + 30.))
                    .font_size(12.)
                    .background([147, 51, 234, 255])
                    .foreground([255; 4])
                    .border([147, 51, 234, 255]);
                y += 46.;
            }
            scene
                .inline_control("emerge", Rect::new(424., y, 688., y + 24.))
                .font_size(12.);
            scene.height = 400f64.max(y + 36.);
            scene.objects.insert(
                0,
                Object::rect(
                    "macro-options-panel",
                    Rect::new(412., 0., 704., y + 36.),
                    4.,
                    [249, 250, 251, 255],
                )
                .decorative(),
            );
        }
        Algebra::Builder => {}
    }
}

fn brush_bucket(i: usize) -> usize {
    if i < 4 {
        0
    } else if i < 8 {
        1
    } else {
        2
    }
}
impl Example for Algebra {
    type State = AlgebraState;
    fn initial(&self) -> AlgebraState {
        AlgebraState {
            mode: *self,
            trees: match self {
                Self::Tree => vec![expression()],
                Self::Builder => vec![],
                Self::Macro => vec![macro_expression()],
            },
            palette: vec![],
            brushes: brush_list(*self == Self::Macro),
            buckets: [true, false, true],
            order: vec![0, 1, 2],
            variadic: BTreeSet::new(),
            expanded: false,
            recording: None,
            rules: vec![],
            selected_rule: 0,
            enabled: [
                true, true, true, false, false, false, false, false, false, false, false, false,
            ],
            emerge: true,
            force_scale: false,
            voids: vec![],
            voided: None,
            notice: String::new(),
        }
    }
    fn scene(&self, s: &AlgebraState) -> Scene {
        self.scene_dragging(s, None)
    }
    fn scene_dragging(&self, s: &AlgebraState, id: Option<&str>) -> Scene {
        let mut scene = if *self == Self::Tree {
            fit_nested(&s.trees[0])
        } else {
            editor_layout::scene(s, id)
        };
        if !s.emerge {
            for o in &mut scene.objects {
                o.emerge_from = None;
            }
        }
        if *self == Self::Tree {
            if s.force_scale {
                for o in &mut scene.objects {
                    if o.emerge_mode == EmergeMode::Auto {
                        o.emerge_mode = EmergeMode::Scale;
                    }
                }
            }
            raise_group(&mut scene, id);
        }
        source_controls(&mut scene, s);
        scene
    }
    fn scene_interaction(
        &self,
        s: &AlgebraState,
        dragged: Option<&str>,
        hovered: Option<&str>,
    ) -> Scene {
        let mut scene = self.scene_dragging(s, dragged);
        if *self == Self::Builder {
            if let Some(id) = hovered {
                if let Some(o) = scene.objects.iter_mut().find(|o| o.id == id) {
                    if id == "palette-toggle" {
                        if let Some(Shape::Circle { fill, .. }) = o.shapes.first_mut() {
                            *fill = [51, 51, 51, 255];
                        }
                    } else if let Some(i) = id
                        .strip_prefix("bucket-")
                        .and_then(|i| i.parse::<usize>().ok())
                    {
                        for shape in &mut o.shapes {
                            if let Shape::Text { color, .. } = shape {
                                *color = if s.buckets[i] {
                                    [17, 17, 17, 255]
                                } else {
                                    [153, 153, 153, 255]
                                };
                            }
                        }
                    }
                }
            }
        }
        scene
    }
    fn action_transition(&self, _: &AlgebraState, _: &str) -> Option<dragology::Transition> {
        (*self != Self::Builder).then(dragology::Transition::default)
    }
    fn drag(
        &self,
        s: &AlgebraState,
        id: &str,
        mods: Modifiers,
    ) -> Option<DragSpec<'static, AlgebraState>> {
        if id.starts_with("delete-rule-") {
            return None;
        }
        if *self == Self::Tree || (*self == Self::Macro && s.recording.is_none()) {
            let rules = if *self == Self::Tree {
                rule_sets()
                    .into_iter()
                    .enumerate()
                    .filter(|(i, _)| s.enabled[*i])
                    .flat_map(|(_, (_, r))| r)
                    .collect()
            } else {
                s.rules.clone()
            };
            let candidates = rewrites(&s.trees[0], &rules, id);
            if candidates.is_empty() {
                return Some(between([s.clone()]));
            }
            let mut branches = vec![];
            if *self == Self::Macro {
                branches.push(between([s.clone()]));
            }
            for t in candidates {
                if t.find(id).is_none() {
                    continue;
                }
                let mut a = s.clone();
                a.trees = vec![t];
                branches.push(if *self == Self::Macro {
                    a.palette.clear();
                    fixed(a)
                } else {
                    between([s.clone(), a])
                });
            }
            return Some(if *self == Self::Macro {
                closest(branches)
            } else {
                closest_with(
                    branches,
                    ClosestOptions {
                        lock_past: Some(20.),
                        ..Default::default()
                    },
                )
                .with_snap_radius_options(
                    1.,
                    dragology::SnapOptions {
                        chain: true,
                        ..Default::default()
                    },
                )
            });
        }

        if let Some(b) = id
            .strip_prefix("bucket-")
            .and_then(|i| i.parse::<usize>().ok())
        {
            let old = s.order.iter().position(|i| *i == b)?;
            let mut targets = vec![];
            for i in 0..s.order.len() {
                let mut a = s.clone();
                a.order.remove(old);
                a.order.insert(i, b);
                targets.push(a)
            }
            return Some(between(targets));
        }
        let mods = Modifiers {
            alt: mods.alt && *self == Self::Builder,
            ..mods
        };
        let mut base = s.clone();
        base.voided = None;
        let item;
        let mut excluded_holes = BTreeSet::new();
        let from_brush = s.brushes.iter().any(|b| b.id == id);
        let from_void = s.voids.first().is_some_and(|t| t.id == id);
        if from_void {
            item = base.voids.remove(0);
        } else if let Some(i) = s.brushes.iter().position(|b| b.id == id) {
            item = if *self == Self::Builder && i < 4 {
                Tree::node(
                    id,
                    &s.brushes[i].label,
                    (0..op_arity(&s.brushes[i].label))
                        .map(|j| atom(&format!("{id}-c{j}"), "◯"))
                        .collect(),
                )
            } else {
                s.brushes[i].clone()
            };
            base.brushes[i].id = fresh_id("brush");
            if *self == Self::Builder && (4..8).contains(&i) {
                base.variadic.insert(item.id.clone());
            }
        } else if mods.alt {
            let source = s.trees.iter().chain(&s.palette).find_map(|t| t.find(id))?;
            item = source.clone();
            // Refresh the stationary copy so the original identity follows the pointer.
            let mut copy = source.fresh(&fresh_id("copy"));
            fn clear_emerge(t: &mut Tree) {
                t.emerge = None;
                for c in &mut t.children {
                    clear_emerge(c);
                }
            }
            clear_emerge(&mut copy);
            excluded_holes.extend(
                copy.nodes()
                    .into_iter()
                    .filter(|t| t.label == "◯")
                    .map(|t| t.id.clone()),
            );
            for (old, new) in source.nodes().into_iter().zip(copy.nodes()) {
                if s.variadic.contains(&old.id) {
                    base.variadic.insert(new.id.clone());
                }
            }
            base.trees = base.trees.iter().map(|t| t.replace(id, &copy)).collect();
            base.palette = base.palette.iter().map(|t| t.replace(id, &copy)).collect();
        } else if let Some((trees, t)) =
            remove_from(&s.trees, id, *self == Self::Builder, &s.variadic)
        {
            base.trees = trees;
            item = t;
            if *self == Self::Macro && base.trees.is_empty() {
                base.trees.push(atom(&fresh_id("hole"), "◯"));
            }
        } else if let Some((palette, t)) =
            remove_from(&s.palette, id, *self == Self::Builder, &s.variadic)
        {
            base.palette = palette;
            item = t;
        } else {
            return None;
        }
        if item.label == "◯" {
            return None;
        }
        let mut targets = edit_targets(&base, &item);
        if !excluded_holes.is_empty() {
            targets.retain(|a| {
                excluded_holes.iter().all(|id| {
                    a.trees
                        .iter()
                        .chain(&a.palette)
                        .any(|t| t.find(id).is_some())
                })
            });
        }
        // Source swaps siblings within either the main tree or a macro gutter item.
        if !mods.alt && !from_brush && !from_void {
            for (palette, trees) in [(false, &s.trees), (true, &s.palette)] {
                if palette && *self != Self::Macro {
                    continue;
                }
                for (ti, t) in trees.iter().enumerate() {
                    if let Some(parent) = t
                        .nodes()
                        .into_iter()
                        .find(|n| n.children.iter().any(|c| c.id == id))
                    {
                        let i = parent.children.iter().position(|c| c.id == id).unwrap();
                        for j in 0..parent.children.len() {
                            if i != j && (*self == Self::Macro || parent.children[j].label != "◯")
                            {
                                let mut p = parent.clone();
                                p.children.swap(i, j);
                                let mut n = s.clone();
                                if palette {
                                    n.palette[ti] = t.replace(&parent.id, &p);
                                } else {
                                    n.trees[ti] = t.replace(&parent.id, &p);
                                }
                                targets.push(n);
                            }
                        }
                    }
                }
            }
        }
        targets.push(s.clone());
        if from_brush {
            targets.retain(|a| a != s);
            if *self == Self::Macro {
                targets.retain(|a| a.palette == base.palette && a.trees != base.trees);
            }
        }
        if *self == Self::Macro {
            return Some(closest(targets).when_far(base, 50.).with_floating());
        }
        let mut committed = base.clone();
        if !from_brush && !from_void && !mods.alt {
            committed.voids.insert(0, item.clone());
            committed.voids.truncate(10);
        }
        if from_brush || mods.alt {
            return Some(closest(targets).when_far(base, 50.).with_floating());
        }
        if from_void {
            let original = s.clone();
            return Some(
                closest(targets).with_floating().when_far(
                    fixed(base)
                        .with_floating()
                        .on_drop(move |_| original.clone()),
                    50.,
                ),
            );
        }
        let mut erase = committed.clone();
        erase.voided = Some(item);
        let deletion = fixed(erase).with_floating().on_drop({
            let committed = committed.clone();
            move |_| committed.clone()
        });
        let far = fixed(base)
            .with_floating()
            .on_drop(move |_| committed.clone());
        Some(closest([closest(targets).with_floating(), deletion]).when_far(far, 50.))
    }

    fn controls(&self, s: &AlgebraState) -> Vec<Control> {
        match self {
            Self::Tree => rule_sets()
                .into_iter()
                .enumerate()
                .map(|(i, (name, _))| {
                    Control::toggle(RULE_IDS[i], name.split(" — ").next().unwrap(), s.enabled[i])
                })
                .chain(std::iter::once(Control::toggle(
                    "emerge",
                    "Enable emerge animation for new nodes",
                    s.emerge,
                )))
                .chain(std::iter::once(Control::toggle(
                    "force-scale",
                    "Force transform: scale() for emerge",
                    s.force_scale,
                )))
                .collect(),
            Self::Builder => vec![],
            Self::Macro => {
                let mut cs = if s.recording.is_some() {
                    vec![
                        Control::action_button(
                            "record",
                            "Stop Recording",
                            s.trees.first().is_some_and(well_formed),
                            false,
                        ),
                        Control::action_button("cancel", "Cancel", true, false),
                    ]
                } else {
                    vec![Control::action_button(
                        "record",
                        "Record Macro",
                        true,
                        false,
                    )]
                };
                cs.push(Control::toggle(
                    "emerge",
                    "Enable emerge animation",
                    s.emerge,
                ));
                cs
            }
        }
    }
    fn action(&self, s: &AlgebraState, id: &str, v: Value) -> Option<AlgebraState> {
        let mut n = s.clone();
        if let Some(i) = RULE_IDS.iter().position(|x| *x == id) {
            if let Value::Bool(b) = v {
                n.enabled[i] = b;
                return Some(n);
            }
        }
        match (id, v) {
            ("holes", Value::Bool(v)) => n.buckets[0] = v,
            ("variadic", Value::Bool(v)) => n.buckets[1] = v,
            ("atoms", Value::Bool(v)) => n.buckets[2] = v,
            ("palette", Value::Bool(v)) => n.expanded = v,
            ("emerge", Value::Bool(v)) => n.emerge = v,
            ("force-scale", Value::Bool(v)) => n.force_scale = v,
            ("selected-rule", Value::Index(i)) if i < n.rules.len() => n.selected_rule = i,
            ("record", _) => {
                if let Some(before) = n.recording.take() {
                    if !n.trees.first().is_some_and(well_formed) {
                        n.recording = Some(before);
                        n.notice="Every operator must have its required number of children before recording can finish.".into();
                        return Some(n);
                    }
                    if let Some(r) = derive(&before, &n.trees[0]) {
                        n.rules.push(r)
                    }
                    if let Some(r) = derive(&n.trees[0], &before) {
                        n.rules.push(r)
                    }
                    n.notice.clear();
                } else {
                    n.recording = Some(n.trees[0].clone());
                }
            }
            ("cancel", _) => {
                n.recording = None;
                n.notice.clear();
            }
            ("remove-rule", _) => {
                if !n.rules.is_empty() {
                    n.rules.remove(n.selected_rule.min(n.rules.len() - 1));
                    n.selected_rule = n.selected_rule.min(n.rules.len().saturating_sub(1));
                }
            }
            _ => return None,
        }
        Some(n)
    }
    fn click(
        &self,
        s: &AlgebraState,
        id: &str,
        _: Point,
        _: Modifiers,
        _: usize,
    ) -> Option<AlgebraState> {
        if let Some(i) = id
            .strip_prefix("delete-rule-")
            .and_then(|i| i.parse::<usize>().ok())
        {
            if i >= s.rules.len() {
                return None;
            }
            let mut n = s.clone();
            n.rules.remove(i);
            return Some(n);
        }
        if id == "palette-toggle" {
            return self.action(s, "palette", Value::Bool(!s.expanded));
        }
        let b = id.strip_prefix("bucket-")?.parse::<usize>().ok()?;
        let mut n = s.clone();
        n.buckets[b] = !n.buckets[b];
        Some(n)
    }
    fn status(&self, s: &AlgebraState) -> String {
        if !s.notice.is_empty() {
            return s.notice.clone();
        }
        match self{Self::Tree=>"Drag an operator or operand; enabled laws determine the legal rewrites.".into(),Self::Builder=>"Drag toolkit blocks into the stage or holes. Alt-drag copies; drag a bucket header to reorder it.".into(),Self::Macro=>if s.recording.is_some(){"Recording: rearrange, add or park nodes in the gutter. Red outlines indicate incomplete operators.".into()}else{format!("{} rules. Record a rearrangement to teach a reversible rewrite.",s.rules.len())}}
    }
}
const RULE_IDS: [&str; 12] = [
    "identity",
    "reverse-identity",
    "commute",
    "assoc-up-op",
    "assoc-down-op",
    "assoc-up-operand",
    "assoc-down-operand",
    "distribute-op",
    "distribute-operand",
    "factor-op",
    "factor-operand",
    "assoc-sideways",
];
#[cfg(test)]
mod tests {
    use super::*;
    fn eval(t: &Tree) -> f64 {
        let xs: Vec<_> = t.children.iter().map(eval).collect();
        match t.label.as_str() {
            "+" => xs.iter().sum(),
            "×" => xs.iter().product(),
            "-" => -xs[0],
            "0" => 0.,
            "⛅" => 2.,
            "🍄" => 3.,
            "🎲" => 5.,
            "🦠" => 7.,
            "🐝" => 11.,
            _ => 1.,
        }
    }
    #[test]
    fn all_rule_controls_activate_without_panicking() {
        let initial = Algebra::Tree.initial();
        assert_eq!(rule_sets().len(), RULE_IDS.len());
        for control in Algebra::Tree.controls(&initial) {
            if let Control::Toggle { id, .. } = control {
                let state = Algebra::Tree
                    .action(&initial, id, Value::Bool(true))
                    .unwrap();
                for node in state.trees[0].nodes() {
                    assert!(
                        Algebra::Tree
                            .drag(&state, &node.id, Modifiers::default())
                            .is_some()
                    );
                }
            }
        }
    }
    #[test]
    fn symbolic_rules_preserve_value_and_unique_ids() {
        // Instantiate every LHS, so no rule can silently escape the semantic
        // checks because the demo's initial expression never matches it.
        fn instantiate(p: &Pattern, triggers: &mut Vec<String>) -> Tree {
            let id = fresh_id("fixture");
            let tree = if let Some(label) = &p.label {
                Tree::node(
                    &id,
                    label,
                    p.children
                        .iter()
                        .map(|p| instantiate(p, triggers))
                        .collect(),
                )
            } else if p.key == "A" {
                Tree::node(
                    &id,
                    "+",
                    vec![
                        Tree::leaf(fresh_id("fixture"), "⛅"),
                        Tree::leaf(fresh_id("fixture"), "🍄"),
                    ],
                )
            } else {
                Tree::leaf(&id, if p.key == "B" { "🎲" } else { "🐝" })
            };
            if p.trigger {
                triggers.push(id);
            }
            tree
        }
        for (name, rules) in rule_sets() {
            for rule in rules {
                let mut triggers = vec![];
                let tree = instantiate(&rule.from, &mut triggers);
                assert!(!triggers.is_empty());
                for trigger in triggers {
                    let outputs = rewrites(&tree, std::slice::from_ref(&rule), &trigger);
                    assert!(!outputs.is_empty(), "{name}: missing rewrite");
                    for result in outputs {
                        assert_eq!(
                            eval(&tree),
                            eval(&result),
                            "{name}: changed expression value"
                        );
                        let ids: BTreeSet<_> = result.nodes().iter().map(|t| &t.id).collect();
                        assert_eq!(
                            ids.len(),
                            result.nodes().len(),
                            "{name}: duplicated object identity"
                        );
                        assert!(
                            result.find(&trigger).is_some(),
                            "{name}: lost grabbed identity"
                        );
                    }
                }
            }
        }
    }
    #[test]
    fn builder_pickup_respects_parent_arity_and_stage_capacity() {
        let t = Tree::node("plus", "+", vec![atom("a", "A"), atom("b", "B")]);
        let (fixed, _) = remove_from(&[t.clone()], "a", true, &BTreeSet::new()).unwrap();
        assert_eq!(fixed[0].children.len(), 2);
        assert_eq!(fixed[0].children[0].label, "◯");
        let (variadic, _) =
            remove_from(&[t.clone()], "a", true, &BTreeSet::from(["plus".into()])).unwrap();
        assert_eq!(variadic[0].children.len(), 1);
        let mut s = Algebra::Builder.initial();
        s.trees = vec![t];
        assert!(
            edit_targets(&s, &atom("c", "C"))
                .iter()
                .all(|s| s.trees.len() == 1)
        );
    }
    #[test]
    fn recording_generalizes_stable_operands() {
        let before = macro_expression();
        let mut after = before.clone();
        after.children.swap(0, 1);
        let r = derive(&before, &after).unwrap();
        let mut other = before.clone();
        other.children[0].label = "×".into();
        let results = rewrites(&other, &[r], "dice");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].children[0].id, "dice");
        assert_eq!(results[0].children[1].label, "×");
    }
}
