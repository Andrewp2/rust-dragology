use crate::{DragContext, DragError, DragFrame, DragSpec, Point, Preview, Rect, Transition};
use std::rc::Rc;

/// A single candidate model. Geometry is evaluated once per gesture.
pub fn fixed<'a, S: 'a>(state: S) -> DragSpec<'a, S> {
    let state = Rc::new(state);
    let preview = Rc::new(Preview::State(state.clone()));
    let mut cached_position = None;
    DragSpec::operator("fixed", move |ctx, pointer| {
        let position = match cached_position {
            Some(position) => position,
            None => *cached_position.insert(ctx.grab_in(state.as_ref())?),
        };
        Ok(DragFrame {
            preview: preview.clone(),
            drop_state: state.clone(),
            gap: position.map_or(f64::INFINITY, |p| p.distance(pointer)),
            grab_position: position,
            active_path: String::new(),
            drop_transition: None,
            branch_transition: None,
            chain: None,
        })
    })
}

/// Use this model inside a fixed, axis-aligned canvas rectangle. Compose with
/// `when_far` for a fallback outside it. All four edges belong to the target.
pub fn drop_target<'a, S: 'a>(state: S, bounds: Rect) -> DragSpec<'a, S> {
    let mut inner = fixed(state);
    DragSpec::operator("dropTarget", move |ctx, pointer| {
        let mut result = inner.evaluate(ctx, pointer)?;
        let inside = (bounds.x0..=bounds.x1).contains(&pointer.x)
            && (bounds.y0..=bounds.y1).contains(&pointer.y);
        result.gap = if inside { 0.0 } else { f64::INFINITY };
        result.active_path.clear();
        Ok(result)
    })
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ClosestOptions {
    /// Favor the previous winner by this many canvas units.
    pub stickiness: f64,
    /// Stay on a branch once its grab point travels this far from the press.
    /// The branch unlocks when it brings the grab point home again.
    pub lock_past: Option<f64>,
}

/// Follow the branch with the smallest gap. A tie favors declaration order.
/// An empty list previews the starting model with infinite gap.
pub fn closest<'a, S: 'a, T: Into<DragSpec<'a, S>>>(
    targets: impl IntoIterator<Item = T>,
) -> DragSpec<'a, S> {
    closest_with(targets, ClosestOptions::default())
}

pub fn closest_with<'a, S: 'a, T: Into<DragSpec<'a, S>>>(
    targets: impl IntoIterator<Item = T>,
    options: ClosestOptions,
) -> DragSpec<'a, S> {
    let mut branches: Vec<_> = targets.into_iter().map(Into::into).collect();
    let mut previous = None;
    let mut locked: Option<usize> = None;
    DragSpec::operator("closest", move |ctx, pointer| {
        if !valid_distance(options.stickiness)
            || options.lock_past.is_some_and(|d| !valid_distance(d))
        {
            return Err(DragError::InvalidSpec(
                "closest distances must be finite and nonnegative",
            ));
        }
        let past_lock = |result: &DragFrame<S>| -> Result<bool, DragError> {
            match options.lock_past {
                Some(radius) => Ok(ctx
                    .grab_in_frame(result)?
                    .is_none_or(|p| p.distance(ctx.press) > radius)),
                None => Ok(false),
            }
        };
        // If the locked branch comes home, reuse its result in the search.
        let mut evaluated = None;
        if let Some(index) = locked {
            let mut result = branches[index].evaluate(ctx, pointer)?;
            if past_lock(&result)? {
                prefix_path(&mut result.active_path, &index.to_string());
                return Ok(result);
            }
            evaluated = Some((index, result));
            locked = None;
        }
        let mut best: Option<(usize, f64, DragFrame<S>)> = None;
        for (index, branch) in branches.iter_mut().enumerate() {
            let result = if evaluated.as_ref().is_some_and(|(i, _)| *i == index) {
                evaluated.take().unwrap().1
            } else {
                branch.evaluate(ctx, pointer)?
            };
            let bonus = if previous == Some(index) {
                options.stickiness
            } else {
                0.0
            };
            let score = result.gap - bonus;
            if best.as_ref().is_none_or(|(_, low, _)| score < *low) {
                best = Some((index, score, result));
            }
        }
        if let Some((winner, _, mut result)) = best {
            previous = Some(winner);
            if past_lock(&result)? {
                locked = Some(winner);
            }
            prefix_path(&mut result.active_path, &winner.to_string());
            Ok(result)
        } else {
            let mut result = ctx.state_frame(ctx.start.clone(), pointer)?;
            result.gap = f64::INFINITY;
            result.active_path = "none".into();
            Ok(result)
        }
    })
}

