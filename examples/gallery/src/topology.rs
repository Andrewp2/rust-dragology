//! Ports of the MIT-licensed Dragology topology and composition demos.
mod braids;
mod catalan;
mod strings;
mod targets;
use crate::*;
use std::rc::Rc;

pub fn entries() -> Vec<Entry> {
    let mut all = targets::entries();
    all.extend(braids::entries());
    all.extend(catalan::entries());
    all.extend(strings::entries());
    all
}

fn ring(id: impl Into<String>, p: Point, r: f64, color: Color, width: f64) -> Object {
    Object::new(
        id,
        Affine::translate(p.to_vec2()),
        None,
        vec![Shape::Circle {
            center: Point::ORIGIN,
            radius: r,
            fill: [0; 4],
            stroke: Some((color, width)),
        }],
    )
}
