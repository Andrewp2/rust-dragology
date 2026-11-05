/*
Portions of the interpolation implementation below are translated from:

d3-color
Copyright 2010-2022 Mike Bostock

Permission to use, copy, modify, and/or distribute this software for any purpose
with or without fee is hereby granted, provided that the above copyright notice
and this permission notice appear in all copies.

THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES WITH
REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF MERCHANTABILITY AND
FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY SPECIAL, DIRECT,
INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES WHATSOEVER RESULTING FROM LOSS
OF USE, DATA OR PROFITS, WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER
TORTIOUS ACTION, ARISING OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF
THIS SOFTWARE.

d3-interpolate
Copyright 2010-2021 Mike Bostock

Permission to use, copy, modify, and/or distribute this software for any purpose
with or without fee is hereby granted, provided that the above copyright notice
and this permission notice appear in all copies.

THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES WITH
REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF MERCHANTABILITY AND
FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY SPECIAL, DIRECT,
INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES WHATSOEVER RESULTING FROM LOSS
OF USE, DATA OR PROFITS, WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER
TORTIOUS ACTION, ARISING OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF
THIS SOFTWARE.

d3-interpolate-path
Copyright 2016, Peter Beshai
All rights reserved.

Redistribution and use in source and binary forms, with or without modification,
are permitted provided that the following conditions are met:

* Redistributions of source code must retain the above copyright notice, this
  list of conditions and the following disclaimer.

* Redistributions in binary form must reproduce the above copyright notice,
  this list of conditions and the following disclaimer in the documentation
  and/or other materials provided with the distribution.

* Neither the name of the author nor the names of contributors may be used to
  endorse or promote products derived from this software without specific prior
  written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND
ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT OWNER OR CONTRIBUTORS BE LIABLE FOR
ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES
(INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES;
LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON
ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
(INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
*/

use crate::*;
use dragology::Preview;
use kurbo::{ParamCurveNearest, PathEl, Shape as KurboShape};
use std::collections::{HashMap, HashSet};

pub fn hit(object: &Object, point: Point) -> bool {
    let determinant = object.transform.determinant();
    if !determinant.is_finite()
        || determinant.abs() < 1e-12
        || (object.opacity <= 0. && !object.hit_when_transparent)
    {
        return false;
    }
    let p = object.transform.inverse() * point;
    if object.clip.as_ref().is_some_and(|clip| match clip {
        Clip::Rect(bounds) => !bounds.contains(p),
        Clip::Path(path) => path.winding(p) == 0,
    }) {
        return false;
    }
    match &object.hit {
        Some(Hit::Rect(bounds)) => bounds.contains(p),
        Some(Hit::Circle(center, radius)) => p.distance(*center) <= *radius,
        Some(Hit::Path(path, width)) => {
            (path
                .elements()
                .iter()
                .any(|el| matches!(el, PathEl::ClosePath))
                && path.winding(p) != 0)
                || path
                    .segments()
                    .any(|segment| segment.nearest(p, 0.1).distance_sq <= width.powi(2) * 0.25)
        }
        None => false,
    }
}

pub(super) fn group(scene: &Scene, id: &str) -> HashSet<String> {
    let mut ids = HashSet::from([id.to_owned()]);
    // Objects may appear before their parent in paint order.
    for _ in 0..scene.objects.len() {
        let before = ids.len();
        for object in &scene.objects {
            if object
                .parent
                .as_ref()
                .is_some_and(|parent| ids.contains(parent))
            {
                ids.insert(object.id.clone());
            }
        }
        if before == ids.len() {
            break;
        }
    }
    ids
}

pub fn resolve_interaction<E: Example>(
    example: &E,
    id: &str,
    hovered: Option<&str>,
    preview: &Preview<E::State>,
) -> Scene {
    match preview {
        Preview::State(state) => example.scene_interaction(state, Some(id), hovered),
        Preview::Blend(parts) => blend(
            parts
                .iter()
                .filter(|(_, w)| *w > 0.)
                .map(|(p, w)| (resolve_interaction(example, id, hovered, p), *w))
                .collect(),
        ),
        Preview::Floating {
            backdrop,
            source,
            translation,
            ghost_opacity,
        } => {
            let mut backdrop = resolve_interaction(example, id, hovered, backdrop);
            let source = resolve_interaction(example, id, hovered, source);
            let removed = group(&backdrop, id);
            let floating = group(&source, id);
            let mut ghosts = Vec::new();
            if let Some(opacity) = ghost_opacity {
                for object in backdrop.objects.iter().filter(|o| removed.contains(&o.id)) {
                    let mut ghost = object.clone();
                    ghost.id = format!("ghost-{}", ghost.id);
                    ghost.parent = ghost.parent.map(|p| {
                        if removed.contains(&p) {
                            format!("ghost-{p}")
                        } else {
                            p
                        }
                    });
                    ghost.opacity *= opacity;
                    ghost.hit = None;
                    ghosts.push(ghost);
                }
            }
            backdrop
                .objects
                .retain(|object| !removed.contains(&object.id));
            backdrop.objects.extend(ghosts);
            for mut object in source.objects {
                if floating.contains(&object.id) {
                    object.prepend_translation(*translation);
                    object.transition = false;
                    backdrop.objects.push(object);
                }
            }
            backdrop
        }
    }
}

