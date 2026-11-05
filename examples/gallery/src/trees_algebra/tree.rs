use crate::*;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicU64, Ordering};
pub fn fresh_id(prefix: &str) -> String {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    // Keep allocated IDs separate from seed IDs such as `brush-1`, including
    // when a demo is opened first or reset while the counter keeps advancing.
    format!(
        "{prefix}-generated-{}",
        NEXT.fetch_add(1, Ordering::Relaxed)
    )
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tree {
    pub id: String,
    pub label: String,
    pub children: Vec<Tree>,
    pub emerge: Option<String>,
    pub emerge_mode: EmergeMode,
}
impl Tree {
    pub fn leaf(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            children: vec![],
            emerge: None,
            emerge_mode: EmergeMode::Auto,
        }
    }
    pub fn node(id: impl Into<String>, label: impl Into<String>, children: Vec<Tree>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            children,
            emerge: None,
            emerge_mode: EmergeMode::Auto,
        }
    }
    pub fn find(&self, id: &str) -> Option<&Tree> {
        if self.id == id {
            Some(self)
        } else {
            self.children.iter().find_map(|c| c.find(id))
        }
    }
    pub fn nodes(&self) -> Vec<&Tree> {
        let mut out = vec![self];
        for c in &self.children {
            out.extend(c.nodes());
        }
        out
    }
    pub fn replace(&self, id: &str, new: &Tree) -> Tree {
        if self.id == id {
            return new.clone();
        }
        Self {
            children: self.children.iter().map(|c| c.replace(id, new)).collect(),
            ..self.clone()
        }
    }
    pub fn remove(&self, id: &str) -> Option<(Tree, Tree)> {
        for (i, c) in self.children.iter().enumerate() {
            if c.id == id {
                let mut t = self.clone();
                return Some((
                    {
                        t.children.remove(i);
                        t
                    },
                    c.clone(),
                ));
            }
            if let Some((child, removed)) = c.remove(id) {
                let mut t = self.clone();
                t.children[i] = child;
                return Some((t, removed));
            }
        }
        None
    }
    pub fn structural_eq(&self, b: &Tree) -> bool {
        self.label == b.label
            && self.children.len() == b.children.len()
            && self
                .children
                .iter()
                .zip(&b.children)
                .all(|(a, b)| a.structural_eq(b))
    }
    pub fn fresh(&self, prefix: &str) -> Self {
        Self {
            id: format!("{prefix}/{}", self.id),
            label: self.label.clone(),
            children: self.children.iter().map(|c| c.fresh(prefix)).collect(),
            emerge: Some(self.id.clone()),
            emerge_mode: EmergeMode::Auto,
        }
    }
    fn clone_emerging(&self, prefix: &str) -> Self {
        Self {
            id: format!("{prefix}/{}", self.id),
            label: self.label.clone(),
            children: self
                .children
                .iter()
                .map(|c| c.clone_emerging(prefix))
                .collect(),
            emerge: Some(self.id.clone()),
            emerge_mode: EmergeMode::Clone,
        }
    }
    pub fn insertion_targets(&self, item: &Tree) -> Vec<Tree> {
        let mut out = vec![];
        for i in 0..=self.children.len() {
            let mut t = self.clone();
            t.children.insert(i, item.clone());
            out.push(t);
        }
        for (i, c) in self.children.iter().enumerate() {
            for child in c.insertion_targets(item) {
                let mut t = self.clone();
                t.children[i] = child;
                out.push(t);
            }
        }
        out
    }
}

pub fn raise_group(scene: &mut Scene, id: Option<&str>) {
    let Some(id) = id else {
        return;
    };
    let mut ids = BTreeSet::from([id.to_string()]);
    loop {
        let before = ids.len();
        for o in &scene.objects {
            if o.parent.as_ref().is_some_and(|p| ids.contains(p)) {
                ids.insert(o.id.clone());
            }
        }
        if ids.len() == before {
            break;
        }
    }
    let (raised, mut rest): (Vec<_>, Vec<_>) =
        scene.objects.drain(..).partition(|o| ids.contains(&o.id));
    rest.extend(raised);
    scene.objects = rest;
}

