//! An embeddable egui widget for a Dragology canvas.
pub use dragology_scene as scene;
use scene::{Canvas, Diagram, Point, Pointer, Rect, Viewport};

/// Allocate `size` logical egui points. The returned response belongs to the canvas.
pub fn show<D: Diagram>(
    ui: &mut egui::Ui,
    canvas: &mut Canvas<D>,
    size: egui::Vec2,
) -> egui::Response {
    canvas.advance(std::time::Duration::from_secs_f32(
        ui.input(|i| i.stable_dt.min(0.05)),
    ));
    let (rect, mut response) = ui.allocate_exact_size(size, egui::Sense::drag());
    let area = Rect::new(
        rect.min.x.into(),
        rect.min.y.into(),
        rect.max.x.into(),
        rect.max.y.into(),
    );
    let Some(view) = Viewport::fit(canvas.size(), area) else {
        canvas.handle(Pointer::Cancel);
        return response;
    };
    let point = |p: egui::Pos2| view.to_scene(Point::new(p.x.into(), p.y.into()));
    let button = egui::PointerButton::Primary;
    let (focused, escape, origin, down, released, pos) = ui.input(|i| {
        (
            i.focused,
            i.key_pressed(egui::Key::Escape),
            i.pointer.press_origin(),
            i.pointer.button_down(button),
            i.pointer.button_released(button),
            i.pointer.interact_pos(),
        )
    });
    // Raw input positions are global, while an embedded widget can live on a transformed layer.
    let from_global = ui
        .ctx()
        .layer_transform_from_global(response.layer_id)
        .unwrap_or_default();
    if !focused || escape {
        canvas.handle(Pointer::Cancel);
    } else {
        if response.drag_started_by(button)
            && let Some(origin) = origin
        {
            canvas.handle(Pointer::Press(point(from_global * origin)));
        }
        if canvas.is_dragging() {
            match (pos, released, down) {
                (Some(p), true, _) => {
                    canvas.handle(Pointer::Release(point(from_global * p)));
                    if canvas.last_error().is_none() {
                        response.mark_changed();
                    }
                }
                (Some(p), _, true) => {
                    canvas.handle(Pointer::Move(point(from_global * p)));
                }
                (_, _, false) => {
                    canvas.handle(Pointer::Cancel);
                }
                _ => {}
            }
        }
    }
    let painter = ui.painter_at(rect);
    if canvas.is_animating() {
        ui.ctx().request_repaint();
    }
    for quad in canvas.quads() {
        let q = view.quad(quad);
        let b = q.bounds;
        let rect = egui::Rect::from_min_max(
            egui::pos2(b.x0 as f32, b.y0 as f32),
            egui::pos2(b.x1 as f32, b.y1 as f32),
        );
        let [r, g, b, a] = q.color;
        painter.rect_filled(
            rect,
            q.radius as f32,
            egui::Color32::from_rgba_unmultiplied(r, g, b, a),
        );
    }
    response
}
