//! A renderer-independent port of the model-driven drag operators in Dragology.
//!
//! On pointer down, build a specification and start a [`DragSession`]. Supply the
//! same element transforms your layout uses for drawing. On pointer move, draw
//! the returned [`Preview`]. On release, commit the state returned by
//! [`DragSession::finish`].
//!
//! ```
//! use dragology::{Affine, DragSession, between};
//!
//! let knob = |on: &bool| Some(Affine::translate((if *on { 90.0 } else { 30.0 }, 30.0)));
//! let mut drag = DragSession::begin(false, (33.0, 30.0), knob, between([false, true]))?;
//! let frame = drag.frame((48.0, 30.0))?;
//! assert_eq!(frame.grab_position.unwrap().x, 48.0); // the knob follows the pointer
//! assert_eq!(*frame.drop_state, false);              // release here would switch it off
//! assert!(drag.finish((88.0, 30.0))?);
//! # Ok::<_, dragology::DragError>(())
//! ```
#![forbid(unsafe_code)]

mod between;
mod cobyla;
mod spec;
mod substate;
mod transition;
mod vary;

pub use between::{BetweenOptions, Interpolation, between, between_with};
pub use kurbo::{Affine, Point, Rect, Vec2};
pub use spec::{Chaining, DragResultPatch, FloatingOptions, SnapOptions, react_to};
pub use spec::{ClosestOptions, closest, closest_with, drop_target, fixed};
pub use substate::substate;
pub use transition::{Easing, Transition};
pub use vary::{vary, vary_constrained};

use std::{error::Error, fmt, rc::Rc};

/// What to draw for one pointer position. Every leaf is an application model.
/// A renderer can match corresponding elements across leaves by its own IDs.
#[derive(Clone, Debug)]
pub enum Preview<S> {
    /// Draw this model.
    State(Rc<S>),
    /// Blend the rendered children using the given weights (which sum to one).
    Blend(Vec<(Rc<Preview<S>>, f64)>),
    /// Draw the backdrop with the dragged element removed. Extract that element
    /// from `source`, translate it in canvas coordinates, and draw it on top.
    Floating {
        backdrop: Rc<Preview<S>>,
        source: Rc<Preview<S>>,
        translation: Vec2,
        /// Retain a copy at the candidate position with this opacity.
        ghost_opacity: Option<f64>,
    },
}

/// A frame's drawing instructions and the real model a drop here would commit.
#[derive(Clone, Debug)]
pub struct DragFrame<S> {
    pub preview: Rc<Preview<S>>,
    pub drop_state: Rc<S>,
    /// Distance in canvas units used by enclosing operators to choose a branch.
    /// A floating or snapping modifier preserves its child's gap.
    pub gap: f64,
    /// Cached canvas position of the point grabbed on press. A blend reports
    /// its weighted target position, which can differ from its rendered geometry
    /// (for example when transforms interpolate rotations). `None` requests
    /// geometry from the rendered preview, or denotes an absent element.
    pub grab_position: Option<Point>,
    /// Selected operator branches; hosts animate only when this path changes.
    pub active_path: String,
    /// `None` uses the host default; `Some(None)` disables the transition.
    pub drop_transition: Option<Option<Transition>>,
    /// Override the transition when `active_path` changes.
    pub branch_transition: Option<Option<Transition>>,
    /// Restart from the drop model as soon as it differs from the start model.
    pub chain: Option<Chaining>,
}

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum DragError {
    InvalidPointer,
    MissingElement,
    InvalidTransform,
    EmptyBetween,
    CoincidentTargets { first: usize, second: usize },
    GeometryOutOfRange,
    InvalidSpec(&'static str),
    OptimizationFailed,
}

impl fmt::Display for DragError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPointer => write!(f, "pointer coordinates must be finite"),
            Self::MissingElement => write!(f, "the drag needs an element missing from this state"),
            Self::InvalidTransform => write!(f, "invalid element transform or grab position"),
            Self::EmptyBetween => write!(f, "between needs at least one target"),
            Self::CoincidentTargets { first, second } => {
                write!(
                    f,
                    "between targets {first} and {second} have the same grab position"
                )
            }
            Self::GeometryOutOfRange => {
                write!(f, "geometry is outside the supported numeric range")
            }
            Self::InvalidSpec(message) => f.write_str(message),
            Self::OptimizationFailed => write!(f, "vary could not find a feasible model"),
        }
    }
}

