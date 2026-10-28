use cantrip_api::*;
use cantrip_mechanics::{effects, triggers::LANDFALL};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        LANDFALL,
        effects::this_gets::<1, 1>,
        "Landfall \u{2014} Whenever a land you control enters, this creature gets +1/+1 until end of turn.",
    )],
    ..oracle::ATTERCOP
};
