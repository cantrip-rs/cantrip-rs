use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    // Eerie is an ability word (CR 207.2c): the ability is an ordinary triggered ability. "Surveil 1" (CR 701.25a) is the
    // ability's controller looking at their own top card.
    abilities: &[triggered(
        triggers::EERIE,
        |g, item| effects::surveil(g, item, 1),
        "Eerie \u{2014} Whenever an enchantment you control enters and whenever you fully unlock a Room, surveil 1.",
    )],
    ..oracle::SKULLSNAP_NUISANCE
};
