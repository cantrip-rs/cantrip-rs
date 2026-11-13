use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            tokens::create_a_dragon_5_5,
            "When Kiora enters, create a 5/5 red Dragon creature token with flying.",
        ),
        ability(cost("{8}"), tokens::create_a_dragon_5_5, "{8}: Create a 5/5 red Dragon creature token with flying."),
    ],
    ..oracle::KIORA_OF_FIRE_AND_ASHES
};

pub static EXTRA: &[&CardDef] = &[&tokens::DRAGON_5_5];
