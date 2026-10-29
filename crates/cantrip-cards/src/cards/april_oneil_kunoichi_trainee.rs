use cantrip_api::*;
use cantrip_mechanics::{effects, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "April O'Neil can't be blocked by creatures with power 3 or greater" (CR 509.1b): of a possible block with April as the
/// attacker, the blocker's power must be less than 3, as blockers are declared. A block already made stands if powers change
/// afterwards, since this is only asked when blocks are declared.
fn power_three_or_greater_cant_block(q: &dyn Query, source: ObjectId, blocker: ObjectId, attacker: ObjectId) -> bool {
    source != attacker || q.chars(blocker).power.unwrap_or(0) < 3
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, effects::scry_two, "When April O'Neil enters, scry 2.")],
    block_rules: &[power_three_or_greater_cant_block],
    ..oracle::APRIL_ONEIL_KUNOICHI_TRAINEE
};
