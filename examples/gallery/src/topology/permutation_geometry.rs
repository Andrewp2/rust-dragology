//! The source permutation lab's five simultaneous views and rewrite geometry.
use super::*;
pub(super) fn cols(s: &PermState) -> usize {
    13.max(s.n * (s.n - 1) / 2 + 4)
}
pub(super) fn cw(s: &PermState) -> f64 {
    570. / cols(s) as f64
}
pub(super) fn blank(s: &PermState, j: usize) -> bool {
    matches!(s.motion,Some(PermMotion::Pair{start,amplitude,..})if (j==start||j==start+1)&&amplitude==0.)
}
fn width(s: &PermState, j: usize) -> f64 {
    cw(s)
        * match s.motion {
            Some(PermMotion::Pair { start, width, .. }) if j == start || j == start + 1 => width,
            _ => 1.,
        }
}
fn x(s: &PermState, j: usize) -> f64 {
    48. + (0..j).map(|k| width(s, k)).sum::<f64>()
}
fn after(s: &PermState, j: usize, row: usize) -> usize {
    if blank(s, j) {
        return row;
    }
    s.word.get(j).map_or(row, |l| {
        if row == l.i {
            row + 1
        } else if row == l.i + 1 {
            row - 1
        } else {
            row
        }
    })
}
fn label(
    scene: &mut Scene,
    id: impl Into<String>,
    p: impl Into<Point>,
    value: impl Into<String>,
    size: f64,
    color: Color,
) {
    let id = id.into();
    let is_caption = id != "word-status" && id != "ph-unavailable";
    let value = value.into();
    scene.push(
        Object::text(
            id,
            p,
            if is_caption {
                value.to_uppercase()
            } else {
                value
            },
            size,
            color,
        )
        .text_baseline(TextBaseline::Alphabetic)
        .letter_spacing(if is_caption { 0.5 } else { 0. }),
    );
}
fn heading(scene: &mut Scene, id: &str, p: Point, value: &str, width: f64) {
    scene.push(
        Object::text(id, p, value.to_uppercase(), 11., [55, 65, 81, 255])
            .text_baseline(TextBaseline::Alphabetic)
            .font_weight(600)
            .letter_spacing(1.),
    );
    scene.push(Object::line(
        format!("{id}-rule"),
        p + Vec2::new(0., 7.),
        p + Vec2::new(width, 7.),
        [229, 231, 235, 255],
        1.,
    ));
}
fn tile(id: String, p: Point, v: usize, muted: bool) -> Object {
    let mut o = Object::rect(
        id,
        Rect::from_origin_size(p, (34., 34.)),
        5.,
        if muted {
            [243, 244, 246, 255]
        } else {
            STRAND_FILLS[v % 6]
        },
    )
    .label(
        (v + 1).to_string(),
        (17., 17.),
        17.,
        if muted {
            [156, 163, 175, 255]
        } else {
            [31, 41, 55, 255]
        },
    )
    .text_anchor(TextAnchor::Middle)
    .text_baseline(TextBaseline::Central)
    .font_weight(500);
    if let Shape::Rect { stroke, .. } = &mut o.shapes[0] {
        *stroke = Some((
            if muted {
                [209, 213, 219, 255]
            } else {
                STRAND_STROKES[v % 6]
            },
            1.5,
        ));
    }
    o
}
fn outline_circle(id: impl Into<String>, p: Point, r: f64, color: Color, width: f64) -> Object {
    let mut o = Object::circle(id, p, r, [0; 4]).decorative();
    if let Shape::Circle { stroke, .. } = &mut o.shapes[0] {
        *stroke = Some((color, width));
    }
    o
}
fn arc(center: Point, r: f64, start: f64, sweep: f64) -> BezPath {
    let mut path = BezPath::new();
    let parts = (sweep.abs() / std::f64::consts::FRAC_PI_2).ceil().max(1.) as usize;
    let d = sweep / parts as f64;
    for k in 0..parts {
        let a = start + d * k as f64;
        let b = a + d;
        let c = 4. / 3. * (d / 4.).tan();
        let p = center + Vec2::new(r * a.cos(), r * a.sin());
        let q = center + Vec2::new(r * b.cos(), r * b.sin());
        if k == 0 {
            path.move_to(p);
        }
        path.curve_to(
            p + Vec2::new(-r * a.sin() * c, r * a.cos() * c),
            q - Vec2::new(-r * b.sin() * c, r * b.cos() * c),
            q,
        );
    }
    path
}
fn arrow(scene: &mut Scene, id: String, tip: Point, direction: Vec2, len: f64, color: Color) {
    let u = direction / direction.hypot();
    let normal = Vec2::new(-u.y, u.x);
    let back = tip - u * (len * (std::f64::consts::PI / 6.).cos());
    let delta = normal * (len * (std::f64::consts::PI / 6.).sin());
    let mut path = BezPath::new();
    path.move_to(tip);
    path.line_to(back + delta);
    path.line_to(back - delta);
    path.close_path();
    scene.push(Object::new(
        id,
        Affine::IDENTITY,
        None,
        vec![Shape::Path {
            path,
            fill: Some(color),
            stroke: None,
        }],
    ));
}
fn front_vertex(s: &PermState, p: &[usize]) -> bool {
    s.n != 4
        || !s.solid
        || poly_faces()
            .iter()
            .any(|f| face_facing(s, f) > 0. && f.vertices.iter().any(|v| v == p))
}
fn front_edge(s: &PermState, a: &[usize], b: &[usize]) -> bool {
    s.n != 4
        || !s.solid
        || poly_faces().iter().any(|f| {
            face_facing(s, f) > 0.
                && f.vertices.iter().any(|v| v == a)
                && f.vertices.iter().any(|v| v == b)
        })
}
fn ph(scene: &mut Scene, s: &PermState, p: &[usize]) {
    label(
        scene,
        "ph-caption",
        (484., 44.),
        "VERTEX = π, WALK = WORD",
        11.,
        [107, 114, 128, 255],
    );
    if s.n > 4 {
        label(
            scene,
            "ph-unavailable",
            (492., 160.),
            "(drawn for n ≤ 4 only)",
            11.,
            [156, 163, 175, 255],
        );
        return;
    }
    let solid = s.n == 4 && s.solid;
    let mut layers: Vec<(u8, Object)> = Vec::new();
    if solid {
        label(
            scene,
            "face-hint",
            (484., 272.),
            "DRAG A FACE TO TURN IT",
            11.,
            [107, 114, 128, 255],
        );
        for face in poly_faces() {
            let front = face_facing(s, face) > 0.;
            let center = face_center(s, face);
            let mut path = BezPath::new();
            for (i, p) in face.vertices.iter().enumerate() {
                let p = perm_vertex(p, s) - center.to_vec2();
                if i == 0 {
                    path.move_to(p)
                } else {
                    path.line_to(p)
                }
            }
            path.close_path();
            let light = dot3(
                quat_rotate(view_rotation(s), face.normal),
                unit3([-0.5, 0.7, 0.8]),
            )
            .max(0.);
            let rgb = if face.vertices.len() == 4 {
                [245., 222., 179.]
            } else {
                [214., 224., 239.]
            };
            let color = [
                (rgb[0] * (0.84 + 0.2 * light)).round().min(255.) as u8,
                (rgb[1] * (0.84 + 0.2 * light)).round().min(255.) as u8,
                (rgb[2] * (0.84 + 0.2 * light)).round().min(255.) as u8,
                if front { 184 } else { 0 },
            ];
            layers.push((
                if front { 4 } else { 1 },
                Object::new(
                    format!("face-{}", face.mask),
                    Affine::translate(center.to_vec2()),
                    Some(Hit::Path(path.clone(), 1.)),
                    vec![Shape::Path {
                        path,
                        fill: Some(color),
                        stroke: None,
                    }],
                ),
            ));
        }
    }
    for a in perms((0..s.n).collect()) {
        let pos = perm_vertex(&a, s);
        for i in 0..s.n - 1 {
            let mut b = a.clone();
            b.swap(i, i + 1);
            if a >= b {
                continue;
            }
            let front = front_edge(s, &a, &b);
            let line = Object::line(
                format!("edge-{a:?}-{i}"),
                pos,
                perm_vertex(&b, s),
                if solid {
                    if front {
                        [71, 85, 105, 255]
                    } else {
                        [100, 116, 139, 255]
                    }
                } else {
                    [209, 213, 219, 255]
                },
                if solid && front { 1.25 } else { 1. },
            );
            layers.push((
                if solid { if front { 5 } else { 2 } } else { 0 },
                if solid && !front {
                    line.dash(vec![3., 3.])
                } else {
                    line
                },
            ));
        }
        if !solid {
            layers.push((
                0,
                Object::circle(format!("vertex-{a:?}"), pos, 2.5, [156, 163, 175, 255])
                    .decorative(),
            ));
            if s.n == 3 {
                let v = projected_permutation(&a);
                layers.push((
                    0,
                    Object::text(
                        format!("vertex-label-{a:?}"),
                        pos + v.to_vec2() * 14.,
                        a.iter().map(|v| (v + 1).to_string()).collect::<String>(),
                        9.,
                        [156, 163, 175, 255],
                    )
                    .text_anchor(TextAnchor::Middle)
                    .text_baseline(TextBaseline::Central),
                ));
            }
        }
    }
    let mut walk = (0..s.n).collect::<Vec<_>>();
    let mut verts = vec![walk.clone()];
    for j in 0..s.word.len() {
        if !blank(s, j) {
            walk.swap(s.word[j].i, s.word[j].i + 1);
        }
        verts.push(walk.clone());
    }
    verts.push(walk.clone());
    for k in 1..verts.len() {
        let front = front_edge(s, &verts[k - 1], &verts[k]);
        let mut line = Object::line(
            format!("walk-line-{}", k - 1),
            perm_vertex(&verts[k - 1], s),
            perm_vertex(&verts[k], s),
            [37, 99, 235, 255],
            3.,
        )
        .line_cap(LineCap::Round);
        line.opacity = if k <= s.word.len() && blank(s, k - 1) {
            0.
        } else if front {
            0.4
        } else {
            0.5
        };
        layers.push((if solid { if front { 6 } else { 3 } } else { 1 }, line));
    }
    for j in 1..s.word.len() {
        let a = &s.word[j - 1];
        let b = &s.word[j];
        let front = front_vertex(s, &verts[j]);
        let z = if solid { if front { 6 } else { 3 } } else { 1 };
        if a.i == b.i || a.i.abs_diff(b.i) >= 2 {
            let mut o = Object::circle(
                format!("walk-corner-{j}"),
                perm_vertex(&verts[j], s),
                7.,
                [0; 4],
            );
            o.shapes.push(Shape::Circle {
                center: Point::ORIGIN,
                radius: 2.5,
                fill: [37, 99, 235, 179],
                stroke: None,
            });
            layers.push((z, o));
        }
        if braid_move(s.n, &s.word, j - 1, false).is_some() {
            let a = perm_vertex(&verts[j], s);
            let b = perm_vertex(&verts[j + 1], s);
            let mid = a.lerp(b, 0.5);
            let mut path = BezPath::new();
            path.move_to(a - mid.to_vec2());
            path.line_to(b - mid.to_vec2());
            layers.push((
                z,
                Object::new(
                    format!("walk-braid-{j}"),
                    Affine::translate(mid.to_vec2()),
                    Some(Hit::Path(path.clone(), 10.)),
                    vec![Shape::Path {
                        path,
                        fill: None,
                        stroke: Some(([0; 4], 10.)),
                    }],
                )
                .line_cap(LineCap::Round),
            ));
        }
    }
    let mut token = Object::circle("walk", perm_vertex(p, s), 8., [37, 99, 235, 255]);
    if let Shape::Circle { stroke, .. } = &mut token.shapes[0] {
        *stroke = Some(([255; 4], 2.));
    }
    token.opacity = if front_vertex(s, p) { 1. } else { 0.55 };
    layers.push((7, token));
    layers.sort_by_key(|(z, _)| *z);
    for (_, o) in layers {
        scene.push(o);
    }
}
pub(super) fn drawing(s: &PermState) -> Scene {
    let expanded;
    let s = if let Some((start, letters)) = &s.closed {
        expanded = {
            let mut t = s.clone();
            t.closed = None;
            t.word.splice(*start..*start, letters.clone());
            t.motion = Some(PermMotion::Pair {
                start: *start,
                width: 0.,
                amplitude: 0.,
            });
            t
        };
        &expanded
    } else {
        s
    };
    let mut scene = Scene::new(680., 560.);
    let p = permutation(s.n, &s.word);
    let inv = inversions(&p);
    heading(
        &mut scene,
        "permutation-heading",
        Point::new(20., 16.),
        "the permutation π",
        440.,
    );
    heading(
        &mut scene,
        "poly-heading",
        Point::new(484., 16.),
        "permutohedron",
        196.,
    );
    heading(
        &mut scene,
        "word-heading",
        Point::new(20., 290.),
        "a word for π",
        640.,
    );
    label(
        &mut scene,
        "two-line-caption",
        (20., 44.),
        "TWO-LINE NOTATION",
        11.,
        [107, 114, 128, 255],
    );
    for (i, &v) in p.iter().enumerate() {
        scene.push(
            tile(
                format!("top-{i}"),
                Point::new(38. + i as f64 * 34., 56.),
                i,
                true,
            )
            .decorative(),
        );
        scene.push(tile(
            format!("tile-{v}"),
            Point::new(38. + i as f64 * 34., 98.),
            v,
            false,
        ));
    }
    for (id, a, b, c) in [
        ("left", (-6., -4.), (-18., 38.), (-6., 80.)),
        (
            "right",
            (s.n as f64 * 34. + 6., -4.),
            (s.n as f64 * 34. + 18., 38.),
            (s.n as f64 * 34. + 6., 80.),
        ),
    ] {
        let mut path = BezPath::new();
        path.move_to((38. + a.0, 56. + a.1));
        path.quad_to((38. + b.0, 56. + b.1), (38. + c.0, 56. + c.1));
        scene.push(Object::path(
            format!("notation-{id}"),
            path,
            [156, 163, 175, 255],
            1.5,
        ));
    }
    label(
        &mut scene,
        "cycle-caption",
        (20., 162.),
        "CYCLE DIAGRAM",
        11.,
        [107, 114, 128, 255],
    );
    let mut offset = 0.;
    for cycle in cycle_list(&p) {
        let n = cycle.len();
        let r = if n == 1 { 0. } else { 22f64.max(6. * n as f64) };
        let center = Point::new(20. + offset + r + 12., 216.);
        if n > 1 {
            scene.push(outline_circle(
                format!("cycle-ring-{}", cycle[0]),
                center,
                r,
                [229, 231, 235, 255],
                1.,
            ));
        }
        for (k, &v) in cycle.iter().enumerate() {
            let theta = -std::f64::consts::FRAC_PI_2 + TAU * k as f64 / n as f64;
            let pos = center + Vec2::new(r * theta.cos(), r * theta.sin());
            if n == 1 {
                let center = center + Vec2::new(0., -15.245);
                let a = (6.245f64 / 8.).asin();
                let path = arc(
                    center,
                    8.,
                    std::f64::consts::PI - a,
                    TAU - (std::f64::consts::PI - 2. * a),
                );
                scene.push(Object::path(
                    format!("cycle-self-{v}"),
                    path,
                    [156, 163, 175, 255],
                    1.5,
                ));
                arrow(
                    &mut scene,
                    format!("cycle-arrow-{v}"),
                    pos + Vec2::new(5., -9.),
                    Vec2::new(-0.6, 1.),
                    6.,
                    [156, 163, 175, 255],
                );
            } else {
                let delta = (16. / r).min(1.).asin();
                let a = theta + delta;
                let b = theta + TAU / n as f64 - delta;
                scene.push(Object::path(
                    format!("cycle-arc-{v}"),
                    arc(center, r, a, b - a),
                    [107, 114, 128, 255],
                    1.5,
                ));
                arrow(
                    &mut scene,
                    format!("cycle-arrow-{v}"),
                    center + Vec2::new(r * b.cos(), r * b.sin()),
                    Vec2::new(-b.sin(), b.cos()),
                    7.,
                    [107, 114, 128, 255],
                );
            }
            let mut o = Object::circle(format!("cycle-{v}"), pos, 12., STRAND_FILLS[v % 6])
                .label((v + 1).to_string(), (0., 0.), 13., [31, 41, 55, 255])
                .text_anchor(TextAnchor::Middle)
                .text_baseline(TextBaseline::Central)
                .font_weight(500);
            if let Shape::Circle { stroke, .. } = &mut o.shapes[0] {
                *stroke = Some((STRAND_STROKES[v % 6], 1.5));
            }
            scene.push(o);
        }
        offset += 2. * (r + 12.) + 14.;
    }
    label(
        &mut scene,
        "matrix-caption",
        (330., 44.),
        "PERMUTATION MATRIX",
        11.,
        [107, 114, 128, 255],
    );
    let mut matrix = Object::rect(
        "matrix-border",
        Rect::new(330., 72., 330. + s.n as f64 * 24., 72. + s.n as f64 * 24.),
        0.,
        [255; 4],
    )
    .decorative();
    if let Shape::Rect { stroke, .. } = &mut matrix.shapes[0] {
        *stroke = Some(([156, 163, 175, 255], 1.));
    }
    scene.push(matrix);
    for k in 1..s.n {
        scene.push(Object::line(
            format!("grid-v-{k}"),
            (330. + k as f64 * 24., 72.),
            (330. + k as f64 * 24., 72. + s.n as f64 * 24.),
            [229, 231, 235, 255],
            1.,
        ));
        scene.push(Object::line(
            format!("grid-h-{k}"),
            (330., 72. + k as f64 * 24.),
            (330. + s.n as f64 * 24., 72. + k as f64 * 24.),
            [229, 231, 235, 255],
            1.,
        ));
    }
    for (row, &v) in p.iter().enumerate() {
        scene.push(
            Object::text(
                format!("matrix-col-label-{row}"),
                (342. + row as f64 * 24., 65.),
                (row + 1).to_string(),
                9.,
                [156, 163, 175, 255],
            )
            .text_anchor(TextAnchor::Middle)
            .text_baseline(TextBaseline::Alphabetic),
        );
        scene.push(
            Object::text(
                format!("matrix-row-label-{row}"),
                (323., 84. + row as f64 * 24.),
                (row + 1).to_string(),
                9.,
                [156, 163, 175, 255],
            )
            .text_anchor(TextAnchor::Middle)
            .text_baseline(TextBaseline::Central),
        );
        let mut dot = Object::circle(
            format!("matrix-{v}"),
            (342. + v as f64 * 24., 84. + row as f64 * 24.),
            10.,
            [0; 4],
        );
        dot.shapes.push(Shape::Circle {
            center: Point::ORIGIN,
            radius: 7.,
            fill: STRAND_STROKES[v % 6],
            stroke: None,
        });
        scene.push(dot);
    }
    label(
        &mut scene,
        "matrix-footnote",
        (330., 86. + s.n as f64 * 24.),
        "ROW I, COLUMN π(I)",
        11.,
        [107, 114, 128, 255],
    );
    ph(&mut scene, s, &p);
    let active = s
        .word
        .iter()
        .enumerate()
        .filter(|(j, _)| !blank(s, *j))
        .collect::<Vec<_>>();
    let length = active.len();
    scene.push(
        Object::text("word-pi", (20., 332.), "π =", 18., [31, 41, 55, 255])
            .text_baseline(TextBaseline::Alphabetic)
            .font_family(FontFamily::Serif)
            .italic(),
    );
    if length == 0 {
        scene.push(
            Object::text("word-identity", (54., 332.), "e", 18., [156, 163, 175, 255])
                .text_baseline(TextBaseline::Alphabetic)
                .font_family(FontFamily::Serif),
        );
    }
    let step = 22f64.min(350. / length.max(1) as f64);
    for (k, (_, l)) in active.iter().enumerate() {
        scene.push(
            Object::text(
                format!("letter-{}", l.key),
                (54. + k as f64 * step, 332.),
                format!("s{}", ['₁', '₂', '₃', '₄', '₅'][l.i]),
                18.,
                [31, 41, 55, 255],
            )
            .text_baseline(TextBaseline::Alphabetic)
            .font_family(FontFamily::Serif)
            .italic(),
        );
    }
    label(
        &mut scene,
        "word-status",
        (20., 362.),
        format!(
            "length {length} · inversions {inv} · {}",
            if length == inv {
                "reduced ✓".into()
            } else {
                format!("not reduced ({} to cancel)", length - inv)
            }
        ),
        13.,
        [55, 65, 81, 255],
    );
    let full = length + 1 >= cols(s);
    label(
        &mut scene,
        "wire-caption",
        (20., 398.),
        if full {
            "WIRING DIAGRAM IS FULL — CANCEL A DOUBLE CROSSING, OR RESET"
        } else {
            "WIRING DIAGRAM (DRAG CROSSINGS PAST EACH OTHER, OR A STRAND OVER ITS NEIGHBOR)"
        },
        11.,
        [107, 114, 128, 255],
    );
    for v in 0..s.n {
        scene.push(
            Object::text(
                format!("wire-start-{v}"),
                (26., 420. + v as f64 * 24.),
                (v + 1).to_string(),
                11.,
                [107, 114, 128, 255],
            )
            .text_anchor(TextAnchor::Middle)
            .text_baseline(TextBaseline::Central),
        );
        let row = p.iter().position(|x| *x == v).unwrap();
        let end = Object::circle(
            format!("wire-end-{v}"),
            (640., 420. + row as f64 * 24.),
            11.,
            [0; 4],
        )
        .label((v + 1).to_string(), (0., 0.), 11., STRAND_STROKES[v % 6])
        .text_anchor(TextAnchor::Middle)
        .text_baseline(TextBaseline::Central)
        .font_weight(600);
        scene.push(end);
        let mut path = BezPath::new();
        path.move_to((34., 420. + v as f64 * 24.));
        let mut row = v;
        let mut segments = 0;
        for (j, l) in s.word.iter().enumerate() {
            if !s.by_column && row != l.i && row != l.i + 1 {
                continue;
            }
            let next = after(s, j, row);
            let x = x(s, j);
            let w = width(s, j);
            let y = 420. + row as f64 * 24.;
            let y2 = 420. + next as f64 * 24.;
            path.line_to((x, y));
            path.curve_to((x + w / 2., y), (x + w / 2., y2), (x + w, y2));
            row = next;
            segments += 1;
        }
        let free = x(s, s.word.len());
        let w = if s.by_column {
            618. - free
        } else {
            (618. - free).clamp(0., cw(s))
        };
        let y = 420. + row as f64 * 24.;
        path.line_to((free, y));
        path.curve_to((free + w / 2., y), (free + w / 2., y), (free + w, y));
        segments += 1;
        if !s.by_column {
            while segments < cols(s) + 2 {
                path.line_to((free + w, y));
                path.curve_to((free + w, y), (free + w, y), (free + w, y));
                segments += 1;
            }
        }
        path.line_to((632., y));
        scene.push(
            Object::path(format!("wire-{v}"), path, STRAND_STROKES[v % 6], 2.5)
                .line_cap(LineCap::Round),
        );
    }
    for (j, l) in s.word.iter().enumerate() {
        if blank(s, j) {
            continue;
        }
        scene.push(Object::circle(
            format!("word-{}", l.key),
            (x(s, j) + cw(s) / 2., 420. + (l.i as f64 + 0.5) * 24.),
            cw(s).min(24.) / 2. - 1.,
            [0; 4],
        ));
    }
    // Pair identical-generator runs from the right, matching doublePairs.
    let mut pairs = BTreeSet::new();
    let mut k = 0;
    while k < s.word.len() {
        let mut run = 1;
        while k + run < s.word.len()
            && s.word[k + run].i == s.word[k].i
            && blank(s, k + run) == blank(s, k)
        {
            run += 1;
        }
        if run >= 2 && run % 2 == 0 {
            pairs.insert(k);
            k += 2;
        } else {
            k += 1;
        }
    }
    for v in 0..s.n {
        let mut row = v;
        let mut j = 0;
        while j <= s.word.len() {
            let next = after(s, j, row);
            let apex = pairs.contains(&j) && next != row;
            let gap = blank(s, j);
            let side = j > 0
                && braid_move(s.n, &s.word, j - 1, false).is_some()
                && s.word.get(j).is_some_and(|l| row != l.i && row != l.i + 1)
                && (row == s.word[j - 1].i || row == s.word[j - 1].i + 1);
            let straight = next == row && (length + 2 < cols(s) || side);
            if apex || gap || straight {
                let bounds = if apex || gap {
                    Rect::new(-8.4, -8.4, 8.4, 8.4)
                } else {
                    Rect::new(-cw(s) * 0.9, -7.2, -cw(s) * 0.1, 7.2)
                };
                let o = Object::new(
                    format!("insert-{j}-{v}"),
                    Affine::translate((
                        x(s, j) + cw(s),
                        420. + if apex { next } else { row } as f64 * 24.,
                    )),
                    if gap { None } else { Some(Hit::Rect(bounds)) },
                    vec![Shape::Rect {
                        bounds,
                        radius: 4.,
                        fill: [0; 4],
                        stroke: None,
                    }],
                );
                scene.push(o);
            }
            if apex || gap {
                j += 2;
            } else {
                row = next;
                j += 1;
            }
        }
    }
    scene
}
