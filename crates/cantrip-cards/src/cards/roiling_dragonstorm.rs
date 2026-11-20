use cantrip_api::*;
use cantrip_mechanics::triggers::{DRAGON_YOU_CONTROL_ENTERS, THIS_ENTERS};
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, effects::draw_then_discard::<2, 1>, "When this enchantment enters, draw two cards, then discard a card."),
        triggered(DRAGON_YOU_CONTROL_ENTERS, effects::return_this_to_hand, "When a Dragon you control enters, return this enchantment to its owner's hand."),
    ],
    ..oracle::ROILING_DRAGONSTORM
};
