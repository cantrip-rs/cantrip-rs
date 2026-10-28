use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "This creature can't be blocked by tokens" (CR 509.1b): of a possible block with the Hunter as the attacker, the blocker
/// must be a card, as it is as blockers are declared; a block that has been made stands. Nothing to say of any other attacker.
fn tokens_cant_block_it(q: &dyn Query, source: ObjectId, blocker: ObjectId, attacker: ObjectId) -> bool {
    source != attacker || !q.obj(blocker).token
}

/// "When this creature enters, put a +1/+1 counter on target creature": any creature, the Hunter itself or an opponent's among
/// them (CR 115.1).
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        effects::plus_one_counter_on_target,
        "When this creature enters, put a +1/+1 counter on target creature.",
    )
    .targets(&[targets::CREATURE])],
    block_rules: &[tokens_cant_block_it],
    ..oracle::DUSKWATCH_HUNTER
};
