//! Rare SVG mask effects (arbitrary clip paths and Gaussian shadows).
//! Ordinary geometry and glyph outlines stay in the native vector renderer.
use dragology_gallery::{Affine, Color, Point, Rect};
use gpui::{App, AssetSource, Bounds, SharedString, TransformationMatrix, Window, point, px, size};
use std::borrow::Cow;

const PREFIX: &str = "dragology-mask:";
pub struct GeometryAssets;
impl AssetSource for GeometryAssets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        Ok(path
            .strip_prefix(PREFIX)
            .map(|svg| Cow::Owned(svg.as_bytes().to_vec())))
    }
    fn list(&self, _: &str) -> gpui::Result<Vec<SharedString>> {
        Ok(Vec::new())
    }
}

pub fn paint(
    fragment: &str,
    bounds: Rect,
    transform: Affine,
    color: Color,
    opacity: f64,
    window: &mut Window,
    cx: &App,
) {
    if !bounds.is_finite() || bounds.width() <= 0. || bounds.height() <= 0. {
        return;
    }
    let [a, b, c, d, _, _] = transform.as_coeffs();
    let scale = (a * a + b * b + c * c + d * d).sqrt().max(0.001);
    let local_center = bounds.center();
    let center = transform * local_center;
    // Padded square bounds also cover the transformed ink. GPUI's primitive
    // culling happens before its sprite transform, so an unpadded rotated
    // rectangle could otherwise disappear at the stage edge.
    let extent = bounds.width().max(bounds.height());
    let view_box = Rect::from_center_size(local_center, (extent * 2., extent * 2.));
    let source = format!(
        "{PREFIX}<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"{} {} {} {}\">{fragment}</svg>",
        extent * 2.,
        extent * 2.,
        view_box.x0,
        view_box.y0,
        view_box.width(),
        view_box.height()
    );
    let side = (extent * 2. * scale) as f32;
    let screen = Bounds::new(
        point(
            px(center.x as f32 - side / 2.),
            px(center.y as f32 - side / 2.),
        ),
        size(px(side), px(side)),
    );
    let linear = Affine::new([a / scale, b / scale, c / scale, d / scale, 0., 0.]);
    let shift = center - linear * Point::new(center.x, center.y);
    let dpi = window.scale_factor();
    let matrix = TransformationMatrix {
        rotation_scale: [
            [(a / scale) as f32, (c / scale) as f32],
            [(b / scale) as f32, (d / scale) as f32],
        ],
        translation: [shift.x as f32 * dpi, shift.y as f32 * dpi],
    };
    let _ = window.paint_svg(
        screen,
        source.into(),
        matrix,
        super::painter::color(color, opacity),
        cx,
    );
}