/// Supply source-equivalent synthetic counterparts in every missing scene.
/// Origins come from the original scenes, so synthetic objects cannot become
/// origins for other emerging objects in the same interpolation pass.
fn augment_emerging(parts: &mut [(Scene, f64)]) {
    let additions = {
        let mut templates: HashMap<&str, (&Object, f64, &Scene, usize)> = HashMap::new();
        let mut ids = Vec::new();
        for (scene, weight) in parts.iter() {
            for (index, object) in scene.objects.iter().enumerate() {
                if !templates.contains_key(object.id.as_str()) {
                    ids.push(object.id.as_str());
                }
                let entry = templates
                    .entry(object.id.as_str())
                    .or_insert((object, *weight, scene, index));
                if *weight > entry.1 {
                    *entry = (object, *weight, scene, index);
                }
            }
        }
        let mut additions = Vec::new();
        for id in ids {
            let (object, _, template_scene, index) = templates[id];
            let Some(origin_id) = &object.emerge_from else {
                continue;
            };
            for (scene_index, (scene, _)) in parts.iter().enumerate() {
                if scene.object(id).is_some() {
                    continue;
                }
                let Some(origin) = scene.object(origin_id) else {
                    continue;
                };
                let mut synthetic = object.clone();
                synthetic.transform = origin.transform;
                synthetic.transform_components = origin.transform_components.clone();
                let bounds = |o: &Object| {
                    o.emerge_bounds
                        .then(|| {
                            o.shapes.iter().find_map(|shape| match shape {
                                Shape::Rect { bounds, .. } => Some(*bounds),
                                _ => None,
                            })
                        })
                        .flatten()
                };
                let morph_bounds = object.emerge_mode != EmergeMode::Scale
                    && bounds(object).is_some()
                    && bounds(origin).is_some();
                if morph_bounds {
                    let from = bounds(origin).unwrap();
                    let old = bounds(object).unwrap();
                    let new = Rect::from_origin_size(old.origin(), from.size());
                    if let Some(Shape::Rect { bounds, .. }) = synthetic
                        .shapes
                        .iter_mut()
                        .find(|s| matches!(s, Shape::Rect { .. }))
                    {
                        *bounds = new;
                    }
                    if synthetic.hit == Some(Hit::Rect(old)) {
                        synthetic.hit = Some(Hit::Rect(new));
                    }
                    if let Some(y) = origin.shapes.iter().find_map(|s| match s {
                        Shape::Text { origin, .. } => Some(origin.y),
                        _ => None,
                    }) {
                        if let Some(Shape::Text { origin, .. }) = synthetic
                            .shapes
                            .iter_mut()
                            .find(|s| matches!(s, Shape::Text { .. }))
                        {
                            origin.y = y;
                        }
                    }
                } else if object.emerge_mode != EmergeMode::Clone {
                    synthetic.transform *= Affine::scale(0.);
                    // Preserve the origin's orientation even though its matrix is singular.
                    let mut components = origin
                        .transform_components
                        .clone()
                        .unwrap_or_else(|| affine_components(origin.transform));
                    components.push(TransformComponent::Scale(Vec2::ZERO));
                    synthetic.transform_components = Some(components);
                }
                if object.emerge_mode != EmergeMode::Clone {
                    synthetic.opacity = 0.;
                }
                // Preserve the destination's stacking path relative to shared siblings.
                let next = template_scene.objects[index + 1..]
                    .iter()
                    .find(|next| scene.object(&next.id).is_some())
                    .map(|next| next.id.clone());
                additions.push((scene_index, synthetic, next));
            }
        }
        additions
    };
    for (scene_index, object, next) in additions {
        let scene = &mut parts[scene_index].0;
        let index = next
            .and_then(|next| scene.objects.iter().position(|o| o.id == next))
            .unwrap_or(scene.objects.len());
        scene.objects.insert(index, object);
    }
}

pub fn blend(parts: Vec<(Scene, f64)>) -> Scene {
    blend_with_properties(parts, false)
}
/// Spring interpolation always takes discrete properties and stacking from its
/// target, even while that target has a small (or overshooting) blend weight.
pub fn blend_transition(target: Scene, source: Scene, progress: f64) -> Scene {
    blend_with_properties(vec![(target, progress), (source, 1. - progress)], true)
}
fn blend_with_properties(mut parts: Vec<(Scene, f64)>, target_properties: bool) -> Scene {
    augment_emerging(&mut parts);
    let Some((first, _)) = parts.first() else {
        return Scene::new(1., 1.);
    };
    // Source `lerpLayeredWeighted` gives ties to the earlier state.
    let selected_index = if target_properties {
        0
    } else {
        parts.iter().enumerate().fold(
            0,
            |best, (i, (_, weight))| if *weight > parts[best].1 { i } else { best },
        )
    };
    let selected_scene = &parts[selected_index].0;
    let mut output = Scene::new(first.width, first.height);
    output.presentation = selected_scene.presentation;
    output.inline_controls = selected_scene.inline_controls.clone();
    let mut order: Vec<&str> = selected_scene
        .objects
        .iter()
        .map(|o| o.id.as_str())
        .collect();
    let mut by_id: HashMap<&str, Vec<(&Object, f64)>> = HashMap::new();
    for (scene, weight) in &parts {
        for object in &scene.objects {
            if !order.contains(&object.id.as_str()) {
                order.push(object.id.as_str());
            }
            by_id
                .entry(object.id.as_str())
                .or_default()
                .push((object, *weight));
        }
    }
    for id in order {
        let samples = &by_id[id];
        let total: f64 = samples.iter().map(|(_, w)| w).sum();
        if total.abs() <= 1e-12 {
            continue;
        }
        let selected = selected_scene.object(id).unwrap_or_else(|| {
            samples
                .iter()
                .copied()
                .reduce(|best, next| if next.1 > best.1 { next } else { best })
                .unwrap()
                .0
        });
        let mut result = selected.clone();
        if result.interpolate {
            let mut weight = samples[0].1;
            let mut geometry = samples[0].0.clone();
            for (next, w) in &samples[1..] {
                weight += w;
                if weight.abs() <= 1e-12 {
                    continue;
                }
                let t = w / weight;
                let components = object_transforms(&geometry, next, t);
                geometry.transform = components
                    .as_ref()
                    .map(|cs| components_affine(cs))
                    .unwrap_or_else(|| affine(geometry.transform, next.transform, t));
                geometry.transform_components = components;
                geometry.hit = match (&geometry.hit, &next.hit) {
                    (Some(Hit::Rect(a)), Some(Hit::Rect(b))) => Some(Hit::Rect(rect(*a, *b, t))),
                    (Some(Hit::Circle(a, ar)), Some(Hit::Circle(b, br))) => {
                        Some(Hit::Circle(point(*a, *b, t), number(*ar, *br, t)))
                    }
                    (Some(Hit::Path(a, aw)), Some(Hit::Path(b, bw))) => {
                        Some(Hit::Path(path(a, b, t), number(*aw, *bw, t)))
                    }
                    _ => selected.hit.clone(),
                };
                if geometry.shapes.len() == next.shapes.len() {
                    geometry.shapes = geometry
                        .shapes
                        .iter()
                        .zip(&next.shapes)
                        .map(|(a, b)| shape(a, b, t))
                        .collect();
                } else {
                    geometry.shapes = selected.shapes.clone();
                }
                geometry.blur = number(geometry.blur, next.blur, t);
                geometry.text_style.letter_spacing = number(
                    geometry.text_style.letter_spacing,
                    next.text_style.letter_spacing,
                    t,
                );
                geometry.text_style.weight = number(
                    f64::from(geometry.text_style.weight),
                    f64::from(next.text_style.weight),
                    t,
                )
                .round() as u16;
            }
            result.transform = geometry.transform;
            result.transform_components = geometry.transform_components;
            result.hit = geometry.hit;
            result.shapes = geometry.shapes;
            result.blur = geometry.blur;
            result.text_style.weight = geometry.text_style.weight;
            result.text_style.letter_spacing = geometry.text_style.letter_spacing;
            for (shape, source) in result.shapes.iter_mut().zip(&selected.shapes) {
                if let (Shape::Text { text, .. }, Shape::Text { text: chosen, .. }) =
                    (shape, source)
                {
                    *text = chosen.clone();
                }
            }
        }
        result.opacity = samples
            .iter()
            .map(|(o, w)| o.opacity * w)
            .sum::<f64>()
            .clamp(0., 1.);
        output.push(result);
    }
    output
}

