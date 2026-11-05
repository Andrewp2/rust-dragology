//! Both columns and every specimen from upstream demos/controlled.tsx.
use super::*;
use dragology::vary_constrained;
use std::{cell::RefCell, time::Duration};

#[derive(Clone, PartialEq)]
pub(super) struct ControlledState {
    parents: [[bool; 4]; 2],
    sliders: [f64; 2],
    external: f64,
}
struct Controlled;
const SWITCH_Y: [f64; 4] = [200., 350., 525., 670.];
fn words(scene: &mut Scene, id: &str, x: f64, y: f64, lines: &[&str]) {
    for (i, line) in lines.iter().enumerate() {
        scene.push(Object::text(
            format!("{id}-{i}"),
            (x, y + 18. * i as f64),
            *line,
            14.,
            [75, 85, 99, 255],
        ));
    }
}
impl Controlled {
    fn initial(&self) -> ControlledState {
        ControlledState {
            parents: [[false; 4]; 2],
            sliders: [0.5; 2],
            external: 100.,
        }
    }
    fn scene(&self, s: &ControlledState) -> Scene {
        let mut scene = Scene::new(640., 1090.).scrolling();
        for column in 0..2 {
            let x = column as f64 * 332.;
            scene.push(Object::text(
                format!("column-{column}"),
                (x, 0.),
                ["using onDropState", "using onDragState"][column],
                20.,
                INK,
            ));
            for (sample, y) in [50., 270., 470., 595., 740.].into_iter().enumerate() {
                scene.push(Object::text(
                    format!("title-{column}-{sample}"),
                    (x, y),
                    [
                        "simple",
                        "double switch",
                        "rejection",
                        "override",
                        "double slider",
                    ][sample],
                    16.,
                    INK,
                ));
            }
            words(
                &mut scene,
                &format!("simple-note-{column}"),
                x,
                80.,
                &[
                    "A normal use-case – the parent accepts",
                    "changes from the draggable, but also",
                    "makes its own changes.",
                ],
            );
            scene.push(Object::text(
                format!("parent-{column}"),
                (x, 140.),
                format!("value: {}", s.parents[column][0]),
                14.,
                [100, 116, 139, 255],
            ));
            scene.push(
                Object::rect(
                    format!("toggle-{column}"),
                    Rect::new(x, 165., x + 142., 193.),
                    4.,
                    [226, 232, 240, 255],
                )
                .label("Toggle externally", (12., 5.), 14., INK)
                .clickable(),
            );
            words(
                &mut scene,
                &format!("double-note-{column}"),
                x,
                300.,
                &["Two draggable components controlled", "by the same state."],
            );
            words(
                &mut scene,
                &format!("reject-note-{column}"),
                x,
                500.,
                &["Here the parent only accepts changes to \"true\"."],
            );
            words(
                &mut scene,
                &format!("override-note-{column}"),
                x,
                625.,
                &[
                    "Here the parent interprets all changes as",
                    "the opposite of what the draggable asks for.",
                ],
            );
            words(
                &mut scene,
                &format!("slider-note-{column}"),
                x,
                770.,
                &["Two draggable components controlled", "by the same state."],
            );
        }
        scene.push(Object::text(
            "external-title",
            (0., 900.),
            "external control",
            20.,
            INK,
        ));
        words(
            &mut scene,
            "external-note",
            0.,
            935.,
            &[
                "A native slider controls the draggable's definition. The draggable itself",
                "has no state – it changes because a new draggable is passed in.",
                "The draggable should update without a transition.",
            ],
        );
        scene.inline_control("external", Rect::new(0., 990., 129., 1010.));
        scene.push(
            Object::circle("external", (s.external, 1050.), 12., [99, 102, 241, 255]).decorative(),
        );
        for object in &mut scene.objects {
            if object.id.starts_with("track-") || object.id.starts_with("slider-track-") {
                object.stroke_style.cap = LineCap::Round;
            }
            if object.id.starts_with("title-") {
                object.text_style.italic = true;
                object.text_style.weight = 500;
            }
            if object.id.starts_with("column-") || object.id == "external-title" {
                object.text_style.weight = 500;
            }
        }
        scene
    }
    fn scene_interaction(
        &self,
        s: &ControlledState,
        _: Option<&str>,
        hovered: Option<&str>,
    ) -> Scene {
        let mut scene = self.scene(s);
        if let Some(id) = hovered.filter(|id| id.starts_with("toggle-")) {
            if let Some(object) = scene.objects.iter_mut().find(|object| object.id == id) {
                if let Shape::Rect { fill, .. } = &mut object.shapes[0] {
                    *fill = TRACK;
                }
            }
        }
        scene
    }
}

