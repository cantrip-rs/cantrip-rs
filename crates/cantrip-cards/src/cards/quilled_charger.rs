use cantrip_api::*;
use cantrip_mechanics::effects::this_gets_and_gains_menace;
use cantrip_mechanics::saddle;
use cantrip_mechanics::triggers::THIS_ATTACKS_WHILE_SADDLED;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ATTACKS_WHILE_SADDLED,
            this_gets_and_gains_menace::<1, 2>,
            "Whenever this creature attacks while saddled, it gets +1/+2 and gains menace until end of turn.",
        ),
        saddle::ability(2, "Saddle 2 (Tap any number of other creatures you control with total power 2 or more: This Mount becomes saddled until end of turn. Saddle only as a sorcery.)"),
    ],
    ..oracle::QUILLED_CHARGER
};