fn number(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}
fn point(a: Point, b: Point, t: f64) -> Point {
    a + (b - a) * t
}
fn rect(a: Rect, b: Rect, t: f64) -> Rect {
    Rect::new(
        number(a.x0, b.x0, t),
        number(a.y0, b.y0, t),
        number(a.x1, b.x1, t),
        number(a.y1, b.y1, t),
    )
}
// Lab interpolation follows d3-color 3.1.0 and d3-interpolate 3.0.1. A zero-alpha
// CSS color has undefined channels in D3; the other endpoint supplies its hue.
fn color(a: Color, b: Color, t: f64) -> Color {
    if a == b {
        return a;
    }
    const XN: f64 = 0.96422;
    const ZN: f64 = 0.82521;
    const T0: f64 = 4. / 29.;
    const T1: f64 = 6. / 29.;
    const T2: f64 = 3. * T1 * T1;
    const T3: f64 = T1 * T1 * T1;
    fn linear(x: u8) -> f64 {
        let x = f64::from(x) / 255.;
        if x <= 0.04045 {
            x / 12.92
        } else {
            ((x + 0.055) / 1.055).powf(2.4)
        }
    }
    fn xyz_to_lab(x: f64) -> f64 {
        if x > T3 { x.powf(1. / 3.) } else { x / T2 + T0 }
    }
    fn lab_to_xyz(x: f64) -> f64 {
        if x > T1 { x * x * x } else { T2 * (x - T0) }
    }
    fn to_lab(c: Color) -> [f64; 3] {
        if c[3] == 0 {
            return [f64::NAN; 3];
        }
        let [r, g, b] = [linear(c[0]), linear(c[1]), linear(c[2])];
        let y = xyz_to_lab(0.2225045 * r + 0.7168786 * g + 0.0606169 * b);
        let (x, z) = if r == g && g == b {
            (y, y)
        } else {
            (
                xyz_to_lab((0.4360747 * r + 0.3850649 * g + 0.1430804 * b) / XN),
                xyz_to_lab((0.0139322 * r + 0.0971045 * g + 0.7141733 * b) / ZN),
            )
        };
        [116. * y - 16., 500. * (x - y), 200. * (y - z)]
    }
    fn channel(a: f64, b: f64, t: f64) -> f64 {
        if a.is_nan() {
            b
        } else if b.is_nan() {
            a
        } else {
            number(a, b, t)
        }
    }
    fn srgb(x: f64) -> u8 {
        let x = if x <= 0.0031308 {
            12.92 * x
        } else {
            1.055 * x.powf(1. / 2.4) - 0.055
        };
        (255. * x).round().clamp(0., 255.) as u8
    }
    let la = to_lab(a);
    let lb = to_lab(b);
    let [l, a_, b_] = std::array::from_fn(|i| channel(la[i], lb[i], t));
    let y = (l + 16.) / 116.;
    let x = XN * lab_to_xyz(if a_.is_nan() { y } else { y + a_ / 500. });
    let z = ZN * lab_to_xyz(if b_.is_nan() { y } else { y - b_ / 200. });
    let y = lab_to_xyz(y);
    [
        srgb(3.1338561 * x - 1.6168667 * y - 0.4906146 * z),
        srgb(-0.9787684 * x + 1.9161415 * y + 0.0334540 * z),
        srgb(0.0719453 * x - 0.2289914 * y + 1.4052427 * z),
        number(f64::from(a[3]), f64::from(b[3]), t)
            .round()
            .clamp(0., 255.) as u8,
    ]
}
fn fill(a: Option<Color>, b: Option<Color>, t: f64) -> Option<Color> {
    fn none_pair(c: Color) -> (Color, Color) {
        let rgb = if c[3] == 0 {
            [0, 0, 0]
        } else {
            [c[0], c[1], c[2]]
        };
        ([rgb[0], rgb[1], rgb[2], 255], [rgb[0], rgb[1], rgb[2], 0])
    }
    match (a, b) {
        (Some(a), Some(b)) => Some(color(a, b, t)),
        // Upstream's explicit "none" branch constructs an opaque counterpart,
        // even when the supplied color itself has an alpha channel.
        (Some(a), None) => {
            let (opaque, transparent) = none_pair(a);
            Some(color(opaque, transparent, t))
        }
        (None, Some(b)) => {
            let (opaque, transparent) = none_pair(b);
            Some(color(transparent, opaque, t))
        }
        (None, None) => None,
    }
}
fn stroke(a: Option<(Color, f64)>, b: Option<(Color, f64)>, t: f64) -> Option<(Color, f64)> {
    match (a, b) {
        (Some((a, aw)), Some((b, bw))) => Some((color(a, b, t), number(aw, bw, t))),
        (Some((a, width)), None) => Some((fill(Some(a), None, t).unwrap(), width)),
        (None, Some((b, width))) => Some((fill(None, Some(b), t).unwrap(), width)),
        (None, None) => None,
    }
}

