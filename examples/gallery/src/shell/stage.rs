use super::painter::{self, Fit};
use dragology_gallery::{Affine, ControlPlacement, FontFamily, Presentation, Scene};
use gpui::{
    AnyElement, App, AvailableSpace, Context, MouseButton, MouseMoveEvent, MouseUpEvent, Render,
    ScrollHandle, Window, canvas, div, point, prelude::*, px, size,
};
use std::{cell::Cell, rc::Rc};

type MouseListener<E> = Rc<dyn Fn(&E, &mut Window, &mut App)>;

/// Keep vector paint when unrelated gallery controls change. Inline native
/// widgets are laid out against the final (including floating) object geometry.
pub struct Stage {
    pub scene: Scene,
    pub fit: Rc<Cell<Option<Fit>>>,
    pub on_move: MouseListener<MouseMoveEvent>,
    pub on_up: MouseListener<MouseUpEvent>,
    pub controls: Vec<(ControlPlacement, AnyElement)>,
    pub content_width: f64,
    pub scroll: ScrollHandle,
}

impl Render for Stage {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let scene = self.scene.clone();
        let scrolling = scene.presentation == Presentation::Scroll;
        let content_height =
            scene.height * ((self.content_width - 40.).max(1.) / scene.width).min(1.) + 40.;
        let fit_cell = self.fit.clone();
        let on_move = self.on_move.clone();
        let on_up = self.on_up.clone();
        let controls = std::mem::take(&mut self.controls);
        let stage = canvas(
            move |bounds, window, cx| {
                let fit = Fit::new(&scene, bounds);
                fit_cell.set(fit);
                let mut widgets = Vec::new();
                if let Some(fit) = fit {
                    for (placement, element) in controls {
                        let (parent, opacity) = match &placement.parent {
                            Some(id) => match scene.object(id) {
                                Some(object) if object.opacity > 0. => {
                                    (object.transform, object.opacity)
                                }
                                _ => continue,
                            },
                            None => (Affine::IDENTITY, 1.),
                        };
                        let transform = fit.transform * parent;
                        let rect = transform.transform_rect_bbox(placement.bounds);
                        let scale = (transform.as_coeffs()[2].powi(2)
                            + transform.as_coeffs()[3].powi(2))
                        .sqrt();
                        let dimensions = size(px(rect.width() as f32), px(rect.height() as f32));
                        let mut widget = div()
                            .w(dimensions.width)
                            .h(dimensions.height)
                            .text_size(
                                px((placement.style.font_size.unwrap_or(13.) * scale) as f32),
                            )
                            .when_some(placement.style.font_family.clone(), |view, family| {
                                view.font_family(match family {
                                    FontFamily::Sans => "sans-serif".into(),
                                    FontFamily::Serif => "serif".into(),
                                    FontFamily::Mono => "monospace".into(),
                                    FontFamily::Named(name) => gpui::SharedString::from(name),
                                })
                            })
                            .opacity(opacity as f32)
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .child(element)
                            .into_any_element();
                        widget.prepaint_as_root(
                            point(px(rect.x0 as f32), px(rect.y0 as f32)),
                            size(
                                AvailableSpace::Definite(dimensions.width),
                                AvailableSpace::Definite(dimensions.height),
                            ),
                            window,
                            cx,
                        );
                        widgets.push(widget);
                    }
                }
                (bounds, fit, scene.clone(), widgets)
            },
            move |_, (bounds, fit, scene, mut widgets), window, cx| {
                window.on_mouse_event(move |event, phase, window, cx| {
                    if phase.capture() {
                        on_move(event, window, cx);
                    }
                });
                window.on_mouse_event(move |event, phase, window, cx| {
                    if phase.capture() {
                        on_up(event, window, cx);
                    }
                });
                if let Some(fit) = fit {
                    painter::paint(&scene, fit, bounds, window, cx);
                }
                for widget in &mut widgets {
                    widget.paint(window, cx);
                }
            },
        )
        .w_full()
        .h_full()
        .when(scrolling, |view| view.h(px(content_height as f32)));
        div()
            .id("scene-viewport")
            .size_full()
            .when(scrolling, |view| {
                view.overflow_y_scroll().track_scroll(&self.scroll)
            })
            .child(stage)
    }
}