#[derive(Default)]
struct Candidate {
    value: f64,
    serial: u64,
    pending: bool,
}
struct Specimen {
    slider: bool,
    signal: Rc<RefCell<Candidate>>,
}
impl Example for Specimen {
    type State = f64;
    fn initial(&self) -> f64 {
        if self.slider { 0.5 } else { 0. }
    }
    fn scene(&self, value: &f64) -> Scene {
        let mut scene = Scene::new(150., if self.slider { 16. } else { 40. });
        if self.slider {
            scene.push(
                Object::line("track", (0., 10.), (100., 10.), TRACK, 4.).line_cap(LineCap::Round),
            );
            scene.push(Object::circle("knob", (value * 100., 10.), 8., INK));
        } else {
            scene.push(
                Object::line("track", (20., 20.), (80., 20.), TRACK, 6.).line_cap(LineCap::Round),
            );
            scene.push(Object::rect(
                "knob",
                Rect::new(value * 60., 0., value * 60. + 40., 40.),
                4.,
                INK,
            ));
        }
        scene
    }
    fn drag(&self, state: &f64, id: &str, _: Modifiers) -> Option<DragSpec<'static, f64>> {
        if id != "knob" {
            return None;
        }
        let slider = self.slider;
        let mut inner = if slider {
            vary_constrained([*state], |[t]| t, |t| [-t, t - 1.])
        } else {
            between([1., 0.])
                .with_snap_radius(10.)
                .with_drop_transition(Some(elastic()))
        };
        let signal = self.signal.clone();
        let mut previous = None;
        Some(DragSpec::custom(move |ctx, pointer| {
            let frame = inner.evaluate(ctx, pointer)?;
            let value = *frame.drop_state;
            // Source fixed states retain identity; vary builds a new state on
            // every frame, including held-pointer frames. These identities
            // determine when React's controlled parent notifies idle peers.
            if slider || previous != Some(value) {
                let mut signal = signal.borrow_mut();
                signal.value = value;
                signal.serial += 1;
                signal.pending = true;
                previous = Some(value);
            }
            Ok(frame)
        }))
    }
    fn action(&self, _: &f64, id: &str, value: Value) -> Option<f64> {
        if let ("external", Value::Number(value)) = (id, value) {
            Some(value)
        } else {
            None
        }
    }
}
struct Widget {
    column: usize,
    sample: usize,
    id: String,
    origin: Vec2,
    signal: Rc<RefCell<Candidate>>,
    model: Box<dyn Demo>,
}
impl Widget {
    fn new(column: usize, sample: usize, copy: usize) -> Self {
        let slider = sample == 4;
        let origin = Vec2::new(
            column as f64 * 332.,
            if slider {
                820. + 26. * copy as f64
            } else {
                SWITCH_Y[sample] + 48. * copy as f64
            },
        );
        let signal = Rc::new(RefCell::new(Candidate {
            value: if slider { 0.5 } else { 0. },
            ..Default::default()
        }));
        let model = demo(Specimen {
            slider,
            signal: signal.clone(),
        });
        Self {
            column,
            sample,
            id: if slider {
                format!("slider-{column}-{copy}")
            } else {
                format!("switch-{column}-{sample}-{copy}")
            },
            origin,
            signal,
            model,
        }
    }
    fn hits(&self, point: Point) -> bool {
        let Some(object) = self.model.scene().object("knob") else {
            return false;
        };
        let local = object.transform.inverse() * (point - self.origin);
        match object.hit {
            Some(Hit::Rect(bounds)) => bounds.contains(local),
            Some(Hit::Circle(center, radius)) => center.distance(local) <= radius,
            _ => false,
        }
    }
}

