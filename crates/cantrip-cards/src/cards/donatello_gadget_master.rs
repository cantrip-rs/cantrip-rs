use cantrip_api::*;
use cantrip_mechanics::copy::create_token_copy_of_target;
use cantrip_mechanics::triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER;
use cantrip_mechanics::{sneak, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        DEALS_COMBAT_DAMAGE_TO_PLAYER,
        create_token_copy_of_target,
        "Whenever Donatello deals combat damage to a player, create a token that's a copy of target artifact you control.",
    )
    .targets(&[targets::ARTIFACT_YOU_CONTROL])],
    ways: &[sneak::way(mana("{1}{U}"), "Sneak {1}{U}")],
    ..oracle::DONATELLO_GADGET_MASTER
};
