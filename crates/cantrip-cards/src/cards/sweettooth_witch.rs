use cantrip_api::*;
use cantrip_mechanics::targets::{self, filters};
use cantrip_mechanics::{effects, tokens, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, tokens::create_a_food_token, "When this creature enters, create a Food token."),
        ability(
            cost("{2}").and(Cost::sacrifice(1, filters::food)),
            effects::target_player_loses_life::<2>,
            "{2}, Sacrifice a Food: Target player loses 2 life.",
        )
        .targets(&[targets::PLAYER]),
    ],
    ..oracle::SWEETTOOTH_WITCH
};

pub static EXTRA: &[&CardDef] = &[&tokens::FOOD];
