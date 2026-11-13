use cantrip_api::*;
use cantrip_mechanics::{effects, saddle, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ATTACKS_WHILE_SADDLED, effects::this_gets::<2, 2>, "Whenever this creature attacks while saddled, it gets +2/+2 until end of turn."),
        saddle::ability(1, "Saddle 1 (Tap any number of other creatures you control with total power 1 or more: This Mount becomes saddled until end of turn. Saddle only as a sorcery.)"),
    ],
    ..oracle::ALACRIAN_JAGUAR
};