// The absolute M/L/Q/C/Z subset of d3-interpolate-path 2.3.0. Its conversion is
// deliberately asymmetric: a line becomes a curve by copying its endpoint to
// each missing control, rather than by degree-elevating the line.
fn endpoint(command: PathEl) -> Point {
    match command {
        PathEl::MoveTo(p) | PathEl::LineTo(p) | PathEl::QuadTo(_, p) | PathEl::CurveTo(_, _, p) => {
            p
        }
        PathEl::ClosePath => Point::new(f64::NAN, f64::NAN),
    }
}
fn control(command: PathEl, second: bool) -> Option<Point> {
    match (command, second) {
        (PathEl::QuadTo(p, _), false)
        | (PathEl::CurveTo(p, _, _), false)
        | (PathEl::CurveTo(_, p, _), true) => Some(p),
        _ => None,
    }
}
fn path_point(a: Point, b: Point, t: f64) -> Point {
    Point::new((1. - t) * a.x + t * b.x, (1. - t) * a.y + t * b.y)
}
fn split_segment(start: PathEl, end: PathEl, count: usize) -> Vec<PathEl> {
    if !matches!(
        end,
        PathEl::LineTo(_) | PathEl::QuadTo(_, _) | PathEl::CurveTo(_, _, _)
    ) {
        let copy = match start {
            PathEl::MoveTo(p) => PathEl::LineTo(p),
            _ => start,
        };
        let mut out = vec![copy; count - 1];
        out.push(end);
        return out;
    }
    let mut remaining = vec![endpoint(start)];
    if let Some(c) = control(end, false) {
        remaining.push(c);
    }
    if let Some(c) = control(end, true) {
        remaining.push(c);
    }
    remaining.push(endpoint(end));
    fn command(points: &[Point]) -> PathEl {
        match points {
            [_, end] => PathEl::LineTo(*end),
            [_, control, end] => PathEl::QuadTo(*control, *end),
            [_, c1, c2, end] => PathEl::CurveTo(*c1, *c2, *end),
            _ => unreachable!(),
        }
    }
    let mut out = Vec::with_capacity(count);
    let increment = 1. / count as f64;
    for i in 0..count - 1 {
        let t = increment / (1. - increment * i as f64);
        let mut level = remaining;
        let mut left = Vec::new();
        let mut right = Vec::new();
        loop {
            left.push(level[0]);
            right.push(*level.last().unwrap());
            if level.len() == 1 {
                break;
            }
            level = level
                .windows(2)
                .map(|pair| path_point(pair[0], pair[1], t))
                .collect();
        }
        right.reverse();
        out.push(command(&left));
        remaining = right;
    }
    out.push(command(&remaining));
    out
}
fn extend_path(commands: &[PathEl], count: usize) -> Vec<PathEl> {
    let ratio = (commands.len() - 1) as f64 / (count - 1) as f64;
    let mut segments = vec![0; commands.len()];
    for i in 0..count - 1 {
        segments[(ratio * i as f64).floor() as usize] += 1;
    }
    let mut out = vec![commands[0]];
    for (i, count) in segments.into_iter().enumerate().filter(|(_, n)| *n != 0) {
        if i + 1 == commands.len() {
            let copy = match commands[i] {
                PathEl::MoveTo(p) => PathEl::LineTo(p),
                other => other,
            };
            out.extend(std::iter::repeat_n(copy, count));
        } else {
            out.extend(split_segment(commands[i], commands[i + 1], count));
        }
    }
    out
}
fn convert_command(a: PathEl, b: PathEl) -> PathEl {
    if std::mem::discriminant(&a) == std::mem::discriminant(&b) || matches!(b, PathEl::MoveTo(_)) {
        return a;
    }
    let p = if matches!(a, PathEl::ClosePath) {
        Point::ORIGIN
    } else {
        endpoint(a)
    };
    match b {
        PathEl::LineTo(_) => PathEl::LineTo(p),
        PathEl::QuadTo(_, _) => PathEl::QuadTo(control(a, false).unwrap_or(p), p),
        PathEl::CurveTo(_, _, _) => PathEl::CurveTo(
            control(a, false).unwrap_or(p),
            control(a, true).unwrap_or(p),
            p,
        ),
        PathEl::ClosePath => PathEl::ClosePath,
        PathEl::MoveTo(_) => unreachable!(),
    }
}
fn path(a: &BezPath, b: &BezPath, t: f64) -> BezPath {
    if a == b {
        return a.clone();
    }
    // At t=1 D3 restores the original target, including its command count and Z.
    if t == 1. || (a.is_empty() && b.is_empty()) {
        return b.clone();
    }
    let mut ac = a.elements().to_vec();
    let mut bc = b.elements().to_vec();
    let closed = |c: &[PathEl]| c.is_empty() || matches!(c.last(), Some(PathEl::ClosePath));
    let add_close = closed(&ac) && closed(&bc);
    if matches!(ac.last(), Some(PathEl::ClosePath)) {
        ac.pop();
    }
    if matches!(bc.last(), Some(PathEl::ClosePath)) {
        bc.pop();
    }
    // A Z-only path has no drawable command (D3's parser cannot interpolate it).
    if ac.is_empty() && bc.is_empty() {
        return BezPath::new();
    }
    if ac.is_empty() {
        ac.push(bc[0]);
    } else if bc.is_empty() {
        bc.push(ac[0]);
    }
    if ac.len() < bc.len() {
        ac = extend_path(&ac, bc.len());
    }
    if bc.len() < ac.len() {
        bc = extend_path(&bc, ac.len());
    }
    let mut out = BezPath::new();
    for (a, b) in ac.into_iter().zip(bc) {
        let a = convert_command(a, b);
        if t == 0. {
            out.push(a);
            continue;
        }
        let p = path_point(endpoint(a), endpoint(b), t);
        let c = |second| {
            path_point(
                control(a, second).unwrap_or(Point::new(f64::NAN, f64::NAN)),
                control(b, second).unwrap_or(Point::new(f64::NAN, f64::NAN)),
                t,
            )
        };
        out.push(match a {
            PathEl::MoveTo(_) => PathEl::MoveTo(p),
            PathEl::LineTo(_) => PathEl::LineTo(p),
            PathEl::QuadTo(_, _) => PathEl::QuadTo(c(false), p),
            PathEl::CurveTo(_, _, _) => PathEl::CurveTo(c(false), c(true), p),
            PathEl::ClosePath => PathEl::ClosePath,
        });
    }
    if add_close {
        out.close_path();
    }
    out
}
fn shape(a: &Shape, b: &Shape, t: f64) -> Shape {
    match (a, b) {
        (
            Shape::Rect {
                bounds: a,
                radius: ar,
                fill: af,
                stroke: as_,
            },
            Shape::Rect {
                bounds: b,
                radius: br,
                fill: bf,
                stroke: bs,
            },
        ) => Shape::Rect {
            bounds: rect(*a, *b, t),
            radius: number(*ar, *br, t),
            fill: color(*af, *bf, t),
            stroke: stroke(*as_, *bs, t),
        },
        (
            Shape::Circle {
                center: a,
                radius: ar,
                fill: af,
                stroke: as_,
            },
            Shape::Circle {
                center: b,
                radius: br,
                fill: bf,
                stroke: bs,
            },
        ) => Shape::Circle {
            center: point(*a, *b, t),
            radius: number(*ar, *br, t),
            fill: color(*af, *bf, t),
            stroke: stroke(*as_, *bs, t),
        },
        (
            Shape::Path {
                path: a,
                fill: af,
                stroke: as_,
            },
            Shape::Path {
                path: b,
                fill: bf,
                stroke: bs,
            },
        ) => Shape::Path {
            path: path(a, b, t),
            fill: fill(*af, *bf, t),
            stroke: stroke(*as_, *bs, t),
        },
        (
            Shape::Text {
                origin: a,
                text: at,
                size: az,
                color: ac,
            },
            Shape::Text {
                origin: b,
                text: bt,
                size: bz,
                color: bc,
            },
        ) => Shape::Text {
            origin: point(*a, *b, t),
            text: if t < 0.5 { at.clone() } else { bt.clone() },
            size: number(*az, *bz, t),
            color: color(*ac, *bc, t),
        },
        _ => {
            if t < 0.5 {
                a.clone()
            } else {
                b.clone()
            }
        }
    }
}

