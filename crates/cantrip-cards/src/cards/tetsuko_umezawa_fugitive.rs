use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Creatures you control with power or toughness 1 or less can't be blocked" (CR 509.1b, 613.1g):
/// read final characteristics at block declaration; later changes never undo a legal block.
fn small_creatures_cant_be_blocked(q: &dyn Query, source: ObjectId, _: ObjectId, attacker: ObjectId) -> bool {
    let c = q.chars(attacker);
    q.controller(attacker) != q.controller(source)
        || !(c.power.is_some_and(|p| p <= 1) || c.toughness.is_some_and(|t| t <= 1))
}

pub static CARD: CardDef = CardDef {
    block_rules: &[small_creatures_cant_be_blocked],
    ..oracle::TETSUKO_UMEZAWA_FUGITIVE
};
