use cantrip_api::*;
use cantrip_mechanics::{effects, targets, tokens, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, tokens::create_a_food_token, "When this artifact enters, create a Food token."),
        ability(
            cost("{5}, {T}").and(SACRIFICE_THIS),
            effects::destroy_first_target,
            "{5}, {T}, Sacrifice this artifact: Destroy target nonland permanent. Activate only as a sorcery.",
        )
        .targets(&[targets::NONLAND_PERMANENT])
        .sorcery_speed(),
    ],
    ..oracle::BUMBLEFLOWERS_SHAREPOT
};

pub static EXTRA: &[&CardDef] = &[&tokens::FOOD];
