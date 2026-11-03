use cantrip_api::*;
use cantrip_oracle as oracle;

/// "This creature can't block creatures with power 2 or greater." (CR 604.1).
fn creature_cant_block_creatures_power_rule(
    q: &dyn Query,
    source: ObjectId,
    blocker: ObjectId,
    attacker: ObjectId,
) -> bool {
    let asker = &Asker::new(source, q.controller(source));
    !(blocker == asker.source
        && (q.chars(attacker).is(CardType::Creature)
            && q.chars(attacker).power.is_some_and(|v| (i64::from(v)) >= 2)))
}

pub static CARD: CardDef = CardDef {
    block_rules: &[creature_cant_block_creatures_power_rule],
    ..oracle::BRASSCLAW_ORCS
};
