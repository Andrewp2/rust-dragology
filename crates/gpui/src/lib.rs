//! A GPUI view for a Dragology canvas. Embed its entity as a child of another view.
pub use dragology_scene as scene;
use gpui::{
    Bounds, Context, FocusHandle, KeyDownEvent, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, Pixels, Subscription, Window, canvas, div, fill, point, prelude::*, px, rgba,
    size,
};
use scene::{Canvas, Diagram, Point, Pointer, Rect, Viewport};
use std::{cell::Cell, rc::Rc, time::Instant};

pub struct CanvasView<D: Diagram> {
    pub canvas: Canvas<D>,
    viewport: Rc<Cell<Option<Viewport>>>,
    focus: FocusHandle,
    _activation: Subscription,
    last_frame: Instant,
}

impl<D: Diagram> CanvasView<D> {
    pub fn new(canvas: Canvas<D>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let activation = cx.observe_window_activation(window, |this, window, cx| {
            if !window.is_window_active() {
                this.handle(Pointer::Cancel, cx);
            }
        });
        Self {
            canvas,
            viewport: Default::default(),
            focus: cx.focus_handle(),
            _activation: activation,
            last_frame: Instant::now(),
        }
    }

    fn pointer(
        &mut self,
        p: gpui::Point<Pixels>,
        event: fn(Point) -> Pointer,
        cx: &mut Context<Self>,
    ) -> bool {
        let input = match self.viewport.get() {
            Some(view) => event(view.to_scene(Point::new(p.x.into(), p.y.into()))),
            None => Pointer::Cancel,
        };
        self.handle(input, cx)
    }

    fn handle(&mut self, input: Pointer, cx: &mut Context<Self>) -> bool {
        if !self.canvas.is_animating() {
            self.last_frame = Instant::now();
        }
        let changed = self.canvas.handle(input);
        if changed {
            cx.notify();
        }
        changed
    }
}

impl<D: Diagram> Render for CanvasView<D> {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let now = Instant::now();
        if self.canvas.advance(now - self.last_frame) {
            window.request_animation_frame();
        }
        self.last_frame = now;
        let quads: Vec<_> = self.canvas.quads().collect();
        let scene_size = self.canvas.size();
        let viewport = self.viewport.clone();
        let motion = cx.listener(|this, e: &MouseMoveEvent, _, cx| {
            if this.canvas.is_dragging() {
                if e.pressed_button == Some(MouseButton::Left) {
                    this.pointer(e.position, Pointer::Move, cx);
                } else {
                    this.handle(Pointer::Cancel, cx);
                }
            }
        });
        let release = cx.listener(|this, e: &MouseUpEvent, _, cx| {
            if e.button == MouseButton::Left && this.canvas.is_dragging() {
                this.pointer(e.position, Pointer::Release, cx);
            }
        });
        div()
            .size_full()
            .overflow_hidden()
            .track_focus(&self.focus)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, e: &MouseDownEvent, w, cx| {
                    if this.pointer(e.position, Pointer::Press, cx) {
                        this.focus.focus(w);
                        cx.stop_propagation();
                    }
                }),
            )
            .on_key_down(cx.listener(|this, e: &KeyDownEvent, _, cx| {
                if e.keystroke.key == "escape" && this.handle(Pointer::Cancel, cx) {
                    cx.stop_propagation();
                }
            }))
            .child(
                canvas(
                    move |bounds, _, _| {
                        let rect = Rect::from_origin_size(
                            (f64::from(bounds.origin.x), f64::from(bounds.origin.y)),
                            (f64::from(bounds.size.width), f64::from(bounds.size.height)),
                        );
                        let view = Viewport::fit(scene_size, rect);
                        viewport.set(view);
                        view
                    },
                    move |_, view, window, _| {
                        // The hit-tested element starts the gesture. Window listeners keep
                        // following it after the mouse leaves an embedded canvas.
                        window.on_mouse_event(move |e, phase, w, cx| {
                            if phase.capture() {
                                motion(e, w, cx)
                            }
                        });
                        window.on_mouse_event(move |e, phase, w, cx| {
                            if phase.capture() {
                                release(e, w, cx)
                            }
                        });
                        if let Some(view) = view {
                            for q in quads {
                                let q = view.quad(q);
                                let b = q.bounds;
                                let bounds = Bounds::new(
                                    point(px(b.x0 as f32), px(b.y0 as f32)),
                                    size(px(b.width() as f32), px(b.height() as f32)),
                                );
                                let mut quad = fill(bounds, rgba(u32::from_be_bytes(q.color)));
                                quad.corner_radii = px(q.radius as f32).into();
                                window.paint_quad(quad);
                            }
                        }
                    },
                )
                .size_full(),
            )
    }
}
