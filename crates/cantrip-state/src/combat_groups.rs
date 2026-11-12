//! Count restrictions on a complete declaration, shared by attacking and blocking.
use crate::{ObjectId, count_u32};

/// No more than `at_most` distinct `members` may participate (CR 508.1c, 509.1b): Caverns of Despair's "No more than two
/// creatures can attack each combat", every creature that could counted. Membership is fixed while declaring, before
/// paying costs. A creature blocking several attackers is still one member.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CombatGroup {
    pub members: Vec<ObjectId>,
    pub at_most: u32,
}

impl CombatGroup {
    pub fn count(&self, participates: impl Fn(ObjectId) -> bool) -> u32 {
        count_u32(self.members.iter().filter(|&&id| participates(id)).count())
    }
    pub fn allows(&self, participates: impl Fn(ObjectId) -> bool) -> bool {
        self.count(participates) <= self.at_most
    }
}
