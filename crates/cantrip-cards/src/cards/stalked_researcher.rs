use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::EERIE,
        effects::this_can_attack_as_though_it_didnt_have_defender,
        "Eerie — Whenever an enchantment you control enters and whenever you fully unlock a Room, this creature can attack this turn as though it didn't have defender.",
    )],
    ..oracle::STALKED_RESEARCHER
};
