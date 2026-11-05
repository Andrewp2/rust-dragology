//! Preserve upstream pages that contain independently draggable canvases.
use crate::*;
use dragology::{DragFrame, DragSession, Preview};
use std::rc::Rc;
pub fn compose<S>(
    states: &[S],
    labels: &[&str],
    dragged: Option<&str>,
    render: impl Fn(&S, Option<&str>) -> Scene,
) -> Scene {
    let mut out = Scene::new(0., 0.).scrolling();
    let mut y = 0.;
    for (i, s) in states.iter().enumerate() {
        let prefix = format!("panel{i}/");
        let mut label = Object::text(
            format!("panel-title-{i}"),
            (0., y + 6.),
            labels[i],
            16.,
            [0, 0, 0, 255],
        )
        .italic()
        .font_weight(500);
        label.hit = None;
        out.push(label);
        y += 30.;
        let local = render(s, dragged.and_then(|id| id.strip_prefix(&prefix)));
        out.width = out.width.max(local.width);
        let height = local.height;
        for mut o in local.objects {
            o.id = format!("{prefix}{}", o.id);
            o.parent = o.parent.map(|id| format!("{prefix}{id}"));
            o.emerge_from = o.emerge_from.map(|id| format!("{prefix}{id}"));
            o.transform = Affine::translate((0., y)) * o.transform;
            out.push(o);
        }
        y += height + 24.;
    }
    out.height = y - 24.;
    out
}
pub fn resolve_id(id: &str) -> Option<(usize, &str)> {
    let (panel, id) = id.split_once('/')?;
    Some((panel.strip_prefix("panel")?.parse().ok()?, id))
}
pub fn offset<S>(states: &[S], index: usize, render: impl Fn(&S) -> Scene) -> Vec2 {
    Vec2::new(
        0.,
        30. + states
            .iter()
            .take(index)
            .map(|s| render(s).height + 54.)
            .sum::<f64>(),
    )
}
pub fn lift<S: Clone + 'static>(
    states: Vec<S>,
    index: usize,
    offset: Vec2,
    spec: DragSpec<'static, S>,
    locate: impl Fn(&S) -> Option<Affine> + 'static,
) -> DragSpec<'static, Vec<S>> {
    fn map_preview<S: Clone>(p: &Preview<S>, states: &[S], index: usize) -> Rc<Preview<Vec<S>>> {
        Rc::new(match p {
            Preview::State(s) => {
                let mut out = states.to_vec();
                out[index] = (**s).clone();
                Preview::State(Rc::new(out))
            }
            Preview::Blend(xs) => Preview::Blend(
                xs.iter()
                    .map(|(p, w)| (map_preview(p, states, index), *w))
                    .collect(),
            ),
            Preview::Floating {
                backdrop,
                source,
                translation,
                ghost_opacity,
            } => Preview::Floating {
                backdrop: map_preview(backdrop, states, index),
                source: map_preview(source, states, index),
                translation: *translation,
                ghost_opacity: *ghost_opacity,
            },
        })
    }
    let mut setup = Some((spec, locate));
    let mut session = None;
    DragSpec::custom(move |ctx, pointer| {
        if session.is_none() {
            let (spec, locate) = setup.take().unwrap();
            session = Some(DragSession::begin(
                states[index].clone(),
                ctx.press_position() - offset,
                locate,
                spec,
            )?);
        }
        let frame = session.as_mut().unwrap().frame(pointer - offset)?;
        let mut dropped = states.clone();
        dropped[index] = (*frame.drop_state).clone();
        Ok(DragFrame {
            preview: map_preview(&frame.preview, &states, index),
            drop_state: Rc::new(dropped),
            gap: frame.gap,
            grab_position: frame.grab_position.map(|p| p + offset),
            active_path: frame.active_path,
            drop_transition: frame.drop_transition,
            branch_transition: frame.branch_transition,
            chain: frame.chain.map(|mut chain| {
                chain.dragged_id = chain.dragged_id.map(|id| format!("panel{index}/{id}"));
                chain
            }),
        })
    })
}
