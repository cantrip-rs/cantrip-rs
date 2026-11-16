//! Count restrictions on a complete declaration, shared by attacking and blocking.
use crate::{ObjectId, count_u32};

/// Where any of `subjects` participates, at least `at_least` and no more than `at_most` distinct `members` must (CR 508.1c,
/// 509.1b): Caverns of Despair's "No more than two creatures can attack each combat", every creature that could a subject
/// and a member; Errantry's "Enchanted creature can only attack alone" (CR 506.5), the enchanted creature the subject and
/// every creature that could attack a member, at most one; Orcish Conscripts' "can't attack unless at least two other
/// creatures attack", itself the subject and every creature that could attack a member, at least three with it. Membership
/// is fixed while declaring, before paying costs. A creature blocking several attackers is still one member.
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
    /// Whether a subject participates: the restriction holds only then.
    pub fn binds(&self, participates: impl Fn(ObjectId) -> bool) -> bool {
        self.subjects.iter().any(|&id| participates(id))
    }
    /// Whether a complete declaration obeys it.
    pub fn allows(&self, participates: impl Fn(ObjectId) -> bool + Copy) -> bool {
        !self.binds(participates) || (self.at_least..=self.at_most).contains(&self.count(participates))
    }
    /// Whether a declaration has more members than it allows where it binds: it still does with more participants, so a
    /// search may cut there. Too few is not: more participants may make up the minimum.
    pub fn exceeded(&self, participates: impl Fn(ObjectId) -> bool + Copy) -> bool {
        self.binds(participates) && self.count(participates) > self.at_most
    }
}
