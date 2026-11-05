use dragology_gpui::CanvasView;
use gpui::{
    App, Application, Bounds, Context, Entity, FocusHandle, MouseButton, PathBuilder, Pixels,
    ResizeEdge, Subscription, Window, WindowBounds, WindowControlArea, WindowDecorations,
    WindowOptions, canvas, div, point, prelude::*, px, rgb, size,
};

struct HanoiWindow {
    board: Entity<CanvasView<hanoi::Hanoi>>,
    focus: FocusHandle,
    _board_changes: Subscription,
}

fn window_button(id: &'static str, action: WindowControlArea) -> impl IntoElement {
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
fn resize_edge(p: gpui::Point<Pixels>, s: gpui::Size<Pixels>) -> Option<ResizeEdge> {
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

impl Render for HanoiWindow {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let status = self.board.read(cx).canvas.model().status();
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0xf6f4eb))
            .text_color(rgb(0x51463c))
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                if event.keystroke.key == "r" && !event.keystroke.modifiers.modified() {
                    this.board.update(cx, |view, cx| {
                        view.canvas.set_model(Default::default());
                        cx.notify();
                    });
                    cx.stop_propagation();
                }
            }))
            .when(cfg!(target_os = "linux"), |view| {
                view.capture_any_mouse_down(|event, window, cx| {
                    if event.button == MouseButton::Left
                        && !window.is_maximized()
                        && let Some(edge) = resize_edge(event.position, window.viewport_size())
                    {
                        window.start_window_resize(edge);
                        cx.stop_propagation();
                    }
                })
                .child(
                    div()
                        .flex()
                        .flex_none()
                        .h(px(40.))
                        .w_full()
                        .bg(rgb(0xeeeade))
                        .child(
                            div()
                                .flex()
                                .flex_1()
                                .min_w_0()
                                .h_full()
                                .items_center()
                                .px_4()
                                .text_sm()
                                .child(hanoi::TITLE)
                                .on_mouse_down(MouseButton::Left, |event, window, _| {
                                    if event.click_count == 2 {
                                        window.zoom_window();
                                    } else {
                                        window.start_window_move();
                                    }
                                })
                                .on_mouse_down(MouseButton::Right, |event, window, _| {
                                    window.show_window_menu(event.position);
                                }),
                        )
                        .child(window_button("Minimize", WindowControlArea::Min))
                        .child(window_button("Maximize", WindowControlArea::Max))
                        .child(window_button("Close", WindowControlArea::Close)),
                )
            })
            .child(div().flex_1().min_h_0().child(self.board.clone()))
            .child(
                div()
                    .flex()
                    .flex_none()
                    .h(px(40.))
                    .px_4()
                    .items_center()
                    .text_xs()
                    .child(status),
            )
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        let height = hanoi::HEIGHT + 40. + if cfg!(target_os = "linux") { 40. } else { 0. };
        let bounds = Bounds::centered(None, size(px(hanoi::WIDTH as f32), px(height as f32)), cx);
        let mut options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            window_min_size: Some(size(px(540.), px(340.))),
            ..Default::default()
        };
        if cfg!(target_os = "linux") {
            options.titlebar = None;
            options.window_decorations = Some(WindowDecorations::Client);
        }
        cx.open_window(options, |window, cx| {
            window.set_window_title(hanoi::TITLE);
            cx.new(|cx| {
                let board = cx.new(|cx| CanvasView::new(hanoi::board(), window, cx));
                let subscription = cx.observe(&board, |_, _, cx| cx.notify());
                let focus = cx.focus_handle();
                focus.focus(window);
                HanoiWindow {
                    board,
                    focus,
                    _board_changes: subscription,
                }
            })
        })
        .expect("open Hanoi window");
        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit()
            }
        })
        .detach();
        cx.activate(true);
    });
}
