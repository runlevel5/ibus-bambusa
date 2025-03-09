//! The composition: an ordered list of [`Transformation`]s and the machinery
//! that builds it from keystrokes.

// The construction and editing algorithms land incrementally; drop once the
// engine consumes them.
#![allow(dead_code)]

use crate::rules::Rule;

/// Stable, position-independent identity for a [`Transformation`].
pub type TransId = u32;

/// One keystroke's effect on the composition: an appended character, or a
/// tone/mark applied to an earlier transformation identified by `target`.
#[derive(Clone, Debug)]
pub struct Transformation {
    pub id: TransId,
    pub rule: Rule,
    pub target: Option<TransId>,
    pub is_upper_case: bool,
}

/// Hands out unique [`TransId`]s for a single engine's lifetime.
#[derive(Debug, Default)]
pub struct IdGen {
    next: TransId,
}

impl IdGen {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn next_id(&mut self) -> TransId {
        let id = self.next;
        self.next += 1;
        id
    }
}
