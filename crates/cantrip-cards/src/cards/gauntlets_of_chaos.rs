use cantrip_api::*;
use cantrip_oracle as oracle;

/// "{5}, Sacrifice this artifact: Exchange control of target artifact, creature, or land you control and target permanent an opponent controls that shares one of those types with it. If those permanents are exchanged this way, destroy all Auras attached to them." (CR 115.1).
fn exchange_control_target_artifact_creature_target_0(
    q: &dyn Query,
    asker: &Asker,
    t: Target,
) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && (((q
        .chars(id).is(CardType::Artifact) || q.chars(id).is(CardType::Creature) || q
        .chars(id).is(CardType::Land)) && asker.controller == q.controller(id)))
    )
}

/// "{5}, Sacrifice this artifact: Exchange control of target artifact, creature, or land you control and target permanent an opponent controls that shares one of those types with it. If those permanents are exchanged this way, destroy all Auras attached to them." (CR 115.1).
fn exchange_control_target_artifact_creature_target_1(
    q: &dyn Query,
    asker: &Asker,
    t: Target,
) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && ((q
        .state().opponents(asker.controller).collect:: < Vec < PlayerId > > ().contains(&
        q.controller(id)) && q.chosen_targets().get(0).copied().and_then(| t : Target | t
        .object()).is_some_and(| other : ObjectId | [CardType::Artifact,
        CardType::Creature, CardType::Land].into_iter().any(| t : CardType | q
        .chars(other).is(t) && q.chars(id).is(t)))))
    )
}

/// "{5}, Sacrifice this artifact: Exchange control of target artifact, creature, or land you control and target permanent an opponent controls that shares one of those types with it. If those permanents are exchanged this way, destroy all Auras attached to them." (CR 608.2, 701.12a, 701.8a).
fn exchange_control_target_artifact_creature(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let first = cantrip_mechanics::targets::object(g, item, 0);
    let second = cantrip_mechanics::targets::object(g, item, 1);
    let exchanged = match (first, second) {
        (Some(first), Some(second)) => g.exchange_control(item.object, first, second),
        _ => false,
    };
    g.end_sentence();
    if exchanged {
        let auras: Vec<ObjectId> = g.state().battlefield().iter().copied().filter(|&id| {
            g.chars(id).has_subtype(g.words(item.object).subtype(Subtype::Aura))
                && (g.obj(id).attached_to == first || g.obj(id).attached_to == second)
        }).collect();
        g.destroy_all(&auras);
        g.end_sentence();
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{5}").and(SACRIFICE_THIS),
                exchange_control_target_artifact_creature,
                "{5}, Sacrifice this artifact: Exchange control of target artifact, creature, or land you control and target permanent an opponent controls that shares one of those types with it. If those permanents are exchanged this way, destroy all Auras attached to them.",
            )
            .targets(
                &[
                    target(exchange_control_target_artifact_creature_target_0),
                    target(exchange_control_target_artifact_creature_target_1),
                ],
            ),
    ],
    ..oracle::GAUNTLETS_OF_CHAOS
};
