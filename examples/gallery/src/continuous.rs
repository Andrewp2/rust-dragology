//! Ports of Dragology's continuous geometry examples (MIT, Dragology contributors).
use crate::*;
use kurbo::Shape as KurboShape;
use std::f64::consts::TAU;

mod basic;
mod geometry;
mod mechanisms;

pub fn entries() -> Vec<Entry> {
    let mut entries = basic::entries();
    entries.extend(geometry::entries());
    entries.extend(mechanisms::entries());
    entries
}

const fn rgb(hex: u32) -> Color {
    [(hex >> 16) as u8, (hex >> 8) as u8, hex as u8, 255]
}
fn dashed_path(
    id: impl Into<String>,
    path: BezPath,
    color: Color,
    width: f64,
    dashes: &[f64],
) -> Object {
    Object::path(
        id,
        kurbo::dash(path.elements().iter().copied(), 0., dashes).collect(),
        color,
        width,
    )
}
fn circle_outline(
    id: impl Into<String>,
    center: Point,
    radius: f64,
    color: Color,
    width: f64,
    dashes: &[f64],
) -> Object {
    let path = kurbo::Circle::new(center, radius).to_path(0.01);
    if dashes.is_empty() {
        Object::path(id, path, color, width)
    } else {
        dashed_path(id, path, color, width, dashes)
    }
}
fn stroked_circle(
    id: impl Into<String>,
    center: Point,
    radius: f64,
    fill: Color,
    stroke: Color,
    width: f64,
) -> Object {
    let mut object = Object::circle(id, center, radius, fill);
    if let Shape::Circle { stroke: target, .. } = &mut object.shapes[0] {
        *target = Some((stroke, width));
    }
    object
}

fn source_cursors(mut scene: Scene) -> Scene {
    for object in &mut scene.objects {
        if object.hit.is_some() && object.cursor == Cursor::Default {
            object.cursor = Cursor::Grab;
        }
    }
    scene
}
fn path(points: impl IntoIterator<Item = Point>, closed: bool) -> BezPath {
    let mut path = BezPath::new();
    let mut points = points.into_iter();
    if let Some(point) = points.next() {
        path.move_to(point);
    }
    for point in points {
        path.line_to(point);
    }
    if closed {
        path.close_path();
    }
    path
}

fn polar(radius: f64, angle: f64) -> Vec2 {
    Vec2::new(radius * angle.cos(), radius * angle.sin())
}

fn index(id: &str, prefix: &str) -> Option<usize> {
    id.strip_prefix(prefix)?.parse().ok()
}

fn slider(id: &'static str, label: &str, value: f64, min: f64, max: f64, step: f64) -> Control {
    Control::Slider {
        id,
        label: label.into(),
        value,
        min,
        max,
        step,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reset_preserves_configuration_in_all_configured_continuous_demos() {
        for (id, control, value) in [
            ("dragon", "levels", Value::Number(5.)),
            ("face-creator", "mickey", Value::Bool(true)),
            ("scene-3d", "az", Value::Number(-75.)),
            ("chain-of-links", "mode", Value::Index(2)),
            ("robot-arm", "down", Value::Bool(false)),
            ("sprouting-tree", "compact", Value::Bool(true)),
        ] {
            let entry = entries().into_iter().find(|e| e.id == id).unwrap();
            let mut d = (entry.create)();
            d.action(control, value);
            let before = format!("{:?}", d.controls());
            d.reset();
            assert_eq!(
                format!("{:?}", d.controls()),
                before,
                "{id} discarded configuration on reset"
            );
        }
    }
    #[test]
    fn every_demo_accepts_representative_drags_without_engine_errors() {
        for entry in entries() {
            let initial = (entry.create)();
            let objects: Vec<_> = initial
                .scene()
                .objects
                .iter()
                .filter(|o| o.hit.is_some())
                .cloned()
                .collect();
            let step = (objects.len() / 8).max(1);
            for object in objects.into_iter().step_by(step) {
                let local = match object.hit.unwrap() {
                    Hit::Circle(p, r) => p + Vec2::new(r * 0.2, r * 0.1),
                    Hit::Rect(r) => r.center(),
                    Hit::Path(p, _) => p.bounding_box().center(),
                };
                let press = object.transform * local;
                let mut demo = (entry.create)();
                demo.set_animated(false);
                demo.press(press, Modifiers::default(), 1);
                for delta in [Vec2::new(13., 8.), Vec2::new(30., -16.)] {
                    demo.move_to(press + delta, Modifiers::default());
                    demo.advance(std::time::Duration::ZERO);
                }
                demo.release(press + Vec2::new(30., -16.), Modifiers::default());
                let status = demo.status();
                for error in [
                    "feasible model",
                    "missing from this state",
                    "invalid element transform",
                    "same grab position",
                    "numeric range",
                    "must be finite",
                ] {
                    assert!(
                        !status.contains(error),
                        "{} / {}: {status}",
                        entry.id,
                        object.id
                    );
                }
                assert!(
                    demo.scene().objects.iter().all(|o| o
                        .transform
                        .as_coeffs()
                        .iter()
                        .all(|v| v.is_finite())),
                    "{} produced a non-finite transform",
                    entry.id
                );
            }
        }
    }
}
