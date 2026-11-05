//! The native gallery must cover the pinned upstream page, with usable initial
//! scenes and controls. Behavioral tests remain beside each demo's rules.
use dragology_gallery::{Control, Hit, Object, Scene, Shape, Value, entries};
use kurbo::{BezPath, PathEl, Rect};
use std::collections::{HashMap, HashSet};
use std::panic::{AssertUnwindSafe, catch_unwind};

fn source_ids() -> Vec<&'static str> {
    include_str!("../data/upstream-demo-ids.txt")
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect()
}

#[test]
fn registry_matches_all_119_upstream_ids_in_page_order() {
    let expected = source_ids();
    assert_eq!(expected.len(), 119, "the pinned upstream inventory changed");
    let expected_set: HashSet<_> = expected.iter().copied().collect();
    assert_eq!(expected_set.len(), expected.len(), "duplicate upstream ID");
    let actual: Vec<_> = entries().into_iter().map(|entry| entry.id).collect();
    let mut counts = HashMap::new();
    for id in &actual {
        *counts.entry(*id).or_insert(0usize) += 1;
    }
    let duplicates: Vec<_> = counts.iter().filter(|(_, count)| **count > 1).collect();
    let missing: Vec<_> = expected
        .iter()
        .filter(|id| !counts.contains_key(**id))
        .collect();
    let unexpected: Vec<_> = actual
        .iter()
        .filter(|id| !expected_set.contains(**id))
        .collect();
    assert!(
        duplicates.is_empty() && missing.is_empty() && unexpected.is_empty(),
        "duplicates: {duplicates:?}; missing: {missing:?}; unexpected: {unexpected:?}"
    );
    assert_eq!(
        actual, expected,
        "gallery order must follow the source demo page"
    );
}

fn rect_valid(rect: Rect) -> bool {
    [rect.x0, rect.y0, rect.x1, rect.y1]
        .iter()
        .all(|v| v.is_finite())
        && rect.width() >= 0.
        && rect.height() >= 0.
}
fn path_finite(path: &BezPath) -> bool {
    path.elements().iter().all(|element| match element {
        PathEl::MoveTo(p) | PathEl::LineTo(p) => p.is_finite(),
        PathEl::QuadTo(a, b) => a.is_finite() && b.is_finite(),
        PathEl::CurveTo(a, b, c) => a.is_finite() && b.is_finite() && c.is_finite(),
        PathEl::ClosePath => true,
    })
}
fn geometry_valid(object: &Object) -> bool {
    object.transform.as_coeffs().iter().all(|v| v.is_finite())
        && object.opacity.is_finite()
        && (0. ..=1.).contains(&object.opacity)
        && match &object.hit {
            Some(Hit::Rect(bounds)) => rect_valid(*bounds),
            Some(Hit::Circle(center, radius)) => {
                center.is_finite() && radius.is_finite() && *radius >= 0.
            }
            Some(Hit::Path(path, width)) => path_finite(path) && width.is_finite() && *width >= 0.,
            None => true,
        }
        && object.shapes.iter().all(|shape| match shape {
            Shape::Rect {
                bounds,
                radius,
                stroke,
                ..
            } => {
                rect_valid(*bounds)
                    && radius.is_finite()
                    && *radius >= 0.
                    && stroke
                        .as_ref()
                        .is_none_or(|(_, width)| width.is_finite() && *width >= 0.)
            }
            Shape::Circle {
                center,
                radius,
                stroke,
                ..
            } => {
                center.is_finite()
                    && radius.is_finite()
                    && *radius >= 0.
                    && stroke
                        .as_ref()
                        .is_none_or(|(_, width)| width.is_finite() && *width >= 0.)
            }
            Shape::Path { path, stroke, .. } => {
                path_finite(path)
                    && stroke
                        .as_ref()
                        .is_none_or(|(_, width)| width.is_finite() && *width >= 0.)
            }
            Shape::Text { origin, size, .. } => {
                origin.is_finite() && size.is_finite() && *size > 0.
            }
        })
}
fn check_scene(id: &str, scene: &Scene, errors: &mut Vec<String>) {
    if !scene.width.is_finite()
        || !scene.height.is_finite()
        || scene.width <= 0.
        || scene.height <= 0.
    {
        errors.push(format!(
            "{id}: invalid scene dimensions {} × {}",
            scene.width, scene.height
        ));
    }
    let mut objects = HashSet::new();
    for object in &scene.objects {
        if object.id.is_empty() || !objects.insert(&object.id) {
            errors.push(format!(
                "{id}: empty or duplicate object ID {:?}",
                object.id
            ));
        }
        if !geometry_valid(object) {
            errors.push(format!("{id}: invalid geometry in object {:?}", object.id));
        }
    }
}

