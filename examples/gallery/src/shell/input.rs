use dragology_gallery::ControlStyle;
use gpui::{
    Bounds, ClipboardItem, Context, ElementInputHandler, EntityInputHandler, EventEmitter,
    FocusHandle, KeyDownEvent, MouseButton, Pixels, ShapedLine, TextRun, UTF16Selection, Window,
    canvas, div, fill, point, prelude::*, px, rgb, size,
};
use std::ops::Range;

pub enum InputEvent {
    Changed(String),
    Submit,
    Cancel,
}

pub struct TextInput {
    pub focus: FocusHandle,
    return_focus: FocusHandle,
    value: String,
    placeholder: String,
    anchor: usize,
    cursor: usize,
    marked: Option<Range<usize>>,
    last_line: Option<ShapedLine>,
    last_bounds: Option<Bounds<Pixels>>,
    scroll_x: Pixels,
    appearance: ControlStyle,
    inline: bool,
}
impl EventEmitter<InputEvent> for TextInput {}

fn byte_offset(text: &str, utf16: usize) -> usize {
    let mut offset = 0;
    for (index, ch) in text.char_indices() {
        if offset >= utf16 {
            return index;
        }
        offset += ch.len_utf16();
    }
    text.len()
}

impl TextInput {
    pub fn new(
        value: String,
        placeholder: String,
        return_focus: FocusHandle,
        cx: &mut Context<Self>,
    ) -> Self {
        let cursor = value.len();
        Self {
            focus: cx.focus_handle(),
            return_focus,
            value,
            placeholder,
            anchor: cursor,
            cursor,
            marked: None,
            last_line: None,
            last_bounds: None,
            scroll_x: px(0.),
            appearance: ControlStyle::default(),
            inline: false,
        }
    }

    pub fn set_value(&mut self, value: String, cx: &mut Context<Self>) {
        if self.value != value {
            self.value = value;
            self.cursor = self.value.len();
            self.anchor = self.cursor;
            self.marked = None;
            cx.notify();
        }
    }
    pub fn set_appearance(
        &mut self,
        appearance: ControlStyle,
        inline: bool,
        cx: &mut Context<Self>,
    ) {
        if self.appearance != appearance || self.inline != inline {
            self.appearance = appearance;
            self.inline = inline;
            cx.notify();
        }
    }

    pub fn select_all(&mut self, cx: &mut Context<Self>) {
        self.anchor = 0;
        self.cursor = self.value.len();
        cx.notify();
    }

    fn selection(&self) -> Range<usize> {
        self.anchor.min(self.cursor)..self.anchor.max(self.cursor)
    }

    fn from_utf16(&self, range: Range<usize>) -> Range<usize> {
        byte_offset(&self.value, range.start)..byte_offset(&self.value, range.end)
    }

    fn to_utf16(&self, range: Range<usize>) -> Range<usize> {
        self.value[..range.start].encode_utf16().count()
            ..self.value[..range.end].encode_utf16().count()
    }

    fn replace(&mut self, range: Range<usize>, text: &str, cx: &mut Context<Self>) {
        let text = text.replace(['\r', '\n'], " ");
        self.value.replace_range(range.clone(), &text);
        self.cursor = range.start + text.len();
        self.anchor = self.cursor;
        self.marked = None;
        cx.emit(InputEvent::Changed(self.value.clone()));
        cx.notify();
    }

