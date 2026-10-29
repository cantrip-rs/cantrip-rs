use cantrip_api::*;
use cantrip_mechanics::tokens::{self, MUTAGEN};
use cantrip_mechanics::triggers::THIS_ENTERS_OR_ATTACKS;
use cantrip_oracle as oracle;

/// "Each creature you control with a counter on it can't be blocked by more than one creature" (CR 509.1b): a counter of any
/// kind on the attacker, the attacker's controller being Michelangelo's.
fn counters_limit_blockers_to_one(q: &dyn Query, source: ObjectId, attacker: ObjectId) -> Option<usize> {
    let has_counter = q.obj(attacker).has_counters();
    (q.controller(attacker) == q.controller(source) && has_counter).then_some(1)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS_OR_ATTACKS, tokens::create_mutagen, "Whenever Michelangelo enters or attacks, create a Mutagen token.")],
    blocker_limits: &[counters_limit_blockers_to_one],
    ..oracle::MICHELANGELO_MUTANT_BFF
};

/// The Mutagen token it makes.
pub static EXTRA: &[&CardDef] = &[&MUTAGEN];
