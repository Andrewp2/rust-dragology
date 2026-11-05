use crate::{DragContext, DragFrame, DragSpec, Preview};
use std::rc::Rc;

/// Run a child spec on one field of a larger model. Candidate geometry is
/// computed in the parent layout, so siblings and ancestor transforms can
/// influence where the child appears. Each proposed parent starts from `base`.
///
/// ```
/// use dragology::{Affine, DragSession, between, substate};
///
/// #[derive(Clone)]
/// struct Panel { offset: f64, enabled: bool }
/// let panel = Panel { offset: 20.0, enabled: false };
/// let spec = substate(panel.clone(), |p| &mut p.enabled, between([false, true]));
/// let layout = |p: &Panel| Some(Affine::translate((p.offset + if p.enabled { 60.0 } else { 0.0 }, 0.0)));
/// let drag = DragSession::begin(panel, (20.0, 0.0), layout, spec)?;
/// assert!(drag.finish((75.0, 0.0))?.enabled);
/// # Ok::<_, dragology::DragError>(())
/// ```
pub fn substate<'a, Parent: Clone + 'a, Child: Clone + 'a>(
    mut base: Parent,
    field: impl Fn(&mut Parent) -> &mut Child + 'a,
    mut spec: DragSpec<'a, Child>,
) -> DragSpec<'a, Parent> {
    let start = Rc::new(field(&mut base).clone());
    let replace = move |child: &Child| {
        let mut parent = base.clone();
        *field(&mut parent) = child.clone();
        parent
    };
    DragSpec::operator("substate", move |ctx, pointer| {
        let replace_ref = &replace;
        let preview_geometry = ctx.preview_geometry.as_ref().map(|geometry| {
            Box::new(move |preview: &Preview<Child>, anchor| {
                let parent = map_preview(preview, &|state| Rc::new(replace_ref(state)));
                geometry(&parent, anchor)
            }) as Box<crate::PreviewGeometry<'_, Child>>
        });
        let inner = DragContext {
            start: start.clone(),
            press: ctx.press,
            anchor: ctx.anchor,
            locate: Box::new(|child| (ctx.locate)(&replace(child))),
            preview_geometry,
        };
        let result = spec.evaluate(&inner, pointer)?;
        let drop_state = Rc::new(replace(&result.drop_state));
        let preview = map_preview(&result.preview, &|state| {
            if Rc::ptr_eq(state, &result.drop_state) {
                drop_state.clone()
            } else {
                Rc::new(replace(state))
            }
        });
        Ok(DragFrame {
            preview,
            drop_state,
            gap: result.gap,
            grab_position: result.grab_position,
            active_path: result.active_path,
            drop_transition: result.drop_transition,
            branch_transition: result.branch_transition,
            chain: result.chain,
        })
    })
}

fn map_preview<S, T>(preview: &Preview<S>, map: &impl Fn(&Rc<S>) -> Rc<T>) -> Rc<Preview<T>> {
    Rc::new(match preview {
        Preview::State(state) => Preview::State(map(state)),
        Preview::Blend(parts) => Preview::Blend(
            parts
                .iter()
                .map(|(p, w)| (map_preview(p, map), *w))
                .collect(),
        ),
        Preview::Floating {
            backdrop,
            source,
            translation,
            ghost_opacity,
        } => {
            let view = map_preview(backdrop, map);
            let source = if Rc::ptr_eq(backdrop, source) {
                view.clone()
            } else {
                map_preview(source, map)
            };
            Preview::Floating {
                backdrop: view,
                source,
                translation: *translation,
                ghost_opacity: *ghost_opacity,
            }
        }
    })
}
