use cantrip_api::*;
use cantrip_mechanics::library::look_for_an_artifact;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        look_for_an_artifact::<4>,
        "When Casey Jones enters, look at the top four cards of your library. You may reveal an artifact card from among them and put it into your hand. Put the rest on the bottom of your library in a random order.",
    )],
    ..oracle::CASEY_JONES_JURY_RIG_JUSTICIAR
};
