//! Tree and symbolic-algebra examples ported from the Dragology contributors.
//! Source: declarative-dragging/dragology, MIT, revision b145bdd.
use crate::*;
mod algebra;
mod lambda;
mod panels;
mod proof;
mod structural;
mod tree;

pub fn entries() -> Vec<Entry> {
    vec![
        Entry::new(
            "twisted-trees",
            "Twisted trees",
            "Drag nodes between positions in an order-preserving tree mapping.",
            || demo(structural::Twisted),
        ),
        Entry::new(
            "two-kinds-of-tree",
            "Two kinds of tree",
            "Nest boxes horizontally or attach branches below. Pull a subtree away to detach it.",
            || demo(structural::TwoKinds),
        ),
        Entry::new(
            "outline",
            "Outline",
            "Reparent and reorder entire subtrees in an outline.",
            || demo(structural::Outline),
        ),
        Entry::new(
            "animate-algebra-tree",
            "Algebra tree",
            "Drag operands and operators to apply the enabled algebraic rewrite rules.",
            || demo(algebra::Algebra::Tree),
        ),
        Entry::new(
            "animate-algebra-stage-builder",
            "Algebra stage builder",
            "Pull operators and atoms from the palette to construct expressions.",
            || demo(algebra::Algebra::Builder),
        ),
        Entry::new(
            "animate-algebra-tree-macro",
            "Algebra macro recorder",
            "Record a tree rearrangement, then use the derived rewrite by dragging.",
            || demo(algebra::Algebra::Macro),
        ),
        Entry::new(
            "proof-by-dragging",
            "Proof by dragging",
            "Link matching subformulas of opposite polarity to construct natural-deduction proofs.",
            || demo(proof::Proof),
        ),
        Entry::new(
            "lambda-beta",
            "Lambda calculus",
            "Substitute an argument into its binder, or abstract a subterm out of its enclosing expression.",
            || demo(lambda::Lambda),
        ),
    ]
}