    fn key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();
        let modifiers = event.keystroke.modifiers;
        let command = modifiers.control || modifiers.platform;
        let selection = self.selection();
        match key {
            "a" if command => self.select_all(cx),
            "c" | "x" if command => {
                if !selection.is_empty() {
                    cx.write_to_clipboard(ClipboardItem::new_string(
                        self.value[selection.clone()].into(),
                    ));
                    if key == "x" {
                        self.replace(selection, "", cx);
                    }
                }
            }
            "v" if command => {
                if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
                    self.replace(selection, &text, cx);
                }
            }
            "backspace" | "delete" => {
                let range = if !selection.is_empty() {
                    selection
                } else if key == "backspace" {
                    let previous = self.value[..self.cursor]
                        .char_indices()
                        .next_back()
                        .map_or(0, |(i, _)| i);
                    previous..self.cursor
                } else {
                    let next = self.value[self.cursor..]
                        .chars()
                        .next()
                        .map_or(self.cursor, |c| self.cursor + c.len_utf8());
                    self.cursor..next
                };
                self.replace(range, "", cx);
            }
            "left" | "right" | "home" | "end" => {
                self.cursor = match key {
                    "home" => 0,
                    "end" => self.value.len(),
                    "left" if !modifiers.shift && !selection.is_empty() => selection.start,
                    "right" if !modifiers.shift && !selection.is_empty() => selection.end,
                    "left" => self.value[..self.cursor]
                        .char_indices()
                        .next_back()
                        .map_or(0, |(i, _)| i),
                    _ => self.value[self.cursor..]
                        .chars()
                        .next()
                        .map_or(self.cursor, |c| self.cursor + c.len_utf8()),
                };
                if !modifiers.shift {
                    self.anchor = self.cursor;
                }
                cx.notify();
            }
            "enter" => {
                self.return_focus.focus(window);
                cx.emit(InputEvent::Submit);
            }
            "escape" => {
                self.return_focus.focus(window);
                cx.emit(InputEvent::Cancel);
            }
            _ => return,
        }
        cx.stop_propagation();
    }
}

impl EntityInputHandler for TextInput {
    fn text_for_range(
        &mut self,
        range: Range<usize>,
        actual: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.from_utf16(range);
        *actual = Some(self.to_utf16(range.clone()));
        Some(self.value[range].into())
    }
    fn selected_text_range(
        &mut self,
        _: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.to_utf16(self.selection()),
            reversed: self.cursor < self.anchor,
        })
    }
    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        self.marked.clone().map(|r| self.to_utf16(r))
    }
    fn unmark_text(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.marked = None;
        cx.notify();
    }
    fn replace_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range
            .map(|r| self.from_utf16(r))
            .or_else(|| self.marked.clone())
            .unwrap_or_else(|| self.selection());
        self.replace(range, text, cx);
    }
    fn replace_and_mark_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        selected: Option<Range<usize>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range
            .map(|r| self.from_utf16(r))
            .or_else(|| self.marked.clone())
            .unwrap_or_else(|| self.selection());
        let start = range.start;
        self.replace(range, text, cx);
        if !text.is_empty() {
            self.marked = Some(start..start + text.len());
        }
        if let Some(selected) = selected {
            self.anchor = start + byte_offset(text, selected.start);
            self.cursor = start + byte_offset(text, selected.end);
        }
    }
    fn bounds_for_range(
        &mut self,
        range: Range<usize>,
        _: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let bounds = self.last_bounds?;
        let line = self.last_line.as_ref()?;
        let range = self.from_utf16(range);
        Some(Bounds::from_corners(
            point(
                bounds.left() + line.x_for_index(range.start) - self.scroll_x,
                bounds.top(),
            ),
            point(
                bounds.left() + line.x_for_index(range.end) - self.scroll_x,
                bounds.bottom(),
            ),
        ))
    }
    fn character_index_for_point(
        &mut self,
        p: gpui::Point<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        let bounds = self.last_bounds?;
        let line = self.last_line.as_ref()?;
        let byte = line.closest_index_for_x(p.x - bounds.left() + self.scroll_x);
        Some(
            self.value[..byte.min(self.value.len())]
                .encode_utf16()
                .count(),
        )
    }
}

