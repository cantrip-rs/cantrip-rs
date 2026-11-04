use cantrip_api::*;
use cantrip_mechanics::{replacements, targets};
use cantrip_oracle as oracle;

/// The targeted creature's damage to the ability's controller is redirected, even if that source has left (CR 113.7a, 614.9).
fn applies(_: &dyn Query, asker: &Asker, affected: ObjectId, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::Damage(d) if d.source == affected && d.target == Target::Player(asker.controller))
}

fn redirect(q: &dyn Query, asker: &Asker, _: ObjectId, proposal: &mut Proposal) {
    replacements::dealt_to_this_creature_instead(q, asker, proposal);
}

fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(attacker) = targets::object(g, item, 0) else { return Ok(()) };
    // Damage from the targeted creature's ability still has that source after it leaves (CR 113.7a, 608.2h).
    g.add_effect(item.object, attacker, EffectKind::ReplacementRule { card: item.ability.card, index: 0 }, Duration::EndOfTurn);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{B}").and(TAP),
        effect,
        "{B}, {T}: All damage that would be dealt to you this turn by target attacking creature is dealt to this creature instead.",
    ).targets(&[targets::ATTACKING_CREATURE])],
    granted_replacements: &[granted_replacement(ReplacementKind::Damage, applies, redirect, "the damage is dealt to this creature instead")
        .text("All damage that would be dealt to you this turn by target attacking creature is dealt to this creature instead.")],
    ..oracle::SHIMIAN_NIGHT_STALKER
};
