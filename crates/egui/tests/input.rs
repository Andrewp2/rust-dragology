use dragology_egui::{scene::*, show};
use egui::{Context, Event, Pos2, RawInput, Response, vec2};

struct Switch;
impl Diagram for Switch {
    type Id = u8;
    type State = bool;

    fn layout(&self, on: &bool) -> Vec<Item<u8>> {
        let x = if *on { 80. } else { 0. };
        vec![Item {
            id: 0,
            quad: Quad {
                bounds: Rect::new(x, 0., x + 20., 40.),
                radius: 0.,
                color: [255; 4],
            },
        }]
    }

    fn drag(&self, _: &bool, _: &u8) -> Option<DragSpec<'static, bool>> {
        Some(dragology::between([false, true]))
    }
}

fn frame(
    ctx: &Context,
    canvas: &mut Canvas<Switch>,
    events: Vec<Event>,
    focused: bool,
) -> Response {
    let mut response = None;
    let mut output = ctx.run_ui(
        RawInput {
            screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, vec2(300., 200.))),
            events,
            focused,
            ..Default::default()
        },
        |ui| {
            // Exercise a canvas embedded away from the window origin.
            ui.add_space(30.);
            ui.horizontal(|ui| {
                ui.add_space(40.);
                response = Some(show(ui, canvas, vec2(100., 40.)));
            });
        },
    );
    // This harness exercises input without a GPU texture manager.
    output.textures_delta.clear();
    response.unwrap()
}

fn button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Default::default(),
    }
}

#[test]
fn embedded_drag_releases_outside_the_widget_and_marks_the_response_changed() {
    let ctx = Context::default();
    let mut canvas = Canvas::new(Switch, false, Vec2::new(100., 40.));
    let rect = frame(&ctx, &mut canvas, vec![], true).rect;
    let start = rect.min + vec2(10., 20.);
    frame(
        &ctx,
        &mut canvas,
        vec![Event::PointerMoved(start), button(start, true)],
        true,
    );
    let end = rect.min + vec2(90., 80.);
    assert!(!rect.contains(end));
    frame(&ctx, &mut canvas, vec![Event::PointerMoved(end)], true);
    assert!(canvas.is_dragging());
    assert!(!canvas.model());
    let response = frame(&ctx, &mut canvas, vec![button(end, false)], true);
    assert!(response.changed());
    assert!(*canvas.model());
    assert!(!canvas.is_dragging());
    assert!(canvas.last_error().is_none());
}

#[test]
fn escape_and_focus_loss_cancel_without_committing_the_preview() {
    for focus_loss in [false, true] {
        let ctx = Context::default();
        let mut canvas = Canvas::new(Switch, false, Vec2::new(100., 40.));
        let rect = frame(&ctx, &mut canvas, vec![], true).rect;
        let start = rect.min + vec2(10., 20.);
        frame(
            &ctx,
            &mut canvas,
            vec![Event::PointerMoved(start), button(start, true)],
            true,
        );
        frame(
            &ctx,
            &mut canvas,
            vec![Event::PointerMoved(rect.min + vec2(90., 20.))],
            true,
        );
        assert!(canvas.is_dragging());
        let events = if focus_loss {
            vec![]
        } else {
            vec![Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Default::default(),
            }]
        };
        let response = frame(&ctx, &mut canvas, events, !focus_loss);
        assert!(!response.changed());
        assert!(!canvas.is_dragging());
        assert!(!canvas.model());
        assert_eq!(canvas.items(), Switch.layout(&false));
    }
}
