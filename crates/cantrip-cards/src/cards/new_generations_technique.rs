use cantrip_api::*;
use cantrip_mechanics::{library, sneak};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[], library::search_for_two_basic_lands_onto_the_battlefield_tapped),
    ways: &[sneak::way(mana("{2}{G}"), "Sneak {2}{G}")],
    ..oracle::NEW_GENERATIONS_TECHNIQUE
};
