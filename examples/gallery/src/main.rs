mod shell {
    pub mod chrome;
    pub mod input;
    pub mod notes;
    pub mod painter;
    pub mod stage;
    pub mod svg;
    pub mod text;
}

use dragology_gallery::{Control, ControlPlacement, Demo, Entry, Modifiers, Point, Value};
use gpui::{
    App, Application, Bounds, Context, Entity, FocusHandle, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, PathPromptOptions, Pixels, ScrollHandle, SharedString,
    Subscription, Window, WindowBounds, WindowControlArea, WindowDecorations, WindowOptions,
    canvas, div, fill, point, prelude::*, px, rgb, size,
};
use shell::{
    chrome,
    input::{InputEvent, TextInput},
    painter::{self, Fit},
    stage::Stage,
};
use std::{cell::Cell, collections::HashMap, path::Path, rc::Rc, time::Instant};

fn modifiers(m: gpui::Modifiers) -> Modifiers {
    Modifiers {
        alt: m.alt,
        shift: m.shift,
        control: m.control || m.platform,
    }
}

fn button(
    id: impl Into<gpui::ElementId>,
    label: impl Into<SharedString>,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .px_3()
        .py_1()
        .rounded_md()
        .bg(rgb(0xece9e0))
        .text_sm()
        .whitespace_normal()
        .max_w_full()
        .min_w_0()
        .text_color(rgb(0x3e4546))
        .cursor_pointer()
        .hover(|s| s.bg(rgb(0xdedfd8)))
        .active(|s| s.bg(rgb(0xd0d5cf)))
        .child(label.into())
}

fn inherited_button_text(mut view: gpui::Stateful<gpui::Div>) -> gpui::Stateful<gpui::Div> {
    view.text_style()
        .get_or_insert_with(Default::default)
        .font_size = None;
    view
}

#[derive(Clone, Copy)]
struct SliderDrag {
    id: &'static str,
    bounds: Bounds<Pixels>,
    min: f64,
    max: f64,
    step: f64,
}
impl SliderDrag {
    fn value(self, position: gpui::Point<Pixels>) -> f64 {
        let fraction =
            f64::from(position.x - self.bounds.left()) / f64::from(self.bounds.size.width).max(1.);
        let raw = self.min + fraction.clamp(0., 1.) * (self.max - self.min);
        let value = if self.step > 0. {
            self.min + ((raw - self.min) / self.step).round() * self.step
        } else {
            raw
        };
        value.clamp(self.min, self.max)
    }
}

struct Gallery {
    entries: Vec<Entry>,
    selected: usize,
    demo: Box<dyn Demo>,
    animated: bool,
    focus: FocusHandle,
    search: Entity<TextInput>,
    query: String,
    sidebar_visible: bool,
    fields: HashMap<&'static str, Entity<TextInput>>,
    code_scrolls: HashMap<&'static str, ScrollHandle>,
    choice_open: Option<&'static str>,
    choice_highlight: usize,
    field_subscriptions: Vec<Subscription>,
    _search_subscription: Subscription,
    _activation: Subscription,
    sidebar_scroll: ScrollHandle,
    controls_scroll: ScrollHandle,
    notes_scroll: ScrollHandle,
    fit: Rc<Cell<Option<Fit>>>,
    stage: Entity<Stage>,
    pointer: Option<Point>,
    held: bool,
    slider: Option<SliderDrag>,
    last_frame: Instant,
    message: String,
}

impl Gallery {
    fn new(
        entries: Vec<Entry>,
        selected: usize,
        animated: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut demo = (entries[selected].create)();
        demo.set_animated(animated);
        let focus = cx.focus_handle();
        let search =
            cx.new(|cx| TextInput::new(String::new(), "Find a demo".into(), focus.clone(), cx));
        let search_subscription = cx.subscribe(&search, |this, _, event, cx| {
            match event {
                InputEvent::Changed(text) => {
                    this.query = text.to_lowercase();
                    this.sidebar_scroll.scroll_to_item(0);
                }
                InputEvent::Submit => {
                    let first_match = this.matches().next();
                    if let Some(index) = first_match {
                        this.select(index, cx);
                    }
                }
                InputEvent::Cancel => {}
            }
            cx.notify();
        });
        let activation = cx.observe_window_activation(window, |this, window, cx| {
            if !window.is_window_active() {
                this.advance_to_now();
                this.demo.cancel();
                this.held = false;
                this.slider = None;
                cx.notify();
            }
        });
        focus.focus(window);
        let fit = Rc::new(Cell::new(None));
        let on_move = Rc::new(cx.listener(Self::pointer_move));
        let on_up = Rc::new(cx.listener(Self::pointer_up));
        let stage = cx.new(|_| Stage {
            scene: demo.scene().clone(),
            fit: fit.clone(),
            on_move,
            on_up,
            controls: Vec::new(),
            content_width: 0.,
            scroll: ScrollHandle::new(),
        });
        Self {
            entries,
            selected,
            demo,
            animated,
            focus,
            search,
            query: String::new(),
            sidebar_visible: true,
            fields: HashMap::new(),
            code_scrolls: HashMap::new(),
            choice_open: None,
            choice_highlight: 0,
            field_subscriptions: Vec::new(),
            _search_subscription: search_subscription,
            _activation: activation,
            sidebar_scroll: ScrollHandle::new(),
            controls_scroll: ScrollHandle::new(),
            notes_scroll: ScrollHandle::new(),
            fit,
            stage,
            pointer: None,
            held: false,
            slider: None,
            last_frame: Instant::now(),
            message: String::new(),
        }
    }

