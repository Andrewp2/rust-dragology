use crate::*;
use dragology::{DragError, DragSession};
use std::{collections::VecDeque, rc::Rc, time::Duration};
mod scene;
#[cfg(test)]
mod tests;

struct Active<E: Example> {
    id: String,
    original: E::State,
    basis_state: E::State,
    basis_id: String,
    anchor_state: E::State,
    anchor: Point,
    press: Point,
    session: DragSession<'static, E::State>,
    modifiers: Modifiers,
    path: Option<String>,
    drop_transition: Option<Option<dragology::Transition>>,
    last_drop: Option<Rc<E::State>>,
    pending_click: bool,
}
struct Press {
    id: String,
    point: Point,
    clicks: usize,
    moved: bool,
}
struct Transition {
    from: Scene,
    to: Scene,
    elapsed: Duration,
    spec: dragology::Transition,
}

pub struct ModelDemo<E: Example> {
    example: Rc<E>,
    state: E::State,
    scene: Scene,
    active: Option<Active<E>>,
    press: Option<Press>,
    transition: Option<Transition>,
    animated: bool,
    error: Option<String>,
    time: Duration,
    pending: VecDeque<(Duration, Point)>,
    delayed: Point,
    hovered: Option<String>,
    pointer: Point,
    pointer_modifiers: Modifiers,
}
impl<E: Example> ModelDemo<E> {
    pub fn new(example: E) -> Self {
        let state = example.initial();
        let scene = example.scene(&state);
        Self {
            example: Rc::new(example),
            state,
            scene,
            active: None,
            press: None,
            transition: None,
            animated: true,
            error: None,
            time: Duration::ZERO,
            pending: VecDeque::new(),
            delayed: Point::ORIGIN,
            hovered: None,
            pointer: Point::ORIGIN,
            pointer_modifiers: Modifiers::default(),
        }
    }
    fn replace(&mut self, state: E::State, animate: bool) {
        self.replace_transition(state, animate.then(dragology::Transition::default));
    }
    fn replace_transition(&mut self, state: E::State, transition: Option<dragology::Transition>) {
        self.active = None;
        self.press = None;
        self.pending.clear();
        self.state = state;
        let next = self
            .example
            .scene_interaction(&self.state, None, self.hovered.as_deref());
        if let Some(spec) = transition.filter(|spec| !spec.duration.is_zero())
            && self.animated
            && self.scene != next
        {
            self.transition = Some(Transition {
                from: self.scene.clone(),
                to: next,
                elapsed: Duration::ZERO,
                spec,
            });
        } else {
            self.scene = next;
            self.transition = None;
        }
    }
    fn fail(&mut self, error: DragError) {
        let restore = self
            .active
            .take()
            .map(|a| a.original)
            .unwrap_or_else(|| self.state.clone());
        self.replace(restore, true);
        self.error = Some(error.to_string());
    }
    fn session(
        &self,
        state: E::State,
        id: &str,
        press: Point,
        mods: Modifiers,
    ) -> Result<Option<DragSession<'static, E::State>>, DragError> {
        let Some(spec) = self.example.drag(&state, id, mods) else {
            return Ok(None);
        };
        let example = self.example.clone();
        let key = id.to_owned();
        let preview_example = self.example.clone();
        let preview_id = id.to_owned();
        DragSession::begin(state, press, move |state| example.locate(state, &key), spec).map(
            |session| {
                Some(session.with_preview_geometry(move |preview, anchor| {
                    let rendered =
                        scene::resolve_interaction(&*preview_example, &preview_id, None, preview);
                    Some(rendered.object(&preview_id)?.transform * anchor)
                }))
            },
        )
    }
    fn evaluate(&mut self, point: Point, mods: Modifiers) {
        self.evaluate_frame(point, mods, true);
    }
    fn evaluate_frame(&mut self, point: Point, mods: Modifiers, allow_chaining: bool) {
        let Some(mut active) = self.active.take() else {
            return;
        };
        if mods != active.modifiers {
            let (start, id) = self
                .example
                .begin_drag(&active.basis_state, &active.basis_id, mods)
                .unwrap_or_else(|| (active.basis_state.clone(), active.basis_id.clone()));
            let press = self
                .example
                .locate(&start, &id)
                .map(|transform| transform * active.anchor)
                .unwrap_or(active.press);
            match self.session(start.clone(), &id, press, mods) {
                Ok(Some(session)) => {
                    active.session = session;
                    active.anchor_state = start;
                    active.press = press;
                    active.id = id;
                }
                Ok(None) => {
                    self.active = Some(active);
                    self.cancel();
                    return;
                }
                Err(error) => {
                    self.active = Some(active);
                    self.fail(error);
                    return;
                }
            }
            active.modifiers = mods;
        }
        match active.session.frame(point) {
            Ok(mut frame) => {
                let mut next_scene = scene::resolve_interaction(
                    &*self.example,
                    &active.id,
                    self.hovered.as_deref(),
                    &frame.preview,
                );
                let branch_changed = allow_chaining
                    && active
                        .path
                        .as_ref()
                        .is_some_and(|path| path != &frame.active_path);
                let mut transition = branch_changed.then(|| {
                    frame
                        .branch_transition
                        .unwrap_or(Some(dragology::Transition::default()))
                });
                if allow_chaining
                    && *frame.drop_state != active.anchor_state
                    && let Some(chain) = frame.chain.clone()
                {
                    let next_id = chain.dragged_id.unwrap_or_else(|| active.id.clone());
                    let Some(transform) = self.example.locate(&frame.drop_state, &next_id) else {
                        self.active = Some(active);
                        self.fail(DragError::MissingElement);
                        return;
                    };
                    let next = (*frame.drop_state).clone();
                    let press = transform * active.anchor;
                    let chained_session = match self.session(next.clone(), &next_id, press, mods) {
                        Ok(session) => session,
                        Err(error) => {
                            self.active = Some(active);
                            self.fail(error);
                            return;
                        }
                    };
                    if let Some(mut session) = chained_session {
                        transition = Some(chain.transition);
                        let next_frame = match session.frame(point) {
                            Ok(frame) => frame,
                            Err(error) => {
                                self.active = Some(active);
                                self.fail(error);
                                return;
                            }
                        };
                        next_scene = scene::resolve_interaction(
                            &*self.example,
                            &next_id,
                            self.hovered.as_deref(),
                            &next_frame.preview,
                        );
                        frame = next_frame;
                        active.id = next_id;
                        active.session = session;
                        active.basis_state = next.clone();
                        active.basis_id = active.id.clone();
                        active.anchor_state = next;
                        active.press = press;
                    }
                }
                active.path = Some(frame.active_path);
                active.drop_transition = frame.drop_transition;
                active.last_drop = Some(frame.drop_state);
                if self.animated {
                    // Source starts the spring on the branch change itself,
                    // including when both previews currently snap to identical
                    // geometry. Its age matters once the pointer leaves the snap zone.
                    if let Some(Some(spec)) = transition {
                        self.transition = Some(Transition {
                            from: self.scene.clone(),
                            to: next_scene.clone(),
                            elapsed: Duration::ZERO,
                            spec,
                        });
                    } else if matches!(transition, Some(None)) {
                        self.transition = None;
                    }
                    if let Some(transition) = &mut self.transition {
                        transition.to = next_scene.clone();
                        self.draw_transition();
                    } else {
                        self.scene = next_scene;
                    }
                } else {
                    self.scene = next_scene;
                    self.transition = None;
                }
                self.active = Some(active);
            }
            Err(error) => {
                self.active = Some(active);
                self.fail(error);
            }
        }
    }
    fn pin_group(&mut self) {
        let Some(transition) = &self.transition else {
            return;
        };
        // Replace in place, preserving the target layer ordering. Floating
        // layers opt out of spring interpolation so they follow the pointer.
        for object in &mut self.scene.objects {
            if !object.transition
                && let Some(target) = transition.to.object(&object.id)
            {
                *object = target.clone();
            }
        }
    }
    fn draw_transition(&mut self) {
        if let Some(transition) = &self.transition {
            let t = transition.spec.progress(transition.elapsed);
            self.scene = scene::blend_transition(transition.to.clone(), transition.from.clone(), t);
        }
        self.pin_group();
    }
    fn delay_point(&mut self, point: Point) -> Point {
        let delay = self.example.pointer_delay(&self.state);
        if delay.is_zero() {
            return point;
        }
        self.pending.push_back((self.time, point));
        self.update_delayed(delay);
        self.delayed
    }
    fn update_delayed(&mut self, delay: Duration) {
        while self
            .pending
            .front()
            .is_some_and(|(time, _)| time.saturating_add(delay) <= self.time)
        {
            self.delayed = self.pending.pop_front().unwrap().1;
        }
    }
}
impl<E: Example> Demo for ModelDemo<E> {
    fn scene(&self) -> &Scene {
        &self.scene
    }
    fn press(&mut self, point: Point, modifiers: Modifiers, clicks: usize) {
        if !point.is_finite() {
            return;
        }
        if self.active.is_some() {
            self.cancel()
        }
        let Some(object) = self
            .scene
            .objects
            .iter()
            .rev()
            .find(|o| scene::hit(o, point))
            .cloned()
        else {
            return;
        };
        self.error = None;
        self.press = Some(Press {
            id: object.id.clone(),
            point,
            clicks,
            moved: false,
        });
        self.pending.clear();
        self.delayed = point;
        self.pointer = point;
        self.pointer_modifiers = modifiers;
        // Upstream installs callbacks on the unsprung target tree before applying
        // visual interpolation. Regrabbing therefore uses that target's transform.
        let target_transform = self
            .example
            .locate(&self.state, &object.id)
            .unwrap_or(object.transform);
        let anchor =
            self.example
                .anchor(&self.state, &object.id, target_transform.inverse() * point);
        let mut ancestor = Some(&object);
        let mut pending_click = false;
        let mut visited = std::collections::HashSet::new();
        while let Some(node) = ancestor {
            if !visited.insert(node.id.as_str()) {
                break;
            }
            pending_click |= node.click_handler;
            ancestor = node.parent.as_deref().and_then(|id| self.scene.object(id));
        }
        let (start, id) = self
            .example
            .begin_drag(&self.state, &object.id, modifiers)
            .unwrap_or_else(|| (self.state.clone(), object.id.clone()));
        let Some(transform) = self.example.locate(&start, &id) else {
            return;
        };
        let press = transform * anchor;
        match self.session(start.clone(), &id, press, modifiers) {
            Ok(Some(session)) => {
                self.transition = None;
                self.active = Some(Active {
                    id,
                    original: self.state.clone(),
                    basis_state: self.state.clone(),
                    basis_id: object.id.clone(),
                    anchor_state: start,
                    anchor,
                    press,
                    session,
                    modifiers,
                    path: None,
                    drop_transition: None,
                    last_drop: None,
                    pending_click,
                });
                // initDrag evaluates once, but deliberately defers chaining.
                self.evaluate_frame(point, modifiers, false);
                if pending_click {
                    self.scene =
                        self.example
                            .scene_interaction(&self.state, None, self.hovered.as_deref());
                }
            }
            Ok(None) => {}
            Err(error) => self.fail(error),
        }
    }
    fn move_to(&mut self, point: Point, modifiers: Modifiers) {
        if !point.is_finite() {
            return;
        }
        self.pointer = point;
        self.pointer_modifiers = modifiers;
        if let Some(press) = &mut self.press {
            press.moved |= point.distance(press.point) > 2.;
        }
        let hovered = self
            .scene
            .objects
            .iter()
            .rev()
            .find(|o| scene::hit(o, point))
            .map(|o| o.id.clone());
        let hover_changed = hovered != self.hovered;
        self.hovered = hovered;
        if let Some(active) = &mut self.active {
            if active.pending_click {
                if !self.press.as_ref().is_some_and(|press| press.moved) {
                    return;
                }
                active.pending_click = false;
            }
            // Upstream's pointer listener records input only. Stateful specs
            // and chaining advance once in the host's animation-frame callback.
            self.delay_point(point);
        } else if self.press.is_none() {
            if let Some(state) = self.example.hover(&self.state, point)
                && state != self.state
            {
                self.replace(state, false);
            } else if hover_changed || self.transition.is_none() {
                let next =
                    self.example
                        .scene_interaction(&self.state, None, self.hovered.as_deref());
                if hover_changed
                    && self.animated
                    && let Some(spec) = self.example.hover_transition(&self.state)
                {
                    self.transition = Some(Transition {
                        from: self.scene.clone(),
                        to: next,
                        elapsed: Duration::ZERO,
                        spec,
                    });
                } else {
                    self.scene = next;
                }
            }
        }
    }
    fn release(&mut self, point: Point, modifiers: Modifiers) {
        if let Some(press) = self.press.take()
            && !press.moved
            && point.distance(press.point) <= 2.
            && let Some(state) =
                self.example
                    .click(&self.state, &press.id, point, modifiers, press.clicks)
        {
            let transition = self.example.action_transition(&self.state, &press.id);
            self.replace_transition(state, transition);
            return;
        }
        if self
            .active
            .as_ref()
            .is_some_and(|active| active.pending_click)
        {
            self.active = None;
            self.pending.clear();
            return;
        }
        if self.active.is_none() {
            return;
        }
        let point = self.delay_point(point);
        self.evaluate_frame(point, modifiers, false);
        let Some(active) = self.active.take() else {
            return;
        };
        let transition = active
            .drop_transition
            .unwrap_or_else(|| self.example.drop_transition(&active.anchor_state));
        // The release frame was evaluated once above. Re-evaluating here would
        // advance stateful operators a second time and could chain after release.
        if let Some(state) = active.last_drop {
            self.replace_transition((*state).clone(), transition);
        }
    }
    fn cancel(&mut self) {
        let state = self
            .active
            .take()
            .map(|a| a.original)
            .unwrap_or_else(|| self.state.clone());
        self.replace(state, true);
    }
    fn reset(&mut self) {
        self.error = None;
        self.replace(self.example.reset(&self.state), false);
    }
    fn controls(&self) -> Vec<Control> {
        self.example.controls(&self.state)
    }
    fn action(&mut self, id: &str, value: Value) {
        if let Some(state) = self.example.action(&self.state, id, value) {
            self.error = None;
            let transition = self.example.action_transition(&self.state, id);
            self.replace_transition(state, transition)
        }
    }
    fn key(&mut self, key: &str, modifiers: Modifiers) {
        if let Some(state) = self.example.key(&self.state, key, modifiers) {
            self.error = None;
            self.replace(state, true)
        }
    }
    fn status(&self) -> String {
        self.error
            .clone()
            .unwrap_or_else(|| self.example.status(&self.state))
    }
    fn tooltip(&self) -> Option<String> {
        self.hovered
            .as_ref()
            .and_then(|id| self.scene.object(id))
            .and_then(|object| object.tooltip.clone())
    }
    fn cursor(&self) -> Cursor {
        if self
            .active
            .as_ref()
            .is_some_and(|active| !active.pending_click)
        {
            return Cursor::Grabbing;
        }
        self.hovered
            .as_ref()
            .and_then(|id| self.scene.object(id))
            .map(|object| object.cursor)
            .unwrap_or_default()
    }
    fn advance(&mut self, elapsed: Duration) -> bool {
        self.time += elapsed;
        // Age the previous spring before evaluating: a spring created on this
        // frame starts at t=0, just like makeSpringOrigin(performance.now()).
        if let Some(transition) = &mut self.transition {
            transition.elapsed += elapsed;
            if transition.elapsed >= transition.spec.duration {
                self.scene = transition.to.clone();
                self.transition = None;
            }
        }
        self.draw_transition();
        let tracking = self
            .active
            .as_ref()
            .is_some_and(|active| !active.pending_click);
        if tracking {
            let delay = self.example.pointer_delay(&self.state);
            if !delay.is_zero() {
                self.update_delayed(delay);
            }
            let mods = self.pointer_modifiers;
            self.evaluate(
                if delay.is_zero() {
                    self.pointer
                } else {
                    self.delayed
                },
                mods,
            );
        }
        self.transition.is_some() || tracking
    }
    fn set_animated(&mut self, enabled: bool) {
        self.animated = enabled;
        if !enabled && let Some(transition) = self.transition.take() {
            self.scene = transition.to;
        }
    }
    fn can_import(&self) -> bool {
        self.example.can_import()
    }
    fn export(&self) -> Option<(String, String)> {
        self.example.export(&self.state)
    }
    fn import(&mut self, text: &str) -> Result<(), String> {
        let state = self.example.import(&self.state, text)?;
        self.error = None;
        self.replace(state, false);
        Ok(())
    }
}
