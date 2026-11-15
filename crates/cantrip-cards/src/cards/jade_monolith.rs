use cantrip_api::*;
use cantrip_mechanics::{prevention, replacements, targets};
use cantrip_oracle as oracle;

/// "The next time a source of your choice would deal damage to target creature this turn": damage from the source chosen
/// (CR 609.7a, `prevention::by_chosen_source`). The core asks the effect only about damage to the creature it names (its
/// `to`). What it does is "that source deals that damage to you instead" (CR 614.1a, 614.9): the same source deals the same
/// damage to the player who activated the ability (CR 109.5), the effect's controller.
fn from_chosen_source(q: &dyn Query, _: &Asker, chosen: ObjectId, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::Damage(d) if prevention::by_chosen_source(q, chosen, d))
}

/// The creature is targeted as the ability is activated; the source is chosen as it resolves (CR 609.7a), and the effect
/// watches for the next damage from it to that creature this turn.
fn monolith(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let Some(source) = prevention::choose_source(g, item, prevention::SOURCE)? else { return Ok(()) };
    let kind = EffectKind::Replacement { card: item.ability.card, index: 0, to: Target::Object(creature) };
    g.add_effect(item.object, source, kind, Duration::EndOfTurn);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{1}"), monolith,
        "{1}: The next time a source of your choice would deal damage to target creature this turn, that source deals that damage to you instead.")
        .targets(&[targets::CREATURE])],
    granted_replacements: &[granted_replacement(ReplacementKind::Damage, from_chosen_source, replacements::damage_dealt_to_you_instead,
        "that source deals that damage to you instead")
        .once()
        .text("The next time a source of your choice would deal damage to target creature this turn, that source deals that damage to you instead.")],
    ..oracle::JADE_MONOLITH
};
