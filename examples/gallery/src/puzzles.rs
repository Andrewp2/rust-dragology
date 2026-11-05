//! Ports of the MIT-licensed Dragology puzzle and tiling examples.
//! Upstream: declarative-dragging/dragology, b145bdd56530f7cfa9a61bf6a7170459d2f140e1.
use crate::*;
use dragology::{FloatingOptions, SnapOptions, Transition, between, closest, fixed, vary};
use std::f64::consts::{PI, TAU};

mod boards;
mod collections;
mod tilings;

pub fn entries() -> Vec<Entry> {
    let mut entries = boards::entries();
    entries.extend(collections::entries());
    entries.extend(tilings::entries());
    entries
}

fn index(id: &str, prefix: &str) -> Option<usize> {
    id.strip_prefix(prefix)?.parse().ok()
}

const BLACK: Color = [0, 0, 0, 255];
const WHITE: Color = [255, 255, 255, 255];
const GRAY: Color = [128, 128, 128, 255];
const fn rgb(value: u32) -> Color {
    [(value >> 16) as u8, (value >> 8) as u8, value as u8, 255]
}
fn source_rect(
    id: impl Into<String>,
    bounds: Rect,
    radius: f64,
    fill: Color,
    stroke: Option<(Color, f64)>,
) -> Object {
    let mut object = Object::rect(id, bounds, radius, fill);
    if let Shape::Rect { stroke: style, .. } = &mut object.shapes[0] {
        *style = stroke;
    }
    object
}
fn centered_label(
    object: Object,
    text: impl Into<String>,
    center: Point,
    size: f64,
    color: Color,
) -> Object {
    object
        .label(text, center, size, color)
        .text_anchor(TextAnchor::Middle)
        .text_baseline(TextBaseline::Central)
}
fn raise_dragged(scene: &mut Scene, id: Option<&str>) {
    if let Some(i) = id.and_then(|id| scene.objects.iter().position(|o| o.id == id)) {
        let object = scene.objects.remove(i);
        scene.push(object);
    }
}

fn grid(scene: &mut Scene, width: i32, height: i32, size: f64, offset: f64) {
    for x in 0..=width {
        scene.push(Object::line(
            format!("grid-x-{x}"),
            (offset + f64::from(x) * size, offset),
            (
                offset + f64::from(x) * size,
                offset + f64::from(height) * size,
            ),
            GRAY,
            1.,
        ));
    }
    for y in 0..=height {
        scene.push(Object::line(
            format!("grid-y-{y}"),
            (offset, offset + f64::from(y) * size),
            (
                offset + f64::from(width) * size,
                offset + f64::from(y) * size,
            ),
            GRAY,
            1.,
        ));
    }
}

fn branches<S: Clone + 'static>(state: &S, targets: Vec<S>) -> DragSpec<'static, S> {
    closest(
        targets
            .into_iter()
            .map(|target| between([state.clone(), target])),
    )
}

fn polygon(points: impl IntoIterator<Item = Point>) -> BezPath {
    let mut points = points.into_iter();
    let mut path = BezPath::new();
    if let Some(start) = points.next() {
        path.move_to(start);
        for point in points {
            path.line_to(point);
        }
        path.close_path();
    }
    path
}

fn slider(id: &'static str, label: &str, value: f64, min: f64, max: f64) -> Control {
    Control::Slider {
        id,
        label: label.into(),
        value,
        min,
        max,
        step: 1.,
    }
}

#[cfg(test)]
fn exercise<E: Example>(example: E, states: Vec<E::State>) {
    use kurbo::Shape as _;
    let example = std::rc::Rc::new(example);
    for state in states {
        for object in example.scene(&state).objects {
            let Some(hit) = &object.hit else { continue };
            let center = match hit {
                Hit::Rect(bounds) => bounds.center(),
                Hit::Circle(center, _) => *center,
                Hit::Path(path, _) => path.bounding_box().center(),
            };
            let Some(spec) = example.drag(&state, &object.id, Modifiers::default()) else {
                continue;
            };
            let point = object.transform * center;
            let renderer = example.clone();
            let id = object.id.clone();
            let mut session = dragology::DragSession::begin(
                state.clone(),
                point,
                move |state| renderer.locate(state, &id),
                spec,
            )
            .unwrap();
            for delta in [Vec2::ZERO, Vec2::new(17., 11.)] {
                assert!(
                    session.frame(point + delta).is_ok(),
                    "{}: {}",
                    std::any::type_name::<E>(),
                    object.id
                );
            }
            assert!(
                session.finish(point).is_ok(),
                "{}: {}",
                std::any::type_name::<E>(),
                object.id
            );
        }
    }
}
