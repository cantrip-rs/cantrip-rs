use cantrip_api::*;
use cantrip_oracle as oracle;

/// "This creature can't attack if defending player controls an untapped creature with power 3 or greater. This creature can't block creatures with power 3 or greater." (CR 604.1).
fn creature_cant_attack_if_defending_rule(
    _: &dyn Query,
    asker: &Asker,
    id: ObjectId,
) -> bool {
    id == asker.source
}

/// "This creature can't attack if defending player controls an untapped creature with power 3 or greater. This creature can't block creatures with power 3 or greater." (CR 604.1).
fn creature_cant_attack_if_defending_rule_attack(
    q: &dyn Query,
    asker: &Asker,
    attacker: ObjectId,
    at: Target,
) -> bool {
    if !creature_cant_attack_if_defending_rule(q, asker, attacker) {
        return true;
    }
    let Some(defender) = defending_player_of(q.state(), at) else { return true };
    !(q
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| {
            q.controller(id) == Some(defender).unwrap_or(PlayerId(u8::MAX))
                && (!(q.state().obj(id).tapped) && q.chars(id).is(CardType::Creature)
                    && q.chars(id).power.is_some_and(|v| (i64::from(v)) >= 3))
        })
        .count() >= 1)
}

/// "This creature can't attack if defending player controls an untapped creature with power 3 or greater. This creature can't block creatures with power 3 or greater." (CR 604.1).
fn creature_cant_attack_if_defending_rule_2(
    q: &dyn Query,
    source: ObjectId,
    blocker: ObjectId,
    attacker: ObjectId,
) -> bool {
    let asker = &Asker::new(source, q.controller(source));
    !(blocker == asker.source
        && (q.chars(attacker).is(CardType::Creature)
            && q.chars(attacker).power.is_some_and(|v| (i64::from(v)) >= 3)))
}

pub static CARD: CardDef = CardDef {
    block_rules: &[creature_cant_attack_if_defending_rule_2],
    attack_rules: &[creature_cant_attack_if_defending_rule_attack],
    ..oracle::GOBLIN_MUTANT
};
