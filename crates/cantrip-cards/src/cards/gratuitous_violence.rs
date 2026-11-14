use cantrip_api::*;
use cantrip_mechanics::replacements;
use cantrip_oracle as oracle;

/// "If a creature you control would deal damage to a permanent or player" (CR 614.1a, 120.3): damage, combat or not, from a
/// creature its controller controls, to anything, once it has been divided or assigned among its recipients (CR 510.1, the
/// rulings of Furnace of Rath). Damage already prevented to nothing is no damage (CR 615.6): doubling it leaves nothing. The
/// source is read as the creature it is when the damage would be dealt, so a spell or a noncreature permanent is not doubled.
fn damage_from_a_creature_you_control(q: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    matches!(
        proposal,
        Proposal::Damage(d) if q.controller(d.source) == asker.controller && q.chars(d.source).is(CardType::Creature)
    )
}

pub static CARD: CardDef = CardDef {
    replacements: &[replacement(
        ReplacementKind::Damage,
        damage_from_a_creature_you_control,
        replacements::double_that_damage,
        "deals double that damage instead",
    )],
    ..oracle::GRATUITOUS_VIOLENCE
};