/// Nested rectangles keep parent hits behind child hits, as in upstream SVG groups.
pub fn nested_size(t: &Tree) -> Vec2 {
    let children: Vec<_> = t.children.iter().map(nested_size).collect();
    let h = if children.is_empty() {
        20.
    } else {
        children.iter().map(|s| s.y).sum::<f64>() + 10. * children.len().saturating_sub(1) as f64
    };
    Vec2::new(
        30. + if children.is_empty() {
            0.
        } else {
            10. + children.iter().map(|s| s.x).fold(0., f64::max)
        },
        h + 10.,
    )
}
pub fn nested(scene: &mut Scene, t: &Tree, at: Point, depth: usize) {
    let sz = nested_size(t);
    let alpha = (0.02 + depth as f64 * 0.03).min(0.15);
    let stroke_alpha = (0.25 + depth as f64 * 0.08).min(0.5);
    let mut o = Object::rect(
        &t.id,
        Rect::from_origin_size(at, (sz.x, sz.y)),
        14f64.min(0.3 * sz.x.min(sz.y)),
        [180, 170, 150, (alpha * 255.).round() as u8],
    )
    .label(&t.label, (15., sz.y / 2.), 18., [58, 58, 56, 255])
    .text_anchor(TextAnchor::Middle)
    .text_baseline(TextBaseline::Middle);
    if let Shape::Rect { stroke, .. } = &mut o.shapes[0] {
        *stroke = Some(([140, 130, 115, (stroke_alpha * 255.).round() as u8], 0.75));
    }
    o.emerge_from = t.emerge.clone();
    o.emerge_mode = t.emerge_mode;
    scene.push(o);
    let mut y = at.y + 5.;
    for c in &t.children {
        nested(scene, c, Point::new(at.x + 35., y), depth + 1);
        if let Some(o) = scene.objects.iter_mut().find(|o| o.id == c.id) {
            o.parent = Some(t.id.clone());
        }
        y += nested_size(c).y + 10.;
    }
}
pub fn fit_nested(t: &Tree) -> Scene {
    let mut scene = Scene::new(300., 350.);
    nested(&mut scene, t, Point::ORIGIN, 0);
    scene
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pattern {
    pub key: String,
    pub label: Option<String>,
    pub trigger: bool,
    pub children: Vec<Pattern>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rule {
    pub from: Pattern,
    pub to: Pattern,
}
pub fn pattern(src: &str) -> Pattern {
    let tokens: Vec<String> = src
        .replace('(', " ( ")
        .replace(')', " ) ")
        .replace('#', " # ")
        .split_whitespace()
        .map(str::to_owned)
        .collect();
    fn parse(ts: &[String], p: &mut usize) -> Pattern {
        let trigger = ts.get(*p).is_some_and(|s| s == "#");
        if trigger {
            *p += 1;
        }
        let token = ts[*p].clone();
        *p += 1;
        if token == "(" {
            let key = ts[*p].clone();
            *p += 1;
            let label = key
                .trim_end_matches(|c: char| c.is_ascii_digit())
                .to_string();
            let label = if label.is_empty() { key.clone() } else { label };
            let mut children = vec![];
            while ts[*p] != ")" {
                children.push(parse(ts, p));
            }
            *p += 1;
            Pattern {
                key,
                label: Some(label),
                trigger,
                children,
            }
        } else {
            Pattern {
                key: token,
                label: None,
                trigger,
                children: vec![],
            }
        }
    }
    parse(&tokens, &mut 0)
}
pub fn rule(from: &str, to: &str) -> Rule {
    Rule {
        from: pattern(from),
        to: pattern(to),
    }
}
fn matches(
    p: &Pattern,
    t: &Tree,
    trigger: &str,
    bindings: &mut BTreeMap<String, Tree>,
) -> Option<bool> {
    fn helper(p: &Pattern, t: &Tree, trigger: &str) -> Option<(bool, BTreeMap<String, Tree>)> {
        let mut fired = p.trigger && t.id == trigger;
        let mut bindings = BTreeMap::<String, Tree>::new();
        if let Some(label) = &p.label {
            if label != &t.label || p.children.len() != t.children.len() {
                return None;
            }
            for (pc, tc) in p.children.iter().zip(&t.children) {
                let (child_fired, child_bindings) = helper(pc, tc, trigger)?;
                fired |= child_fired;
                for (key, value) in child_bindings {
                    if key.starts_with("op:") {
                        bindings.entry(key).or_insert(value);
                    } else if let Some(old) = bindings.get(&key) {
                        if !old.structural_eq(&value) {
                            return None;
                        }
                        if value.id == trigger {
                            bindings.insert(key, value);
                        }
                    } else {
                        bindings.insert(key, value);
                    }
                }
            }
            // A parent operator's own binding wins over an identically named descendant.
            bindings.insert(format!("op:{}", p.key), t.clone());
        } else {
            bindings.insert(p.key.clone(), t.clone());
        }
        Some((fired, bindings))
    }
    let (fired, found) = helper(p, t, trigger)?;
    *bindings = found;
    Some(fired)
}
fn build(
    p: &Pattern,
    bindings: &BTreeMap<String, Tree>,
    used: &mut BTreeSet<String>,
    prefix: &str,
    counter: &mut usize,
) -> Tree {
    *counter += 1;
    let key = if p.label.is_some() {
        format!("op:{}", p.key)
    } else {
        p.key.clone()
    };
    let old = bindings.get(&key);
    if p.label.is_none() {
        if let Some(old) = old {
            return if used.insert(key) {
                old.clone()
            } else {
                old.clone_emerging(&format!("{prefix}-{}", counter))
            };
        }
        return Tree::leaf(format!("{prefix}-{}", counter), &p.key);
    }
    let id = if used.insert(key) {
        old.map(|t| t.id.clone())
            .unwrap_or_else(|| format!("{prefix}-{}", counter))
    } else {
        format!("{prefix}-{}", counter)
    };
    let mut result = Tree::node(
        id,
        p.label.as_ref().unwrap(),
        p.children
            .iter()
            .map(|c| build(c, bindings, used, prefix, counter))
            .collect(),
    );
    if old.is_none_or(|o| o.id != result.id) {
        result.emerge = old.map(|o| o.id.clone());
    }
    result
}
pub fn rewrites(t: &Tree, rules: &[Rule], trigger: &str) -> Vec<Tree> {
    let mut out = vec![];
    for (i, r) in rules.iter().enumerate() {
        let mut bindings = BTreeMap::new();
        if matches(&r.from, t, trigger, &mut bindings) == Some(true) {
            let prefix = fresh_id(&format!("{trigger}-r{i}"));
            let mut result = build(&r.to, &bindings, &mut BTreeSet::new(), &prefix, &mut 0);
            fn set_origins(t: &mut Tree, old: &Tree, origin: &str) {
                if old.find(&t.id).is_none() && t.emerge.is_none() {
                    t.emerge = Some(origin.into());
                }
                for c in &mut t.children {
                    set_origins(c, old, origin)
                }
            }
            set_origins(&mut result, t, trigger);
            out.push(result);
        }
    }
    for (i, c) in t.children.iter().enumerate() {
        for replacement in rewrites(c, rules, trigger) {
            let mut nt = t.clone();
            nt.children[i] = replacement;
            out.push(nt);
        }
    }
    out
}
pub fn derive(before: &Tree, after: &Tree) -> Option<Rule> {
    fn same_ids(a: &Tree, b: &Tree) -> bool {
        a.id == b.id
            && a.label == b.label
            && a.children.len() == b.children.len()
            && a.children
                .iter()
                .zip(&b.children)
                .all(|(a, b)| same_ids(a, b))
    }
    let stable: BTreeSet<_> = before
        .nodes()
        .into_iter()
        .filter(|n| after.find(&n.id).is_some_and(|a| same_ids(n, a)))
        .map(|n| n.id.clone())
        .collect();
    if stable.contains(&before.id) {
        return None;
    }
    fn convert(t: &Tree, stable: &BTreeSet<String>, trigger: bool) -> Pattern {
        Pattern {
            key: t.id.clone(),
            label: (!stable.contains(&t.id)).then(|| t.label.clone()),
            trigger,
            children: if stable.contains(&t.id) {
                vec![]
            } else {
                t.children
                    .iter()
                    .map(|c| convert(c, stable, trigger))
                    .collect()
            },
        }
    }
    Some(Rule {
        from: convert(before, &stable, true),
        to: convert(after, &stable, false),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn macro_stability_ignores_animation_annotations() {
        let before = Tree::node("sum", "+", vec![Tree::leaf("a", "x"), Tree::leaf("b", "y")]);
        let mut after = before.clone();
        after.children[0].emerge = Some("previous".into());
        assert!(derive(&before, &after).is_none());
        after.children.swap(0, 1);
        let rule = derive(&before, &after).unwrap();
        assert!(rule.from.children.iter().all(|p| p.label.is_none()));
    }
    #[test]
    fn distribution_clones_wildcard_subtrees_and_auto_morphs_new_operators() {
        let shared = Tree::node("a", "-", vec![Tree::leaf("a-child", "x")]);
        let t = Tree::node(
            "times",
            "×",
            vec![
                shared.clone(),
                Tree::node("sum", "+", vec![Tree::leaf("b", "y"), Tree::leaf("c", "z")]),
            ],
        );
        let output = rewrites(&t, &[rule("(× #A (+ B C))", "(+ (× A B) (× A C))")], "a").remove(0);
        let product = &output.children[1];
        assert_eq!(product.emerge_mode, EmergeMode::Auto);
        assert_eq!(product.emerge.as_deref(), Some("times"));
        let clone = &product.children[0];
        let scene = fit_nested(&output);
        for (copy, original) in clone.nodes().into_iter().zip(shared.nodes()) {
            assert_eq!(copy.emerge.as_deref(), Some(original.id.as_str()));
            assert_eq!(
                scene.object(&copy.id).unwrap().emerge_mode,
                EmergeMode::Clone
            );
        }
    }
}