fn prefix_path(path: &mut String, prefix: &str) {
    *path = if path.is_empty() {
        prefix.to_owned()
    } else {
        format!("{prefix}/{path}")
    };
}

/// Settings for the source `withChaining` operator. A host resolves the next
/// element's drag specification after committing the intermediate model.
#[derive(Clone, Debug, PartialEq)]
pub struct Chaining {
    pub dragged_id: Option<String>,
    pub transition: Option<Transition>,
}

impl Default for Chaining {
    fn default() -> Self {
        Self {
            dragged_id: None,
            transition: Some(Transition::default()),
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SnapOptions {
    pub transition: Option<Transition>,
    pub chain: bool,
}

#[derive(Default)]
pub struct FloatingOptions<'a> {
    pub ghost_opacity: Option<f64>,
    /// Map distance from the candidate position to the desired float distance.
    pub tether: Option<Box<dyn Fn(f64) -> f64 + 'a>>,
}

/// Rebuild a child only when the sampled value changes, retaining its gesture
/// memory between changes. This is the source `reactTo` operator.
pub fn react_to<'a, S: 'a, V: PartialEq + 'a>(
    mut getter: impl FnMut() -> V + 'a,
    mut build: impl FnMut(&V) -> DragSpec<'a, S> + 'a,
) -> DragSpec<'a, S> {
    let mut current: Option<(V, DragSpec<'a, S>)> = None;
    DragSpec::operator("reactTo", move |ctx, pointer| {
        let value = getter();
        if current.as_ref().is_none_or(|(old, _)| *old != value) {
            let child = build(&value);
            current = Some((value, child));
        }
        current.as_mut().unwrap().1.evaluate(ctx, pointer)
    })
}

fn valid_distance(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

impl<'a, S: 'a> DragSpec<'a, S> {
    /// Evaluate the background when the foreground gap exceeds `radius`.
    pub fn when_far(self, background: impl Into<Self>, radius: f64) -> Self {
        self.when_far_with(background, radius, radius)
    }

    /// Leave the foreground beyond `gap_out` and return at or within `gap_in`.
    pub fn when_far_with(mut self, background: impl Into<Self>, gap_in: f64, gap_out: f64) -> Self {
        let mut background = background.into();
        let mut foreground = true;
        Self::operator("whenFar", move |ctx, pointer| {
            if !valid_distance(gap_in) || !valid_distance(gap_out) || gap_in > gap_out {
                return Err(DragError::InvalidSpec(
                    "when_far needs 0 <= gap_in <= gap_out, both finite",
                ));
            }
            let result = self.evaluate(ctx, pointer)?;
            foreground = result.gap <= if foreground { gap_out } else { gap_in };
            let mut result = if foreground {
                result
            } else {
                background.evaluate(ctx, pointer)?
            };
            prefix_path(
                &mut result.active_path,
                if foreground { "fg" } else { "bg" },
            );
            Ok(result)
        })
    }

    /// Change the committed model while keeping the child's preview and gap.
    /// The mapping is pure and is evaluated on each frame.
    pub fn on_drop(mut self, map: impl Fn(&S) -> S + 'a) -> Self {
        Self::operator("onDrop", move |ctx, pointer| {
            let mut result = self.evaluate(ctx, pointer)?;
            result.drop_state = Rc::new(map(&result.drop_state));
            Ok(result)
        })
    }

    /// Transform the candidate model and recompute both the preview and its gap.
    pub fn during(mut self, map: impl Fn(&S) -> S + 'a) -> Self {
        Self::operator("during", move |ctx, pointer| {
            let result = self.evaluate(ctx, pointer)?;
            let mapped = ctx.state_frame(Rc::new(map(&result.drop_state)), pointer)?;
            Ok(DragFrame {
                preview: mapped.preview,
                drop_state: mapped.drop_state,
                gap: mapped.gap,
                grab_position: mapped.grab_position,
                ..result
            })
        })
    }

    /// Snap the preview to its drop model when their grab points are within
    /// `radius` of one another. The inner branch's score is preserved.
    pub fn with_snap_radius(self, radius: f64) -> Self {
        self.with_snap_radius_options(radius, SnapOptions::default())
    }

    pub fn with_snap_radius_options(mut self, radius: f64, options: SnapOptions) -> Self {
        Self::operator("withSnapRadius", move |ctx, pointer| {
            if !valid_distance(radius) {
                return Err(DragError::InvalidSpec(
                    "snap radius must be finite and nonnegative",
                ));
            }
            let mut result = self.evaluate(ctx, pointer)?;
            let grab = ctx
                .grab_in_frame(&result)?
                .ok_or(DragError::MissingElement)?;
            let target = ctx
                .grab_in(&result.drop_state)?
                .ok_or(DragError::MissingElement)?;
            let snapped = grab.distance(target) <= radius;
            if snapped {
                result.preview = Rc::new(Preview::State(result.drop_state.clone()));
            }
            result.grab_position = Some(if snapped { target } else { grab });
            if options.transition.is_some() {
                prefix_path(
                    &mut result.active_path,
                    if snapped { "snapped" } else { "unsnapped" },
                );
            }
            result.branch_transition = options.transition.map(Some);
            result.chain = (options.chain && snapped).then_some(Chaining {
                dragged_id: None,
                transition: options.transition,
            });
            Ok(result)
        })
    }

    /// Float the dragged element over the child's preview, retaining the child's
    /// gap and drop model. If the child removes the element, carry its last appearance.
    pub fn with_floating(self) -> Self {
        self.with_floating_options(FloatingOptions::default())
    }

    pub fn with_floating_options(mut self, options: FloatingOptions<'a>) -> Self {
        let mut last_source: Option<(Rc<Preview<S>>, Point)> = None;
        Self::operator("withFloating", move |ctx: &DragContext<'_, S>, pointer| {
            let mut result = self.evaluate(ctx, pointer)?;
            if options
                .ghost_opacity
                .is_some_and(|v| !v.is_finite() || !(0.0..=1.0).contains(&v))
            {
                return Err(DragError::InvalidSpec(
                    "ghost opacity must be between zero and one",
                ));
            }
            let mut target = pointer;
            // Floating extracts the actual rendered element, not an operator's
            // cached weighted target position (which may differ under rotation).
            if let Some(grab) = ctx.grab_in_preview(&result.preview)? {
                last_source = Some((result.preview.clone(), grab));
                if let Some(tether) = &options.tether {
                    let delta = pointer - grab;
                    let distance = delta.hypot();
                    if distance > 1e-6 {
                        let length = tether(distance);
                        if !length.is_finite() {
                            return Err(DragError::GeometryOutOfRange);
                        }
                        target = grab + delta * (length / distance);
                    }
                }
            }
            let (source, grab) = last_source
                .get_or_insert_with(|| (Rc::new(Preview::State(ctx.start.clone())), ctx.press));
            result.preview = Rc::new(Preview::Floating {
                backdrop: result.preview,
                source: source.clone(),
                translation: target - *grab,
                ghost_opacity: options.ghost_opacity,
            });
            result.grab_position = options.tether.is_none().then_some(target);
            result.drop_transition = None;
            result.branch_transition = None;
            result.chain = None;
            Ok(result)
        })
    }

