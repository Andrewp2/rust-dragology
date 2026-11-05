use gpui::{
    MouseButton, PathBuilder, Pixels, ResizeEdge, WindowControlArea, canvas, div, point,
    prelude::*, px, rgb,
};

pub fn window_button(id: &'static str, action: WindowControlArea) -> impl IntoElement {
    div()
        .id(id)
        .flex_none()
        .w(px(44.))
        .h_full()
        .cursor_pointer()
        .hover(move |s| {
            s.bg(rgb(if action == WindowControlArea::Close {
                0xf0c7bb
            } else {
                0xe4e0d4
            }))
        })
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_click(move |_, window, _| match action {
            WindowControlArea::Min => window.minimize_window(),
            WindowControlArea::Max => window.zoom_window(),
            WindowControlArea::Close => window.remove_window(),
            WindowControlArea::Drag => {}
        })
        .child(
            canvas(
                |bounds, _, _| bounds,
                move |_, bounds, window, _| {
                    let c = bounds.center();
                    let p = |x, y| c + point(px(x), px(y));
                    let mut path = PathBuilder::stroke(px(1.4));
                    match action {
                        WindowControlArea::Min => {
                            path.move_to(p(-5., 3.));
                            path.line_to(p(5., 3.));
                        }
                        WindowControlArea::Max => {
                            path.move_to(p(-5., -5.));
                            path.line_to(p(5., -5.));
                            path.line_to(p(5., 5.));
                            path.line_to(p(-5., 5.));
                            path.close();
                        }
                        WindowControlArea::Close => {
                            path.move_to(p(-4., -4.));
                            path.line_to(p(4., 4.));
                            path.move_to(p(4., -4.));
                            path.line_to(p(-4., 4.));
                        }
                        WindowControlArea::Drag => {}
                    }
                    if let Ok(path) = path.build() {
                        window.paint_path(path, rgb(0x51463c));
                    }
                },
            )
            .size_full(),
        )
}

// Client-decorated Linux windows also need draggable resize edges.
pub fn resize_edge(p: gpui::Point<Pixels>, s: gpui::Size<Pixels>) -> Option<ResizeEdge> {
    let left = p.x < px(5.);
    let right = p.x > s.width - px(5.);
    let top = p.y < px(5.);
    let bottom = p.y > s.height - px(5.);
    match (left, right, top, bottom) {
        (true, _, true, _) => Some(ResizeEdge::TopLeft),
        (_, true, true, _) => Some(ResizeEdge::TopRight),
        (true, _, _, true) => Some(ResizeEdge::BottomLeft),
        (_, true, _, true) => Some(ResizeEdge::BottomRight),
        (true, _, _, _) => Some(ResizeEdge::Left),
        (_, true, _, _) => Some(ResizeEdge::Right),
        (_, _, true, _) => Some(ResizeEdge::Top),
        (_, _, _, true) => Some(ResizeEdge::Bottom),
        _ => None,
    }
}