    fn advance_to_now(&mut self) -> bool {
        let now = Instant::now();
        let animating = self.demo.advance(now - self.last_frame);
        self.last_frame = now;
        animating
    }

    fn matches(&self) -> impl Iterator<Item = usize> + '_ {
        self.entries
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                (self.query.is_empty()
                    || entry.id.contains(&self.query)
                    || entry.title.to_lowercase().contains(&self.query)
                    || entry.description.to_lowercase().contains(&self.query))
                .then_some(index)
            })
    }

    fn select(&mut self, index: usize, cx: &mut Context<Self>) {
        self.demo.cancel();
        self.selected = index;
        self.demo = (self.entries[index].create)();
        self.demo.set_animated(self.animated);
        self.fields.clear();
        self.code_scrolls.clear();
        self.choice_open = None;
        self.field_subscriptions.clear();
        self.controls_scroll.scroll_to_item(0);
        self.notes_scroll.scroll_to_item(0);
        self.stage
            .update(cx, |stage, _| stage.scroll.scroll_to_item(0));
        self.fit.set(None);
        self.pointer = None;
        self.held = false;
        self.slider = None;
        self.message.clear();
        self.last_frame = Instant::now();
        cx.notify();
    }

    fn action(&mut self, id: &str, value: Value, cx: &mut Context<Self>) {
        self.choice_open = None;
        self.advance_to_now();
        self.demo.action(id, value);
        self.message.clear();
        cx.notify();
    }

    fn field(
        &mut self,
        id: &'static str,
        label: String,
        value: String,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Entity<TextInput> {
        if let Some(field) = self.fields.get(id) {
            field.update(cx, |field, cx| {
                // Preserve an in-progress edit (such as an incomplete numeric
                // range) until it is accepted or focus leaves the field.
                if !field.focus.is_focused(window) {
                    field.set_value(value, cx);
                }
            });
            return field.clone();
        }
        let field = cx.new(|cx| TextInput::new(value, label, self.focus.clone(), cx));
        self.field_subscriptions.push(cx.subscribe(
            &field,
            move |this, _, event, cx| match event {
                InputEvent::Changed(text) => this.action(id, Value::Text(text.clone()), cx),
                InputEvent::Submit => {
                    this.advance_to_now();
                    this.demo.key("enter", Modifiers::default());
                    cx.notify();
                }
                InputEvent::Cancel => {
                    this.advance_to_now();
                    this.demo.cancel();
                    this.held = false;
                    this.slider = None;
                    cx.notify();
                }
            },
        ));
        self.fields.insert(id, field.clone());
        field
    }

    fn text_focused(&self, window: &Window, cx: &App) -> bool {
        self.search.read(cx).focus.is_focused(window)
            || self
                .fields
                .values()
                .any(|input| input.read(cx).focus.is_focused(window))
    }

    fn pointer_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(slider) = self.slider {
            if event.pressed_button == Some(MouseButton::Left) {
                self.action(slider.id, Value::Number(slider.value(event.position)), cx);
            } else {
                self.slider = None;
            }
            return;
        }
        let Some(fit) = self.fit.get() else { return };
        let p = fit.to_scene(event.position);
        self.pointer = Some(p);
        if self.held && event.pressed_button != Some(MouseButton::Left) {
            self.demo.cancel();
            self.held = false;
        } else {
            self.demo.move_to(p, modifiers(event.modifiers));
        }
        cx.notify();
    }

    fn pointer_up(&mut self, event: &MouseUpEvent, _: &mut Window, cx: &mut Context<Self>) {
        if event.button != MouseButton::Left {
            return;
        }
        self.slider = None;
        if self.held {
            self.advance_to_now();
            if let Some(fit) = self.fit.get() {
                self.demo
                    .release(fit.to_scene(event.position), modifiers(event.modifiers));
            } else {
                self.demo.cancel();
            }
            self.held = false;
            cx.notify();
        }
    }

    fn save(&mut self, cx: &mut Context<Self>) {
        let Some((filename, text)) = self.demo.export() else {
            return;
        };
        let directory = std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());
        let selection = cx.prompt_for_new_path(&directory, Some(&filename));
        cx.spawn(async move |this, cx| {
            let result = match selection.await {
                Ok(Ok(Some(path))) => std::fs::write(path, text)
                    .map(|_| "Saved.".to_string())
                    .map_err(|e| e.to_string()),
                Ok(Ok(None)) => return,
                Ok(Err(error)) => Err(error.to_string()),
                Err(error) => Err(error.to_string()),
            };
            let _ = this.update(cx, |this, cx| {
                this.message = result.unwrap_or_else(|e| format!("Could not save. {e}"));
                cx.notify();
            });
        })
        .detach();
    }

    fn load(&mut self, cx: &mut Context<Self>) {
        let selected = self.selected;
        let selection = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Load model".into()),
        });
        cx.spawn(async move |this, cx| {
            let result = match selection.await {
                Ok(Ok(Some(paths))) => {
                    let Some(path) = paths.first() else { return };
                    std::fs::read_to_string(path).map_err(|e| e.to_string())
                }
                Ok(Ok(None)) => return,
                Ok(Err(error)) => Err(error.to_string()),
                Err(error) => Err(error.to_string()),
            };
            let _ = this.update(cx, |this, cx| {
                if this.selected != selected {
                    this.message = "Choose the original demo before loading its model.".into();
                } else {
                    this.message = match result.and_then(|text| this.demo.import(&text)) {
                        Ok(()) => "Loaded.".into(),
                        Err(error) => format!("Could not load. {error}"),
                    };
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn control(
        &mut self,
        control: Control,
        placement: Option<&ControlPlacement>,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let inline = placement.is_some();
        let style = placement.map(|p| p.style.clone()).unwrap_or_default();
        match control {
            Control::ActionButton {
                id,
                label,
                enabled,
                selected,
            } => button(id, label)
                .when(selected, |view| view.bg(rgb(0xd9e3e0)))
                .when(!enabled, |view| view.opacity(0.45).cursor_default())
                .when(inline, |mut view| {
                    view.text_style()
                        .get_or_insert_with(Default::default)
                        .font_size = None;
                    view.size_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .px_0()
                        .py_0()
                })
                .when_some(style.background, |view, color| {
                    view.bg(painter::color(color, 1.))
                })
                .when_some(style.foreground, |view, color| {
                    view.text_color(painter::color(color, 1.))
                })
                .when_some(style.border, |view, color| {
                    view.border_1().border_color(painter::color(color, 1.))
                })
                .on_click(cx.listener(move |this, _, window, cx| {
                    if enabled {
                        this.focus.focus(window);
                        this.action(id, Value::Trigger, cx);
                    }
                }))
                .into_any_element(),
            Control::Button { id, label } => button(id, label)
                .when(inline, |mut view| {
                    view.text_style()
                        .get_or_insert_with(Default::default)
                        .font_size = None;
                    view.size_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .px_0()
                        .py_0()
                })
                .when_some(style.background, |view, color| {
                    view.bg(painter::color(color, 1.))
                })
                .when_some(style.foreground, |view, color| {
                    view.text_color(painter::color(color, 1.))
                })
                .when_some(style.border, |view, color| {
                    view.border_1().border_color(painter::color(color, 1.))
                })
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.focus.focus(window);
                    this.action(id, Value::Trigger, cx);
                }))
                .into_any_element(),
            Control::Toggle { id, label, value } => div()
                .id(id)
                .flex()
                .gap_2()
                .items_start()
                .cursor_pointer()
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.focus.focus(window);
                    this.action(id, Value::Bool(!value), cx);
                }))
                .child(
                    div()
                        .flex_none()
                        .size(px(16.))
                        .mt(px(2.))
                        .rounded_sm()
                        .border_1()
                        .border_color(rgb(0x8b999b))
                        .bg(rgb(if value { 0x466c71 } else { 0xffffff })),
                )
                .child(div().when(!inline, |view| view.text_sm()).child(label))
                .into_any_element(),
            Control::Choice {
                id,
                label: _,
                options,
                selected,
            } if style.select => {
                let opened = self.choice_open == Some(id);
                let highlighted = if opened {
                    self.choice_highlight
                } else {
                    selected
                };
                let selected_label = format!(
                    "{}  ▾",
                    options.get(selected).map(String::as_str).unwrap_or("")
                );
                let menu = opened.then(|| {
                    div()
                        .id(SharedString::from(format!("select-menu-{id}")))
                        .absolute()
                        .top_0()
                        .left_0()
                        .w_full()
                        .bg(rgb(0xffffff))
                        .border_1()
                        .border_color(rgb(0x9ca3af))
                        .rounded_sm()
                        .occlude()
                        .on_mouse_down_out(cx.listener(move |this, _, _, cx| {
                            if this.choice_open == Some(id) {
                                this.choice_open = None;
                                cx.notify();
                            }
                        }))
                        .child(
                            button(
                                SharedString::from(format!("select-close-{id}")),
                                selected_label.clone(),
                            )
                            .w_full()
                            .rounded_none()
                            .bg(rgb(0xffffff))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.choice_open = None;
                                cx.notify();
                            })),
                        )
                        .children(options.into_iter().enumerate().map(|(index, label)| {
                            button((id, index), label)
                                .when(inline, inherited_button_text)
                                .w_full()
                                .rounded_none()
                                .bg(rgb(if index == highlighted {
                                    0xe0ebf3
                                } else {
                                    0xffffff
                                }))
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.focus.focus(window);
                                    this.action(id, Value::Index(index), cx);
                                }))
                        }))
                });
                div()
                    .id(SharedString::from(format!("select-{id}")))
                    .relative()
                    .size_full()
                    .child(
                        button(id, selected_label)
                            .when(inline, inherited_button_text)
                            .size_full()
                            .px_2()
                            .py_0()
                            .bg(rgb(0xffffff))
                            .border_1()
                            .border_color(rgb(0x9ca3af))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.focus.focus(window);
                                this.choice_open = (!opened).then_some(id);
                                this.choice_highlight = selected;
                                cx.notify();
                            })),
                    )
                    .when_some(menu, |view, menu| view.child(menu))
                    .into_any_element()
            }
            Control::Choice {
                id,
                label,
                options,
                selected,
            } => div()
                .flex()
                .flex_col()
                .gap_2()
                .child(div().text_sm().child(label))
                .child(div().flex().flex_wrap().gap_1().children(
                    options.into_iter().enumerate().map(|(index, label)| {
                        button((id, index), label)
                            .when(index == selected, |s| s.bg(rgb(0xd9e3e0)))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.focus.focus(window);
                                this.action(id, Value::Index(index), cx)
                            }))
                    }),
                ))
                .into_any_element(),
            Control::Code { id, label, content } => {
                let scroll = self
                    .code_scrolls
                    .entry(id)
                    .or_insert_with(ScrollHandle::new);
                div()
                    .id(id)
                    .size_full()
                    .overflow_scroll()
                    .track_scroll(scroll)
                    .p_3()
                    .font_family("monospace")
                    .text_size(px(12.))
                    .text_color(rgb(0x374151))
                    .bg(rgb(0xf9fafb))
                    .border_1()
                    .border_color(rgb(0xe5e7eb))
                    .when(!inline, |view| view.child(div().mb_2().child(label)))
                    .child(div().whitespace_nowrap().child(content))
                    .into_any_element()
            }
            Control::Text { id, label, value } => {
                let field = self.field(id, label.clone(), value, window, cx);
                field.update(cx, |field, cx| field.set_appearance(style, inline, cx));
                if inline {
                    return field.into_any_element();
                }
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(div().text_sm().child(label))
                    .child(field)
                    .into_any_element()
            }
            Control::Slider {
                id,
                label,
                value,
                min,
                max,
                step,
            } => {
                let bounds = Rc::new(Cell::new(None::<Bounds<Pixels>>));
                let captured = bounds.clone();
                let fraction = ((value - min) / (max - min).max(f64::EPSILON)).clamp(0., 1.);
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .when(!inline, |view| {
                        view.child(
                            div()
                                .flex()
                                .justify_between()
                                .text_sm()
                                .child(label)
                                .child(format!("{value:.2}")),
                        )
                    })
                    .child(
                        div()
                            .id(id)
                            .h(px(24.))
                            .w_full()
                            .cursor_pointer()
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                                    if let Some(bounds) = captured.get() {
                                        let drag = SliderDrag {
                                            id,
                                            bounds,
                                            min,
                                            max,
                                            step,
                                        };
                                        this.slider = Some(drag);
                                        this.demo.cancel();
                                        this.held = false;
                                        this.focus.focus(window);
                                        this.action(
                                            id,
                                            Value::Number(drag.value(event.position)),
                                            cx,
                                        );
                                        cx.stop_propagation();
                                    }
                                }),
                            )
                            .child(
                                canvas(
                                    move |area, _, _| {
                                        bounds.set(Some(area));
                                        area
                                    },
                                    move |_, bounds, window, _| {
                                        let y = bounds.center().y - px(2.);
                                        let track = Bounds::new(
                                            point(bounds.left(), y),
                                            size(bounds.size.width, px(4.)),
                                        );
                                        let mut background = fill(track, rgb(0xd5d7cf));
                                        background.corner_radii = px(2.).into();
                                        window.paint_quad(background);
                                        let filled = Bounds::new(
                                            track.origin,
                                            size(
                                                track.size.width * fraction as f32,
                                                track.size.height,
                                            ),
                                        );
                                        window.paint_quad(fill(filled, rgb(0x5f8584)));
                                        let mut thumb = fill(
                                            Bounds::new(
                                                point(
                                                    bounds.left()
                                                        + bounds.size.width * fraction as f32
                                                        - px(6.),
                                                    y - px(4.),
                                                ),
                                                size(px(12.), px(12.)),
                                            ),
                                            rgb(0x365e65),
                                        );
                                        thumb.corner_radii = px(6.).into();
                                        window.paint_quad(thumb);
                                    },
                                )
                                .size_full(),
                            ),
                    )
                    .into_any_element()
            }
        }
    }

    fn controls(
        &mut self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> (
        Vec<gpui::AnyElement>,
        Vec<(ControlPlacement, gpui::AnyElement)>,
    ) {
        let placements = self.demo.scene().inline_controls.clone();
        let mut options = Vec::new();
        let mut inline = Vec::new();
        for control in self.demo.controls() {
            if let Some(placement) = placements.iter().find(|p| p.id == control.id()) {
                inline.push((
                    placement.clone(),
                    self.control(control, Some(placement), window, cx),
                ));
            } else {
                options.push(self.control(control, None, window, cx));
            }
        }
        inline.sort_by_key(|(placement, _)| Some(placement.id) == self.choice_open);
        (options, inline)
    }
}

