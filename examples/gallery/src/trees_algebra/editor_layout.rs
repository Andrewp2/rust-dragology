//! Layout formulas from animate-algebra/nool-tree.ts and the two editor canvases.
use super::*;
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Style {
    Builder,
    Macro,
    Normal,
}
pub(super) fn size(t: &Tree, style: Style) -> Vec2 {
    let children: Vec<_> = t.children.iter().map(|c| size(c, style)).collect();
    let area = !children.is_empty() || (style != Style::Normal && op_arity(&t.label) > 0);
    let cw = children
        .iter()
        .map(|s| s.x)
        .reduce(f64::max)
        .unwrap_or(if area { 12. } else { 0. });
    let ch = if !children.is_empty() {
        children.iter().map(|s| s.y).sum::<f64>() + 10. * children.len().saturating_sub(1) as f64
    } else if area {
        12.
    } else {
        20.
    };
    Vec2::new(30. + if area { 10. + cw } else { 0. }, 10. + ch.max(20.))
}
pub(super) fn draw(
    sc: &mut Scene,
    t: &Tree,
    at: Point,
    style: Style,
    parent: Option<&str>,
    in_arrow: bool,
    interactive: bool,
    opacity: f64,
) {
    let sz = size(t, style);
    let hole = t.label == "◯";
    let arrow = t.label == "→";
    let valid = t.children.len() == op_arity(&t.label);
    let area = !t.children.is_empty() || op_arity(&t.label) > 0;
    let stroke = if style == Style::Normal {
        [128, 128, 128, 255]
    } else if hole && style == Style::Builder {
        if in_arrow {
            [196, 181, 253, 255]
        } else {
            [187, 187, 187, 255]
        }
    } else if !valid {
        [221, 51, 51, 255]
    } else if arrow && style == Style::Builder {
        [124, 58, 237, 255]
    } else if style == Style::Macro || area {
        [128, 128, 128, 255]
    } else {
        [0, 0, 0, 0]
    };
    let fill = if hole && style == Style::Builder {
        if in_arrow {
            [237, 233, 254, 255]
        } else {
            [238, 238, 238, 255]
        }
    } else {
        [0, 0, 0, 0]
    };
    let inset = if hole && style == Style::Builder {
        3.
    } else {
        0.
    };
    let bounds = Rect::new(inset, inset, sz.x - inset, sz.y - inset);
    let radius = if hole && style == Style::Builder {
        (sz.y - 6.) / 2.
    } else {
        14f64.min(0.3 * sz.x.min(sz.y))
    };
    let mut o = Object::new(
        &t.id,
        Affine::translate(at.to_vec2()),
        interactive.then_some(Hit::Rect(Rect::new(0., 0., sz.x, sz.y))),
        vec![Shape::Rect {
            bounds,
            radius,
            fill,
            stroke: Some((
                stroke,
                if arrow && style == Style::Builder {
                    2.
                } else {
                    1.
                },
            )),
        }],
    );
    if !(hole && style == Style::Builder) {
        o = o
            .label(
                &t.label,
                (15., sz.y / 2.),
                if style == Style::Builder { 24. } else { 20. },
                if style == Style::Normal {
                    [0, 0, 0, 255]
                } else if !valid {
                    [221, 51, 51, 255]
                } else if arrow && style == Style::Builder {
                    [124, 58, 237, 255]
                } else {
                    [0, 0, 0, 255]
                },
            )
            .text_anchor(TextAnchor::Middle)
            .text_baseline(TextBaseline::Middle);
    }
    o.parent = parent.map(str::to_owned);
    o.opacity = opacity;
    o.emerge_from = t.emerge.clone();
    o.emerge_mode = t.emerge_mode;
    sc.push(o);
    let mut y = at.y + 5.;
    for c in &t.children {
        draw(
            sc,
            c,
            Point::new(at.x + 35., y),
            style,
            Some(&t.id),
            in_arrow || arrow,
            interactive && c.label != "◯",
            opacity,
        );
        y += size(c, style).y + 10.;
    }
}
fn bordered(id: &str, r: Rect, radius: f64, fill: Color, stroke: Color, width: f64) -> Object {
    let mut o = Object::rect(id, r, radius, fill).decorative();
    if let Shape::Rect { stroke: s, .. } = &mut o.shapes[0] {
        *s = Some((stroke, width));
    }
    o
}
pub(super) fn scene(s: &AlgebraState, dragged: Option<&str>) -> Scene {
    if s.mode == Algebra::Macro {
        return macro_scene(s, dragged);
    }
    builder_scene(s, dragged)
}
fn macro_scene(s: &AlgebraState, _dragged: Option<&str>) -> Scene {
    let style = Style::Macro;
    let mut sc = Scene::new(400., 400.);
    if s.recording.is_none() {
        draw(
            &mut sc,
            &s.trees[0],
            Point::ORIGIN,
            Style::Normal,
            None,
            false,
            true,
            1.,
        );
        return sc;
    }
    let toolkit_w = s
        .brushes
        .iter()
        .map(|t| size(t, style).x)
        .fold(30., f64::max)
        + 16.;
    let toolkit_h = 16. + s.brushes.iter().map(|t| size(t, style).y + 8.).sum::<f64>() - 8.;
    let gutter_w = (s
        .palette
        .iter()
        .map(|t| size(t, style).x)
        .fold(0., f64::max)
        + 16.)
        .max(46.);
    let gutter_h =
        (16. + s.palette.iter().map(|t| size(t, style).y + 8.).sum::<f64>() - 8.).max(62.);
    let gx = toolkit_w + 15.;
    let tx = gx + gutter_w + 15.;
    let tree_size = size(&s.trees[0], style);
    let trash = tx + tree_size.x + 15.;
    let h = toolkit_h.max(gutter_h).max(tree_size.y);
    sc.push(
        bordered(
            "macro-border",
            Rect::new(-8., -8., trash + 38., h + 8.),
            12.,
            [0, 0, 0, 0],
            [124, 58, 237, 255],
            3.,
        )
        .dash(vec![8., 4.]),
    );
    sc.push(bordered(
        "macro-toolkit-bg",
        Rect::new(0., 0., toolkit_w, toolkit_h),
        14f64.min(0.3 * toolkit_w.min(toolkit_h)),
        [240, 240, 240, 255],
        [204, 204, 204, 255],
        1.,
    ));
    sc.push(
        bordered(
            "macro-gutter-bg",
            Rect::new(gx, 0., gx + gutter_w, gutter_h.max(toolkit_h)),
            14f64.min(0.3 * gutter_w.min(gutter_h.max(toolkit_h))),
            [248, 248, 248, 255],
            [221, 221, 221, 255],
            1.,
        )
        .dash(vec![4., 4.]),
    );
    let can_insert = s.trees[0].nodes().iter().any(|n| op_arity(&n.label) > 0);
    let mut y = 8.;
    for t in &s.brushes {
        draw(
            &mut sc,
            t,
            Point::new(8., y),
            style,
            None,
            false,
            false,
            if can_insert { 1. } else { 0.35 },
        );
        sc.objects.iter_mut().find(|o| o.id == t.id).unwrap().hit = Some(Hit::Rect(Rect::new(
            0.,
            0.,
            size(t, style).x,
            size(t, style).y,
        )));
        y += size(t, style).y + 8.;
    }
    let mut y = 8.;
    for t in &s.palette {
        draw(
            &mut sc,
            t,
            Point::new(gx + 8., y),
            style,
            None,
            false,
            true,
            1.,
        );
        y += size(t, style).y + 8.;
    }
    sc.push(
        bordered(
            "trash",
            Rect::new(trash, 0., trash + 30., 30.),
            4.,
            [0, 0, 0, 0],
            [204, 204, 204, 255],
            1.,
        )
        .dash(vec![4., 4.])
        .label("🗑", (15., 15.), 16., [204, 204, 204, 255])
        .text_anchor(TextAnchor::Middle)
        .text_baseline(TextBaseline::Middle),
    );
    draw(
        &mut sc,
        &s.trees[0],
        Point::new(tx, 0.),
        style,
        None,
        false,
        true,
        1.,
    );
    sc
}
fn builder_scene(s: &AlgebraState, dragged: Option<&str>) -> Scene {
    let style = Style::Builder;
    let mut sc = Scene::new(600., 800.);
    let mut brushes = vec![];
    let mut first_bucket = true;
    for &bucket in &s.order {
        if !s.buckets[bucket] {
            continue;
        }
        for (i, t) in s
            .brushes
            .iter()
            .enumerate()
            .filter(|(i, _)| brush_bucket(*i) == bucket)
        {
            let mut t = t.clone();
            if i < 4 {
                t.children = (0..op_arity(&t.label))
                    .map(|j| atom(&format!("{}-c{j}", t.id), "◯"))
                    .collect();
            }
            let bucket_start = i
                == if bucket == 0 {
                    0
                } else if bucket == 1 {
                    4
                } else {
                    8
                };
            brushes.push((t, bucket, bucket_start && !first_bucket));
        }
        first_bucket = false;
    }
    let mut max_w = brushes
        .iter()
        .map(|(t, _, _)| size(t, style).x)
        .fold(30., f64::max);
    for pair in brushes.windows(2) {
        if pair[0].1 == 2 && pair[1].1 == 2 && !pair[1].2 {
            max_w = max_w.max(size(&pair[0].0, style).x + 8. + size(&pair[1].0, style).x);
        }
    }
    let bw = max_w + 16.;
    let mut by = 8.;
    let mut atom_col = 0;
    let mut placements = vec![];
    let mut dividers = vec![];
    for (i, (t, bucket, start)) in brushes.iter().enumerate() {
        if *start {
            if atom_col == 1 {
                by += size(&brushes[i - 1].0, style).y + 8.;
                atom_col = 0;
            }
            by += 12.;
            dividers.push(by - 10.);
        }
        let x = if *bucket == 2 && atom_col == 1 {
            size(&brushes[i - 1].0, style).x + 8.
        } else {
            0.
        };
        placements.push(Point::new(x, by));
        if *bucket == 2 {
            if atom_col == 0 {
                atom_col = 1;
            } else {
                by += size(t, style).y.max(size(&brushes[i - 1].0, style).y) + 8.;
                atom_col = 0;
            }
        } else {
            by += size(t, style).y + 8.;
        }
    }
    if atom_col == 1 {
        by += brushes
            .last()
            .map(|(t, _, _)| size(t, style).y + 8.)
            .unwrap_or(0.);
    }
    let bh = by;
    let pw = (s
        .palette
        .iter()
        .map(|t| size(t, style).x)
        .fold(0., f64::max)
        + 16.)
        .max(46.);
    let ph = (16. + s.palette.iter().map(|t| size(t, style).y + 8.).sum::<f64>() - 8.).max(62.);
    let sw = (s
        .trees
        .iter()
        .map(|t| size(t, style).x)
        .reduce(f64::max)
        .unwrap_or(46.)
        + 16.)
        .max(46.);
    let sh = (16. + s.trees.iter().map(|t| size(t, style).y + 8.).sum::<f64>() - 8.).max(62.);
    let bx = 60.;
    let px = bx + bw + 12.;
    let sx = if s.expanded { px + pw + 12. } else { px };
    let vx = sx + sw + 12.;
    let toggle = bx + bw + 6.;
    let mut separators = vec![(54., bh.max(74.))];
    if s.expanded {
        separators.extend([(toggle, bh.max(ph)), (px + pw + 6., ph.max(sh))]);
    } else {
        separators.push((toggle, bh.max(sh)));
    }
    separators.push((sx + sw + 6., sh.max(30.)));
    for (i, (x, h)) in separators.into_iter().enumerate() {
        sc.push(
            Object::line(
                format!("sep-{i}"),
                (x, 9.),
                (x, h - 9.),
                [221, 221, 221, 255],
                1.,
            )
            .line_cap(LineCap::Round),
        );
    }
    for (i, &bucket) in s.order.iter().enumerate() {
        let y = 15. + i as f64 * 22.;
        let active = s.buckets[bucket];
        let mut icon = Object::text(
            format!("bucket-{bucket}"),
            (8., y),
            ["◯ →", "◎ →", "⊙ →"][bucket],
            14.,
            if active {
                [51, 51, 51, 255]
            } else {
                [204, 204, 204, 255]
            },
        )
        .text_baseline(TextBaseline::Middle);
        icon = icon.clickable();
        icon.hit = Some(Hit::Rect(Rect::new(0., -7., 36., 7.)));
        sc.push(icon);
    }
    sc.push(
        Object::circle(
            "palette-toggle",
            (toggle, 9.),
            4.,
            if s.expanded {
                [153, 153, 153, 255]
            } else {
                [221, 221, 221, 255]
            },
        )
        .clickable(),
    );
    for (i, y) in dividers.into_iter().enumerate() {
        sc.push(Object::line(
            format!("brush-div-{i}"),
            (bx + 8., y),
            (bx + bw - 8., y),
            [221, 221, 221, 255],
            1.,
        ));
    }
    for ((t, _, _), at) in brushes.iter().zip(placements) {
        draw(
            &mut sc,
            t,
            at + Vec2::new(bx + 8., 0.),
            style,
            None,
            false,
            false,
            1.,
        );
        let sz = size(t, style);
        sc.objects.iter_mut().find(|o| o.id == t.id).unwrap().hit =
            Some(Hit::Rect(Rect::new(0., 0., sz.x, sz.y)));
    }
    if s.expanded {
        let mut y = 8.;
        for t in &s.palette {
            draw(
                &mut sc,
                t,
                Point::new(px + 8., y),
                style,
                None,
                false,
                false,
                1.,
            );
            let sz = size(t, style);
            sc.objects.iter_mut().find(|o| o.id == t.id).unwrap().hit =
                Some(Hit::Rect(Rect::new(0., 0., sz.x, sz.y)));
            y += sz.y + 8.;
        }
    }
    let mut y = 8.;
    for t in &s.trees {
        draw(
            &mut sc,
            t,
            Point::new(sx + 8., y),
            style,
            None,
            false,
            true,
            1.,
        );
        y += size(t, style).y + 8.;
    }
    if s.trees.is_empty() {
        sc.push(Object::new(
            "empty-stage",
            Affine::translate((sx + 8., 8.)),
            None,
            vec![],
        ));
    }
    let voided = s.voided.is_some();
    let mut trash = Object::circle(
        "trash",
        (vx + 15., 15.),
        15.,
        [245, 198, 198, if voided { 204 } else { 0 }],
    )
    .label(
        "🗑",
        (0., 0.),
        if voided { 18. } else { 16. },
        if s.voids.is_empty() {
            [204, 204, 204, 255]
        } else {
            [153, 153, 153, 255]
        },
    )
    .text_anchor(TextAnchor::Middle)
    .text_baseline(TextBaseline::Middle)
    .decorative();
    trash.hit = None;
    sc.push(trash);
    if !s.voids.is_empty() {
        sc.push(
            Object::circle("void-count", (vx + 27., 7.), 7., [197, 48, 48, 255])
                .label(s.voids.len().to_string(), (0., 0.), 9., [255; 4])
                .font_weight(700)
                .text_anchor(TextAnchor::Middle)
                .text_baseline(TextBaseline::Middle)
                .decorative(),
        );
    }
    if let Some(t) = s.voided.as_ref().or_else(|| s.voids.first()) {
        let opacity = if !voided && dragged == Some(t.id.as_str()) {
            1.
        } else {
            0.
        };
        draw(
            &mut sc,
            t,
            Point::new(vx, 0.),
            style,
            None,
            false,
            false,
            opacity,
        );
        if !voided {
            let o = sc.objects.iter_mut().find(|o| o.id == t.id).unwrap();
            let sz = size(t, style);
            o.hit = Some(Hit::Rect(Rect::new(0., 0., sz.x, sz.y)));
            *o = o.clone().hit_when_transparent();
        }
    }
    sc
}
