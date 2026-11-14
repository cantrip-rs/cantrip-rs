use cantrip_api::*;
use cantrip_mechanics::mana;
use cantrip_oracle as oracle;

/// "This land enters tapped. {T}: Add {U} or {B}." (CR 614.1d, 605.1a): the shared enters_tapped and one Blue-or-Black tap.
pub static CARD: CardDef = CardDef {
    enters: Some(mana::enters_tapped),
    abilities: &[mana_ability(TAP, add_one_of(&[Color::Blue, Color::Black]), "{T}: Add {U} or {B}.")],
    ..oracle::DIMIR_GUILDGATE
};
