use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER_EVENT,
        effects::draw_that_many,
        "Whenever this creature deals combat damage to a player, draw that many cards.",
    )
    .damage_amount()],
    ..oracle::FEAR_OF_FAILED_TESTS
};
