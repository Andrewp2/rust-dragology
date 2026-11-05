//! Exact source notes, rendered as native wrapping text with active links.
use gpui::{
    AnyElement, Bounds, FontStyle, FontWeight, InteractiveText, ScrollHandle, StyledText, TextRun,
    UnderlineStyle, canvas, div, fill, font, point, prelude::*, px, rgb, size,
};
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use serde::Deserialize;
use std::{collections::HashMap, ops::Range, sync::OnceLock};

#[derive(Deserialize)]
struct Note {
    markdown: String,
    #[serde(default)]
    native_markdown: Option<String>,
    context: String,
}
#[derive(Deserialize)]
struct SourceNotes {
    notes: Vec<Note>,
    #[serde(default)]
    native_adaptation: Option<String>,
}
#[derive(Default)]
struct Block {
    text: String,
    runs: Vec<TextRun>,
    links: Vec<(Range<usize>, String)>,
}
#[derive(Clone, Default)]
struct Inline {
    strong: usize,
    emphasis: usize,
    code: bool,
    link: bool,
}
impl Block {
    fn append(&mut self, text: &str, style: &Inline) {
        if text.is_empty() {
            return;
        }
        self.text.push_str(text);
        let mut font = font(if style.code {
            "monospace"
        } else {
            "sans-serif"
        });
        if style.strong > 0 {
            font.weight = FontWeight::SEMIBOLD;
        }
        if style.emphasis > 0 {
            font.style = FontStyle::Italic;
        }
        let link_color = rgb(0x237b91).into();
        self.runs.push(TextRun {
            len: text.len(),
            font,
            color: if style.link {
                link_color
            } else {
                rgb(0x697375).into()
            },
            background_color: style.code.then(|| rgb(0xece9e0).into()),
            underline: style.link.then_some(UnderlineStyle {
                thickness: px(1.),
                color: Some(link_color),
                wavy: false,
            }),
            strikethrough: None,
        });
    }
}
fn blocks(markdown: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut block = Block::default();
    let mut style = Inline::default();
    let mut link_start = None;
    let mut lists: Vec<Option<u64>> = Vec::new();
    let finish = |block: &mut Block, blocks: &mut Vec<Block>| {
        if !block.text.is_empty() {
            blocks.push(std::mem::take(block));
        }
    };
    for event in Parser::new_ext(
        markdown,
        Options::ENABLE_STRIKETHROUGH | Options::ENABLE_MATH,
    ) {
        match event {
            Event::Start(Tag::Strong) => style.strong += 1,
            Event::End(TagEnd::Strong) => style.strong = style.strong.saturating_sub(1),
            Event::Start(Tag::Emphasis) => style.emphasis += 1,
            Event::End(TagEnd::Emphasis) => style.emphasis = style.emphasis.saturating_sub(1),
            Event::Start(Tag::Link { dest_url, .. }) => {
                style.link = true;
                link_start = Some((block.text.len(), dest_url.into_string()));
            }
            Event::End(TagEnd::Link) => {
                if let Some((start, url)) = link_start.take() {
                    block.links.push((start..block.text.len(), url));
                }
                style.link = false;
            }
            Event::Start(Tag::List(start)) => {
                finish(&mut block, &mut blocks);
                lists.push(start);
            }
            Event::End(TagEnd::List(_)) => {
                lists.pop();
            }
            Event::Start(Tag::Item) => {
                finish(&mut block, &mut blocks);
                let prefix = match lists.last_mut() {
                    Some(Some(n)) => {
                        let p = format!("{n}. ");
                        *n += 1;
                        p
                    }
                    _ => "• ".to_owned(),
                };
                block.append(&prefix, &style);
            }
            Event::End(TagEnd::Paragraph | TagEnd::Item | TagEnd::CodeBlock) => {
                finish(&mut block, &mut blocks);
                style.code = false;
            }
            Event::Start(Tag::Heading { .. }) => style.strong += 1,
            Event::End(TagEnd::Heading(_)) => {
                finish(&mut block, &mut blocks);
                style.strong = style.strong.saturating_sub(1);
            }
            Event::Start(Tag::CodeBlock(_)) => {
                finish(&mut block, &mut blocks);
                style.code = true;
            }
            Event::Text(text) => block.append(&text, &style),
            Event::Code(text) => block.append(
                &text,
                &Inline {
                    code: true,
                    ..style.clone()
                },
            ),
            // Keep original mathematical notation intact; this is not a TeX renderer.
            Event::InlineMath(text) => block.append(&format!("${text}$"), &style),
            Event::DisplayMath(text) => block.append(&format!("$${text}$$"), &style),
            Event::SoftBreak => block.append(" ", &style),
            Event::HardBreak => block.append("\n", &style),
            Event::Html(text) | Event::InlineHtml(text) => block.append(&text, &style),
            _ => {}
        }
    }
    finish(&mut block, &mut blocks);
    blocks
}

