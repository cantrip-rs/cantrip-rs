use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    // Eerie is an ability word (CR 207.2c): the ability is an ordinary triggered ability.
    abilities: &[triggered(
        triggers::EERIE,
        effects::this_gets::<1, 1>,
        "Eerie \u{2014} Whenever an enchantment you control enters and whenever you fully unlock a Room, this creature gets +1/+1 until end of turn.",
    )],
    ..oracle::ERRATIC_APPARITION
};
