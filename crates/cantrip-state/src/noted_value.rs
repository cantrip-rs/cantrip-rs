//! Immutable values explicitly noted by linked abilities (CR 607.2e).
use crate::{CounterKind, Linked, ManaPool};

/// Full-width data captured by an instruction, not a summary tally of prior events.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum NotedValue {
    Mana(ManaPool),
    Counters(Vec<(CounterKind, i32)>),
}

/// The precise object incarnation, text and acquisition that recorded a value.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct NotedValueRecord {
    pub by: Linked,
    pub value: NotedValue,
}
