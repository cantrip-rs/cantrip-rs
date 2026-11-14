use cantrip_api::*;
use cantrip_mechanics::{effects, triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(DEALS_COMBAT_DAMAGE_TO_PLAYER, effects::each_player_discards_hand_then_draws_seven,
        "Whenever this creature deals combat damage to a player, each player discards their hand, then draws seven cards.")],
    ..oracle::DRAGON_MAGE
};