fn components_affine(components: &[TransformComponent]) -> Affine {
    components
        .iter()
        .fold(Affine::IDENTITY, |m, c| m * c.affine())
}
fn affine_components(m: Affine) -> Vec<TransformComponent> {
    let [a, b, _, _, x, y] = m.as_coeffs();
    let sx = a.hypot(b);
    let mut components = vec![TransformComponent::Translate(Vec2::new(x, y))];
    let angle = b.atan2(a);
    if angle != 0. {
        components.push(TransformComponent::Rotate {
            radians: angle,
            center: Point::ORIGIN,
        });
    }
    let sy = if sx == 0. { 0. } else { m.determinant() / sx };
    if (sx - 1.).abs() > 1e-12 || (sy - 1.).abs() > 1e-12 {
        components.push(TransformComponent::Scale(Vec2::new(sx, sy)));
    }
    components
}
fn object_transforms(a: &Object, b: &Object, t: f64) -> Option<Vec<TransformComponent>> {
    if a.transform_components.is_none() && b.transform_components.is_none() {
        return None;
    }
    let ac = a
        .transform_components
        .clone()
        .unwrap_or_else(|| affine_components(a.transform));
    let bc = b
        .transform_components
        .clone()
        .unwrap_or_else(|| affine_components(b.transform));
    let matches = |components: &[TransformComponent], matrix: Affine| {
        components_affine(components)
            .as_coeffs()
            .into_iter()
            .zip(matrix.as_coeffs())
            .all(|(a, b)| (a - b).abs() <= 1e-9)
    };
    if !matches(&ac, a.transform) || !matches(&bc, b.transform) {
        return None;
    }
    interpolate_components(ac, bc, t)
}
/// Port of svgx/transform.ts: collapse neighboring translations, normalize the
/// identity translation/scale for emergence, then interpolate like operations.
fn interpolate_components(
    mut a: Vec<TransformComponent>,
    mut b: Vec<TransformComponent>,
    t: f64,
) -> Option<Vec<TransformComponent>> {
    use TransformComponent::{Rotate, Scale, Translate};
    fn collapse(components: Vec<TransformComponent>) -> Vec<TransformComponent> {
        let mut out = Vec::new();
        for c in components {
            if let (Translate(next), Some(Translate(previous))) = (c, out.last_mut()) {
                *previous += next;
            } else {
                out.push(c);
            }
        }
        out
    }
    if a.is_empty() && b.is_empty() {
        return Some(Vec::new());
    }
    if a.is_empty() && b.iter().all(|c| matches!(c, Translate(_))) {
        a.push(Translate(Vec2::ZERO));
    }
    if b.is_empty() && a.iter().all(|c| matches!(c, Translate(_))) {
        b.push(Translate(Vec2::ZERO));
    }
    let a_scale = a.iter().any(|c| matches!(c, Scale(_)));
    let b_scale = b.iter().any(|c| matches!(c, Scale(_)));
    if a_scale || b_scale {
        a = collapse(a);
        b = collapse(b);
        let a_translate = a.iter().any(|c| matches!(c, Translate(_)));
        let b_translate = b.iter().any(|c| matches!(c, Translate(_)));
        if a_translate && !b_translate {
            b.insert(0, Translate(Vec2::ZERO));
        }
        if b_translate && !a_translate {
            a.insert(0, Translate(Vec2::ZERO));
        }
        if !a_scale {
            a.push(Scale(Vec2::new(1., 1.)));
        }
        if !b_scale {
            b.push(Scale(Vec2::new(1., 1.)));
        }
    } else {
        if a.is_empty() {
            return Some(b);
        }
        if b.is_empty() {
            return Some(a);
        }
    }
    a = collapse(a);
    b = collapse(b);
    if a.len() != b.len() {
        return None;
    }
    a.into_iter()
        .zip(b)
        .map(|(a, b)| match (a, b) {
            (Translate(a), Translate(b)) => Some(Translate(a + (b - a) * t)),
            (Scale(a), Scale(b)) => Some(Scale(a + (b - a) * t)),
            (
                Rotate {
                    radians: a,
                    center: ac,
                },
                Rotate {
                    radians: b,
                    center: bc,
                },
            ) => {
                let delta = ((b - a) % std::f64::consts::TAU + 3. * std::f64::consts::PI)
                    % std::f64::consts::TAU
                    - std::f64::consts::PI;
                Some(Rotate {
                    radians: a + delta * t,
                    center: point(ac, bc, t),
                })
            }
            _ => None,
        })
        .collect()
}

