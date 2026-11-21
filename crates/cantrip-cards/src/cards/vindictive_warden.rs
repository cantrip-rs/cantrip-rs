use cantrip_api::*;
use cantrip_mechanics::{effects, firebending};
use cantrip_oracle as oracle;

/// Menace is a core keyword read from the card's characteristics.
pub static CARD: CardDef = CardDef {
    abilities: &[
        firebending::trigger::<1>("Firebending 1 (Whenever this creature attacks, add {R}. This mana lasts until end of combat.)"),
        ability(cost("{3}"), effects::damage_each_opponent_n::<1>, "{3}: This creature deals 1 damage to each opponent."),
    ],
    ..oracle::VINDICTIVE_WARDEN
};