impl Error for DragError {}

type Locate<'a, S> = dyn Fn(&S) -> Option<Affine> + 'a;
type PreviewGeometry<'a, S> = dyn Fn(&Preview<S>, Point) -> Option<Point> + 'a;

/// Geometry captured on press, available to custom operators.
pub struct DragContext<'a, S> {
    start: Rc<S>,
    press: Point,
    anchor: Point,
    locate: Box<Locate<'a, S>>,
    preview_geometry: Option<Box<PreviewGeometry<'a, S>>>,
}

impl<S> DragContext<'_, S> {
    pub fn start_state(&self) -> &Rc<S> {
        &self.start
    }

    pub fn press_position(&self) -> Point {
        self.press
    }

    /// The point grabbed on press, expressed in the element's local coordinates.
    pub fn anchor(&self) -> Point {
        self.anchor
    }

    /// Transform the original local anchor into the coordinate system of the pointer.
    pub fn grab_in(&self, state: &S) -> Result<Option<Point>, DragError> {
        let Some(transform) = (self.locate)(state) else {
            return Ok(None);
        };
        let position = transform * self.anchor;
        if !position.is_finite() {
            return Err(DragError::InvalidTransform);
        }
        Ok(Some(position))
    }

    /// Locate the original local anchor in a rendered preview. A host callback
    /// is authoritative, including `None` when the element is absent. Without a
    /// callback, model transforms are blended as point positions; hosts with
    /// other transform interpolation rules should provide preview geometry.
    pub fn grab_in_preview(&self, preview: &Preview<S>) -> Result<Option<Point>, DragError> {
        if let Some(geometry) = &self.preview_geometry {
            let point = geometry(preview, self.anchor);
            if point.is_some_and(|p| !p.is_finite()) {
                return Err(DragError::InvalidTransform);
            }
            return Ok(point);
        }
        match preview {
            Preview::State(state) => self.grab_in(state),
            Preview::Blend(parts) => {
                let mut position = Vec2::ZERO;
                for (part, weight) in parts {
                    if *weight == 0. {
                        continue;
                    }
                    let Some(point) = self.grab_in_preview(part)? else {
                        return Ok(None);
                    };
                    position += point.to_vec2() * *weight;
                }
                let point = position.to_point();
                if !point.is_finite() {
                    return Err(DragError::InvalidTransform);
                }
                Ok((!parts.is_empty()).then_some(point))
            }
            Preview::Floating {
                source,
                translation,
                ..
            } => {
                let point = self
                    .grab_in_preview(source)?
                    .map(|point| point + *translation);
                if point.is_some_and(|p| !p.is_finite()) {
                    return Err(DragError::InvalidTransform);
                }
                Ok(point)
            }
        }
    }

    /// Use the operator's cached grab position, rendering its preview only when
    /// the cache is absent. This is the source closest/snap lookup policy.
    pub fn grab_in_frame(&self, frame: &DragFrame<S>) -> Result<Option<Point>, DragError> {
        match frame.grab_position {
            Some(point) if point.is_finite() => Ok(Some(point)),
            Some(_) => Err(DragError::InvalidTransform),
            None => self.grab_in_preview(&frame.preview),
        }
    }

    /// Preview and drop the same model. A missing element has infinite gap.
    pub fn state_frame(&self, state: Rc<S>, pointer: Point) -> Result<DragFrame<S>, DragError> {
        let position = self.grab_in(&state)?;
        Ok(state_frame(state, position, pointer))
    }
}

fn state_frame<S>(state: Rc<S>, position: Option<Point>, pointer: Point) -> DragFrame<S> {
    DragFrame {
        preview: Rc::new(Preview::State(Rc::clone(&state))),
        drop_state: state,
        gap: position.map_or(f64::INFINITY, |p| p.distance(pointer)),
        grab_position: position,
        active_path: String::new(),
        drop_transition: None,
        branch_transition: None,
        chain: None,
    }
}

