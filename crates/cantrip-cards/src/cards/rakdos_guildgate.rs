use cantrip_api::*;
use cantrip_mechanics::mana;
use cantrip_oracle as oracle;

/// "This land enters tapped. {T}: Add {B} or {R}." (CR 614.1d, 605.1a): the shared enters_tapped and one Black-or-Red tap.
pub static CARD: CardDef = CardDef {
    enters: Some(mana::enters_tapped),
    abilities: &[mana_ability(TAP, add_one_of(&[Color::Black, Color::Red]), "{T}: Add {B} or {R}.")],
    ..oracle::RAKDOS_GUILDGATE
};
