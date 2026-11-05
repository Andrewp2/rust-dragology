use dragology_gallery::{
    Affine, Clip, Color, FillRule, LineCap, LineJoin, Object, Point, Presentation, Scene, Shape,
    StrokeStyle, TextAnchor, TextBaseline, TextStyle,
};
use gpui::{App, Bounds, ContentMask, PathBuilder, Pixels, TextRun, Window, point, px, rgba};
use kurbo::{BezPath, Circle, PathEl, RoundedRect, Shape as _};

#[derive(Clone, Copy)]
pub struct Fit {
    pub transform: Affine,
}

impl Fit {
    pub fn new(scene: &Scene, bounds: Bounds<Pixels>) -> Option<Self> {
        let full_width = f64::from(bounds.size.width);
        let full_height = f64::from(bounds.size.height);
        let width = full_width - 40.;
        let height = full_height - 40.;
        if scene.width <= 0. || scene.height <= 0. || width <= 0. || height <= 0. {
            return None;
        }
        let scale = if scene.presentation == Presentation::Scroll {
            (width / scene.width).min(1.)
        } else {
            (width / scene.width).min(height / scene.height).min(2.)
        };
        Some(Self {
            transform: Affine::translate((
                f64::from(bounds.origin.x) + (full_width - scene.width * scale) * 0.5,
                f64::from(bounds.origin.y)
                    + if scene.presentation == Presentation::Scroll {
                        20.
                    } else {
                        (full_height - scene.height * scale) * 0.5
                    },
            )) * Affine::scale(scale),
        })
    }

    pub fn to_scene(self, p: gpui::Point<Pixels>) -> Point {
        self.transform.inverse() * Point::new(f64::from(p.x), f64::from(p.y))
    }
}

pub(crate) fn color(mut color: Color, opacity: f64) -> gpui::Hsla {
    color[3] = (f64::from(color[3]) * opacity.clamp(0., 1.)).round() as u8;
    rgba(u32::from_be_bytes(color)).into()
}

fn path_builder(path: &BezPath, transform: Affine, rule: FillRule) -> PathBuilder {
    let mut builder = PathBuilder::fill();
    builder.style =
        gpui::PathStyle::Fill(gpui::FillOptions::default().with_fill_rule(match rule {
            FillRule::NonZero => gpui::FillRule::NonZero,
            FillRule::EvenOdd => gpui::FillRule::EvenOdd,
        }));
    let mapped = |p: Point| {
        let p = transform * p;
        point(px(p.x as f32), px(p.y as f32))
    };
    for element in path.elements() {
        match *element {
            PathEl::MoveTo(p) => builder.move_to(mapped(p)),
            PathEl::LineTo(p) => builder.line_to(mapped(p)),
            PathEl::QuadTo(control, p) => builder.curve_to(mapped(p), mapped(control)),
            PathEl::CurveTo(c1, c2, p) => {
                builder.cubic_bezier_to(mapped(p), mapped(c1), mapped(c2))
            }
            PathEl::ClosePath => builder.close(),
        }
    }
    builder
}

struct Ink {
    path: BezPath,
    transform: Affine,
    color: Color,
    rule: FillRule,
}

fn stroke_outline(path: &BezPath, width: f64, style: &StrokeStyle) -> BezPath {
    let mut stroke = kurbo::Stroke::new(width);
    stroke.start_cap = match style.cap {
        LineCap::Butt => kurbo::Cap::Butt,
        LineCap::Round => kurbo::Cap::Round,
        LineCap::Square => kurbo::Cap::Square,
    };
    stroke.end_cap = stroke.start_cap;
    stroke.join = match style.join {
        LineJoin::Miter => kurbo::Join::Miter,
        LineJoin::Round => kurbo::Join::Round,
        LineJoin::Bevel => kurbo::Join::Bevel,
    };
    stroke.miter_limit = style.miter_limit;
    let mut dash: Vec<_> = style
        .dash
        .iter()
        .copied()
        .filter(|v| v.is_finite() && *v >= 0.)
        .collect();
    if dash.len() % 2 == 1 {
        dash.extend(dash.clone());
    }
    if dash.iter().any(|v| *v > 0.) {
        stroke = stroke.with_dashes(style.dash_offset, dash);
    }
    kurbo::stroke(
        path.elements().iter().copied(),
        &stroke,
        &kurbo::StrokeOpts::default(),
        0.05,
    )
}

