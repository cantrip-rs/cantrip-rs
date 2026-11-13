use cantrip_api::*;
use cantrip_mechanics::{effects, saddle, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ATTACKS_WHILE_SADDLED,
            effects::damage_first_target::<2>,
            "Whenever this creature attacks while saddled, it deals 2 damage to any target.",
        )
        .targets(&[targets::ANY]),
        saddle::ability(3, "Saddle 3 (Tap any number of other creatures you control with total power 3 or more: This Mount becomes saddled until end of turn. Saddle only as a sorcery.)"),
    ],
    ..oracle::DRACOSAUR_AUXILIARY
};
