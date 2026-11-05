use crate::{DragError, DragFrame, DragSpec, Point, Preview, Vec2};
use spade::{DelaunayTriangulation, FloatTriangulation, Point2, Triangulation};
use std::rc::Rc;

#[derive(Clone, Copy, Debug, Default)]
pub enum Interpolation {
    #[default]
    Delaunay,
    NaturalNeighbor,
}

#[derive(Clone, Copy, Debug)]
pub struct BetweenOptions {
    pub interpolation: Interpolation,
    /// Positive exponent applied to blend weights. At 1 the preview follows
    /// the projection. Larger values pull it toward nearby targets.
    pub sharpness: f64,
}

impl Default for BetweenOptions {
    fn default() -> Self {
        Self {
            interpolation: Interpolation::Delaunay,
            sharpness: 1.0,
        }
    }
}

/// Interpolate rendered states across their grab points' convex hull. Dropping
/// selects the candidate closest to the pointer, with ties in declaration order.
pub fn between<'a, S: 'a, T: Into<DragSpec<'a, S>>>(
    targets: impl IntoIterator<Item = T>,
) -> DragSpec<'a, S> {
    between_with(targets, BetweenOptions::default())
}

pub fn between_with<'a, S: 'a, T: Into<DragSpec<'a, S>>>(
    targets: impl IntoIterator<Item = T>,
    options: BetweenOptions,
) -> DragSpec<'a, S> {
    let mut branches: Vec<_> = targets.into_iter().map(Into::into).collect();
    let mut hull: Option<Hull> = None;
    DragSpec::operator("between", move |ctx, pointer| {
        if !options.sharpness.is_finite() || options.sharpness <= 0.0 {
            return Err(DragError::InvalidSpec(
                "between sharpness must be positive and finite",
            ));
        }
        if branches.is_empty() {
            return Err(DragError::EmptyBetween);
        }
        let mut frames = Vec::with_capacity(branches.len());
        let mut positions = Vec::with_capacity(branches.len());
        for branch in &mut branches {
            let frame = branch.evaluate(ctx, pointer)?;
            // Source between always measures the rendered child, even when the
            // child cached a weighted endpoint estimate for its own callers.
            positions.push(
                ctx.grab_in_preview(&frame.preview)?
                    .ok_or(DragError::MissingElement)?,
            );
            frames.push(frame);
        }
        // Fixed targets share one triangulation for the whole gesture. Nested
        // behaviors (e.g. two varying tracks) may move their targets on each frame.
        if hull.as_ref().is_none_or(|h| h.positions != positions) {
            hull = Some(Hull::new(positions)?);
        }
        let hull = hull.as_ref().unwrap();
        let (mut weights, gap) = hull.weights(pointer, options.interpolation)?;
        weights.retain(|(_, w)| *w > 0.0);
        if options.sharpness != 1.0 {
            let max = weights.iter().map(|&(_, w)| w).fold(0.0, f64::max);
            let mut total = 0.0;
            for (_, w) in &mut weights {
                *w = (*w / max).powf(options.sharpness);
                total += *w;
            }
            for (_, w) in &mut weights {
                *w /= total;
            }
        }
        let mut grab = Vec2::ZERO;
        let mut blend = Vec::with_capacity(weights.len());
        for (i, w) in weights {
            grab += hull.positions[i].to_vec2() * w;
            blend.push((frames[i].preview.clone(), w));
        }
        let preview = if blend.len() == 1 {
            blend.pop().unwrap().0
        } else {
            Rc::new(Preview::Blend(blend))
        };
        let nearest = hull
            .positions
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                a.distance_squared(pointer)
                    .total_cmp(&b.distance_squared(pointer))
            })
            .unwrap()
            .0;
        Ok(DragFrame {
            preview,
            drop_state: frames[nearest].drop_state.clone(),
            gap,
            grab_position: Some(grab.to_point()),
            active_path: String::new(),
            drop_transition: None,
            branch_transition: None,
            chain: None,
        })
    })
}

struct Hull {
    positions: Vec<Point>,
    mesh: DelaunayTriangulation<Point2<f64>>,
}

type Weights = Vec<(usize, f64)>;

impl Hull {
    fn new(positions: Vec<Point>) -> Result<Self, DragError> {
        let mut mesh = DelaunayTriangulation::new();
        for (i, p) in positions.iter().enumerate() {
            let vertex = mesh
                .insert(Point2::new(p.x, p.y))
                .map_err(|_| DragError::GeometryOutOfRange)?;
            if vertex.index() != i {
                return Err(DragError::CoincidentTargets {
                    first: vertex.index(),
                    second: i,
                });
            }
        }
        Ok(Self { positions, mesh })
    }

    fn weights(&self, pointer: Point, method: Interpolation) -> Result<(Weights, f64), DragError> {
        if let [only] = self.positions.as_slice() {
            return Ok((vec![(0, 1.0)], pointer.distance(*only)));
        }
        let query = Point2::new(pointer.x, pointer.y);
        spade::validate_vertex(&query).map_err(|_| DragError::GeometryOutOfRange)?;
        if self.mesh.num_inner_faces() > 0 {
            let mut weights = Vec::new();
            match method {
                Interpolation::Delaunay => self.mesh.barycentric().get_weights(query, &mut weights),
                Interpolation::NaturalNeighbor => self
                    .mesh
                    .natural_neighbor()
                    .get_weights(query, &mut weights),
            }
            if !weights.is_empty() {
                return Ok((
                    weights.into_iter().map(|(v, w)| (v.index(), w)).collect(),
                    0.0,
                ));
            }
        }
        // Outside the hull (or a collinear set): project onto its closest edge.
        let (a, b, t, gap) = self
            .mesh
            .convex_hull()
            .map(|edge| {
                let a = edge.from().index();
                let b = edge.to().index();
                let start = self.positions[a];
                let delta = self.positions[b] - start;
                let t = ((pointer - start).dot(delta) / delta.hypot2()).clamp(0.0, 1.0);
                (a, b, t, pointer.distance(start + delta * t))
            })
            .min_by(|a, b| a.3.total_cmp(&b.3))
            .expect("two distinct sites have a hull edge");
        Ok((vec![(a, 1.0 - t), (b, t)], gap))
    }
}
