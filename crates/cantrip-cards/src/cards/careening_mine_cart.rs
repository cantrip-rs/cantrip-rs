use cantrip_api::*;
use cantrip_mechanics::crew::crew;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ATTACKS,
            tokens::create_a_treasure,
            "Whenever this Vehicle attacks, create a Treasure token.",
        ),
        crew(1, "Crew 1 (Tap any number of creatures you control with total power 1 or more: This Vehicle becomes an artifact creature until end of turn.)"),
    ],
    ..oracle::CAREENING_MINE_CART
};

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];
