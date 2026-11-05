//! Independent source figures presented together on one scrolling demo page.
use crate::*;
use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
    time::Duration,
};

pub struct Panel {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub demo: Box<dyn Demo>,
}

pub struct CompositeDemo {
    panels: Vec<Panel>,
    offsets: Vec<Vec2>,
    scene: Scene,
    active: Option<usize>,
    hovered: Option<usize>,
}

// Control IDs are static in the scene API. Intern each finite panel/control
// pair once, including when users revisit the demo, rather than leaking per frame.
fn control_id(panel: &str, id: &str) -> &'static str {
    static IDS: OnceLock<Mutex<HashMap<String, &'static str>>> = OnceLock::new();
    let mut ids = IDS.get_or_init(Default::default).lock().unwrap();
    let key = format!("{panel}/{id}");
    *ids.entry(key.clone())
        .or_insert_with(|| Box::leak(key.into_boxed_str()))
}

fn rename_control(mut control: Control, panel: &str) -> Control {
    let id = match &mut control {
        Control::ActionButton { id, .. }
        | Control::Button { id, .. }
        | Control::Toggle { id, .. }
        | Control::Choice { id, .. }
        | Control::Slider { id, .. }
        | Control::Text { id, .. }
        | Control::Code { id, .. } => id,
    };
    *id = control_id(panel, id);
    control
}

impl CompositeDemo {
    pub fn new(panels: Vec<Panel>) -> Self {
        let mut result = Self {
            panels,
            offsets: Vec::new(),
            scene: Scene::new(1., 1.),
            active: None,
            hovered: None,
        };
        result.refresh();
        result
    }

    fn refresh(&mut self) {
        let width = self
            .panels
            .iter()
            .map(|p| p.demo.scene().width)
            .fold(400., f64::max)
            + 40.;
        let mut scene = Scene::new(width, 1.).scrolling();
        self.offsets.clear();
        let mut y = 20.;
        for panel in &self.panels {
            if !panel.title.is_empty() {
                scene.push(
                    Object::text(
                        format!("{}/title", panel.id),
                        (20., y),
                        panel.title,
                        20.,
                        [0, 0, 0, 255],
                    )
                    .font_weight(600),
                );
                y += 34.;
            }
            if !panel.description.is_empty() {
                let mut line = String::new();
                let mut line_index = 0;
                for word in panel.description.split_whitespace() {
                    if line.len() + word.len() + 1 > ((width - 40.) / 7.).max(20.) as usize {
                        scene.push(Object::text(
                            format!("{}/note-{line_index}", panel.id),
                            (20., y),
                            line,
                            14.,
                            [0, 0, 0, 255],
                        ));
                        y += 21.;
                        line = String::new();
                        line_index += 1;
                    }
                    if !line.is_empty() {
                        line.push(' ');
                    }
                    line.push_str(word);
                }
                if !line.is_empty() {
                    scene.push(Object::text(
                        format!("{}/note-{line_index}", panel.id),
                        (20., y),
                        line,
                        14.,
                        [0, 0, 0, 255],
                    ));
                    y += 27.;
                }
            }
            let child = panel.demo.scene();
            for control in panel.demo.controls() {
                if !child.inline_controls.iter().any(|p| p.id == control.id()) {
                    scene.inline_control(
                        control_id(panel.id, control.id()),
                        Rect::new(20., y, width - 20., y + 30.),
                    );
                    y += 36.;
                }
            }
            let offset = Vec2::new(20., y);
            self.offsets.push(offset);
            for object in &child.objects {
                let mut object = object.clone();
                object.id = format!("{}/{}", panel.id, object.id);
                object.parent = object.parent.map(|id| format!("{}/{}", panel.id, id));
                object.emerge_from = object.emerge_from.map(|id| format!("{}/{}", panel.id, id));
                object.prepend_translation(offset);
                scene.push(object);
            }
            for placement in &child.inline_controls {
                let mut placement = placement.clone();
                placement.id = control_id(panel.id, placement.id);
                if let Some(parent) = &mut placement.parent {
                    *parent = format!("{}/{}", panel.id, parent);
                } else {
                    placement.bounds = placement.bounds + offset;
                }
                scene.inline_controls.push(placement);
            }
            y += child.height + 32.;
        }
        scene.height = y;
        self.scene = scene;
    }

