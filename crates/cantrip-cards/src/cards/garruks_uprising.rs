use cantrip_api::*;
use cantrip_mechanics::statics::{have_trample, your_creatures};
use cantrip_mechanics::triggers::{CREATURE_WITH_POWER_FOUR_OR_MORE_YOU_CONTROL_ENTERS, THIS_ENTERS};
use cantrip_mechanics::{effects, ferocious};
use cantrip_oracle as oracle;

/// "When this enchantment enters, if you control a creature with power 4 or greater, draw a card." (CR 603.4, 121.1): an intervening
/// "if", asked as the ability would trigger and again as it resolves, of the creatures its controller controls. "Creatures you control
/// have trample" (CR 702.19, 613.1f) reaches only its controller's creatures, and "Whenever a creature you control with power 4 or
/// greater enters" (CR 603.6a) is a separate trigger that the Uprising's own entering does not set off.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS,
            |g, item| effects::draw(g, item, 1),
            "When this enchantment enters, if you control a creature with power 4 or greater, draw a card.",
        )
        .condition(ferocious::you_control_asked),
        triggered(
            CREATURE_WITH_POWER_FOUR_OR_MORE_YOU_CONTROL_ENTERS,
            |g, item| effects::draw(g, item, 1),
            "Whenever a creature you control with power 4 or greater enters, draw a card.",
        ),
    ],
    statics: &[static_ability(Layer::Ability, your_creatures, have_trample, "Creatures you control have trample.")],
    ..oracle::GARRUKS_UPRISING
};