fn affine(a: Affine, b: Affine, t: f64) -> Affine {
    if a == b {
        return a;
    }
    fn decompose(m: Affine) -> [f64; 6] {
        let [a, b, c, d, x, y] = m.as_coeffs();
        let sx = a.hypot(b);
        [
            x,
            y,
            b.atan2(a),
            sx,
            m.determinant() / sx,
            (a * c + b * d) / (sx * sx),
        ]
    }
    let da = decompose(a);
    let db = decompose(b);
    if da.iter().chain(&db).any(|n| !n.is_finite()) {
        let aa = a.as_coeffs();
        let bb = b.as_coeffs();
        return Affine::new(std::array::from_fn(|i| number(aa[i], bb[i], t)));
    }
    let mut delta = (db[2] - da[2]).rem_euclid(std::f64::consts::TAU);
    if delta >= std::f64::consts::PI {
        delta -= std::f64::consts::TAU;
    }
    let rotation = da[2] + delta * t;
    let sx = number(da[3], db[3], t);
    let sy = number(da[4], db[4], t);
    let shear = number(da[5], db[5], t);
    let (sin, cos) = rotation.sin_cos();
    Affine::new([
        sx * cos,
        sx * sin,
        shear * sx * cos - sy * sin,
        shear * sx * sin + sy * cos,
        number(da[0], db[0], t),
        number(da[1], db[1], t),
    ])
}

#[cfg(test)]
mod emergence_tests {
    use super::*;

    fn origin(position: Point) -> Object {
        Object::circle("origin", position, 30., BLUE).decorative()
    }
    fn clone_at(position: Point) -> Object {
        let mut object = Object::circle("copy", position, 8., GOLD);
        object.opacity = 0.6;
        object.emerge_from = Some("origin".into());
        object.emerge_mode = EmergeMode::Clone;
        object
    }
    fn scene(objects: Vec<Object>) -> Scene {
        let mut scene = Scene::new(200., 100.);
        scene.objects = objects;
        scene
    }
    fn assert_position(object: &Object, expected: Point) {
        assert!((object.transform * Point::ORIGIN).distance(expected) < 1e-9);
    }

    #[test]
    fn clones_keep_destination_geometry_and_opacity_in_both_directions() {
        let before = scene(vec![origin(Point::new(20., 50.))]);
        let copy = clone_at(Point::new(100., 70.));
        let after = scene(vec![origin(Point::new(40., 55.)), copy.clone()]);
        for parts in [
            vec![(before.clone(), 0.75), (after.clone(), 0.25)],
            vec![(after, 0.25), (before, 0.75)],
        ] {
            let mixed = blend(parts);
            let object = mixed.object("copy").unwrap();
            assert_position(object, Point::new(40., 55.));
            assert!((object.opacity - 0.6).abs() < 1e-9);
            assert_eq!(object.shapes, copy.shapes);
            assert_eq!(object.hit, copy.hit);
            assert!((object.transform.determinant() - 1.).abs() < 1e-9);
        }
    }

    #[test]
    fn clones_use_the_origin_in_each_missing_scene_of_a_multi_state_blend() {
        let mixed = blend(vec![
            (scene(vec![origin(Point::new(0., 0.))]), 0.25),
            (scene(vec![origin(Point::new(40., 0.))]), 0.25),
            (scene(vec![clone_at(Point::new(120., 0.))]), 0.5),
        ]);
        let object = mixed.object("copy").unwrap();
        assert_position(object, Point::new(70., 0.));
        assert!((object.opacity - 0.6).abs() < 1e-9);
        assert!((object.transform.determinant() - 1.).abs() < 1e-9);
    }

    #[test]
    fn clones_stay_behind_existing_labels_in_destination_paint_order() {
        let label = Object::text("label", (50., 20.), "key", 12., INK);
        let before = scene(vec![origin(Point::new(20., 50.)), label.clone()]);
        let after = scene(vec![
            origin(Point::new(20., 50.)),
            clone_at(Point::new(100., 70.)),
            label,
        ]);
        let mixed = blend(vec![(before, 0.75), (after, 0.25)]);
        let at = |id| {
            mixed
                .objects
                .iter()
                .position(|object| object.id == id)
                .unwrap()
        };
        assert!(at("copy") < at("label"));
    }

    #[test]
    fn absent_origins_fade_normally_and_default_emergence_still_scales() {
        let copy = clone_at(Point::new(100., 70.));
        let mixed = blend(vec![(scene(vec![]), 0.5), (scene(vec![copy.clone()]), 0.5)]);
        let missing = mixed.object("copy").unwrap();
        assert_position(missing, Point::new(100., 70.));
        assert!((missing.opacity - 0.3).abs() < 1e-9);
        let mut scaled = copy;
        scaled.emerge_mode = EmergeMode::Scale;
        let mixed = blend(vec![
            (scene(vec![origin(Point::new(20., 50.))]), 0.5),
            (scene(vec![scaled]), 0.5),
        ]);
        let object = mixed.object("copy").unwrap();
        assert_position(object, Point::new(60., 60.));
        assert!((object.opacity - 0.3).abs() < 1e-9);
        assert!(object.transform.determinant() < 0.3);
        assert_eq!(
            Object::circle("default", Point::ORIGIN, 1., BLUE).emerge_mode,
            EmergeMode::Auto
        );
    }
}

#[cfg(test)]
mod interpolation_tests {
    use super::*;

