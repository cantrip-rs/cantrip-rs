//! Count restrictions on a complete declaration, shared by attacking and blocking.
use crate::{ObjectId, count_u32};

/// If any of `subjects` participates, between `at_least` and `at_most` distinct members
/// must participate too. Membership is fixed while declaring, before paying costs.
/// Caverns of Despair caps all participants; Errantry and Orcish Conscripts
/// constrain a particular creature's companions. A creature blocking several
/// attackers is still one member.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CombatGroup {
    pub subjects: Vec<ObjectId>,
    pub members: Vec<ObjectId>,
    pub at_least: u32,
    pub at_most: u32,
}

impl CombatGroup {
    pub fn count(&self, participates: impl Fn(ObjectId) -> bool) -> u32 {
        count_u32(self.members.iter().filter(|&&id| participates(id)).count())
    }
    pub fn allows(&self, participates: impl Fn(ObjectId) -> bool) -> bool {
        !self.subjects.iter().any(|&id| participates(id)) || (self.at_least..=self.at_most).contains(&self.count(participates))
    }
}