fn shape_ink(
    path: BezPath,
    fill: Option<Color>,
    stroke: Option<(Color, f64)>,
    object: &Object,
    transform: Affine,
    inks: &mut Vec<Ink>,
) {
    if let Some(color) = fill.filter(|c| c[3] != 0) {
        inks.push(Ink {
            path: path.clone(),
            transform,
            color,
            rule: object.fill_rule,
        });
    }
    if let Some((color, width)) = stroke.filter(|(c, w)| c[3] != 0 && *w > 0.) {
        let (path, transform) = if object.stroke_style.non_scaling {
            (
                stroke_outline(&(transform * &path), width, &object.stroke_style),
                Affine::IDENTITY,
            )
        } else {
            (
                stroke_outline(&path, width, &object.stroke_style),
                transform,
            )
        };
        inks.push(Ink {
            path,
            transform,
            color,
            rule: FillRule::NonZero,
        });
    }
}

fn object_ink(object: &Object, transform: Affine) -> (Vec<Ink>, Vec<&Shape>) {
    let mut inks = Vec::new();
    let mut bitmap_text = Vec::new();
    for shape in &object.shapes {
        match shape {
            Shape::Rect {
                bounds,
                radius,
                fill,
                stroke,
            } => {
                // SVG rectangles with nonpositive dimensions do not render.
                // Spring overshoot can invert a shrinking rectangle; kurbo
                // normalizes those bounds and would paint it on the other side.
                if bounds.width() > 0. && bounds.height() > 0. {
                    shape_ink(
                        RoundedRect::from_rect(*bounds, *radius).to_path(0.05),
                        Some(*fill),
                        *stroke,
                        object,
                        transform,
                        &mut inks,
                    );
                }
            }
            Shape::Circle {
                center,
                radius,
                fill,
                stroke,
            } => shape_ink(
                Circle::new(*center, *radius).to_path(0.05),
                Some(*fill),
                *stroke,
                object,
                transform,
                &mut inks,
            ),
            Shape::Path { path, fill, stroke } => {
                shape_ink(path.clone(), *fill, *stroke, object, transform, &mut inks)
            }
            Shape::Text {
                origin,
                text,
                size,
                color,
            } => {
                let outlines = super::text::outlines(text, *size, &object.text_style);
                if outlines.bitmap_glyphs || (outlines.paths.is_empty() && !text.trim().is_empty())
                {
                    bitmap_text.push(shape);
                    continue;
                }
                let transform = transform * Affine::translate(origin.to_vec2());
                for glyph in &outlines.paths {
                    if let Some((color, width)) =
                        object.text_outline.filter(|(_, width)| *width > 0.)
                    {
                        inks.push(Ink {
                            path: stroke_outline(&glyph.path, width, &object.stroke_style),
                            transform,
                            color,
                            rule: FillRule::NonZero,
                        });
                    }
                    inks.push(Ink {
                        path: glyph.path.clone(),
                        transform,
                        color: glyph.color.unwrap_or(*color),
                        rule: FillRule::NonZero,
                    });
                }
            }
        }
    }
    (inks, bitmap_text)
}

fn clip_path(clip: &Clip) -> BezPath {
    match clip {
        Clip::Rect(rect) => rect.to_path(0.05),
        Clip::Path(path) => path.clone(),
    }
}

fn svg_path(path: &BezPath, rule: FillRule, opacity: f64) -> String {
    format!(
        "<path d=\"{}\" fill=\"white\" fill-rule=\"{}\" opacity=\"{opacity}\"/>",
        path.to_svg(),
        if rule == FillRule::EvenOdd {
            "evenodd"
        } else {
            "nonzero"
        }
    )
}

