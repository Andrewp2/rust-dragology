use crate::cobyla::{Status, find_minimum};
use crate::{DragError, DragSpec};
use std::rc::Rc;

/// Vary numerical parameters to place the grabbed point as close as possible
/// to the pointer. The model builder receives parameters in their original units.
/// Each frame starts the local search at the previous solution.
pub fn vary<'a, S: 'a, const N: usize>(
    initial: [f64; N],
    model: impl Fn([f64; N]) -> S + 'a,
) -> DragSpec<'a, S> {
    vary_constrained(initial, model, |_| [])
}

/// Like [`vary`], with model-space inequalities. Return one value per constraint,
/// each satisfied at `<= 0`. For example, a slider
/// in `[0, 240]` returns `[-value, value - 240.0]`. To pin a quantity, constrain
/// its difference from the initial value in both directions.
///
/// Use finite initial parameters that locate the dragged element. The optimizer
/// can explore infeasible models, so callbacks must also accept trial parameters.
/// Like upstream, this returns the solver's final iterate after at most 200
/// evaluations; it does not enforce a separate feasibility threshold. Constraints
/// are numerical targets, and an impossible system can return a violating model.
pub fn vary_constrained<'a, S: 'a, const N: usize, const M: usize>(
    initial: [f64; N],
    model: impl Fn([f64; N]) -> S + 'a,
    constraints: impl Fn(&S) -> [f64; M] + 'a,
) -> DragSpec<'a, S> {
    let mut current = initial;
    let mut scales = None;
    DragSpec::operator("vary", move |ctx, pointer| {
        if N == 0 || !current.iter().all(|p| p.is_finite()) {
            return Err(DragError::InvalidSpec(
                "vary requires a nonempty array of finite initial parameters",
            ));
        }
        let base = ctx
            .grab_in(&model(current))?
            .ok_or(DragError::MissingElement)?;
        // One scaled unit should move the grab point by about one canvas unit,
        // even if one parameter is measured in radians and another in pixels.
        let scales = *scales.get_or_insert_with(|| {
            std::array::from_fn::<_, N, _>(|i| {
                let mut sample = current;
                sample[i] += 1e-3;
                ctx.grab_in(&model(sample))
                    .ok()
                    .flatten()
                    .map_or(1.0, |p| (p.distance(base) / 1e-3).max(0.01))
            })
        });
        let to_params = |scaled: &[f64]| std::array::from_fn(|i| scaled[i] / scales[i]);
        let mut scaled: [f64; N] = std::array::from_fn(|i| current[i] * scales[i]);
        // Match DistanceMinimizer's avoidance of a zero simplex vertex.
        if scaled.iter().all(|&value| value == 0.) {
            scaled[0] = 1e-4;
        }
        let radius = ctx
            .grab_in(&model(to_params(&scaled)))?
            .ok_or(DragError::MissingElement)?
            .distance(pointer)
            .max(1.0);
        if !radius.is_finite() || !scaled.iter().all(|p| p.is_finite()) {
            return Err(DragError::GeometryOutOfRange);
        }
        let outcome = find_minimum(
            |scaled, con| {
                let state = model(to_params(scaled));
                for (out, value) in con.iter_mut().zip(constraints(&state)) {
                    *out = -value;
                }
                ctx.grab_in(&state)
                    .ok()
                    .flatten()
                    .map_or(f64::INFINITY, |position| position.distance_squared(pointer))
            },
            M,
            &mut scaled,
            radius,
            1e-3,
            200,
        );
        debug_assert!(outcome.evaluations <= 200);
        // DistanceMinimizer adopts the final iterate for all three source
        // outcomes, including the evaluation budget and a rounding-error stop.
        match outcome.status {
            Status::Normal | Status::MaxEvaluations | Status::RoundingErrors => {}
        }
        let params = to_params(&scaled);
        if !params.iter().all(|value| value.is_finite()) {
            return Err(DragError::OptimizationFailed);
        }
        let frame = ctx.state_frame(Rc::new(model(params)), pointer)?;
        if frame.grab_position.is_none() {
            return Err(DragError::MissingElement);
        }
        current = params;
        Ok(frame)
    })
}
