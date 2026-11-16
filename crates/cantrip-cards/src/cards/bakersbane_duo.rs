use cantrip_api::*;
use cantrip_mechanics::expend::EXPEND_4;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, tokens};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, tokens::create_a_food_token, "When this creature enters, create a Food token."),
        triggered(
            EXPEND_4,
            effects::this_gets::<1, 1>,
            "Whenever you expend 4, this creature gets +1/+1 until end of turn. (You expend 4 as you spend your fourth total mana to cast spells during a turn.)",
        ),
    ],
    ..oracle::BAKERSBANE_DUO
};

pub static EXTRA: &[&CardDef] = &[&tokens::FOOD];
