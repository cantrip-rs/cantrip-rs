use cantrip_api::*;
use cantrip_mechanics::ninjutsu;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        ninjutsu::ninjutsu(mana("{3}{B}"), "Ninjutsu {3}{B} ({3}{B}, Return an unblocked attacker you control to hand: Put this card onto the battlefield from your hand tapped and attacking.)"),
    ],
    ..oracle::DOKUCHI_SHADOW_WALKER
};
