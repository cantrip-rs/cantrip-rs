use cantrip_api::*;
use cantrip_mechanics::mana;
use cantrip_oracle as oracle;

/// "This land enters tapped. {T}: Add {B} or {G}." (CR 614.1d, 605.1a): the shared enters_tapped and one Black-or-Green tap.
pub static CARD: CardDef = CardDef {
    enters: Some(mana::enters_tapped),
    abilities: &[mana_ability(TAP, add_one_of(&[Color::Black, Color::Green]), "{T}: Add {B} or {G}.")],
    ..oracle::GOLGARI_GUILDGATE
};