impl Render for TextInput {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focused = self.focus.is_focused(window);
        let entity = cx.entity();
        let font_size = if self.inline {
            window.text_style().font_size.to_pixels(window.rem_size())
        } else {
            px(13.)
        };
        let line_height = font_size * 1.4;
        let appearance = self.appearance.clone();
        div()
            .id("text-field")
            .h_full()
            .when(!self.inline, |view| view.h(px(34.)))
            .w_full()
            .px_2()
            .border_1()
            .border_color(rgb(if focused { 0x9caaad } else { 0xd8d5cc }))
            .rounded_md()
            .bg(rgb(0xffffff))
            .when_some(appearance.background, |view, color| {
                view.bg(super::painter::color(color, 1.))
            })
            .when_some(appearance.border, |view, color| {
                view.border_color(super::painter::color(color, 1.))
            })
            .track_focus(&self.focus)
            .cursor(gpui::CursorStyle::IBeam)
            .on_key_down(cx.listener(Self::key))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &gpui::MouseDownEvent, window, cx| {
                    this.focus.focus(window);
                    if event.click_count > 1 {
                        this.select_all(cx);
                    } else {
                        let index = this
                            .last_line
                            .as_ref()
                            .zip(this.last_bounds)
                            .map(|(line, bounds)| {
                                line.closest_index_for_x(
                                    event.position.x - bounds.left() + this.scroll_x,
                                )
                            })
                            .unwrap_or(this.value.len())
                            .min(this.value.len());
                        this.cursor = index;
                        if !event.modifiers.shift {
                            this.anchor = index;
                        }
                        cx.notify();
                    }
                    cx.stop_propagation();
                }),
            )
            .child(
                canvas(
                    |bounds, _, _| bounds,
                    move |_, bounds, window, cx| {
                        let input = entity.read(cx);
                        let focus = input.focus.clone();
                        let selection = input.selection();
                        let cursor = input.cursor;
                        let value = input.value.clone();
                        let placeholder = input
                            .appearance
                            .placeholder
                            .clone()
                            .unwrap_or_else(|| input.placeholder.clone());
                        let focused = focus.is_focused(window);
                        let shown = if value.is_empty() {
                            placeholder
                        } else {
                            value.clone()
                        };
                        let run = TextRun {
                            len: shown.len(),
                            font: window.text_style().font(),
                            color: input
                                .appearance
                                .foreground
                                .map(|color| super::painter::color(color, 1.))
                                .unwrap_or_else(|| {
                                    rgb(if value.is_empty() { 0x858178 } else { 0x2d302f }).into()
                                }),
                            background_color: None,
                            underline: None,
                            strikethrough: None,
                        };
                        let line =
                            window
                                .text_system()
                                .shape_line(shown.into(), font_size, &[run], None);
                        let caret = if value.is_empty() {
                            px(0.)
                        } else {
                            line.x_for_index(cursor)
                        };
                        let scroll = (caret - bounds.size.width + px(4.)).max(px(0.));
                        let top = bounds.origin.y + (bounds.size.height - line_height) * 0.5;
                        window.with_content_mask(Some(gpui::ContentMask { bounds }), |window| {
                            if focused && !selection.is_empty() {
                                let x0 = line.x_for_index(selection.start);
                                let x1 = line.x_for_index(selection.end);
                                window.paint_quad(fill(
                                    Bounds::new(
                                        point(bounds.left() + x0 - scroll, top),
                                        size(x1 - x0, line_height),
                                    ),
                                    rgb(0xd6e4ec),
                                ));
                            }
                            let _ = line.paint(
                                point(bounds.left() - scroll, top),
                                line_height,
                                window,
                                cx,
                            );
                            if focused && selection.is_empty() {
                                window.paint_quad(fill(
                                    Bounds::new(
                                        point(bounds.left() + caret - scroll, top + px(2.)),
                                        size(px(1.3), (line_height - px(4.)).max(px(2.))),
                                    ),
                                    rgb(0x3e5260),
                                ));
                            }
                        });
                        window.handle_input(
                            &focus,
                            ElementInputHandler::new(bounds, entity.clone()),
                            cx,
                        );
                        entity.update(cx, |input, _| {
                            input.last_line = Some(line);
                            input.last_bounds = Some(bounds);
                            input.scroll_x = scroll;
                        });
                    },
                )
                .size_full(),
            )
    }
}
