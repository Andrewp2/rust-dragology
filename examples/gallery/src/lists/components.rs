use super::*;

#[derive(Clone, PartialEq)]
struct Task {
    id: String,
    text: String,
    completed: bool,
}
#[derive(Clone, PartialEq)]
pub(super) struct TodoState {
    tasks: Vec<Task>,
    draft: String,
    floating: bool,
    next: usize,
}
pub(super) struct Todo;
/// CSS `white-space: normal` with Unicode line-break opportunities. A flex
/// child's automatic minimum width is its longest unbreakable segment, so a
/// single long word can overflow the source card rather than being split.
fn todo_lines(text: &str) -> Vec<String> {
    let text = text
        .split(|c| matches!(c, ' ' | '\t' | '\n' | '\r' | '\u{c}'))
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if text.is_empty() {
        return Vec::new();
    }
    let style = TextStyle::default();
    let width = |s: &str| crate::text_layout::measure_text(s, 16., &style).advance;
    let breaks = unicode_linebreak::linebreaks(&text).collect::<Vec<_>>();
    let mut previous = 0;
    let mut available = 284f64;
    for &(end, _) in &breaks {
        available = available.max(width(text[previous..end].trim_matches(' ')));
        previous = end;
    }
    let mut lines = Vec::new();
    let mut start = 0;
    let mut previous = 0;
    for (end, opportunity) in breaks {
        let candidate = text[start..end].trim_matches(' ');
        if width(candidate) > available + 1e-6 && previous > start {
            lines.push(text[start..previous].trim_matches(' ').to_owned());
            start = previous;
        }
        previous = end;
        if opportunity == unicode_linebreak::BreakOpportunity::Mandatory {
            lines.push(text[start..end].trim_matches(' ').to_owned());
            start = end;
        }
    }
    lines
}

fn todo_card(scene: &mut Scene, task: &Task, position: Point, hidden: bool) {
    let lines = todo_lines(&task.text);
    let height = 26. + (24. * lines.len() as f64).max(20.);
    let color = if task.completed {
        [156, 163, 175, 255]
    } else {
        [55, 65, 81, 255]
    };
    let mut card = stroked(
        Object::rect(
            &task.id,
            Rect::from_origin_size(position, (350., height)),
            8.,
            WHITE,
        ),
        [229, 231, 235, 255],
        1.,
    )
    .cursor(Cursor::Move)
    .text_baseline(TextBaseline::Alphabetic);
    // The source uses a 45px foreignObject with overflow:visible, independent
    // of the HTML card's growing content height and its fixed 55px list stride.
    card.hit = Some(Hit::Rect(Rect::new(0., 0., 350., 45.)));
    let metrics = crate::text_layout::measure_text("Mg", 16., &card.text_style);
    let baseline = 13. + (24. - metrics.ascent - metrics.descent) / 2. + metrics.ascent;
    for (line, text) in lines.into_iter().enumerate() {
        card = card.label(text, (49., baseline + 24. * line as f64), 16., color);
    }
    if task.completed {
        card = card.strikethrough();
    }
    if hidden {
        card.opacity = 0.;
        card.hit = None;
    }
    scene.push(card);
    let mut check = Object::rect(
        format!("check-{}", task.id),
        Rect::from_origin_size(position + Vec2::new(17., height / 2. - 10.), (20., 20.)),
        4.,
        if task.completed {
            [59, 130, 246, 255]
        } else {
            WHITE
        },
    );
    check.parent = Some(task.id.clone());
    if task.completed {
        let mut path = BezPath::new();
        path.move_to((4., 9.));
        path.line_to((8., 13.));
        path.line_to((15., 5.));
        check.shapes.push(Shape::Path {
            path,
            fill: None,
            stroke: Some((WHITE, 2.)),
        });
    }
    if hidden {
        check.opacity = 0.;
        check.hit = None;
    }
    scene.push(
        stroked(check, [209, 213, 219, 255], 2.)
            .cursor(Cursor::Pointer)
            .clickable(),
    );
}

