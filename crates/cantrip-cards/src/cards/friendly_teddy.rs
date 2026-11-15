use cantrip_api::*;
use cantrip_mechanics::{effects, triggers::THIS_DIES};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_DIES, effects::each_player_draws_a_card, "When this creature dies, each player draws a card.")],
    ..oracle::FRIENDLY_TEDDY
};