    // Executed against pinned upstream dependencies (d3-color 3.1.0,
    // d3-interpolate 3.0.1, d3-interpolate-path 2.3.0), not calculated by Rust.
    #[test]
    fn lab_colors_match_javascript_reference_including_transparency_and_extrapolation() {
        let fixtures = [
            ([255, 0, 0, 255], [0, 0, 255, 255], 0.5, [193, 0, 136, 255]),
            (
                [0, 0, 0, 255],
                [255, 255, 255, 255],
                0.5,
                [119, 119, 119, 255],
            ),
            (
                [0, 255, 0, 255],
                [0, 0, 255, 255],
                0.25,
                [106, 200, 116, 255],
            ),
            (
                [250, 100, 15, 128],
                [20, 50, 240, 64],
                0.6,
                [168, 63, 159, 90],
            ),
            ([255, 0, 0, 0], [0, 80, 220, 255], 0.25, [0, 80, 220, 64]),
            ([220, 20, 100, 255], [0, 0, 0, 0], 0.75, [220, 20, 100, 64]),
            ([3, 5, 8, 255], [25, 17, 4, 255], 0.5, [16, 12, 6, 255]),
            (
                [255, 100, 200, 255],
                [10, 210, 50, 255],
                1.25,
                [0, 226, 0, 255],
            ),
            ([0, 0, 0, 0], [0, 0, 0, 0], 0.5, [0, 0, 0, 0]),
        ];
        for (a, b, t, expected) in fixtures {
            assert_eq!(color(a, b, t), expected, "{a:?} -> {b:?} at {t}");
        }
    }
    fn svg(s: &str) -> BezPath {
        if s.is_empty() {
            BezPath::new()
        } else {
            BezPath::from_svg(s).unwrap()
        }
    }
    fn assert_path(actual: &BezPath, expected: &BezPath) {
        assert_eq!(
            actual.elements().len(),
            expected.elements().len(),
            "{actual:?} != {expected:?}"
        );
        for (a, b) in actual.elements().iter().zip(expected.elements()) {
            assert_eq!(std::mem::discriminant(a), std::mem::discriminant(b));
            if !matches!(a, PathEl::ClosePath) {
                assert!(
                    endpoint(*a).distance(endpoint(*b)) < 1e-10,
                    "{a:?} != {b:?}"
                );
            }
            for second in [false, true] {
                if let Some(c) = control(*a, second) {
                    assert!(
                        c.distance(control(*b, second).unwrap()) < 1e-10,
                        "{a:?} != {b:?}"
                    );
                }
            }
        }
    }
    #[test]
    fn paths_match_javascript_reference_for_changed_counts_types_closure_and_empty_inputs() {
        let fixtures = [
            (
                "M0,0L12,0",
                "M0,10L4,10L8,14L12,10",
                0.5,
                "M0,5L4,5L8,7L12,5",
            ),
            (
                "M0,0C0,12,12,12,12,0",
                "M0,6C0,8,2,8,4,6C6,8,8,8,10,6C10,8,12,8,12,6",
                0.25,
                "M0,1.5C0,5,1.5,7,3.3333333333333335,7.5C5.166666666666666,9,7.333333333333332,9,9.166666666666664,7.499999999999998C10.499999999999998,7,12,5,12,1.5",
            ),
            ("M0,0Q6,12,12,0", "M0,6L6,8L12,6", 0.5, "M0,3L6,7L12,3"),
            (
                "M0,0L12,12",
                "M0,2C2,4,8,10,12,14",
                0.0,
                "M0,0C12,12,12,12,12,12",
            ),
            (
                "M0,0Q6,12,12,0",
                "M0,2C2,4,8,10,12,14",
                0.5,
                "M0,1C4,8,10,5,12,7",
            ),
            ("", "M4,8L12,20L20,8Z", 0.5, "M4,8L8,14L12,8Z"),
            ("M4,8L12,20L20,8Z", "", 0.75, "M4,8L6,11L8,8Z"),
            ("M0,0L12,0Z", "M0,2L6,8L12,2Z", 0.5, "M0,1L6,4L12,1Z"),
            ("M0,0L12,0Z", "M0,2L6,8L12,2", 0.5, "M0,1L6,4L12,1"),
            ("M0,0L12,0L12,12", "M0,2L12,2", 1.0, "M0,2L12,2"),
            (
                "M0,0L12,0M20,0L30,0",
                "M0,4L12,4M20,4L30,4",
                0.25,
                "M0,1L12,1M20,1L30,1",
            ),
            (
                "M0,0L12,0",
                "M0,10L4,10L8,14L12,10",
                -0.25,
                "M0,-2.5L4,-2.5L7.999999999999998,-3.5L12,-2.5",
            ),
            ("M2,3", "M0,0Q6,12,12,0", 0.5, "M1,1.5Q4,7.5,7,1.5"),
            (
                "M0,0L6,8L12,0",
                "M0,0L3,2L6,4L9,2L12,0",
                0.25,
                "M0,0L3,3.5L6,7L9,3.5L12,0",
            ),
            (
                "M0,0Q6,12,12,0",
                "M0,0Q2,2,4,0Q6,2,8,0Q10,2,12,0",
                0.3,
                "M0,0Q2,3.4,4,3.7333333333333334Q5.999999999999999,5.266666666666666,7.999999999999998,3.733333333333333Q10,3.4,11.999999999999998,0",
            ),
            (
                "M0,0C0,12,12,12,12,0",
                "M0,2Q6,8,12,2",
                0.4,
                "M0,0.8Q2.4000000000000004,10.399999999999999,12,0.8",
            ),
            ("M0,0L12,0", "M0,2L6,8L12,2", 0.0, "M0,0L6,0L12,0"),
            (
                "M0,0L12,0M20,0L30,0",
                "M0,4L6,4L12,4M20,4L25,4L30,4",
                0.5,
                "M0,2L6,2L12,2L16,2L22.5,2L30,2",
            ),
        ];
        for (a, b, t, expected) in fixtures {
            assert_path(&path(&svg(a), &svg(b), t), &svg(expected));
        }
    }
    fn assert_affine(actual: Affine, expected: Affine) {
        for (a, b) in actual.as_coeffs().into_iter().zip(expected.as_coeffs()) {
            assert!((a - b).abs() < 1e-10, "{actual:?} != {expected:?}");
        }
    }
    #[test]
    fn rotations_match_upstream_negative_half_turn_and_wraparound_references() {
        // Actual svgx/transform.ts outputs: rotate(-90), rotate(90), rotate(355).
        for (a, b, t, expected) in [
            (0_f64, 180_f64, 0.5, -90_f64),
            (180., 0., 0.5, 90.),
            (350., 10., 0.25, 355.),
        ] {
            assert_affine(
                affine(
                    Affine::rotate(a.to_radians()),
                    Affine::rotate(b.to_radians()),
                    t,
                ),
                Affine::rotate(expected.to_radians()),
            );
        }
        // Source: translate(40,20) rotate(180) scale(3,2).
        let a = Affine::translate((20., 30.))
            * Affine::rotate(170_f64.to_radians())
            * Affine::scale_non_uniform(2., 3.);
        let b = Affine::translate((60., 10.))
            * Affine::rotate(-170_f64.to_radians())
            * Affine::scale_non_uniform(4., 1.);
        assert_affine(
            affine(a, b, 0.5),
            Affine::translate((40., 20.))
                * Affine::rotate(std::f64::consts::PI)
                * Affine::scale_non_uniform(3., 2.),
        );
    }
    #[test]
    fn none_paint_fades_with_the_other_color_without_a_midpoint_jump() {
        let blue = [0, 80, 220, 255];
        assert_eq!(fill(None, Some(blue), 0.25), Some([0, 80, 220, 64]));
        assert_eq!(
            stroke(Some((blue, 3.)), None, 0.75),
            Some(([0, 80, 220, 64], 3.))
        );
        assert_eq!(fill(None, None, 0.5), None);
        assert_eq!(
            fill(None, Some([0, 80, 220, 128]), 0.25),
            Some([0, 80, 220, 64])
        );
        assert_eq!(fill(None, Some([0, 80, 220, 0]), 0.25), Some([0, 0, 0, 64]));
    }
    #[test]
    fn retained_transform_lists_match_pivoted_javascript_reference() {
        use TransformComponent::{Rotate as R, Translate as T};
        let rotate = |degrees: f64| R {
            radians: degrees.to_radians(),
            center: Point::ORIGIN,
        };
        let a = vec![T(Vec2::new(2., 3.)), rotate(0.), T(Vec2::new(20., 0.))];
        let b = vec![T(Vec2::new(2., 3.)), rotate(90.), T(Vec2::new(0., 20.))];
        let mut before = Scene::new(100., 100.);
        before.push(Object::circle("p", Point::ORIGIN, 5., BLUE).transforms(a));
        let mut after = Scene::new(100., 100.);
        after.push(Object::circle("p", Point::ORIGIN, 5., BLUE).transforms(b));
        let mixed = blend(vec![(before, 0.5), (after, 0.5)]);
        // Actual pinned transform.ts output: T(2,3) R(45) T(10,10).
        let expected = Affine::translate((2., 3.))
            * Affine::rotate(45_f64.to_radians())
            * Affine::translate((10., 10.));
        assert_affine(mixed.object("p").unwrap().transform, expected);
        assert!((mixed.object("p").unwrap().transform * Point::ORIGIN).y > 17.);
    }
    #[test]
    fn spring_discrete_properties_are_from_target_and_between_ties_use_first() {
        let old = Object::text("label", (0., 0.), "old", 10., BLUE).tooltip("old");
        let mut target = Object::text("label", (100., 0.), "new", 20., GOLD).tooltip("new");
        target.cursor = Cursor::Pointer;
        let mut before = Scene::new(200., 100.);
        before.push(old);
        before.push(Object::circle("other", (0., 20.), 5., BLUE));
        let mut after = Scene::new(200., 100.);
        after.push(Object::circle("other", (0., 20.), 5., BLUE));
        after.push(target);
        let spring = blend_transition(after.clone(), before.clone(), 0.1);
        let label = spring.object("label").unwrap();
        assert_eq!(label.tooltip.as_deref(), Some("new"));
        assert_eq!(label.cursor, Cursor::Pointer);
        assert!(
            matches!(&label.shapes[0],Shape::Text { text,size,.. } if text == "new" && (*size-11.).abs()<1e-10)
        );
        assert_affine(label.transform, Affine::translate((10., 0.)));
        assert_eq!(spring.objects[0].id, "other");
        let mixed = blend(vec![(before, 0.5), (after, 0.5)]);
        assert!(
            matches!(&mixed.object("label").unwrap().shapes[0],Shape::Text { text,.. } if text == "old")
        );
        assert_eq!(
            mixed.object("label").unwrap().tooltip.as_deref(),
            Some("old")
        );
    }
    #[test]
    fn emergence_matches_executed_upstream_direct_and_wrapped_rect_fixtures() {
        let mut origin = Object::new(
            "origin",
            Affine::translate((20., 50.)),
            None,
            vec![Shape::Rect {
                bounds: Rect::new(0., 0., 40., 20.),
                radius: 0.,
                fill: BLUE,
                stroke: None,
            }],
        )
        .label("origin", (0., 12.), 10., INK);
        origin.opacity = 1.;
        let bounds = Rect::new(2., 3., 14., 11.);
        let copy = Object::new(
            "copy",
            Affine::translate((100., 70.)),
            Some(Hit::Rect(bounds)),
            vec![Shape::Rect {
                bounds,
                radius: 0.,
                fill: GOLD,
                stroke: None,
            }],
        )
        .label("copy", (4., 7.), 10., INK);
        // These six cases are from actual createSyntheticBefore in svgx/lerp.tsx.
        let fixtures = [
            (EmergeMode::Auto, true, 40., 20., 12., 0., 1.),
            (EmergeMode::Auto, false, 12., 8., 7., 0., 0.),
            (EmergeMode::Clone, true, 40., 20., 12., 0.6, 1.),
            (EmergeMode::Clone, false, 12., 8., 7., 0.6, 1.),
            (EmergeMode::Scale, true, 12., 8., 7., 0., 0.),
            (EmergeMode::Scale, false, 12., 8., 7., 0., 0.),
        ];
        for (mode, direct, width, height, text_y, opacity, scale) in fixtures {
            let mut copy = copy.clone();
            copy.opacity = 0.6;
            copy.emerge_from = Some("origin".into());
            copy.emerge_mode = mode;
            copy.emerge_bounds = direct;
            let mut before = Scene::new(200., 100.);
            before.push(origin.clone());
            let mut after = Scene::new(200., 100.);
            after.push(copy);
            let mut parts = vec![(before, 0.75), (after, 0.25)];
            augment_emerging(&mut parts);
            let synthetic = parts[0].0.object("copy").unwrap();
            assert_eq!(synthetic.opacity, opacity);
            assert_affine(
                synthetic.transform,
                Affine::translate((20., 50.)) * Affine::scale(scale),
            );
            assert!(
                matches!(synthetic.shapes[0],Shape::Rect { bounds,.. } if bounds==Rect::new(2.,3.,2.+width,3.+height))
            );
            assert!(
                matches!(synthetic.shapes[1],Shape::Text { origin,.. } if origin==Point::new(4.,text_y))
            );
        }
    }
}
