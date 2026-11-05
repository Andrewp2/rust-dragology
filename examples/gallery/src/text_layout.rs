//! SVG text shaping and cached glyph outlines. Unlike native glyph sprites,
//! outlines follow arbitrary object rotation, shear, and nonuniform scale.
use crate::{Affine, Color, FontFamily, Point, TextAnchor, TextBaseline, TextStyle};
use kurbo::BezPath;
use std::{cell::RefCell, collections::HashMap, rc::Rc, sync::Arc};

#[derive(Clone)]
pub struct GlyphPath {
    pub path: BezPath,
    /// None means the text's current fill. Color-font outline layers retain
    /// their own palette color instead of being tinted by the text fill.
    pub color: Option<Color>,
}
#[derive(Default)]
pub struct Outlines {
    pub paths: Vec<GlyphPath>,
    pub bitmap_glyphs: bool,
    /// SVG's logical text bounds use glyph advances and font ascent/descent,
    /// unlike the tight bounds of the visible outline paths.
    pub logical_bounds: Option<kurbo::Rect>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TextMetrics {
    pub advance: f64,
    pub ascent: f64,
    pub descent: f64,
}

/// Measure with exactly the font database, fallback, shaping and spacing used
/// by the scene renderer. Baseline/anchor only place text and do not change its
/// advance; normalizing them also makes this suitable for HTML-style line boxes.
pub fn measure_text(text: &str, size: f64, style: &TextStyle) -> TextMetrics {
    let style = TextStyle {
        anchor: TextAnchor::Start,
        baseline: TextBaseline::Alphabetic,
        strikethrough: false,
        ..style.clone()
    };
    let shaped = outlines(if text.is_empty() { "M" } else { text }, size, &style);
    let Some(bounds) = shaped.logical_bounds else {
        return TextMetrics::default();
    };
    TextMetrics {
        advance: if text.is_empty() { 0. } else { bounds.width() },
        ascent: -bounds.y0,
        descent: bounds.y1,
    }
}
struct Cache {
    options: usvg::Options<'static>,
    outlines: HashMap<String, Rc<Outlines>>,
}
impl Cache {
    fn new() -> Self {
        let mut fonts = usvg::fontdb::Database::new();
        fonts.load_system_fonts();
        Self {
            options: usvg::Options {
                fontdb: Arc::new(fonts),
                ..Default::default()
            },
            outlines: HashMap::new(),
        }
    }
}
thread_local! { static CACHE: RefCell<Option<Cache>> = const { RefCell::new(None) }; }

pub fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

pub fn font_family(style: &TextStyle) -> &str {
    match &style.family {
        FontFamily::Sans => "sans-serif",
        FontFamily::Serif => "serif",
        FontFamily::Mono => "monospace",
        FontFamily::Named(name) => name,
    }
}

fn collect(group: &usvg::Group, output: &mut Outlines) {
    for node in group.children() {
        match node {
            usvg::Node::Group(group) => collect(group, output),
            usvg::Node::Text(text) => {
                let bounds = text.abs_bounding_box();
                let bounds = kurbo::Rect::new(
                    bounds.left() as f64,
                    bounds.top() as f64,
                    bounds.right() as f64,
                    bounds.bottom() as f64,
                );
                output.logical_bounds = Some(
                    output
                        .logical_bounds
                        .map_or(bounds, |old| old.union(bounds)),
                );
                collect(text.flattened(), output);
            }
            usvg::Node::Image(_) => output.bitmap_glyphs = true,
            usvg::Node::Path(path) => {
                let Some(fill) = path.fill() else { continue };
                let t = path.abs_transform();
                let transform = Affine::new([
                    t.sx as f64,
                    t.ky as f64,
                    t.kx as f64,
                    t.sy as f64,
                    t.tx as f64,
                    t.ty as f64,
                ]);
                let point =
                    |p: usvg::tiny_skia_path::Point| transform * Point::new(p.x as f64, p.y as f64);
                let mut outline = BezPath::new();
                for segment in path.data().segments() {
                    use usvg::tiny_skia_path::PathSegment;
                    match segment {
                        PathSegment::MoveTo(p) => outline.move_to(point(p)),
                        PathSegment::LineTo(p) => outline.line_to(point(p)),
                        PathSegment::QuadTo(a, b) => outline.quad_to(point(a), point(b)),
                        PathSegment::CubicTo(a, b, c) => {
                            outline.curve_to(point(a), point(b), point(c))
                        }
                        PathSegment::Close => outline.close_path(),
                    }
                }
                let color = match fill.paint() {
                    usvg::Paint::Color(color)
                        if (color.red, color.green, color.blue) != (1, 2, 3) =>
                    {
                        Some([
                            color.red,
                            color.green,
                            color.blue,
                            (fill.opacity().get() * 255.).round() as u8,
                        ])
                    }
                    _ => None,
                };
                output.paths.push(GlyphPath {
                    path: outline,
                    color,
                });
            }
        }
    }
}

pub fn outlines(text: &str, size: f64, style: &TextStyle) -> Rc<Outlines> {
    let anchor = match style.anchor {
        TextAnchor::Start => "start",
        TextAnchor::Middle => "middle",
        TextAnchor::End => "end",
    };
    let baseline = match style.baseline {
        TextBaseline::Top => "text-before-edge",
        TextBaseline::Alphabetic => "alphabetic",
        TextBaseline::Middle => "middle",
        TextBaseline::Central => "central",
    };
    let decoration = if style.strikethrough {
        "line-through"
    } else {
        "none"
    };
    let font_style = if style.italic { "italic" } else { "normal" };
    let svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100000\" height=\"10000\" overflow=\"visible\"><text x=\"0\" y=\"0\" xml:space=\"preserve\" fill=\"#010203\" font-size=\"{size}\" font-family=\"{}\" font-weight=\"{}\" letter-spacing=\"{}\" font-style=\"{font_style}\" text-decoration=\"{decoration}\" text-anchor=\"{anchor}\" dominant-baseline=\"{baseline}\">{}</text></svg>",
        escape(font_family(style)),
        style.weight,
        style.letter_spacing,
        escape(text)
    );
    CACHE.with(|slot| {
        let mut slot = slot.borrow_mut();
        let cache = slot.get_or_insert_with(Cache::new);
        if let Some(outline) = cache.outlines.get(&svg) {
            return outline.clone();
        }
        let mut result = Outlines::default();
        if let Ok(tree) = usvg::Tree::from_str(&svg, &cache.options) {
            collect(tree.root(), &mut result);
        }
        let result = Rc::new(result);
        // Numeric labels change frequently while dragging. Bound their cache.
        if cache.outlines.len() >= 4096 {
            cache.outlines.clear();
        }
        cache.outlines.insert(svg, result.clone());
        result
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use kurbo::Shape;

    fn bounds(outline: &Outlines) -> kurbo::Rect {
        outline
            .paths
            .iter()
            .map(|p| p.path.bounding_box())
            .reduce(|a, b| a.union(b))
            .expect("a system font must produce glyph outlines")
    }

    #[test]
    fn svg_anchor_baseline_and_full_affine_transform_are_preserved() {
        let mut style = TextStyle {
            baseline: TextBaseline::Alphabetic,
            ..Default::default()
        };
        let start = outlines("Rotate me", 24., &style);
        let a = bounds(&start);
        assert!(a.y0 < 0. && a.width() > 20.);
        style.anchor = TextAnchor::Middle;
        let b = bounds(&outlines("Rotate me", 24., &style));
        style.anchor = TextAnchor::End;
        let c = bounds(&outlines("Rotate me", 24., &style));
        assert!((a.x0 - b.x0 - (b.x0 - c.x0)).abs() < 0.01);
        let rotation = Affine::rotate(std::f64::consts::FRAC_PI_2);
        let rotated = start
            .paths
            .iter()
            .map(|p| (rotation * &p.path).bounding_box())
            .reduce(|a, b| a.union(b))
            .unwrap();
        assert!((rotated.width() - a.height()).abs() < 0.01);
        assert!((rotated.height() - a.width()).abs() < 0.01);
        style.baseline = TextBaseline::Top;
        assert!(bounds(&outlines("Rotate me", 24., &style)).y0 >= -0.01);
    }

    #[test]
    fn xml_metacharacters_remain_literal_text() {
        let outline = outlines("A < B & C > D", 18., &TextStyle::default());
        let plain = outlines("ABCD", 18., &TextStyle::default());
        assert!(bounds(&outline).width() > bounds(&plain).width() * 1.5);
    }

    #[test]
    fn logical_metrics_match_shaped_anchor_shift_and_keep_spaces() {
        let style = TextStyle::default();
        let text = "Thin iii, wide WWW";
        let measured = measure_text(text, 16., &style);
        let a = bounds(&outlines(text, 16., &style));
        let end = TextStyle {
            anchor: TextAnchor::End,
            ..style.clone()
        };
        let b = bounds(&outlines(text, 16., &end));
        assert!((measured.advance - (a.x0 - b.x0)).abs() < 0.001);
        assert!(measured.ascent > 0. && measured.descent >= 0.);
        assert!(
            measure_text("WWW", 16., &style).advance
                > measure_text("iii", 16., &style).advance * 2.
        );
        assert!(measure_text("a ", 16., &style).advance > measure_text("a", 16., &style).advance);
        assert_eq!(measure_text("", 16., &style).advance, 0.);
    }
}