#[test]
fn every_initial_scene_has_finite_geometry_and_unique_object_ids() {
    let mut errors = Vec::new();
    for entry in entries() {
        let Ok(demo) = catch_unwind(entry.create) else {
            errors.push(format!("{}: initial scene creation panicked", entry.id));
            continue;
        };
        check_scene(entry.id, demo.scene(), &mut errors);
    }
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}

#[test]
fn every_initial_control_has_a_valid_identity_and_value() {
    let mut errors = Vec::new();
    for entry in entries() {
        let Ok(demo) = catch_unwind(entry.create) else {
            errors.push(format!("{}: initial scene creation panicked", entry.id));
            continue;
        };
        let Ok(controls) = catch_unwind(AssertUnwindSafe(|| demo.controls())) else {
            errors.push(format!("{}: initial controls panicked", entry.id));
            continue;
        };
        check_controls(entry.id, &controls, &mut errors);
    }
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}

fn check_controls(entry_id: &str, controls: &[Control], errors: &mut Vec<String>) {
    let mut ids = HashSet::new();
    for control in controls {
        let (id, label, valid) = match &control {
            Control::Button { id, label }
            | Control::ActionButton { id, label, .. }
            | Control::Toggle { id, label, .. }
            | Control::Text { id, label, .. }
            | Control::Code { id, label, .. } => (*id, label, true),
            Control::Choice {
                id,
                label,
                options,
                selected,
            } => (*id, label, !options.is_empty() && *selected < options.len()),
            Control::Slider {
                id,
                label,
                value,
                min,
                max,
                step,
            } => (
                *id,
                label,
                [value, min, max, step].iter().all(|n| n.is_finite())
                    && min <= max
                    && value >= min
                    && value <= max
                    && (*step > 0. || min == max),
            ),
        };
        if id.is_empty() || label.trim().is_empty() || !ids.insert(id) || !valid {
            errors.push(format!("{}: invalid control {control:?}", entry_id));
        }
    }
}

#[test]
fn each_top_level_choice_and_toggle_has_a_valid_result() {
    let mut errors = Vec::new();
    let mut exercised = 0;
    for entry in entries() {
        let Ok(initial_controls) = catch_unwind(AssertUnwindSafe(|| (entry.create)().controls()))
        else {
            errors.push(format!("{}: initial controls panicked", entry.id));
            continue;
        };
        for control in initial_controls {
            let (id, values) = match control {
                Control::Choice { id, options, .. } => {
                    (id, (0..options.len()).map(Value::Index).collect::<Vec<_>>())
                }
                Control::Toggle { id, value, .. } => (id, vec![Value::Bool(!value)]),
                _ => continue,
            };
            for value in values {
                let context = format!("{} / {id} = {value:?}", entry.id);
                let result = catch_unwind(AssertUnwindSafe(|| {
                    // Each option starts with a fresh demo. This covers declared
                    // presets and modes without exploring recursive combinations.
                    let mut demo = (entry.create)();
                    demo.set_animated(false);
                    demo.action(id, value.clone());
                    check_scene(&context, demo.scene(), &mut errors);
                    let controls = demo.controls();
                    check_controls(&context, &controls, &mut errors);
                    // When the chosen control remains visible, its selected
                    // value must agree with the action we just sent.
                    for updated in &controls {
                        let applied = match (updated, &value) {
                            (
                                Control::Choice {
                                    id: updated_id,
                                    selected,
                                    ..
                                },
                                Value::Index(expected),
                            ) if *updated_id == id => Some(selected == expected),
                            (
                                Control::Toggle {
                                    id: updated_id,
                                    value: selected,
                                    ..
                                },
                                Value::Bool(expected),
                            ) if *updated_id == id => Some(selected == expected),
                            _ => None,
                        };
                        if applied == Some(false) {
                            errors.push(format!(
                                "{context}: selected value was not applied ({updated:?})"
                            ));
                        }
                    }
                }));
                if result.is_err() {
                    errors.push(format!(
                        "{context}: action or resulting scene/controls panicked"
                    ));
                }
                exercised += 1;
            }
        }
    }
    assert!(exercised > 0);
    assert!(errors.is_empty(), "{}", errors.join("\n"));
    println!("Validated {exercised} independent choice/toggle alternatives across the gallery.");
}