/// None means this demo has no introductory DemoNotes; specimen notes belong to
/// their source positions within the scene and are intentionally not duplicated.
pub fn render(id: &str, scroll: &ScrollHandle) -> Option<AnyElement> {
    static NOTES: OnceLock<HashMap<String, SourceNotes>> = OnceLock::new();
    let sources = NOTES.get_or_init(|| {
        serde_json::from_str(include_str!("../../data/upstream-notes.json"))
            .expect("pinned source notes are valid")
    });
    let source = sources.get(id)?;
    let mut body = source
        .notes
        .iter()
        .filter(|note| note.context == "intro")
        .flat_map(|note| blocks(note.native_markdown.as_deref().unwrap_or(&note.markdown)))
        .collect::<Vec<_>>();
    if body.is_empty() {
        return None;
    }
    if let Some(adaptation) = &source.native_adaptation {
        body.extend(blocks(adaptation));
    }
    let body = div()
        .id("source-notes")
        .w_full()
        .max_h(px(132.))
        .overflow_y_scroll()
        .track_scroll(scroll)
        .text_sm()
        .flex()
        .flex_col()
        .gap_2()
        .children(body.into_iter().enumerate().map(|(index, block)| {
            let ranges = block.links.iter().map(|(range, _)| range.clone()).collect();
            let urls = block
                .links
                .into_iter()
                .map(|(_, url)| url)
                .collect::<Vec<_>>();
            InteractiveText::new(
                ("note-paragraph", index),
                StyledText::new(block.text).with_runs(block.runs),
            )
            .on_click(ranges, move |index, _, cx| {
                if let Some(url) = urls.get(index) {
                    cx.open_url(url);
                }
            })
        }))
        .into_any_element();
    let handle = scroll.clone();
    Some(
        div()
            .relative()
            .w_full()
            .pr_2()
            .child(body)
            .child(
                canvas(
                    |bounds, _, _| bounds,
                    move |_, bounds, window, _| {
                        let maximum = handle.max_offset().height;
                        if maximum <= px(0.) {
                            return;
                        }
                        let height = bounds.size.height;
                        let thumb = (height * (height / (height + maximum))).max(px(16.));
                        let progress = (-handle.offset().y / maximum).clamp(0., 1.);
                        let thumb_bounds = Bounds::new(
                            point(bounds.left(), bounds.top() + (height - thumb) * progress),
                            size(bounds.size.width, thumb),
                        );
                        let mut quad = fill(thumb_bounds, rgb(0xadb7b5));
                        quad.corner_radii = px(2.).into();
                        window.paint_quad(quad);
                    },
                )
                .absolute()
                .right_0()
                .top_0()
                .w(px(3.))
                .h_full(),
            )
            .into_any_element(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_notes_keep_unicode_and_nested_link_ranges() {
        let parsed =
            blocks("Drag **λ** or [*this* `node`](https://example.org/a).\n\n- Keep $x+y$.");
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].text, "Drag λ or this node.");
        let (range, url) = &parsed[0].links[0];
        assert_eq!(&parsed[0].text[range.clone()], "this node");
        assert_eq!(url, "https://example.org/a");
        assert_eq!(
            parsed[0].runs.iter().map(|r| r.len).sum::<usize>(),
            parsed[0].text.len()
        );
        assert_eq!(parsed[1].text, "• Keep $x+y$.");
    }
}
