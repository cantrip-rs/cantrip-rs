use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "This creature can't be blocked by creatures with greater power" (CR 509.1b): of a possible block, the blocker's power
/// against the Pet's, both as they are as blockers are declared; a block that has been made stands if powers change
/// afterwards (the ruling).
fn not_blocked_by_greater_power(q: &dyn Query, source: ObjectId, blocker: ObjectId, attacker: ObjectId) -> bool {
    attacker != source || q.chars(blocker).power <= q.chars(source).power
}

pub static CARD: CardDef = CardDef {
    block_rules: &[not_blocked_by_greater_power],
    abilities: &[ability(
        cost("{1}{W}").and(TAP),
        effects::return_first_target_to_hand,
        "{1}{W}, {T}: Return another target creature you control to its owner's hand. Activate only during your turn.",
    )
    .targets(&[targets::OTHER_CREATURE_YOU_CONTROL])
    .timing(Timing::OwnTurn)],
    ..oracle::PREHISTORIC_PET
};
