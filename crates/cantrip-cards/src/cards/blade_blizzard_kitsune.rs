use cantrip_api::*;
use cantrip_mechanics::ninjutsu;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        ninjutsu::ninjutsu(mana("{3}{W}"), "Ninjutsu {3}{W} ({3}{W}, Return an unblocked attacker you control to hand: Put this card onto the battlefield from your hand tapped and attacking.)"),
    ],
    ..oracle::BLADE_BLIZZARD_KITSUNE
};
