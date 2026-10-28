use cantrip_api::*;
use cantrip_mechanics::triggers::YOUR_FIRST_MAIN_PHASE;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Target creature on the battlefield or creature card in a graveyard" (CR 115.1): any creature permanent, or a creature card in
/// any player's graveyard.
fn creature_or_creature_card_in_a_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    (targets::CREATURE.filter)(q, asker, t) || targets::card_in_graveyard(q, t, &[CardType::Creature])
}

/// Photographic Reflexes: "At the beginning of your first main phase, until your next turn, Taskmaster becomes a copy of up to one
/// target creature on the battlefield or creature card in a graveyard, except his name is Taskmaster, Mercenary Mimic and he's a
/// legendary Human Mercenary Villain creature." The copiable values of the target (a card's printed ones, a permanent's with its
/// own copy exceptions, CR 707.2) with the exceptions stated (CR 707.9b): his card types and subtypes are those named instead of
/// the copied ones (no "in addition to"), and he is legendary. It lasts until the controller's next turn begins (CR 611.2b), so
/// counters and other effects on him stay, and the copy has no Photographic Reflexes unless the copied creature does. Nothing if
/// no target was chosen or it is no longer legal (CR 608.2b).
fn become_copy(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(target) = targets::object(g, item, 0) else { return Ok(()) };
    let words = g.words(item.object);
    let values = g
        .copiable_values(target)
        .named("Taskmaster, Mercenary Mimic")
        .typed(
            TypeSet::of(&[CardType::Creature]),
            SubtypeSet::of(&[words.subtype(Subtype::Human), words.subtype(Subtype::Mercenary), words.subtype(Subtype::Villain)]),
        )
        .legendary();
    g.become_copy(item.object, item.object, values, Duration::UntilNextTurnOf(item.controller));
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOUR_FIRST_MAIN_PHASE,
        become_copy,
        "Photographic Reflexes \u{2014} At the beginning of your first main phase, until your next turn, Taskmaster becomes a copy of up to one target creature on the battlefield or creature card in a graveyard, except his name is Taskmaster, Mercenary Mimic and he's a legendary Human Mercenary Villain creature.",
    )
    .targets(&[target(creature_or_creature_card_in_a_graveyard).optional()])],
    ..oracle::TASKMASTER_MERCENARY_MIMIC
};
