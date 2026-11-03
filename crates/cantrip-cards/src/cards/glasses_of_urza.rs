use cantrip_api::*;
use cantrip_mechanics::{hand, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(TAP, hand::look_at_target_players_hand, "{T}: Look at target player's hand.")
            .targets(&[targets::PLAYER]),
    ],
    ..oracle::GLASSES_OF_URZA
};
