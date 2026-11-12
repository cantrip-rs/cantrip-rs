//! Count restrictions on a complete declaration, shared by attacking and blocking.
use crate::{ObjectId, count_u32};

/// Where any of `subjects` participates, no more than `at_most` distinct `members` may (CR 508.1c, 509.1b): Caverns of
/// Despair's "No more than two creatures can attack each combat", every creature that could a subject and a member;
/// Errantry's "Enchanted creature can only attack alone" (CR 506.5), the enchanted creature the subject and every creature
/// that could attack a member, at most one. Membership is fixed while declaring, before paying costs. A creature blocking
/// several attackers is still one member.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CombatGroup {
    pub subjects: Vec<ObjectId>,
    pub members: Vec<ObjectId>,
    pub at_most: u32,
}

impl CombatGroup {
    pub fn count(&self, participates: impl Fn(ObjectId) -> bool) -> u32 {
        count_u32(self.members.iter().filter(|&&id| participates(id)).count())
    }
    /// Whether a subject participates: the restriction holds only then.
    pub fn binds(&self, participates: impl Fn(ObjectId) -> bool) -> bool {
        self.subjects.iter().any(|&id| participates(id))
    }
    /// Whether a declaration obeys it. A declaration that breaks it still does with more participants, so a search may
    /// cut there.
    pub fn allows(&self, participates: impl Fn(ObjectId) -> bool + Copy) -> bool {
        !self.binds(participates) || self.count(participates) <= self.at_most
    }
}
