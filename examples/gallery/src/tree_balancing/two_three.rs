//! Split-propagating 2–3 insertion with stable node IDs and split origins.
use crate::Point;
use std::collections::BTreeMap;
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub id: usize,
    pub keys: Vec<i32>,
    pub children: Vec<Node>,
    pub emerge: Option<usize>,
}
enum Insert {
    Node(Node),
    Split(Node, i32, Node),
}
fn fresh(next: &mut usize) -> usize {
    let id = *next;
    *next += 1;
    id
}
fn split(mut n: Node, next: &mut usize) -> Insert {
    let right = Node {
        id: fresh(next),
        keys: vec![n.keys[2]],
        children: if n.children.is_empty() {
            vec![]
        } else {
            n.children.split_off(2)
        },
        emerge: Some(n.id),
    };
    let mid = n.keys[1];
    n.keys.truncate(1);
    n.emerge = None;
    Insert::Split(n, mid, right)
}
fn insert_into(mut n: Node, key: i32, next: &mut usize) -> Insert {
    if n.keys.contains(&key) {
        return Insert::Node(n);
    }
    let slot = n.keys.iter().position(|k| *k > key).unwrap_or(n.keys.len());
    if n.children.is_empty() {
        n.keys.insert(slot, key)
    } else {
        match insert_into(n.children.remove(slot), key, next) {
            Insert::Node(child) => n.children.insert(slot, child),
            Insert::Split(left, mid, right) => {
                n.keys.insert(slot, mid);
                n.children.insert(slot, right);
                n.children.insert(slot, left);
            }
        }
    }
    if n.keys.len() <= 2 {
        Insert::Node(n)
    } else {
        split(n, next)
    }
}
pub fn insert(root: Option<Node>, key: i32, next: &mut usize) -> Node {
    let Some(root) = root else {
        return Node {
            id: fresh(next),
            keys: vec![key],
            children: vec![],
            emerge: None,
        };
    };
    let origin = root.id;
    match insert_into(root, key, next) {
        Insert::Node(n) => n,
        Insert::Split(left, mid, right) => Node {
            id: fresh(next),
            keys: vec![mid],
            children: vec![left, right],
            emerge: Some(origin),
        },
    }
}
pub fn width(n: &Node) -> f64 {
    let own = n.keys.len() as f64 * 30.;
    if n.children.is_empty() {
        own
    } else {
        own.max(n.children.iter().map(width).sum::<f64>() + (n.children.len() - 1) as f64 * 14.)
    }
}
pub fn layout(n: &Node, center: Point, out: &mut BTreeMap<usize, Point>) {
    out.insert(n.id, center);
    let mut x = center.x - width(n) / 2.;
    for c in &n.children {
        let w = width(c);
        layout(c, Point::new(x + w / 2., center.y + 64.), out);
        x += w + 14.;
    }
}
pub fn nodes<'a>(n: &'a Node, out: &mut Vec<&'a Node>) {
    out.push(n);
    for c in &n.children {
        nodes(c, out);
    }
}