    pub fn with_drop_transition(mut self, transition: Option<Transition>) -> Self {
        Self::operator("withDropTransition", move |ctx, pointer| {
            let mut result = self.evaluate(ctx, pointer)?;
            result.drop_transition = Some(transition);
            Ok(result)
        })
    }

    pub fn with_branch_transition(mut self, transition: Option<Transition>) -> Self {
        Self::operator("withBranchTransition", move |ctx, pointer| {
            let mut result = self.evaluate(ctx, pointer)?;
            result.branch_transition = Some(transition);
            Ok(result)
        })
    }

    pub fn with_chaining(self) -> Self {
        self.with_chaining_options(Chaining::default())
    }

    pub fn with_chaining_options(mut self, options: Chaining) -> Self {
        Self::operator("withChaining", move |ctx, pointer| {
            let mut result = self.evaluate(ctx, pointer)?;
            result.chain.get_or_insert_with(|| options.clone());
            Ok(result)
        })
    }

    pub fn change_gap(mut self, map: impl Fn(f64) -> f64 + 'a) -> Self {
        Self::operator("changeGap", move |ctx, pointer| {
            let mut result = self.evaluate(ctx, pointer)?;
            result.gap = map(result.gap);
            Ok(result)
        })
    }

    pub fn change_frame(mut self, map: impl Fn(Point) -> Point + 'a) -> Self {
        Self::operator("changeFrame", move |ctx, pointer| {
            let pointer = map(pointer);
            if !pointer.is_finite() {
                return Err(DragError::InvalidPointer);
            }
            self.evaluate(ctx, pointer)
        })
    }

