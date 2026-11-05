//! Native Rust ports of the upstream Dragology demo gallery.
mod api;
pub use api::*;
mod composite;
mod runtime;
pub mod text_layout;
pub use composite::{CompositeDemo, Panel};

mod basics;
mod continuous;
mod designer;
mod editors;
mod graspable_math;
mod knot;
mod lists;
mod puzzles;
mod topology;
mod tree_balancing;
mod trees_algebra;

pub fn entries() -> Vec<Entry> {
    let mut entries = basics::entries();
    entries.extend(lists::entries());
    entries.extend(puzzles::entries());
    entries.extend(continuous::entries());
    entries.push(graspable_math::entry());
    entries.extend(trees_algebra::entries());
    entries.extend(editors::entries());
    entries.push(knot::entry());
    entries.push(tree_balancing::entry());
    entries.extend(topology::entries());
    entries.push(designer::entry());
    let source_order: std::collections::HashMap<_, _> =
        include_str!("../data/upstream-demo-ids.txt")
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .enumerate()
            .map(|(index, id)| (id, index))
            .collect();
    entries.sort_by_key(|entry| source_order.get(entry.id).copied().unwrap_or(usize::MAX));
    entries
}