fn shadow(object: &Object, transform: Affine, inks: &[Ink], window: &mut Window, cx: &App) {
    let Some(shadow) = &object.shadow else { return };
    let inverse = transform.inverse();
    let local: Vec<_> = inks
        .iter()
        .map(|ink| ((inverse * ink.transform) * &ink.path, ink))
        .collect();
    let Some(bounds) = local
        .iter()
        .map(|(path, _)| path.bounding_box())
        .reduce(|a, b| a.union(b))
    else {
        return;
    };
    let blur = shadow.blur.max(0.);
    let region = bounds.inflate(blur * 3. + 1., blur * 3. + 1.);
    let paths = local
        .iter()
        .map(|(path, ink)| svg_path(path, ink.rule, f64::from(ink.color[3]) / 255.))
        .collect::<String>();
    let clip_definition = object
        .clip
        .as_ref()
        .map(|clip| {
            format!(
                "<clipPath id=\"shadow-clip\"><path d=\"{}\"/></clipPath>",
                clip_path(clip).to_svg()
            )
        })
        .unwrap_or_default();
    let clip_attribute = if object.clip.is_some() {
        " clip-path=\"url(#shadow-clip)\""
    } else {
        ""
    };
    let fragment = format!(
        "<defs>{clip_definition}<filter id=\"blur\" filterUnits=\"userSpaceOnUse\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"><feGaussianBlur stdDeviation=\"{blur}\"/></filter></defs><g{clip_attribute}><g transform=\"translate({} {})\"><g filter=\"url(#blur)\">{paths}</g></g></g>",
        region.x0,
        region.y0,
        region.width(),
        region.height(),
        shadow.offset.x,
        shadow.offset.y,
    );
    super::svg::paint(
        &fragment,
        region + shadow.offset,
        transform,
        shadow.color,
        object.opacity,
        window,
        cx,
    );
}

fn blurred_ink(
    ink: &Ink,
    object: &Object,
    object_transform: Affine,
    window: &mut Window,
    cx: &App,
) {
    let path = (object_transform.inverse() * ink.transform) * &ink.path;
    let blur = object.blur;
    let bounds = path.bounding_box().inflate(blur * 3. + 1., blur * 3. + 1.);
    let clip_definition = object
        .clip
        .as_ref()
        .map(|clip| {
            format!(
                "<clipPath id=\"clip\"><path d=\"{}\"/></clipPath>",
                clip_path(clip).to_svg()
            )
        })
        .unwrap_or_default();
    let clip_attribute = if object.clip.is_some() {
        " clip-path=\"url(#clip)\""
    } else {
        ""
    };
    let fragment = format!(
        "<defs>{clip_definition}<filter id=\"blur\" filterUnits=\"userSpaceOnUse\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"><feGaussianBlur stdDeviation=\"{blur}\"/></filter></defs><g{clip_attribute}><g filter=\"url(#blur)\">{}</g></g>",
        bounds.x0,
        bounds.y0,
        bounds.width(),
        bounds.height(),
        svg_path(&path, ink.rule, 1.)
    );
    super::svg::paint(
        &fragment,
        bounds,
        object_transform,
        ink.color,
        object.opacity,
        window,
        cx,
    );
}