    fn panel_at(&self, point: Point) -> Option<usize> {
        self.panels
            .iter()
            .zip(&self.offsets)
            .position(|(panel, offset)| {
                Rect::from_origin_size(
                    offset.to_point(),
                    (panel.demo.scene().width, panel.demo.scene().height),
                )
                .contains(point)
            })
    }
}

impl Demo for CompositeDemo {
    fn scene(&self) -> &Scene {
        &self.scene
    }
    fn press(&mut self, point: Point, modifiers: Modifiers, clicks: usize) {
        self.cancel();
        self.active = self.panel_at(point);
        if let Some(i) = self.active {
            self.panels[i]
                .demo
                .press(point - self.offsets[i], modifiers, clicks);
        }
        self.refresh();
    }
    fn move_to(&mut self, point: Point, modifiers: Modifiers) {
        let hovered = self.panel_at(point);
        if let Some(previous) = self.hovered
            && Some(previous) != hovered
            && Some(previous) != self.active
        {
            self.panels[previous]
                .demo
                .move_to(point - self.offsets[previous], modifiers);
        }
        self.hovered = hovered;
        if let Some(i) = self.active.or(self.hovered) {
            self.panels[i]
                .demo
                .move_to(point - self.offsets[i], modifiers);
            self.refresh();
        }
    }
    fn release(&mut self, point: Point, modifiers: Modifiers) {
        if let Some(i) = self.active.take() {
            self.panels[i]
                .demo
                .release(point - self.offsets[i], modifiers);
        }
        self.refresh();
    }
    fn cancel(&mut self) {
        if let Some(i) = self.active.take() {
            self.panels[i].demo.cancel();
            self.refresh();
        }
    }
    fn reset(&mut self) {
        for panel in &mut self.panels {
            panel.demo.reset();
        }
        self.active = None;
        self.refresh();
    }
    fn controls(&self) -> Vec<Control> {
        self.panels
            .iter()
            .flat_map(|panel| {
                panel
                    .demo
                    .controls()
                    .into_iter()
                    .map(|c| rename_control(c, panel.id))
            })
            .collect()
    }
    fn action(&mut self, id: &str, value: Value) {
        if let Some((panel_id, id)) = id.split_once('/')
            && let Some(panel) = self.panels.iter_mut().find(|p| p.id == panel_id)
        {
            panel.demo.action(id, value);
            self.refresh();
        }
    }
    fn key(&mut self, key: &str, modifiers: Modifiers) {
        if let Some(i) = self.active {
            self.panels[i].demo.key(key, modifiers);
            self.refresh();
        }
    }
    fn status(&self) -> String {
        self.active
            .map(|i| self.panels[i].demo.status())
            .unwrap_or_default()
    }
    fn tooltip(&self) -> Option<String> {
        self.active
            .or(self.hovered)
            .and_then(|i| self.panels[i].demo.tooltip())
    }
    fn cursor(&self) -> Cursor {
        self.active
            .or(self.hovered)
            .map(|i| self.panels[i].demo.cursor())
            .unwrap_or_default()
    }
    fn advance(&mut self, elapsed: Duration) -> bool {
        let mut changed = false;
        for panel in &mut self.panels {
            let before = panel.demo.scene().clone();
            changed |= panel.demo.advance(elapsed) || *panel.demo.scene() != before;
        }
        if changed {
            self.refresh();
        }
        changed
    }
    fn set_animated(&mut self, enabled: bool) {
        for panel in &mut self.panels {
            panel.demo.set_animated(enabled);
        }
        self.refresh();
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
