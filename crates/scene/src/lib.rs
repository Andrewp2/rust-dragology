//! A small scene format shared by the egui, GPUI and wgpu adapters.
//! A diagram supplies stable IDs, filled rounded rectangles, and drag specs.
//! Coordinates are logical scene units with the origin at the top left.
pub use dragology::{self, Affine, DragError, DragSpec, Point, Rect, Vec2};
use dragology::{DragSession, Preview};
use std::{hash::Hash, rc::Rc, time::Duration};

mod blend;
mod motion;
use motion::Transition;

/// A filled rectangle with a uniform corner radius. Color is unpremultiplied sRGBA.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quad {
    pub bounds: Rect,
    pub radius: f64,
    pub color: [u8; 4],
}

impl Quad {
    pub fn corner_radius(self) -> f64 {
        self.radius
            .max(0.)
            .min(self.bounds.width().min(self.bounds.height()).max(0.) * 0.5)
    }

    pub fn contains(self, p: Point) -> bool {
        if !self.bounds.contains(p) || self.color[3] == 0 {
            return false;
        }
        let r = self.corner_radius();
        let b = self.bounds;
        let offset = p - b.center();
        let dx = (offset.x.abs() - (b.width() * 0.5 - r)).max(0.);
        let dy = (offset.y.abs() - (b.height() * 0.5 - r)).max(0.);
        dx * dx + dy * dy <= r * r
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Item<K> {
    pub id: K,
    pub quad: Quad,
}

pub trait Diagram: 'static {
    type State: Clone + 'static;
    type Id: Clone + Eq + Hash + 'static;

    /// Back-to-front order. IDs must be unique within each state and stable across states.
    fn layout(&self, state: &Self::State) -> Vec<Item<Self::Id>>;
    /// Return None for decoration or an item that cannot be grabbed in this state.
    fn drag(&self, state: &Self::State, id: &Self::Id) -> Option<DragSpec<'static, Self::State>>;

    /// Expand an item's displayed shape into decorative quads, such as a shadow
    /// and a highlight. Hit testing still uses the item's original bounds.
    fn paint(&self, item: &Item<Self::Id>, _dragging: bool) -> Vec<Quad> {
        vec![item.quad]
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Pointer {
    Press(Point),
    Move(Point),
    Release(Point),
    Cancel,
}

struct Active<D: Diagram> {
    id: D::Id,
    session: DragSession<'static, D::State>,
}

// Floating ghosts are paint only: giving one an Item ID would make keyed
// interpolation merge it with the real object and pull the object off pointer.
struct Decoration<K> {
    before: Option<K>,
    quads: Vec<Quad>,
}

struct Resolved<K> {
    items: Vec<Item<K>>,
    decorations: Vec<Decoration<K>>,
}

/// Owns the committed model, the current preview and at most one active pointer.
pub struct Canvas<D: Diagram> {
    diagram: Rc<D>,
    model: D::State,
    size: Vec2,
    items: Vec<Item<D::Id>>,
    decorations: Vec<Decoration<D::Id>>,
    active: Option<Active<D>>,
    error: Option<DragError>,
    animated: bool,
    transition: Option<Transition<D::Id>>,
}

impl<D: Diagram> Canvas<D> {
    /// `size` is the positive, finite design size of the scene.
    pub fn new(diagram: D, model: D::State, size: Vec2) -> Self {
        assert!(
            size.is_finite() && size.x > 0. && size.y > 0.,
            "scene size must be positive and finite"
        );
        let items = diagram.layout(&model);
        Self {
            diagram: Rc::new(diagram),
            model,
            size,
            items,
            decorations: Vec::new(),
            active: None,
            error: None,
            animated: false,
            transition: None,
        }
    }
    pub fn model(&self) -> &D::State {
        &self.model
    }
    pub fn items(&self) -> &[Item<D::Id>] {
        &self.items
    }
    /// Displayed geometry, including the diagram's optional paint decorations.
    pub fn quads(&self) -> impl Iterator<Item = Quad> + '_ {
        let mut quads = Vec::new();
        for item in &self.items {
            for decoration in &self.decorations {
                if decoration.before.as_ref() == Some(&item.id) {
                    quads.extend(decoration.quads.iter().copied());
                }
            }
            let dragging = self.active.as_ref().is_some_and(|a| a.id == item.id);
            quads.extend(self.diagram.paint(item, dragging));
        }
        for decoration in &self.decorations {
            if decoration
                .before
                .as_ref()
                .is_none_or(|id| !self.items.iter().any(|item| &item.id == id))
            {
                quads.extend(decoration.quads.iter().copied());
            }
        }
        quads.into_iter()
    }
    /// Enable short layout and drop transitions. Disable for reduced motion.
    pub fn set_animated(&mut self, animated: bool) {
        self.animated = animated;
        if !animated && let Some(transition) = self.transition.take() {
            self.items = transition.target;
        }
    }
    pub fn is_animating(&self) -> bool {
        self.transition.is_some()
    }
    /// Advance visual time once per host frame. Returns whether another frame
    /// is needed. Pointer tracking and model commits happen immediately.
    pub fn advance(&mut self, elapsed: Duration) -> bool {
        let Some(transition) = &mut self.transition else {
            return false;
        };
        transition.elapsed += elapsed;
        self.items = transition.frame();
        if transition.finished() {
            self.transition = None;
        }
        self.pin_grabbed_item();
        self.is_animating()
    }
    pub fn size(&self) -> Vec2 {
        self.size
    }
    pub fn is_dragging(&self) -> bool {
        self.active.is_some()
    }
    /// A spec error cancels the gesture. The model stays at its last committed value.
    pub fn last_error(&self) -> Option<&DragError> {
        self.error.as_ref()
    }
    /// Replace the model and discard any in-flight gesture.
    pub fn set_model(&mut self, model: D::State) {
        self.active = None;
        self.error = None;
        self.transition = None;
        self.decorations.clear();
        self.items = self.diagram.layout(&model);
        self.model = model;
    }

    /// Feed primary-pointer events in scene coordinates. Returns whether to repaint.
    pub fn handle(&mut self, event: Pointer) -> bool {
        match self.try_handle(event) {
            Ok(changed) => changed,
            Err(error) => {
                self.error = Some(error);
                self.active = None;
                self.decorations.clear();
                self.retarget(self.diagram.layout(&self.model));
                true
            }
        }
    }

    fn try_handle(&mut self, event: Pointer) -> Result<bool, DragError> {
        match event {
            Pointer::Press(p) if self.active.is_none() => {
                let hit = self
                    .items
                    .iter()
                    .rev()
                    .filter(|i| i.quad.contains(p))
                    .find_map(|i| {
                        Some((
                            i.id.clone(),
                            i.quad.bounds,
                            self.diagram.drag(&self.model, &i.id)?,
                        ))
                    });
                let Some((id, displayed, spec)) = hit else {
                    return Ok(false);
                };
                let diagram = self.diagram.clone();
                let key = id.clone();
                let locate = move |s: &D::State| {
                    let b = diagram
                        .layout(s)
                        .into_iter()
                        .find(|i| i.id == key)?
                        .quad
                        .bounds;
                    // Use unit-box coordinates so an off-center grab survives a size change.
                    Some(
                        Affine::translate(b.origin().to_vec2())
                            * Affine::scale_non_uniform(b.width(), b.height()),
                    )
                };
                // A settling item can be grabbed at its displayed location.
                // Preserve that local anchor even while its model is already committed.
                let local = Affine::scale_non_uniform(displayed.width(), displayed.height())
                    .inverse()
                    * (p - displayed.origin()).to_point();
                let press = locate(&self.model).ok_or(DragError::MissingElement)? * local;
                let preview_diagram = self.diagram.clone();
                let preview_key = id.clone();
                let session = DragSession::begin(self.model.clone(), press, locate, spec)?
                    .with_preview_geometry(move |preview, anchor| {
                        let rendered = resolve(&*preview_diagram, &preview_key, preview);
                        let bounds = rendered
                            .items
                            .iter()
                            .find(|item| item.id == preview_key)?
                            .quad
                            .bounds;
                        Some(
                            Affine::translate(bounds.origin().to_vec2())
                                * Affine::scale_non_uniform(bounds.width(), bounds.height())
                                * anchor,
                        )
                    });
                self.active = Some(Active { id, session });
                self.error = None;
                self.preview(p)?;
            }
            Pointer::Move(p) if self.is_dragging() => self.preview(p)?,
            Pointer::Release(p) if self.is_dragging() => {
                let model = self.active.take().unwrap().session.finish(p)?;
                self.model = model;
                self.decorations.clear();
                self.retarget(self.diagram.layout(&self.model));
            }
            Pointer::Cancel if self.is_dragging() => {
                self.active = None;
                self.decorations.clear();
                self.retarget(self.diagram.layout(&self.model));
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    fn preview(&mut self, point: Point) -> Result<(), DragError> {
        let active = self.active.as_mut().unwrap();
        let frame = active.session.frame(point)?;
        let resolved = resolve(&*self.diagram, &active.id, &frame.preview);
        self.decorations = resolved.decorations;
        self.retarget(resolved.items);
        Ok(())
    }

    fn retarget(&mut self, items: Vec<Item<D::Id>>) {
        if !self.animated {
            self.items = items;
            return;
        }
        if self.transition.as_ref().is_some_and(|t| t.target == items) {
            return;
        }
        self.transition = (self.items != items).then(|| Transition::new(self.items.clone(), items));
        self.pin_grabbed_item();
    }

    fn pin_grabbed_item(&mut self) {
        if let (Some(active), Some(transition)) = (&self.active, &self.transition)
            && let Some(item) = transition.target.iter().find(|i| i.id == active.id)
        {
            self.items.retain(|i| i.id != active.id);
            self.items.push(item.clone());
        }
    }
}

fn resolve<D: Diagram>(diagram: &D, id: &D::Id, preview: &Preview<D::State>) -> Resolved<D::Id> {
    match preview {
        Preview::State(s) => Resolved {
            items: diagram.layout(s),
            decorations: Vec::new(),
        },
        Preview::Blend(parts) => {
            let mut decorations = Vec::new();
            let items = blend::scenes(parts.iter().filter(|(_, w)| *w > 0.).map(|(p, w)| {
                let mut resolved = resolve(diagram, id, p);
                for decoration in &mut resolved.decorations {
                    for quad in &mut decoration.quads {
                        quad.color[3] = (f64::from(quad.color[3]) * w).round() as u8;
                    }
                }
                decorations.extend(resolved.decorations);
                (resolved.items, *w)
            }));
            Resolved { items, decorations }
        }
        Preview::Floating {
            backdrop,
            source,
            translation,
            ghost_opacity,
        } => {
            let mut resolved = resolve(diagram, id, backdrop);
            // The source is frequently the same preview as the backdrop.
            let item = if Rc::ptr_eq(source, backdrop) {
                resolved.items.iter().find(|i| &i.id == id).cloned()
            } else {
                resolve(diagram, id, source)
                    .items
                    .into_iter()
                    .find(|i| &i.id == id)
            };
            if let Some(opacity) = ghost_opacity {
                for (index, backdrop_item) in resolved
                    .items
                    .iter()
                    .enumerate()
                    .filter(|(_, i)| &i.id == id)
                {
                    let mut quads = diagram.paint(backdrop_item, false);
                    for quad in &mut quads {
                        quad.color[3] = (f64::from(quad.color[3]) * opacity).round() as u8;
                    }
                    // Keep the ghost at its original depth. If it was last,
                    // paint it just before the floating foreground item.
                    let before = resolved.items[index + 1..]
                        .iter()
                        .find(|i| &i.id != id)
                        .map(|i| i.id.clone())
                        .or_else(|| item.as_ref().map(|i| i.id.clone()));
                    resolved.decorations.push(Decoration { before, quads });
                }
            }
            resolved.items.retain(|i| &i.id != id);
            if let Some(mut item) = item {
                item.quad.bounds = item.quad.bounds + *translation;
                resolved.items.push(item);
            }
            resolved
        }
    }
}

/// Uniform, centered fit in a host rectangle. Use the same fit for input and painting.
#[derive(Clone, Copy, Debug)]
pub struct Viewport {
    origin: Point,
    scale: f64,
}

impl Viewport {
    pub fn fit(size: Vec2, area: Rect) -> Option<Self> {
        if !area.is_finite() || !size.is_finite() || size.x <= 0. || size.y <= 0. {
            return None;
        }
        let scale = (area.width() / size.x).min(area.height() / size.y);
        if !scale.is_finite() || scale <= 0. {
            return None;
        }
        Some(Self {
            origin: area.center() - size * (scale * 0.5),
            scale,
        })
    }

    pub fn to_scene(self, point: Point) -> Point {
        ((point - self.origin) / self.scale).to_point()
    }
    pub fn to_host(self, point: Point) -> Point {
        self.origin + point.to_vec2() * self.scale
    }
    pub fn quad(self, q: Quad) -> Quad {
        Quad {
            bounds: Rect::from_points(
                self.to_host(q.bounds.origin()),
                self.to_host(Point::new(q.bounds.x1, q.bounds.y1)),
            ),
            radius: q.corner_radius() * self.scale,
            color: q.color,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct GhostSwitch;
    impl Diagram for GhostSwitch {
        type State = bool;
        type Id = &'static str;
        fn layout(&self, on: &bool) -> Vec<Item<Self::Id>> {
            [
                ("back", Rect::new(0., 30., 20., 50.), [10, 10, 10, 255]),
                (
                    "thumb",
                    Rect::new(
                        if *on { 80. } else { 0. },
                        0.,
                        if *on { 100. } else { 20. },
                        20.,
                    ),
                    [200, 30, 40, 255],
                ),
                ("front", Rect::new(40., 30., 60., 50.), [20, 20, 20, 255]),
            ]
            .into_iter()
            .map(|(id, bounds, color)| Item {
                id,
                quad: Quad {
                    bounds,
                    radius: 0.,
                    color,
                },
            })
            .collect()
        }
        fn drag(&self, _: &bool, id: &Self::Id) -> Option<DragSpec<'static, bool>> {
            (*id == "thumb").then(|| {
                dragology::closest([false, true]).with_floating_options(
                    dragology::FloatingOptions {
                        ghost_opacity: Some(0.5),
                        ..Default::default()
                    },
                )
            })
        }
        fn paint(&self, item: &Item<Self::Id>, _: bool) -> Vec<Quad> {
            vec![
                Quad {
                    bounds: item.quad.bounds + Vec2::new(1., 1.),
                    color: [0, 0, 0, 128],
                    ..item.quad
                },
                item.quad,
            ]
        }
    }

    #[test]
    fn floating_ghosts_preserve_unique_ids_pointer_tracking_and_paint_depth() {
        let mut canvas = Canvas::new(GhostSwitch, false, Vec2::new(120., 60.));
        canvas.set_animated(true);
        canvas.handle(Pointer::Press(Point::new(10., 10.)));
        canvas.handle(Pointer::Move(Point::new(90., -30.)));
        for elapsed in [
            Duration::ZERO,
            Duration::from_millis(16),
            Duration::from_millis(80),
        ] {
            canvas.advance(elapsed);
            assert_eq!(
                canvas
                    .items()
                    .iter()
                    .filter(|item| item.id == "thumb")
                    .count(),
                1
            );
            let item = canvas
                .items()
                .iter()
                .find(|item| item.id == "thumb")
                .unwrap();
            assert_eq!(item.quad.bounds.center(), Point::new(90., -30.));
            let quads = canvas.quads().collect::<Vec<_>>();
            assert_eq!(quads.len(), 8);
            // The ghost includes custom paint, below the original front layer.
            assert_eq!(quads[2].color, [0, 0, 0, 64]);
            assert_eq!(quads[3].color, [200, 30, 40, 128]);
            assert_eq!(quads[3].bounds, Rect::new(80., 0., 100., 20.));
            assert_eq!(quads[5].color, [20, 20, 20, 255]);
            assert_eq!(quads[7], item.quad);
        }
        canvas.handle(Pointer::Release(Point::new(90., -30.)));
        assert!(*canvas.model());
        assert_eq!(canvas.quads().count(), 6);
        canvas.advance(Duration::from_secs(1));
        assert_eq!(canvas.items(), GhostSwitch.layout(&true));
        canvas.handle(Pointer::Press(Point::new(90., 10.)));
        canvas.handle(Pointer::Move(Point::new(10., -30.)));
        assert_eq!(canvas.quads().count(), 8);
        canvas.handle(Pointer::Cancel);
        assert_eq!(canvas.quads().count(), 6);
        assert!(*canvas.model());
    }

    #[test]
    fn blending_floating_previews_never_averages_a_ghost_into_the_live_item() {
        let floating = |on, translation| {
            let state = Rc::new(Preview::State(Rc::new(on)));
            Preview::Floating {
                backdrop: state.clone(),
                source: state,
                translation,
                ghost_opacity: Some(0.5),
            }
        };
        let preview = Preview::Blend(vec![
            (Rc::new(floating(false, Vec2::new(40., -40.))), 0.25),
            (Rc::new(floating(true, Vec2::new(-40., -40.))), 0.75),
        ]);
        let resolved = resolve(&GhostSwitch, &"thumb", &preview);
        let thumbs = resolved
            .items
            .iter()
            .filter(|item| item.id == "thumb")
            .collect::<Vec<_>>();
        assert_eq!(thumbs.len(), 1);
        assert_eq!(thumbs[0].quad.bounds, Rect::new(40., -40., 60., -20.));
        assert_eq!(thumbs[0].quad.color[3], 255);
        assert_eq!(resolved.decorations.len(), 2);
        assert_eq!(resolved.decorations[0].quads[1].color[3], 32);
        assert_eq!(resolved.decorations[1].quads[1].color[3], 96);
    }
}
