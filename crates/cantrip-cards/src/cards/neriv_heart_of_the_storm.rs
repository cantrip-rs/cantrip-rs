use cantrip_api::*;
use cantrip_mechanics::replacements;
use cantrip_oracle as oracle;

/// "If a creature you control that entered this turn would deal damage" (CR 614.1a, 120.3): a creature that entered the
/// battlefield this turn (CR 400.7: the object that move made), combat or not, once the damage is divided among its recipients
/// (the rulings). A source that has left the battlefield, such as a creature sacrificed to pay for its own ability, is the
/// creature as it last existed there (CR 113.7a, 608.2h). Damage already prevented to nothing is no damage (CR 615.6). "...
/// it deals twice that much damage instead" is `replacements::double_that_damage`: still dealt by the creature, not by Neriv.
fn creature_you_control_that_entered_this_turn_would_deal_damage(q: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::Damage(d)
        if d.amount > 0
            && q.chars(d.source).is(CardType::Creature)
            && q.controller(d.source) == asker.controller
            && q.history().entered_this_turn(d.source))
}

pub static CARD: CardDef = CardDef {
    replacements: &[replacement(
        ReplacementKind::Damage,
        creature_you_control_that_entered_this_turn_would_deal_damage,
        replacements::double_that_damage,
        "it deals twice that much damage instead",
    )],
    ..oracle::NERIV_HEART_OF_THE_STORM
};