/// The source mounts an independent DraggableRenderer for every specimen.
/// Composing independent ModelDemos preserves those transition clocks instead
/// of applying one page-wide spring to all controls.
struct ControlledDemo {
    state: ControlledState,
    widgets: Vec<Widget>,
    parent_tokens: [Option<(usize, u64)>; 10],
    scene: Scene,
    active: Option<usize>,
    button: Option<usize>,
    hovered: Option<String>,
    animated: bool,
}
pub(super) fn create() -> Box<dyn Demo> {
    Box::new(ControlledDemo::new())
}
impl ControlledDemo {
    fn new() -> Self {
        let widgets = (0..2)
            .flat_map(|column| {
                (0..5).flat_map(move |sample| {
                    (0..if sample == 1 || sample == 4 { 2 } else { 1 })
                        .map(move |copy| Widget::new(column, sample, copy))
                })
            })
            .collect();
        let mut result = Self {
            state: Controlled.initial(),
            widgets,
            parent_tokens: [None; 10],
            scene: Scene::new(1., 1.),
            active: None,
            button: None,
            hovered: None,
            animated: true,
        };
        result.draw();
        result
    }
    fn draw(&mut self) {
        self.scene = Controlled.scene_interaction(&self.state, None, self.hovered.as_deref());
        for widget in &self.widgets {
            for object in &widget.model.scene().objects {
                let mut object = object.clone();
                object.id = if object.id == "knob" {
                    widget.id.clone()
                } else {
                    format!("{}-{}", widget.id, object.id)
                };
                object.transform = Affine::translate(widget.origin) * object.transform;
                self.scene.push(object);
            }
        }
    }
    fn button_at(point: Point) -> Option<usize> {
        (0..2).find(|column| {
            Rect::new(
                *column as f64 * 332.,
                165.,
                *column as f64 * 332. + 142.,
                193.,
            )
            .contains(point)
        })
    }
    fn parent_value(&self, index: usize) -> f64 {
        let widget = &self.widgets[index];
        if widget.sample == 4 {
            self.state.sliders[widget.column]
        } else {
            self.state.parents[widget.column][widget.sample] as u8 as f64
        }
    }
    fn callback(&mut self, index: usize) -> bool {
        let (value, serial) = {
            let signal = self.widgets[index].signal.borrow();
            (signal.value, signal.serial)
        };
        let (column, sample) = (self.widgets[index].column, self.widgets[index].sample);
        // Rejection retains the old parent object; override creates a fresh
        // opposite-valued object even when the request's identity is unchanged.
        if sample == 2 && value == 0. {
            return true;
        }
        let accepted = if sample == 3 { 1. - value } else { value };
        let token = (index, serial);
        if sample != 3 && self.parent_tokens[column * 5 + sample] == Some(token) {
            return false;
        }
        self.parent_tokens[column * 5 + sample] = Some(token);
        if sample == 4 {
            self.state.sliders[column] = accepted;
        } else {
            self.state.parents[column][sample] = accepted != 0.;
        }
        for (peer_index, peer) in self.widgets.iter_mut().enumerate() {
            if peer_index != index && peer.column == column && peer.sample == sample {
                peer.model.action("external", Value::Number(accepted));
            }
        }
        sample == 3
    }
    fn consume_live(&mut self, index: usize) {
        let pending = {
            let mut signal = self.widgets[index].signal.borrow_mut();
            let pending = signal.pending;
            signal.pending = false;
            pending
        };
        if pending && self.widgets[index].column == 1 {
            self.callback(index);
        }
    }
}
impl Demo for ControlledDemo {
    fn scene(&self) -> &Scene {
        &self.scene
    }
    fn press(&mut self, point: Point, modifiers: Modifiers, clicks: usize) {
        if self.active.is_some() {
            self.cancel();
        }
        self.button = Self::button_at(point);
        self.active = self.widgets.iter().position(|widget| widget.hits(point));
        if let Some(index) = self.active {
            let widget = &mut self.widgets[index];
            widget.model.press(point - widget.origin, modifiers, clicks);
            self.consume_live(index);
        }
        self.draw();
    }
    fn move_to(&mut self, point: Point, modifiers: Modifiers) {
        self.hovered = Self::button_at(point).map(|column| format!("toggle-{column}"));
        if let Some(index) = self.active {
            let widget = &mut self.widgets[index];
            widget.model.move_to(point - widget.origin, modifiers);
        }
        self.draw();
    }
    fn release(&mut self, point: Point, modifiers: Modifiers) {
        if let Some(index) = self.active.take() {
            let widget = &mut self.widgets[index];
            widget.model.release(point - widget.origin, modifiers);
            widget.signal.borrow_mut().pending = false;
            let correction = self.callback(index);
            if correction {
                let value = self.parent_value(index);
                // Source controlled idle synchronization uses its default
                // incoming-state spring, replacing the rejected drop spring.
                self.widgets[index]
                    .model
                    .action("external", Value::Number(value));
            }
        } else if let Some(column) = self
            .button
            .take()
            .filter(|column| Self::button_at(point) == Some(*column))
        {
            self.state.parents[column][0] = !self.state.parents[column][0];
            self.parent_tokens[column * 5] = None;
            for widget in &mut self.widgets {
                if widget.column == column && widget.sample == 0 {
                    widget.model.action(
                        "external",
                        Value::Number(self.state.parents[column][0] as u8 as f64),
                    );
                }
            }
        }
        self.button = None;
        self.draw();
    }
    fn cancel(&mut self) {
        if let Some(index) = self.active.take() {
            self.widgets[index].model.cancel();
            let value = self.parent_value(index);
            self.widgets[index]
                .model
                .action("external", Value::Number(value));
            self.widgets[index].signal.borrow_mut().pending = false;
        }
        self.button = None;
        self.draw();
    }
    fn reset(&mut self) {
        let animated = self.animated;
        *self = Self::new();
        self.set_animated(animated);
    }
    fn controls(&self) -> Vec<Control> {
        vec![Control::Slider {
            id: "external",
            label: "External control".into(),
            value: self.state.external,
            min: 0.,
            max: 200.,
            step: 1.,
        }]
    }
    fn action(&mut self, id: &str, value: Value) {
        if let ("external", Value::Number(value)) = (id, value) {
            self.state.external = value.clamp(0., 200.);
            self.draw();
        }
    }
    fn key(&mut self, key: &str, _: Modifiers) {
        if key == "escape" {
            self.cancel();
        }
    }
    fn status(&self) -> String {
        self.widgets
            .iter()
            .map(|w| w.model.status())
            .find(|s| !s.is_empty())
            .unwrap_or_default()
    }
    fn cursor(&self) -> Cursor {
        if self.active.is_some() {
            Cursor::Grabbing
        } else {
            Cursor::Default
        }
    }
    fn advance(&mut self, elapsed: Duration) -> bool {
        let mut running = false;
        for widget in &mut self.widgets {
            running |= widget.model.advance(elapsed);
        }
        if let Some(index) = self.active {
            self.consume_live(index);
        }
        self.draw();
        running || self.active.is_some()
    }
    fn set_animated(&mut self, enabled: bool) {
        self.animated = enabled;
        for widget in &mut self.widgets {
            widget.model.set_animated(enabled);
        }
        self.draw();
    }
    fn can_import(&self) -> bool {
        false
    }
    fn export(&self) -> Option<(String, String)> {
        None
    }
    fn import(&mut self, _: &str) -> Result<(), String> {
        Err("This demo does not import files.".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn x(demo: &ControlledDemo, id: &str) -> f64 {
        demo.scene().object(id).unwrap().transform.as_coeffs()[4]
    }
    fn near(a: f64, b: f64) {
        assert!((a - b).abs() < 0.01, "{a} != {b}");
    }
    #[test]
    fn both_source_columns_and_every_specimen_are_mounted() {
        let demo = ControlledDemo::new();
        assert_eq!(demo.widgets.iter().filter(|w| w.sample != 4).count(), 10);
        assert_eq!(demo.widgets.iter().filter(|w| w.sample == 4).count(), 4);
        assert!(demo.scene().object("external").is_some());
    }
    #[test]
    fn live_slider_peer_has_its_own_retargeted_incoming_state_clock() {
        let mut demo = ControlledDemo::new();
        demo.press(Point::new(382., 830.), Modifiers::default(), 1);
        demo.move_to(Point::new(432., 830.), Modifiers::default());
        near(demo.state.sliders[1], 0.5);
        demo.advance(Duration::ZERO);
        near(demo.state.sliders[1], 1.);
        near(x(&demo, "slider-1-0"), 432.);
        near(x(&demo, "slider-1-1"), 382.);
        demo.advance(Duration::from_millis(100));
        near(x(&demo, "slider-1-0"), 432.);
        near(x(&demo, "slider-1-1"), 425.75);
        // vary creates fresh candidate identities while the pointer is held,
        // so the controlled peer retargets its spring again on each frame.
        demo.advance(Duration::from_millis(100));
        near(x(&demo, "slider-1-1"), 431.21875);
        demo.release(Point::new(432., 830.), Modifiers::default());
        demo.advance(Duration::from_millis(200));
        near(x(&demo, "slider-1-1"), 432.);
    }
    #[test]
    fn drop_slider_peer_waits_for_release_then_animates_independently() {
        let mut demo = ControlledDemo::new();
        demo.press(Point::new(50., 830.), Modifiers::default(), 1);
        demo.move_to(Point::new(100., 830.), Modifiers::default());
        demo.advance(Duration::from_millis(100));
        near(x(&demo, "slider-0-0"), 100.);
        near(x(&demo, "slider-0-1"), 50.);
        demo.release(Point::new(100., 830.), Modifiers::default());
        near(x(&demo, "slider-0-1"), 50.);
        demo.advance(Duration::from_millis(100));
        near(x(&demo, "slider-0-0"), 100.);
        near(x(&demo, "slider-0-1"), 93.75);
    }
    #[test]
    fn live_rejection_keeps_accepted_parent_and_uses_cubic_correction() {
        let mut demo = ControlledDemo::new();
        demo.press(Point::new(352., 545.), Modifiers::default(), 1);
        demo.move_to(Point::new(412., 545.), Modifiers::default());
        demo.advance(Duration::ZERO);
        assert!(demo.state.parents[1][2]);
        demo.move_to(Point::new(352., 545.), Modifiers::default());
        demo.advance(Duration::ZERO);
        assert!(demo.state.parents[1][2]);
        demo.release(Point::new(352., 545.), Modifiers::default());
        demo.advance(Duration::from_millis(100));
        near(x(&demo, "switch-1-2-0"), 384.5);
        demo.advance(Duration::from_millis(100));
        near(x(&demo, "switch-1-2-0"), 392.);
    }
    #[test]
    fn coalesced_pointer_events_do_not_accept_an_unrendered_intermediate_request() {
        let mut demo = ControlledDemo::new();
        demo.press(Point::new(352., 545.), Modifiers::default(), 1);
        demo.move_to(Point::new(412., 545.), Modifiers::default());
        demo.move_to(Point::new(352., 545.), Modifiers::default());
        assert!(!demo.state.parents[1][2]);
        demo.advance(Duration::ZERO);
        assert!(!demo.state.parents[1][2]);
        demo.release(Point::new(352., 545.), Modifiers::default());
        assert!(!demo.state.parents[1][2]);
    }
    #[test]
    fn override_correction_and_external_definition_have_different_timing() {
        let mut demo = ControlledDemo::new();
        demo.press(Point::new(20., 690.), Modifiers::default(), 1);
        demo.move_to(Point::new(80., 690.), Modifiers::default());
        demo.advance(Duration::ZERO);
        demo.advance(Duration::from_millis(200));
        near(x(&demo, "switch-0-3-0"), 60.);
        demo.release(Point::new(80., 690.), Modifiers::default());
        assert!(!demo.state.parents[0][3]);
        demo.advance(Duration::from_millis(100));
        near(x(&demo, "switch-0-3-0"), 7.5);
        demo.action("external", Value::Number(180.));
        near(x(&demo, "external"), 180.);
        demo.advance(Duration::from_millis(100));
        near(x(&demo, "external"), 180.);
    }
    #[test]
    fn external_toggle_updates_parent_before_child_spring_finishes() {
        let mut demo = ControlledDemo::new();
        demo.press(Point::new(20., 175.), Modifiers::default(), 1);
        demo.release(Point::new(20., 175.), Modifiers::default());
        assert!(demo.state.parents[0][0]);
        near(x(&demo, "switch-0-0-0"), 0.);
        demo.advance(Duration::from_millis(100));
        near(x(&demo, "switch-0-0-0"), 52.5);
    }
}
