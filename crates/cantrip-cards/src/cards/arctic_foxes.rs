use cantrip_api::*;
use cantrip_oracle as oracle;

/// "This creature can't be blocked by creatures with power 2 or greater as long as defending player controls a snow land." (CR 604.1).
fn creature_cant_be_blocked_by_rule(
    q: &dyn Query,
    source: ObjectId,
    blocker: ObjectId,
    attacker: ObjectId,
) -> bool {
    let asker = &Asker::new(source, q.controller(source));
    !((q
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| {
            q.controller(id)
                == Some(asker.source)
                    .and_then(|o: ObjectId| q.defending_player(o))
                    .unwrap_or(PlayerId(u8::MAX))
                && (q.chars(id).supertypes.snow && q.chars(id).is(CardType::Land))
        })
        .count() >= 1) && attacker == asker.source
        && (q.chars(blocker).is(CardType::Creature)
            && q.chars(blocker).power.is_some_and(|v| (i64::from(v)) >= 2)))
}

pub static CARD: CardDef = CardDef {
    block_rules: &[creature_cant_be_blocked_by_rule],
    ..oracle::ARCTIC_FOXES
};
