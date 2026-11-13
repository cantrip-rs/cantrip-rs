use cantrip_api::*;
use cantrip_mechanics::{effects, saddle, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ATTACKS_WHILE_SADDLED,
            effects::this_gets::<0, 3>,
            "Whenever this creature attacks while saddled, it gets +0/+3 until end of turn.",
        ),
        saddle::ability(
            2,
            "Saddle 2 (Tap any number of other creatures you control with total power 2 or more: This Mount becomes saddled until end of turn. Saddle only as a sorcery.)",
        ),
    ],
    ..oracle::VENOMSAC_LAGAC
};
