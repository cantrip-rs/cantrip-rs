use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Artifact spells you cast cost {1} less to cast" (CR 601.2f): only its controller's spells, judged on the spell's own
/// characteristics as it is cast; a generic reduction takes only the generic part (CR 118.7c). Abilities are not spells.
fn artifact_spells_cost_one_less(q: &dyn Query, source: ObjectId, player: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    let Costed::Spell(spell) = costed else { return CostDelta::NONE };
    if player == q.controller(source) && q.chars(spell).is(CardType::Artifact) { CostDelta::less(1) } else { CostDelta::NONE }
}

/// "Target artifact you control" (CR 115.1).
fn artifact_you_control(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if targets::permanent(q, id) && q.controller(id) == asker.controller && q.chars(id).is(CardType::Artifact))
}

/// "A second target artifact you control": another artifact than the one the first slot chose (CR 115.3: one object is not the
/// target of two instances of the word "target").
fn second_artifact_you_control(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    artifact_you_control(q, asker, t) && !q.chosen_targets().contains(&t)
}

/// "Target artifact you control becomes a copy of a second target artifact you control until end of turn, except it isn't
/// legendary" (CR 707.2, 707.9b, 613.1a): the second artifact's copiable values as it is as this resolves, nonlegendary. Both
/// targets must still be legal (CR 608.2b); with either gone nothing happens.
fn become_a_copy(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let (Some(artifact), Some(copied)) = (targets::object(g, item, 0), targets::object(g, item, 1)) else { return Ok(()) };
    let values = g.copiable_values(copied).nonlegendary();
    g.become_copy(item.object, artifact, values, Duration::EndOfTurn);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    cost_changes: &[artifact_spells_cost_one_less],
    abilities: &[ability(
        cost("{1}, {T}"),
        become_a_copy,
        "{1}, {T}: Target artifact you control becomes a copy of a second target artifact you control until end of turn, except it isn't legendary. Activate only as a sorcery.",
    )
    .sorcery_speed()
    .targets(&[target(artifact_you_control), target(second_artifact_you_control)])],
    ..oracle::SHURI_WAKANDAN_INVENTOR
};