impl Todo {
    fn add(s: &TodoState) -> Option<TodoState> {
        if s.draft.is_empty() {
            return None;
        }
        let mut s = s.clone();
        s.tasks.insert(
            0,
            Task {
                id: format!("task-{}", s.next),
                text: std::mem::take(&mut s.draft),
                completed: false,
            },
        );
        s.next += 1;
        Some(s)
    }
}
impl Example for Todo {
    type State = TodoState;
    fn initial(&self) -> TodoState {
        TodoState {
            tasks: [
                ("a", "Buy milk", false),
                ("b", "Walk the dog", true),
                ("c", "Read a book", false),
            ]
            .into_iter()
            .map(|(id, text, completed)| Task {
                id: id.into(),
                text: text.into(),
                completed,
            })
            .collect(),
            draft: String::new(),
            floating: false,
            next: 0,
        }
    }
    fn scene(&self, s: &TodoState) -> Scene {
        let mut scene = Scene::new(400., 450.);
        scene
            .inline_control("draft", Rect::new(10., 10., 280., 54.))
            .font_size(16.)
            .foreground([55, 65, 81, 255])
            .background(WHITE)
            .border([229, 231, 235, 255])
            .placeholder("What needs to be done?");
        scene
            .inline_control("add", Rect::new(288., 10., 360., 54.))
            .font_size(16.)
            .foreground(WHITE)
            .background([59, 130, 246, 255])
            .border([59, 130, 246, 255]);
        todo_card(
            &mut scene,
            &Task {
                id: format!("task-{}", s.next),
                text: s.draft.clone(),
                completed: false,
            },
            Point::new(10., 10.),
            true,
        );
        for (i, t) in s.tasks.iter().enumerate() {
            todo_card(&mut scene, t, Point::new(10., 80. + i as f64 * 55.), false);
        }
        scene
    }
    fn action_transition(&self, _: &TodoState, id: &str) -> Option<dragology::Transition> {
        if id == "draft" || id.starts_with("check-") {
            None
        } else {
            Some(dragology::Transition::default())
        }
    }
    fn scene_interaction(
        &self,
        s: &TodoState,
        dragged: Option<&str>,
        hovered: Option<&str>,
    ) -> Scene {
        let mut scene = self.scene_dragging(s, dragged);
        if let Some(id) = hovered {
            let id = id.strip_prefix("check-").unwrap_or(id);
            if let Some(o) = scene.objects.iter_mut().find(|o| o.id == id) {
                if let Shape::Rect { stroke, .. } = &mut o.shapes[0] {
                    *stroke = Some(([209, 213, 219, 255], 1.));
                }
                *o = o.clone().shadow(Shadow {
                    offset: Vec2::new(0., 1.),
                    blur: 2.,
                    color: [0, 0, 0, 13],
                });
            }
        }
        scene
    }
    fn scene_dragging(&self, s: &TodoState, id: Option<&str>) -> Scene {
        let mut scene = self.scene(s);
        if let Some(id) = id {
            for o in &mut scene.objects {
                if o.id == id || o.parent.as_deref() == Some(id) {
                    if !s.floating {
                        o.transform = Affine::translate((-5., 0.)) * o.transform;
                    }
                }
            }
            scene
                .objects
                .sort_by_key(|o| o.id == id || o.parent.as_deref() == Some(id));
        }
        scene
    }
    fn drag(&self, s: &TodoState, id: &str, _: Modifiers) -> Option<DragSpec<'static, TodoState>> {
        let i = s.tasks.iter().position(|t| t.id == id)?;
        let states = (0..s.tasks.len())
            .map(|to| {
                let mut n = s.clone();
                n.tasks = moved(&s.tasks, i, to);
                n
            })
            .collect::<Vec<_>>();
        Some(if s.floating {
            let mut without = s.clone();
            without.tasks.remove(i);
            closest(states).when_far(without, 50.).with_floating()
        } else {
            between(states)
        })
    }
    fn click(
        &self,
        s: &TodoState,
        id: &str,
        _: Point,
        _: Modifiers,
        _: usize,
    ) -> Option<TodoState> {
        let task = id.strip_prefix("check-")?;
        let mut s = s.clone();
        let t = s.tasks.iter_mut().find(|t| t.id == task)?;
        t.completed = !t.completed;
        Some(s)
    }
    fn controls(&self, s: &TodoState) -> Vec<Control> {
        vec![
            Control::Text {
                id: "draft",
                label: "What needs to be done?".into(),
                value: s.draft.clone(),
            },
            Control::button("add", "Add"),
            Control::toggle("floating", "Use floating", s.floating),
        ]
    }
    fn action(&self, s: &TodoState, id: &str, value: Value) -> Option<TodoState> {
        let mut next = s.clone();
        match (id, value) {
            ("draft", Value::Text(text)) => next.draft = text,
            ("floating", Value::Bool(v)) => next.floating = v,
            ("add", _) => return Self::add(s),
            _ => return None,
        }
        Some(next)
    }
}

