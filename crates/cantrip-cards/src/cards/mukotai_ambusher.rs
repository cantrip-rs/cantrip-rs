use cantrip_api::*;
use cantrip_mechanics::ninjutsu;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        ninjutsu::ninjutsu(mana("{1}{B}"), "Ninjutsu {1}{B} ({1}{B}, Return an unblocked attacker you control to hand: Put this card onto the battlefield from your hand tapped and attacking.)"),
    ],
    ..oracle::MUKOTAI_AMBUSHER
};
