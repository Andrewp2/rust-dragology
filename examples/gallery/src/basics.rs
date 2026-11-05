//! Ports of the small upstream interaction and study demos.
//! Copyright (c) 2025 Dragology contributors. Distributed under the MIT license.
use crate::*;
use dragology::{between, closest};
use std::rc::Rc;

mod composition;
mod controlled;
mod studies;
mod switches;

pub fn entries() -> Vec<Entry> {
    let mut entries = switches::entries();
    entries.extend(studies::entries());
    entries.extend(composition::entries());
    entries
}

const WHITE: Color = [255, 255, 255, 255];
// SVG/CSS colors copied from the pinned upstream examples. These deliberately
// do not use the gallery shell's themed palette.
const INK: Color = [0, 0, 0, 255];
const BLUE: Color = [59, 130, 246, 255];
const RED: Color = [239, 68, 68, 255];
const GREEN: Color = [34, 197, 94, 255];
const GOLD: Color = [234, 179, 8, 255];
const MUTED: Color = [156, 163, 175, 255];
const TRACK: Color = [203, 213, 225, 255];
const GRAY_200: Color = [229, 231, 235, 255];
const GRAY_300: Color = [209, 213, 219, 255];
const GRAY_500: Color = [107, 114, 128, 255];
const GRAY_700: Color = [55, 65, 81, 255];

fn elastic() -> dragology::Transition {
    dragology::Transition {
        duration: std::time::Duration::from_millis(200),
        easing: dragology::Easing::ElasticOut,
    }
}
fn shadow(object: Object, dy: f64, blur: f64, alpha: u8) -> Object {
    object.shadow(Shadow {
        offset: Vec2::new(0., dy),
        blur,
        color: [0, 0, 0, alpha],
    })
}
fn rounded(value: f64) -> String {
    let n = (value + 0.5).floor();
    if n == 0. { "0".into() } else { n.to_string() }
}

fn stroke(mut object: Object, color: Color, width: f64) -> Object {
    match &mut object.shapes[0] {
        Shape::Rect { stroke, .. } | Shape::Circle { stroke, .. } | Shape::Path { stroke, .. } => {
            *stroke = Some((color, width))
        }
        Shape::Text { .. } => {}
    }
    object
}

fn outlined_rect(id: impl Into<String>, bounds: Rect, radius: f64, fill: Color) -> Object {
    let mut item = Object::rect(id, bounds, radius, fill);
    if let Shape::Rect { stroke, .. } = &mut item.shapes[0] {
        *stroke = Some((TRACK, 1.));
    }
    item
}

fn outlined_circle(
    id: impl Into<String>,
    center: impl Into<Point>,
    radius: f64,
    fill: Color,
) -> Object {
    let mut item = Object::circle(id, center, radius, fill);
    if let Shape::Circle { stroke, .. } = &mut item.shapes[0] {
        *stroke = Some((TRACK, 1.));
    }
    item
}

fn move_item<T: Clone>(items: &[T], from: usize, to: usize) -> Vec<T> {
    let mut result = items.to_vec();
    let item = result.remove(from);
    result.insert(to, item);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn small_demo_ids_are_unique_and_all_start_with_finite_geometry() {
        let entries = entries();
        let mut ids = std::collections::HashSet::new();
        assert_eq!(entries.len(), 28);
        for entry in entries {
            assert!(ids.insert(entry.id), "duplicate {}", entry.id);
            let demo = (entry.create)();
            let scene = demo.scene();
            assert!(scene.width > 0. && scene.height > 0., "{}", entry.id);
            let mut objects = std::collections::HashSet::new();
            for object in &scene.objects {
                assert!(
                    objects.insert(&object.id),
                    "{} duplicate {}",
                    entry.id,
                    object.id
                );
                assert!(
                    object.transform.as_coeffs().iter().all(|v| v.is_finite()),
                    "{}",
                    entry.id
                );
            }
        }
    }
}