pub(super) struct TextBox;
impl Example for TextBox {
    type State = String;
    fn initial(&self) -> String {
        "Hello, world!".into()
    }
    fn scene(&self, s: &String) -> Scene {
        let mut scene = Scene::new(400., 150.);
        // The source's 40px SVG foreignObject lets the 44px HTML input overflow.
        scene
            .inline_control("text", Rect::new(20., 20., 320., 64.))
            .font_size(16.)
            .foreground([55, 65, 81, 255])
            .background(WHITE)
            .border([229, 231, 235, 255])
            .placeholder("Type something...");
        scene.push(
            Object::text("text-output", (20., 100.), s, 20., [51, 51, 51, 255])
                .text_baseline(TextBaseline::Alphabetic),
        );
        scene
    }
    fn action_transition(&self, _: &String, _: &str) -> Option<dragology::Transition> {
        None
    }
    fn drag(&self, _: &String, _: &str, _: Modifiers) -> Option<DragSpec<'static, String>> {
        None
    }
    fn controls(&self, s: &String) -> Vec<Control> {
        vec![Control::Text {
            id: "text",
            label: "Type something...".into(),
            value: s.clone(),
        }]
    }
    fn action(&self, _: &String, id: &str, value: Value) -> Option<String> {
        if let ("text", Value::Text(text)) = (id, value) {
            Some(text)
        } else {
            None
        }
    }
}

