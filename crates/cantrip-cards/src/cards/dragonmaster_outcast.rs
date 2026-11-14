use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers, you_control_at_least_n_lands};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOUR_UPKEEP,
        tokens::create_a_dragon_5_5,
        "At the beginning of your upkeep, if you control six or more lands, create a 5/5 red Dragon creature token with flying.",
    )
    .condition(you_control_at_least_n_lands::<6>)],
    ..oracle::DRAGONMASTER_OUTCAST
};

pub static EXTRA: &[&CardDef] = &[&tokens::DRAGON_5_5];
