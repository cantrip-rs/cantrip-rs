use cantrip_api::*;
use cantrip_mechanics::{copy, targets, triggers::YOUR_UPKEEP};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOUR_UPKEEP,
        copy::create_token_copy_of_target,
        "At the beginning of your upkeep, create a token that's a copy of another target nonland permanent you control.",
    )
    .targets(&[target(targets::other_nonland_permanent_you_control_filter)])],
    ..oracle::EXTRAVAGANT_REPLICATION
};
