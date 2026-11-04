use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Tap target artifact, creature, or land. Draw a card at the beginning of the next turn's upkeep." (CR 115.1).
fn tap_target_artifact_creature_land_target_0(
    q: &dyn Query,
    _: &Asker,
    t: Target,
) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && ((q
        .chars(id).is(CardType::Artifact) || q.chars(id).is(CardType::Creature) || q
        .chars(id).is(CardType::Land)))
    )
}

/// "Tap target artifact, creature, or land. Draw a card at the beginning of the next turn's upkeep." (CR 608.2, 701.26a).
fn tap_target_artifact_creature_land(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    effects::tap_targets(g, item)?;
    g.end_sentence();
    g.delayed_trigger(
        DelayedWhen::StepOfNextTurn {
            step: Step::Upkeep,
            of: None,
        },
        item.object,
        item.controller,
        AbilityRef::delayed(item.ability.card, 0),
        Target::None,
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(
        &[target(tap_target_artifact_creature_land_target_0)],
        tap_target_artifact_creature_land,
    ),
    delayed: &[delayed_trigger(|g, item| effects::draw(g, item, 1), "Draw a card at the beginning of the next turn's upkeep.")],
    ..oracle::ENERVATE
};
