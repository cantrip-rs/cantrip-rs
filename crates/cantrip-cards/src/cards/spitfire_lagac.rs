use cantrip_api::*;
use cantrip_mechanics::{effects, triggers::LANDFALL};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        LANDFALL,
        effects::damage_each_opponent_n::<1>,
        "Landfall \u{2014} Whenever a land you control enters, this creature deals 1 damage to each opponent.",
    )],
    ..oracle::SPITFIRE_LAGAC
};
