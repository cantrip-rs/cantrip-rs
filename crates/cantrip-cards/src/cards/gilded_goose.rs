use cantrip_api::*;
use cantrip_mechanics::targets::filters;
use cantrip_mechanics::tokens;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, tokens::create_a_food_token, "When this creature enters, create a Food token."),
        ability(cost("{1}{G}, {T}"), tokens::create_a_food_token, "{1}{G}, {T}: Create a Food token."),
        // The Food is part of the cost (CR 602.2b); a mana ability (CR 605.1a).
        mana_ability(TAP.and(Cost::sacrifice(1, filters::food)), add_any_color(1), "{T}, Sacrifice a Food: Add one mana of any color."),
    ],
    ..oracle::GILDED_GOOSE
};

pub static EXTRA: &[&CardDef] = &[&tokens::FOOD];