/// Reject distant article content before shaping glyphs. Text bounds are
/// deliberately generous: this is a paint optimization, never a hit-test box.
fn potentially_visible(object: &Object, transform: Affine, viewport: kurbo::Rect) -> bool {
    object.shapes.iter().any(|shape| {
        let (bounds, stroke) = match shape {
            Shape::Rect { bounds, stroke, .. } => (*bounds, *stroke),
            Shape::Circle {
                center,
                radius,
                stroke,
                ..
            } => (
                kurbo::Rect::from_center_size(*center, (radius * 2., radius * 2.)),
                *stroke,
            ),
            Shape::Path { path, stroke, .. } => (path.bounding_box(), *stroke),
            Shape::Text {
                origin, text, size, ..
            } => {
                let extent = (text.chars().count() + 1) as f64
                    * (size * 4. + object.text_style.letter_spacing.abs());
                (
                    kurbo::Rect::new(
                        origin.x - extent,
                        origin.y - size * 4.,
                        origin.x + extent,
                        origin.y + size * 4.,
                    ),
                    object.text_outline,
                )
            }
        };
        let mut padding = stroke.map_or(0., |(_, width)| {
            width * object.stroke_style.miter_limit.max(2.)
        }) + object.blur * 4.;
        if let Some(shadow) = &object.shadow {
            padding += shadow.offset.hypot() + shadow.blur * 4.;
        }
        let mapped = transform.transform_rect_bbox(bounds.inflate(padding, padding));
        let mapped = if object.stroke_style.non_scaling {
            mapped.inflate(padding, padding)
        } else {
            mapped
        };
        mapped.x1 >= viewport.x0
            && mapped.x0 <= viewport.x1
            && mapped.y1 >= viewport.y0
            && mapped.y0 <= viewport.y1
    })
}

/// GPUI paints from a line box's top left; SVG positions an anchor on a baseline.
fn bitmap_text_origin(
    anchor: Point,
    style: &TextStyle,
    metrics: super::text::TextMetrics,
    line_height: f64,
    x_height: f64,
) -> Point {
    let x = match style.anchor {
        TextAnchor::Start => 0.,
        TextAnchor::Middle => -metrics.advance / 2.,
        TextAnchor::End => -metrics.advance,
    };
    let padding = (line_height - metrics.ascent - metrics.descent) / 2.;
    let y = match style.baseline {
        TextBaseline::Top => -padding,
        TextBaseline::Alphabetic => -padding - metrics.ascent,
        TextBaseline::Middle => -padding - metrics.ascent + x_height / 2.,
        TextBaseline::Central => -line_height / 2.,
    };
    anchor + kurbo::Vec2::new(x, y)
}