pub(super) struct Carousel;
impl Example for Carousel {
    type State = usize;
    fn initial(&self) -> usize {
        0
    }
    fn scene(&self, index: &usize) -> Scene {
        let mut scene = Scene::new(450., 300.);
        scene.push(
            stroked(
                Object::rect("background", Rect::new(0., 0., 400., 250.), 12., WHITE),
                [229, 231, 235, 255],
                2.,
            )
            .decorative(),
        );
        let colors = [
            [240, 249, 255, 255],
            [254, 243, 199, 255],
            [240, 253, 244, 255],
            [252, 231, 243, 255],
        ];
        for (i, color) in colors.into_iter().enumerate() {
            let x = 10. + (i as f64 - *index as f64) * 400.;
            let mut slide = Object::rect(
                format!("slide-{i}"),
                Rect::new(x, 10., x + 380., 200.),
                8.,
                color,
            )
            .label(
                format!("Slide {i}"),
                (190., 105.),
                32.,
                [
                    [3, 105, 161, 255],
                    [146, 64, 14, 255],
                    [22, 101, 52, 255],
                    [159, 18, 57, 255],
                ][i],
            )
            .text_anchor(TextAnchor::Middle)
            .text_baseline(TextBaseline::Middle)
            .font_weight(600);
            slide.opacity = if i == *index { 1. } else { 0. };
            if i != *index {
                slide.hit = None;
            }
            scene.push(slide);
        }
        for i in 0..4 {
            scene.push(
                Object::circle(
                    format!("dot-{i}"),
                    (170. + i as f64 * 20., 220.),
                    if i == *index { 6. } else { 4. },
                    if i == *index {
                        [59, 130, 246, 255]
                    } else {
                        [209, 213, 219, 255]
                    },
                )
                .cursor(if i == *index {
                    Cursor::Default
                } else {
                    Cursor::Pointer
                }),
            );
        }
        // Only inactive dots have click handlers; the active overlay drags.
        let mut dot = Object::circle("drag-dot", (170. + *index as f64 * 20., 220.), 6., [0; 4]);
        dot.opacity = 1.;
        scene.push(dot);
        if *index > 0 {
            scene.push(arrow("previous", 30., 220., false));
        }
        if *index < 3 {
            scene.push(arrow("next", 370., 220., true));
        }
        for object in &mut scene.objects {
            if object
                .id
                .strip_prefix("dot-")
                .and_then(|id| id.parse::<usize>().ok())
                .is_some_and(|i| i != *index)
            {
                object.click_handler = true;
            }
        }
        scene
    }
    fn drag(&self, index: &usize, id: &str, _: Modifiers) -> Option<DragSpec<'static, usize>> {
        if id == "drag-dot" {
            return Some(between(0..4));
        }
        if id != format!("slide-{index}") {
            return None;
        }
        let mut branches = Vec::new();
        if *index > 0 {
            branches.push(between([*index, index - 1]));
        }
        if *index < 3 {
            branches.push(between([*index, index + 1]));
        }
        Some(closest(branches))
    }
    fn click(&self, index: &usize, id: &str, _: Point, _: Modifiers, _: usize) -> Option<usize> {
        match id {
            "previous" => Some(index.saturating_sub(1)),
            "next" => Some((index + 1).min(3)),
            _ => id
                .strip_prefix("dot-")?
                .parse::<usize>()
                .ok()
                .filter(|n| *n < 4),
        }
    }
    fn status(&self, index: &usize) -> String {
        format!("Slide {} of 4", index + 1)
    }
}
fn arrow(id: &str, x: f64, y: f64, right: bool) -> Object {
    let mut arrow = Object::circle(id, (x, y), 15., BLACK)
        .cursor(Cursor::Pointer)
        .clickable();
    arrow.opacity = 0.5;
    let direction = if right { 1. } else { -1. };
    let mut path = BezPath::new();
    path.move_to((3. * direction, 0.));
    path.line_to((-3. * direction, -5.));
    path.line_to((-3. * direction, 5.));
    path.close_path();
    arrow.shapes.push(Shape::Path {
        path,
        fill: Some(WHITE),
        stroke: None,
    });
    arrow
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn todo_wraps_measured_text_and_keeps_source_overflow_and_whitespace_rules() {
        let narrow = "iii ".repeat(30);
        let wide = "WWW ".repeat(30);
        assert!(todo_lines(&wide).len() > todo_lines(&narrow).len());
        let text = "  Buy   enough milk\tand bread for every person coming to the picnic tomorrow afternoon.  ";
        let lines = todo_lines(text);
        assert!(lines.len() > 1);
        assert_eq!(
            lines.join(" "),
            "Buy enough milk and bread for every person coming to the picnic tomorrow afternoon."
        );
        for line in &lines {
            assert!(
                crate::text_layout::measure_text(line, 16., &TextStyle::default()).advance
                    <= 284. + 1e-6
            );
        }
        assert_eq!(todo_lines("a\u{a0}b"), ["a\u{a0}b"]);
        assert!(todo_lines(" \t\n ").is_empty());
        let mut state = Todo.initial();
        state.tasks[0].text = text.into();
        state.tasks[0].completed = true;
        let scene = Todo.scene(&state);
        let card = scene.object("a").unwrap();
        let Shape::Rect { bounds, .. } = card.shapes[0] else {
            panic!()
        };
        assert_eq!(bounds.height(), 26. + 24. * lines.len() as f64);
        assert!(card.text_style.strikethrough);
        assert_eq!(
            scene.object("b").unwrap().transform * Point::ORIGIN,
            Point::new(10., 135.)
        );
    }

    #[test]
    fn todo_creation_preserves_the_draft_identity_and_floating_removal() {
        let empty = Todo.initial();
        assert!(Todo::add(&empty).is_none());
        let whitespace = Todo
            .action(&empty, "draft", Value::Text("  ".into()))
            .unwrap();
        let added = Todo::add(&whitespace).unwrap();
        assert_eq!(added.tasks[0].text, "  ");
        assert!(Todo.scene(&whitespace).object(&added.tasks[0].id).is_some());
        assert!(added.draft.is_empty());
        let mut floating = added.clone();
        floating.floating = true;
        let id = floating.tasks[0].id.clone();
        let start = Todo.locate(&floating, &id).unwrap() * Point::new(20., 20.);
        let spec = Todo.drag(&floating, &id, Modifiers::default()).unwrap();
        let dragged = id.clone();
        let result =
            dragology::DragSession::begin(floating, start, move |s| Todo.locate(s, &dragged), spec)
                .unwrap()
                .finish(Point::new(800., 800.))
                .unwrap();
        assert!(!result.tasks.iter().any(|task| task.id == id));
        assert_eq!(result.tasks.len(), empty.tasks.len());
    }
}