impl Render for Gallery {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.advance_to_now() {
            window.request_animation_frame();
        }
        let (options, inline_controls) = self.controls(window, cx);
        let has_options = !options.is_empty();
        let content_width = f64::from(window.viewport_size().width)
            - if self.sidebar_visible { 238. } else { 0. }
            - if has_options { 248. } else { 0. };
        let scene_changed = self.stage.update(cx, |stage, cx| {
            let changed = stage.scene != *self.demo.scene()
                || stage.content_width != content_width
                || !inline_controls.is_empty()
                || !stage.controls.is_empty();
            stage.controls = inline_controls;
            if !changed {
                return false;
            }
            stage.scene = self.demo.scene().clone();
            stage.content_width = content_width;
            cx.notify();
            true
        });
        // GPUI snapshots dirty views before rendering this parent. A scene
        // changed here must bypass the old paint cache in this same frame.
        let stage_view = gpui::AnyView::from(self.stage.clone());
        let stage_view = if scene_changed {
            stage_view
        } else {
            stage_view.cached(div().size_full().style().clone())
        };
        let can_export = self.demo.export().is_some();
        let can_import = self.demo.can_import();
        let cursor = match self.demo.cursor() {
            dragology_gallery::Cursor::Default => gpui::CursorStyle::Arrow,
            dragology_gallery::Cursor::Grab => gpui::CursorStyle::OpenHand,
            dragology_gallery::Cursor::Grabbing => gpui::CursorStyle::ClosedHand,
            dragology_gallery::Cursor::Move => gpui::CursorStyle::ClosedHand,
            dragology_gallery::Cursor::Pointer => gpui::CursorStyle::PointingHand,
            dragology_gallery::Cursor::NotAllowed => gpui::CursorStyle::OperationNotAllowed,
            dragology_gallery::Cursor::ResizeHorizontal => gpui::CursorStyle::ResizeLeftRight,
            dragology_gallery::Cursor::ResizeVertical => gpui::CursorStyle::ResizeUpDown,
            dragology_gallery::Cursor::ResizeDiagonalNwSe => {
                gpui::CursorStyle::ResizeUpLeftDownRight
            }
            dragology_gallery::Cursor::ResizeDiagonalNeSw => {
                gpui::CursorStyle::ResizeUpRightDownLeft
            }
            dragology_gallery::Cursor::Crosshair => gpui::CursorStyle::Crosshair,
            dragology_gallery::Cursor::Text => gpui::CursorStyle::IBeam,
        };
        let entry = &self.entries[self.selected];
        let title = entry.id;
        let source_notes = shell::notes::render(entry.id, &self.notes_scroll);
        let original_url = format!("https://joshuahhh.com/dragology/#/demos/{}", entry.id);
        let matches: Vec<_> = self.matches().collect();
        let list = matches
            .iter()
            .map(|&index| {
                let entry = &self.entries[index];
                div()
                    .id(("demo", index))
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .text_sm()
                    .cursor_pointer()
                    .bg(rgb(if index == self.selected {
                        0xdce6e2
                    } else {
                        0xf3f1ea
                    }))
                    .hover(|s| s.bg(rgb(0xe3e8e1)))
                    .child(entry.id)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.select(index, cx);
                        this.focus.focus(window);
                    }))
                    .into_any_element()
            })
            .collect::<Vec<_>>();
        let status = if self.message.is_empty() {
            self.demo.status()
        } else {
            self.message.clone()
        };
        let tooltip = self
            .demo
            .tooltip()
            .zip(self.pointer)
            .zip(self.fit.get())
            .map(|((text, point), fit)| {
                let point = fit.transform * point;
                div()
                    .absolute()
                    .left(px((point.x + 12.)
                        .min(f64::from(window.viewport_size().width) - 280.)
                        .max(4.) as f32))
                    .top(px((point.y + 18.)
                        .min(f64::from(window.viewport_size().height) - 42.)
                        .max(4.) as f32))
                    .max_w(px(270.))
                    .px_2()
                    .py_1()
                    .rounded_md()
                    .bg(rgb(0x303b3c))
                    .text_color(rgb(0xffffff))
                    .text_xs()
                    .child(text)
            });

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0xfaf9f6))
            .text_color(rgb(0x303b3c))
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                let key = event.keystroke.key.as_str();
                let mods = modifiers(event.keystroke.modifiers);
                if mods.control && key == "f" {
                    this.sidebar_visible = true;
                    this.advance_to_now();
                    this.demo.cancel();
                    this.held = false;
                    this.slider = None;
                    this.search.update(cx, |field, cx| {
                        field.focus.focus(window);
                        field.select_all(cx);
                    });
                    cx.notify();
                    cx.stop_propagation();
                    return;
                }
                if this.text_focused(window, cx) {
                    return;
                }
                if let Some(id) = this.choice_open {
                    if let Some(Control::Choice { options, .. }) = this
                        .demo
                        .controls()
                        .into_iter()
                        .find(|control| control.id() == id)
                    {
                        match key {
                            "up" => {
                                this.choice_highlight = this.choice_highlight.saturating_sub(1);
                                cx.notify();
                                cx.stop_propagation();
                                return;
                            }
                            "down" => {
                                this.choice_highlight = (this.choice_highlight + 1)
                                    .min(options.len().saturating_sub(1));
                                cx.notify();
                                cx.stop_propagation();
                                return;
                            }
                            "enter" => {
                                this.action(id, Value::Index(this.choice_highlight), cx);
                                cx.notify();
                                cx.stop_propagation();
                                return;
                            }
                            _ => {}
                        }
                    }
                }
                this.advance_to_now();
                this.message.clear();
                match key {
                    "escape" => {
                        this.choice_open = None;
                        this.demo.cancel();
                        this.held = false;
                        this.slider = None;
                    }
                    "pagedown" | "pageup" | "home" | "end"
                        if this.demo.scene().presentation
                            == dragology_gallery::Presentation::Scroll
                            && !this.held
                            && !mods.control
                            && !mods.alt =>
                    {
                        this.stage.update(cx, |stage, cx| {
                            let current = stage.scroll.offset();
                            let maximum = stage.scroll.max_offset().height;
                            let page = stage.scroll.bounds().size.height * 0.9;
                            let y = match key {
                                "home" => px(0.),
                                "end" => -maximum,
                                "pageup" => current.y + page,
                                _ => current.y - page,
                            };
                            stage
                                .scroll
                                .set_offset(point(current.x, y.clamp(-maximum, px(0.))));
                            cx.notify();
                        });
                    }
                    "r" if !mods.control && !mods.alt => {
                        this.demo.reset();
                        this.held = false;
                        this.slider = None;
                    }
                    _ => this.demo.key(key, mods),
                }
                cx.notify();
            }))
            .on_modifiers_changed(cx.listener(
                |this, event: &gpui::ModifiersChangedEvent, _, cx| {
                    if this.held
                        && let Some(p) = this.pointer
                    {
                        this.demo.move_to(p, modifiers(event.modifiers));
                        cx.notify();
                    }
                },
            ))
            .when(cfg!(target_os = "linux"), |view| {
                view.capture_any_mouse_down(|event, window, cx| {
                    if event.button == MouseButton::Left
                        && !window.is_maximized()
                        && let Some(edge) =
                            chrome::resize_edge(event.position, window.viewport_size())
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
                        .bg(rgb(0xeae7dc))
                        .child(
                            div()
                                .flex()
                                .flex_1()
                                .h_full()
                                .items_center()
                                .px_4()
                                .text_sm()
                                .child("Dragology demos")
                                .on_mouse_down(MouseButton::Left, |event, window, _| {
                                    if event.click_count == 2 {
                                        window.zoom_window();
                                    } else {
                                        window.start_window_move();
                                    }
                                })
                                .on_mouse_down(MouseButton::Right, |event, window, _| {
                                    window.show_window_menu(event.position)
                                }),
                        )
                        .child(chrome::window_button("Minimize", WindowControlArea::Min))
                        .child(chrome::window_button("Maximize", WindowControlArea::Max))
                        .child(chrome::window_button("Close", WindowControlArea::Close)),
                )
            })
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .when(self.sidebar_visible, |view| {
                        view.child(
                            div()
                                .flex()
                                .flex_col()
                                .flex_none()
                                .w(px(238.))
                                .h_full()
                                .bg(rgb(0xf3f1ea))
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap_2()
                                        .p_3()
                                        .child(self.search.clone())
                                        .child(div().text_xs().text_color(rgb(0x697375)).child(
                                            format!(
                                                "{} of {} demos",
                                                matches.len(),
                                                self.entries.len()
                                            ),
                                        )),
                                )
                                .child(
                                    div()
                                        .id("demo-list")
                                        .flex()
                                        .flex_col()
                                        .flex_1()
                                        .min_h_0()
                                        .px_2()
                                        .pb_2()
                                        .gap_1()
                                        .overflow_y_scroll()
                                        .track_scroll(&self.sidebar_scroll)
                                        .children(list)
                                        .when(matches.is_empty(), |view| {
                                            view.child(
                                                div()
                                                    .p_3()
                                                    .text_sm()
                                                    .child("No demos match your search."),
                                            )
                                        }),
                                )
                                .child(
                                    div()
                                        .id("attribution")
                                        .flex_none()
                                        .p_3()
                                        .text_xs()
                                        .text_color(rgb(0x697375))
                                        .cursor_pointer()
                                        .child("Dragology contributors · MIT")
                                        .on_click(|_, _, cx| {
                                            cx.open_url(
                                                "https://github.com/declarative-dragging/dragology",
                                            )
                                        }),
                                ),
                        )
                    })
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w_0()
                            .h_full()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .flex_none()
                                    .gap_2()
                                    .px_5()
                                    .py_4()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .child(
                                                button(
                                                    "toggle-demos",
                                                    if self.sidebar_visible {
                                                        "Hide demos"
                                                    } else {
                                                        "Show demos"
                                                    },
                                                )
                                                .on_click(cx.listener(|this, _, window, cx| {
                                                    this.sidebar_visible = !this.sidebar_visible;
                                                    this.focus.focus(window);
                                                    cx.notify();
                                                })),
                                            )
                                            .child(div().flex_1().min_w_0().text_xl().child(title))
                                            .child(button("original", "Original").on_click(
                                                move |_, _, cx| cx.open_url(&original_url),
                                            ))
                                            .when(can_import, |view| {
                                                view.child(button("load", "Load").on_click(
                                                    cx.listener(|this, _, _, cx| this.load(cx)),
                                                ))
                                            })
                                            .when(can_export, |view| {
                                                view.child(button("save", "Save").on_click(
                                                    cx.listener(|this, _, _, cx| this.save(cx)),
                                                ))
                                            })
                                            .child(button("reset", "Reset").on_click(cx.listener(
                                                |this, _, window, cx| {
                                                    this.advance_to_now();
                                                    this.demo.reset();
                                                    this.held = false;
                                                    this.slider = None;
                                                    this.message.clear();
                                                    this.focus.focus(window);
                                                    cx.notify();
                                                },
                                            ))),
                                    )
                                    .when_some(source_notes, |view, notes| view.child(notes)),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_1()
                                    .min_h_0()
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w_0()
                                            .h_full()
                                            .overflow_hidden()
                                            .cursor(cursor)
                                            .on_mouse_down(
                                                MouseButton::Left,
                                                cx.listener(
                                                    |this, event: &MouseDownEvent, window, cx| {
                                                        if let Some(fit) = this.fit.get() {
                                                            let p = fit.to_scene(event.position);
                                                            this.pointer = Some(p);
                                                            this.held = true;
                                                            this.advance_to_now();
                                                            this.message.clear();
                                                            this.demo.press(
                                                                p,
                                                                modifiers(event.modifiers),
                                                                event.click_count,
                                                            );
                                                            this.focus.focus(window);
                                                            cx.notify();
                                                            cx.stop_propagation();
                                                        }
                                                    },
                                                ),
                                            )
                                            .child(stage_view),
                                    )
                                    .when(has_options, |view| {
                                        view.child(
                                            div()
                                                .id("demo-options")
                                                .flex()
                                                .flex_col()
                                                .flex_none()
                                                .w(px(248.))
                                                .h_full()
                                                .p_4()
                                                .gap_4()
                                                .overflow_y_scroll()
                                                .track_scroll(&self.controls_scroll)
                                                .bg(rgb(0xf4f2eb))
                                                .children(options),
                                        )
                                    }),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .flex_none()
                                    .min_h(px(38.))
                                    .px_5()
                                    .py_2()
                                    .gap_3()
                                    .text_xs()
                                    .child(div().flex_1().text_color(rgb(0x697375)).child(status))
                                    .child(
                                        div()
                                            .text_color(rgb(0x697375))
                                            .child("Esc cancel · R reset"),
                                    )
                                    .child(
                                        button(
                                            "motion",
                                            if self.animated {
                                                "Motion on"
                                            } else {
                                                "Motion off"
                                            },
                                        )
                                        .text_xs()
                                        .on_click(
                                            cx.listener(|this, _, window, cx| {
                                                this.animated = !this.animated;
                                                this.demo.set_animated(this.animated);
                                                this.focus.focus(window);
                                                cx.notify();
                                            }),
                                        ),
                                    ),
                            ),
                    ),
            )
            .when_some(tooltip, |view, tooltip| view.child(tooltip))
    }
}

