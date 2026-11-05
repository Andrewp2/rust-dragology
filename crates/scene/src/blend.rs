use super::{Item, Quad, Rect};
use std::{collections::HashMap, hash::Hash};

#[derive(Default)]
struct Weighted {
    geometry: [f64; 5],
    rgb: [f64; 3],
    alpha: f64,
    weight: f64,
}

impl Weighted {
    fn add(&mut self, q: Quad, w: f64) {
        let b = q.bounds;
        for (sum, n) in self
            .geometry
            .iter_mut()
            .zip([b.x0, b.y0, b.x1, b.y1, q.corner_radius()])
        {
            *sum += n * w;
        }
        let alpha = f64::from(q.color[3]) * w;
        for (sum, color) in self.rgb.iter_mut().zip(q.color) {
            *sum += f64::from(color) * alpha;
        }
        self.alpha += alpha;
        self.weight += w;
    }
    fn finish(self) -> Quad {
        let [x0, y0, x1, y1, radius] = self.geometry.map(|sum| sum / self.weight);
        let [r, g, b] = self.rgb.map(|sum| {
            if self.alpha > 0. {
                (sum / self.alpha).round() as u8
            } else {
                0
            }
        });
        Quad {
            bounds: Rect::new(x0, y0, x1, y1),
            radius,
            color: [r, g, b, self.alpha.round() as u8],
        }
    }
}

// Missing IDs fade in place. Paint order follows first appearance across the children.
pub(super) fn scenes<K: Eq + Hash + Clone>(
    parts: impl Iterator<Item = (Vec<Item<K>>, f64)>,
) -> Vec<Item<K>> {
    let mut indices = HashMap::new();
    let mut output = Vec::<(K, Weighted)>::new();
    for (items, w) in parts {
        for item in items {
            let i = *indices.entry(item.id.clone()).or_insert_with(|| {
                output.push((item.id, Weighted::default()));
                output.len() - 1
            });
            output[i].1.add(item.quad, w);
        }
    }
    output
        .into_iter()
        .map(|(id, sums)| Item {
            id,
            quad: sums.finish(),
        })
        .collect()
}
