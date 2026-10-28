use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "When this creature enters, create a tapped Treasure token." (CR 111.10a, 110.5b): Flash and Reach are in its content file.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        |g, item| tokens::create_tapped_treasure(g, item.controller),
        "When this creature enters, create a tapped Treasure token.",
    )],
    ..oracle::LONG_BODIED_GREY_DOG
};

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];
