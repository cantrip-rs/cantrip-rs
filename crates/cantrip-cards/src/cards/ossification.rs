use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "When this Aura enters, exile target creature or planeswalker an opponent controls until this Aura leaves the battlefield." (CR 608.2).
fn exile_target_creature_planeswalker_opponent_enchant(
    q: &dyn Query,
    asker: &Asker,
    t: Target,
) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && ((q
        .chars(id).supertypes.basic && q.chars(id).is(CardType::Land) && asker.controller
        == q.controller(id)))
    )
}

/// "When this Aura enters, exile target creature or planeswalker an opponent controls until this Aura leaves the battlefield." (CR 115.1).
fn exile_target_creature_planeswalker_opponent_target_0(
    q: &dyn Query,
    asker: &Asker,
    t: Target,
) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && (((q
        .chars(id).is(CardType::Creature) || q.chars(id).is(CardType::Planeswalker)) && q
        .state().opponents(asker.controller).collect:: < Vec < PlayerId > > ().contains(&
        q.controller(id))))
    )
}

/// "When this Aura enters, exile target creature or planeswalker an opponent controls until this Aura leaves the battlefield." (CR 608.2, 406.3).
fn exile_target_creature_planeswalker_opponent(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let t_0 = vec![g.state().target(item, 0)];
    let objects_0 = t_0
        .get(0)
        .copied()
        .and_then(|t: Target| t.object())
        .into_iter()
        .collect::<Vec<_>>();
    let by_0 = g.linked(item);
    g.exile_with(&objects_0, by_0, true);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
                triggers::THIS_ENTERS,
                exile_target_creature_planeswalker_opponent,
                "When this Aura enters, exile target creature or planeswalker an opponent controls until this Aura leaves the battlefield.",
            )
            .targets(&[target(exile_target_creature_planeswalker_opponent_target_0)]),
    ],
    enchant: Some(target(exile_target_creature_planeswalker_opponent_enchant)),
    ..oracle::OSSIFICATION
};
