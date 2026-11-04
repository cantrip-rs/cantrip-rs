use cantrip_api::*;
use cantrip_mechanics::library::look_for_an_artifact;
use cantrip_mechanics::triggers::THIS_ATTACKS;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ATTACKS,
        look_for_an_artifact::<5>,
        "Whenever Ashe attacks, look at the top five cards of your library. You may reveal an artifact card from among them and put it into your hand. Put the rest on the bottom of your library in a random order.",
    )],
    ..oracle::ASHE_PRINCESS_OF_DALMASCA
};
