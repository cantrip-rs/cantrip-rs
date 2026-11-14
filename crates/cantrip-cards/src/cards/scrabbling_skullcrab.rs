use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::EERIE,
        effects::target_player_mills::<2>,
        "Eerie — Whenever an enchantment you control enters and whenever you fully unlock a Room, target player mills two cards.",
    )
    .targets(&[targets::PLAYER])],
    ..oracle::SCRABBLING_SKULLCRAB
};
