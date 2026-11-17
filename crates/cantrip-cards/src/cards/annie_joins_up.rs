use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "If a triggered ability of a legendary creature you control triggers, that ability triggers an additional time."
/// (CR 603.2d): the source a legendary creature under this enchantment's controller as it is when the ability triggers (as it
/// last existed for one that looks back). A replacement effect or an "as ~ enters" ability is no triggered ability and is
/// not repeated (the rulings).
fn legendary_creature_you_control(q: &dyn Query, asker: &Asker, _: &Event, triggering: &Triggering) -> bool {
    triggering.of_a_permanent_controlled_by(asker.controller) && {
        let c = q.chars(triggering.source);
        c.is(CardType::Creature) && c.supertypes.legendary
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        effects::damage_first_target::<5>,
        "When Annie Joins Up enters, it deals 5 damage to target creature or planeswalker an opponent controls.",
    )
    .targets(&[targets::CREATURE_OR_PLANESWALKER_AN_OPPONENT_CONTROLS])],
    additional_triggers: &[legendary_creature_you_control],
    ..oracle::ANNIE_JOINS_UP
};