pub fn paint(scene: &Scene, fit: Fit, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
    window.with_content_mask(Some(ContentMask { bounds }), |window| {
        let visible = window.content_mask().bounds;
        let viewport = kurbo::Rect::new(f64::from(visible.left()), f64::from(visible.top()), f64::from(visible.right()), f64::from(visible.bottom()));
        for object in &scene.objects {
            if object.opacity <= 0. { continue; }
            let transform = fit.transform * object.transform;
            if !potentially_visible(object, transform, viewport) { continue; }
            let (inks, bitmap_text) = object_ink(object,transform);
            let mask = object.clip.as_ref().and_then(|clip| match clip {
                Clip::Rect(rect) if transform.as_coeffs()[1].abs() < 1e-10 && transform.as_coeffs()[2].abs() < 1e-10 => {
                    let r = transform.transform_rect_bbox(*rect);
                    Some(ContentMask { bounds:Bounds::from_corners(point(px(r.x0 as f32),px(r.y0 as f32)),point(px(r.x1 as f32),px(r.y1 as f32))) })
                }
                _ => None,
            });
            let needs_path_clip=mask.is_none();
            window.with_content_mask(mask, |window| {
                shadow(object,transform,&inks,window,cx);
                for ink in inks {
                    if object.blur > 0. {
                        blurred_ink(&ink,object,transform,window,cx);
                    } else if needs_path_clip && let Some(clip) = &object.clip {
                        let clip = transform * clip_path(clip);
                        let path = ink.transform * &ink.path;
                        let bounds = path.bounding_box().intersect(clip.bounding_box());
                        if bounds.width() <= 0. || bounds.height() <= 0. { continue; }
                        let fragment = format!("<defs><clipPath id=\"clip\"><path d=\"{}\"/></clipPath></defs><g clip-path=\"url(#clip)\">{}</g>",clip.to_svg(),svg_path(&path,ink.rule,1.));
                        super::svg::paint(&fragment,bounds,Affine::IDENTITY,ink.color,object.opacity,window,cx);
                    } else if let Ok(path) = path_builder(&ink.path,ink.transform,ink.rule).build() {
                        window.paint_path(path,color(ink.color,object.opacity));
                    }
                }
                // Bitmap color emoji have no glyph outline. Preserve their
                // native color-font rendering; this fallback remains upright.
                for shape in bitmap_text {
                    if let Shape::Text {origin,text,size,color:text_color} = shape {
                        let scale = transform.determinant().abs().sqrt();
                        let origin = transform * *origin;
                        let size = px((*size*scale).max(1.) as f32);
                        let mut font = window.text_style().font();
                        font.family = super::text::font_family(&object.text_style).to_owned().into();
                        font.weight = gpui::FontWeight(object.text_style.weight as f32);
                        font.style = if object.text_style.italic { gpui::FontStyle::Italic } else { gpui::FontStyle::Normal };
                        let font_id = window.text_system().resolve_font(&font);
                        let x_height = window.text_system().x_height(font_id, size);
                        let line = window.text_system().shape_line(text.clone().into(),size,&[TextRun {len:text.len(),font,color:color(*text_color,object.opacity),background_color:None,underline:None,strikethrough:None}],None);
                        let line_height = size * 1.2;
                        let origin = bitmap_text_origin(origin, &object.text_style, super::text::TextMetrics {
                            advance: f64::from(line.width), ascent: f64::from(line.ascent), descent: f64::from(line.descent),
                        }, f64::from(line_height), f64::from(x_height));
                        let _ = line.paint(point(px(origin.x as f32),px(origin.y as f32)),line_height,window,cx);
                    }
                }
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn collapsed_or_inverted_rectangles_do_not_paint_fill_or_stroke() {
        let mut object = Object::rect(
            "bar",
            kurbo::Rect::new(0., 0., 20., 30.),
            2.,
            [0, 0, 0, 255],
        );
        for (width, height) in [(20., 30.), (0., 30.), (20., 0.), (-3., 30.), (20., -3.)] {
            object.shapes = vec![Shape::Rect {
                bounds: kurbo::Rect::new(0., 0., width, height),
                radius: 2.,
                fill: [0, 0, 0, 255],
                stroke: Some(([255, 0, 0, 255], 1.)),
            }];
            let (ink, _) = object_ink(&object, Affine::IDENTITY);
            assert_eq!(ink.is_empty(), width <= 0. || height <= 0.);
        }
    }

    #[test]
    fn bitmap_text_stays_centered_on_its_svg_anchor() {
        let metrics = super::super::text::TextMetrics {
            advance: 48.,
            ascent: 38.,
            descent: 10.,
        };
        let anchor = Point::new(125., 125.);
        let mut style = TextStyle {
            anchor: TextAnchor::Middle,
            baseline: TextBaseline::Central,
            ..Default::default()
        };
        let top_left = bitmap_text_origin(anchor, &style, metrics, 60., 24.);
        assert_eq!(top_left + kurbo::Vec2::new(24., 30.), anchor);
        style.anchor = TextAnchor::End;
        style.baseline = TextBaseline::Alphabetic;
        let top_left = bitmap_text_origin(anchor, &style, metrics, 60., 24.);
        assert_eq!(top_left + kurbo::Vec2::new(48., 44.), anchor);
    }

    #[test]
    fn stroke_width_follows_nonuniform_transform_and_cap_style() {
        let mut path = BezPath::new();
        path.move_to((0., 0.));
        path.line_to((10., 0.));
        let plain = stroke_outline(&path, 2., &StrokeStyle::default());
        let transformed = (Affine::scale_non_uniform(3., 2.) * &plain).bounding_box();
        assert!((transformed.height() - 4.).abs() < 0.01);
        let round = stroke_outline(
            &path,
            2.,
            &StrokeStyle {
                cap: LineCap::Round,
                ..Default::default()
            },
        )
        .bounding_box();
        assert!((round.width() - 12.).abs() < 0.01);
        assert!((plain.bounding_box().width() - 10.).abs() < 0.01);
    }
}
