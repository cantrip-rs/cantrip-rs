use cantrip_api::*;
use cantrip_mechanics::{lands, mana};
use cantrip_oracle as oracle;

/// "This land enters tapped. When this land enters, scry 1. (Look at the top card of your library. You may put that card on the
/// bottom.) {T}: Add {R} or {G}." (CR 614.1d, 701.22, 605.1a): the shared enters_tapped, the shared SCRY_LAND, one R-or-G tap.
pub static CARD: CardDef = CardDef {
    enters: Some(mana::enters_tapped),
    abilities: &[lands::SCRY_LAND, mana_ability(TAP, add_one_of(&[Color::Red, Color::Green]), "{T}: Add {R} or {G}.")],
    ..oracle::TEMPLE_OF_ABANDON
};