    /// Apply a partial update to the child's result. Supplying a preview clears
    /// its cached grab position unless the patch explicitly supplies one, just
    /// as upstream `changeResult` does. Other omitted fields remain unchanged.
    pub fn change_result(mut self, map: impl Fn(&DragFrame<S>) -> DragResultPatch<S> + 'a) -> Self {
        Self::operator("changeResult", move |ctx, pointer| {
            let mut result = self.evaluate(ctx, pointer)?;
            let patch = map(&result);
            if let Some(preview) = patch.preview {
                result.preview = preview;
                if patch.grab_position.is_none() {
                    result.grab_position = None;
                }
            }
            if let Some(state) = patch.drop_state {
                result.drop_state = state;
            }
            if let Some(gap) = patch.gap {
                result.gap = gap;
            }
            if let Some(position) = patch.grab_position {
                result.grab_position = position;
            }
            if let Some(path) = patch.active_path {
                result.active_path = path;
            }
            if let Some(transition) = patch.drop_transition {
                result.drop_transition = transition;
            }
            if let Some(transition) = patch.branch_transition {
                result.branch_transition = transition;
            }
            if let Some(chain) = patch.chain {
                result.chain = chain;
            }
            Ok(result)
        })
    }
}

/// Fields supplied by [`DragSpec::change_result`]. An outer `None` leaves a
/// field untouched; `Some(value)` replaces it, including clearing optional
/// fields. A supplied preview invalidates the grab cache unless `grab_position`
/// is also supplied. No model clone is required to construct an empty patch.
#[derive(Clone, Debug)]
pub struct DragResultPatch<S> {
    pub preview: Option<Rc<Preview<S>>>,
    pub drop_state: Option<Rc<S>>,
    pub gap: Option<f64>,
    pub grab_position: Option<Option<Point>>,
    pub active_path: Option<String>,
    pub drop_transition: Option<Option<Option<Transition>>>,
    pub branch_transition: Option<Option<Option<Transition>>>,
    pub chain: Option<Option<Chaining>>,
}
impl<S> Default for DragResultPatch<S> {
    fn default() -> Self {
        Self {
            preview: None,
            drop_state: None,
            gap: None,
            grab_position: None,
            active_path: None,
            drop_transition: None,
            branch_transition: None,
            chain: None,
        }
    }
}
