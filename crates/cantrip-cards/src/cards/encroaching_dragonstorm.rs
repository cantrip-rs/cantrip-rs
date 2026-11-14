use cantrip_api::*;
use cantrip_mechanics::triggers::{DRAGON_YOU_CONTROL_ENTERS, THIS_ENTERS};
use cantrip_mechanics::{effects, library};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, library::search_for_two_basic_lands_onto_the_battlefield_tapped, "When this enchantment enters, search your library for up to two basic land cards, put them onto the battlefield tapped, then shuffle."),
        triggered(DRAGON_YOU_CONTROL_ENTERS, effects::return_this_to_hand, "When a Dragon you control enters, return this enchantment to its owner's hand."),
    ],
    ..oracle::ENCROACHING_DRAGONSTORM
};
