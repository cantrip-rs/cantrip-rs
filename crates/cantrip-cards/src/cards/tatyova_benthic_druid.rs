use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "You gain 1 life and draw a card" (CR 119.3, 121.1): the life first, then the card, as the sentence reads.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::LANDFALL,
        effects::gain_life_then_draw::<1, 1>,
        "Landfall \u{2014} Whenever a land you control enters, you gain 1 life and draw a card.",
    )],
    ..oracle::TATYOVA_BENTHIC_DRUID
};
