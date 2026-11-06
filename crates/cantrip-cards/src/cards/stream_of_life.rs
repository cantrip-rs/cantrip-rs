use cantrip_api::*;
use cantrip_mechanics::{effects::target_player_gains_x_life, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::PLAYER], target_player_gains_x_life),
    ..oracle::STREAM_OF_LIFE
};