fn main() {
    let entries = dragology_gallery::entries();
    let mut selected_id = None;
    let mut animated = true;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--list" => {
                for entry in &entries {
                    println!("{}", entry.id);
                }
                return;
            }
            "--demo" => {
                selected_id = args.next();
                if selected_id.is_none() {
                    eprintln!("--demo needs a demo ID. Use --list to see the available demos.");
                    std::process::exit(2);
                }
            }
            "--reduced-motion" => animated = false,
            "--help" | "-h" => {
                println!("Usage: dragology-gallery [--demo ID] [--reduced-motion] [--list]");
                return;
            }
            _ => {
                eprintln!("Unknown option {arg}. Use --help for usage.");
                std::process::exit(2);
            }
        }
    }
    let selected = match selected_id {
        Some(id) => entries
            .iter()
            .position(|entry| entry.id == id)
            .unwrap_or_else(|| {
                eprintln!("Unknown demo {id}. Use --list to see demo IDs.");
                std::process::exit(2);
            }),
        None => 0,
    };
    if entries.is_empty() {
        eprintln!("No demos are registered.");
        return;
    }
    Application::new()
        .with_assets(shell::svg::GeometryAssets)
        .run(move |cx: &mut App| {
            let bounds = Bounds::centered(None, size(px(1180.), px(780.)), cx);
            let mut options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(940.), px(580.))),
                ..Default::default()
            };
            if cfg!(target_os = "linux") {
                options.titlebar = None;
                options.window_decorations = Some(WindowDecorations::Client);
            }
            cx.open_window(options, move |window, cx| {
                window.set_window_title("Dragology demos");
                cx.new(|cx| Gallery::new(entries, selected, animated, window, cx))
            })
            .expect("open demo gallery");
            cx.on_window_closed(|cx| {
                if cx.windows().is_empty() {
                    cx.quit()
                }
            })
            .detach();
            cx.activate(true);
        });
}