type Behavior<'a, S> =
    dyn FnMut(&DragContext<'_, S>, Point) -> Result<DragFrame<S>, DragError> + 'a;

/// A composable behavior for one drag. Build a fresh spec on every press so
/// hysteresis, cached geometry and continuous parameters are local to the gesture.
pub struct DragSpec<'a, S> {
    behavior: Box<Behavior<'a, S>>,
    name: &'static str,
}

impl<'a, S> DragSpec<'a, S> {
    /// Supply a custom behavior. The result can participate in any other operator.
    pub fn custom(
        behavior: impl FnMut(&DragContext<'_, S>, Point) -> Result<DragFrame<S>, DragError> + 'a,
    ) -> Self {
        Self {
            behavior: Box::new(behavior),
            name: "custom",
        }
    }

    pub(crate) fn operator(
        name: &'static str,
        behavior: impl FnMut(&DragContext<'_, S>, Point) -> Result<DragFrame<S>, DragError> + 'a,
    ) -> Self {
        Self {
            behavior: Box::new(behavior),
            name,
        }
    }

    /// Evaluate a child with the active drag context when writing a custom
    /// operator. A spec holds gesture-local memory, so use it in one session.
    pub fn evaluate(
        &mut self,
        context: &DragContext<'_, S>,
        pointer: Point,
    ) -> Result<DragFrame<S>, DragError> {
        let mut result = (self.behavior)(context, pointer)?;
        result.active_path = if result.active_path.is_empty() {
            self.name.to_owned()
        } else {
            format!("{}/{}", self.name, result.active_path)
        };
        Ok(result)
    }
}

impl<'a, S: 'a> From<S> for DragSpec<'a, S> {
    fn from(state: S) -> Self {
        fixed(state)
    }
}

/// One pointer's active drag. The host owns pointer capture, hit testing and
/// application state. Dropping this value discards the gesture.
pub struct DragSession<'a, S> {
    context: DragContext<'a, S>,
    spec: DragSpec<'a, S>,
}

impl<'a, S> DragSession<'a, S> {
    /// `locate` returns the local-to-canvas transform of the same element in
    /// each state. Capture its stable ID in the closure for lists or trees.
    pub fn begin(
        start: S,
        pointer: impl Into<Point>,
        locate: impl Fn(&S) -> Option<Affine> + 'a,
        spec: DragSpec<'a, S>,
    ) -> Result<Self, DragError> {
        let press = pointer.into();
        if !press.is_finite() {
            return Err(DragError::InvalidPointer);
        }
        let transform = locate(&start).ok_or(DragError::MissingElement)?;
        let det = transform.determinant();
        if det == 0.0 || !det.is_finite() {
            return Err(DragError::InvalidTransform);
        }
        let anchor = transform.inverse() * press;
        if !anchor.is_finite() {
            return Err(DragError::InvalidTransform);
        }
        Ok(Self {
            context: DragContext {
                start: Rc::new(start),
                press,
                anchor,
                locate: Box::new(locate),
                preview_geometry: None,
            },
            spec,
        })
    }

    /// Supply the dragged element's actual rendered position for any preview.
    /// The callback receives the original local anchor and returns its canvas
    /// position, or `None` if that preview does not contain the element. Capture
    /// the same stable element ID used by `locate`. This lets nested operators
    /// respect a host's transform interpolation without coupling to its renderer.
    pub fn with_preview_geometry(
        mut self,
        geometry: impl Fn(&Preview<S>, Point) -> Option<Point> + 'a,
    ) -> Self {
        self.context.preview_geometry = Some(Box::new(geometry));
        self
    }

    pub fn frame(&mut self, pointer: impl Into<Point>) -> Result<DragFrame<S>, DragError> {
        let pointer = pointer.into();
        if !pointer.is_finite() {
            return Err(DragError::InvalidPointer);
        }
        self.spec.evaluate(&self.context, pointer)
    }

    /// Evaluate the release position, including when no move event preceded it.
    pub fn finish(mut self, pointer: impl Into<Point>) -> Result<S, DragError>
    where
        S: Clone,
    {
        let state = self.frame(pointer)?.drop_state;
        drop(self);
        Ok(Rc::unwrap_or_clone(state))
    }

    /// Discard the preview and recover the model from pointer down.
    pub fn cancel(self) -> S
    where
        S: Clone,
    {
        Rc::unwrap_or_clone(self.context.start)
    }
}
