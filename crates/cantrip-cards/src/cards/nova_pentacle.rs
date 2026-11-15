use cantrip_api::*;
use cantrip_mechanics::{prevention, replacements, targets};
use cantrip_oracle as oracle;

/// "That damage is dealt to target creature of an opponent's choice instead" (CR 614.9): the same damage, dealt to the
/// creature the effect names (`Query::redirected_to`). Its legality as a target is not checked again; where it has left the
/// battlefield or is no longer a creature or planeswalker, the core leaves the damage with you and the effect waiting (CR
/// 614.9, 609.7b).
fn dealt_to_the_creature_instead(q: &dyn Query, _: &Asker, _: ObjectId, proposal: &mut Proposal) {
    if let Proposal::Damage(d) = proposal {
        d.target = q.redirected_to();
    }
}

/// An opponent chose the creature as the ability was activated (CR 602.3); the source is chosen as it resolves (CR 609.7a).
/// "The next time a source of your choice would deal damage to you this turn": the effect watches for the next damage from
/// that source (`replacements::damage_from_chosen_source`) to the player its `to` names, the one who activated the ability
/// (CR 109.5), and its `into` is the creature.
fn pentacle(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let Some(source) = prevention::choose_source(g, item, prevention::SOURCE)? else { return Ok(()) };
    let kind = EffectKind::Replacement {
        card: item.ability.card,
        index: 0,
        to: Target::Player(item.controller),
        into: Target::Object(creature),
    };
    g.add_effect(item.object, source, kind, Duration::EndOfTurn);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{3}").and(TAP), pentacle,
        "{3}, {T}: The next time a source of your choice would deal damage to you this turn, that damage is dealt to target creature of an opponent's choice instead.")
        .targets(&[targets::CREATURE.labelled("Creature to be dealt instead the next damage to the activating player from a source they choose").of_an_opponents_choice()])],
    granted_replacements: &[granted_replacement(ReplacementKind::Damage, replacements::damage_from_chosen_source, dealt_to_the_creature_instead,
        "that damage is dealt to the creature instead")
        .once()
        .text("The next time a source of your choice would deal damage to you this turn, that damage is dealt to target creature of an opponent's choice instead.")],
    ..oracle::NOVA_PENTACLE
};
