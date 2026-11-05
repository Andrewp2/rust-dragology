//! Immutable algorithms translated from Dragology's rotations-rebalancing/bst.ts.
use crate::Point;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub key: i32,
    pub left: Link,
    pub right: Link,
    pub red: bool,
}
pub type Link = Option<Box<Node>>;
fn leaf(key: i32, red: bool) -> Box<Node> {
    Box::new(Node {
        key,
        left: None,
        right: None,
        red,
    })
}
pub fn height(n: &Link) -> i32 {
    n.as_ref()
        .map_or(0, |n| 1 + height(&n.left).max(height(&n.right)))
}
pub fn balance(n: &Node) -> i32 {
    height(&n.right) - height(&n.left)
}
pub fn inorder<'a>(n: &'a Link, out: &mut Vec<&'a Node>) {
    if let Some(n) = n {
        inorder(&n.left, out);
        out.push(n);
        inorder(&n.right, out);
    }
}
pub fn edges(n: &Link, out: &mut Vec<(i32, i32)>) {
    if let Some(n) = n {
        for c in [&n.left, &n.right].into_iter().flatten() {
            out.push((n.key, c.key));
        }
        edges(&n.left, out);
        edges(&n.right, out);
    }
}
pub fn positions(n: &Link, keys: &[i32], y0: f64) -> BTreeMap<i32, Point> {
    fn walk(n: &Link, keys: &[i32], depth: usize, y0: f64, out: &mut BTreeMap<i32, Point>) {
        if let Some(n) = n {
            out.insert(
                n.key,
                Point::new(
                    40. + keys.iter().position(|k| *k == n.key).unwrap() as f64 * 44.,
                    y0 + depth as f64 * 52.,
                ),
            );
            walk(&n.left, keys, depth + 1, y0, out);
            walk(&n.right, keys, depth + 1, y0, out);
        }
    }
    let mut out = BTreeMap::new();
    walk(n, keys, 0, y0, &mut out);
    out
}
fn rotate_right(mut n: Box<Node>) -> Box<Node> {
    let mut x = n.left.take().expect("left rotation child");
    n.left = x.right.take();
    x.right = Some(n);
    x
}
fn rotate_left(mut n: Box<Node>) -> Box<Node> {
    let mut x = n.right.take().expect("right rotation child");
    n.right = x.left.take();
    x.left = Some(n);
    x
}
fn rotate_up(mut n: Box<Node>, key: i32) -> Box<Node> {
    if n.left.as_ref().is_some_and(|c| c.key == key) {
        return rotate_right(n);
    }
    if n.right.as_ref().is_some_and(|c| c.key == key) {
        return rotate_left(n);
    }
    if key < n.key {
        n.left = n.left.map(|c| rotate_up(c, key));
    } else {
        n.right = n.right.map(|c| rotate_up(c, key));
    }
    n
}
pub fn plain(root: Link, key: i32) -> Link {
    Some(if let Some(mut n) = root {
        if key < n.key {
            n.left = plain(n.left, key)
        } else if key > n.key {
            n.right = plain(n.right, key)
        }
        n
    } else {
        leaf(key, false)
    })
}
pub fn splay(root: &Link, key: i32) -> Option<(Link, &'static str)> {
    let mut path = Vec::new();
    let mut cursor = root.as_ref();
    while let Some(n) = cursor {
        path.push(n.key);
        if key == n.key {
            break;
        }
        cursor = if key < n.key {
            n.left.as_ref()
        } else {
            n.right.as_ref()
        };
    }
    if path.last() != Some(&key) || path.len() < 2 {
        return None;
    }
    let n = root.clone()?;
    if path.len() == 2 {
        return Some((Some(rotate_up(n, key)), "zig"));
    }
    let p = path[path.len() - 2];
    let g = path[path.len() - 3];
    let same = (key < p) == (p < g);
    Some((
        Some(rotate_up(rotate_up(n, if same { p } else { key }), key)),
        if same { "zig-zig" } else { "zig-zag" },
    ))
}
pub fn avl(root: Link, key: i32) -> Link {
    let Some(mut n) = root else {
        return Some(leaf(key, false));
    };
    if key < n.key {
        n.left = avl(n.left, key)
    } else if key > n.key {
        n.right = avl(n.right, key)
    } else {
        return Some(n);
    }
    if balance(&n) < -1 {
        if balance(n.left.as_ref().unwrap()) > 0 {
            n.left = n.left.map(rotate_left);
        }
        return Some(rotate_right(n));
    }
    if balance(&n) > 1 {
        if balance(n.right.as_ref().unwrap()) < 0 {
            n.right = n.right.map(rotate_right);
        }
        return Some(rotate_left(n));
    }
    Some(n)
}
fn red(n: &Link) -> bool {
    n.as_ref().is_some_and(|n| n.red)
}
fn rb_balance(mut n: Box<Node>) -> Box<Node> {
    if n.red {
        return n;
    }
    let ll = n.left.as_ref().is_some_and(|l| l.red && red(&l.left));
    let lr = n.left.as_ref().is_some_and(|l| l.red && red(&l.right));
    let rr = n.right.as_ref().is_some_and(|r| r.red && red(&r.right));
    let rl = n.right.as_ref().is_some_and(|r| r.red && red(&r.left));
    if ll {
        n = rotate_right(n)
    } else if lr {
        n.left = n.left.map(rotate_left);
        n = rotate_right(n)
    } else if rr {
        n = rotate_left(n)
    } else if rl {
        n.right = n.right.map(rotate_right);
        n = rotate_left(n)
    } else {
        return n;
    }
    n.red = true;
    n.left.as_mut().unwrap().red = false;
    n.right.as_mut().unwrap().red = false;
    n
}
pub fn red_black(root: Link, key: i32) -> Link {
    fn insert(root: Link, key: i32) -> Box<Node> {
        let Some(mut n) = root else {
            return leaf(key, true);
        };
        if key < n.key {
            n.left = Some(insert(n.left, key))
        } else if key > n.key {
            n.right = Some(insert(n.right, key))
        } else {
            return n;
        }
        rb_balance(n)
    }
    let mut n = insert(root, key);
    n.red = false;
    Some(n)
}
