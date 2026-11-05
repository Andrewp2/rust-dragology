use crate::{Item, blend};
use std::{hash::Hash, time::Duration};

/// A bounded ease-out, retargeted from the currently displayed geometry.
pub(super) struct Transition<K> {
    from: Vec<Item<K>>,
    pub target: Vec<Item<K>>,
    pub elapsed: Duration,
}

const DURATION: Duration = Duration::from_millis(240);

impl<K: Eq + Hash + Clone> Transition<K> {
    pub fn new(from: Vec<Item<K>>, target: Vec<Item<K>>) -> Self {
        Self {
            from,
            target,
            elapsed: Duration::ZERO,
        }
    }

    pub fn finished(&self) -> bool {
        self.elapsed >= DURATION
    }

    pub fn frame(&self) -> Vec<Item<K>> {
        if self.finished() {
            return self.target.clone();
        }
        let progress = self.elapsed.as_secs_f64() / DURATION.as_secs_f64();
        let weight = 1. - (1. - progress).powi(3);
        blend::scenes(
            [
                (self.target.clone(), weight),
                (self.from.clone(), 1. - weight),
            ]
            .into_iter()
            .filter(|(_, weight)| *weight > 0.),
        )
    }
}
