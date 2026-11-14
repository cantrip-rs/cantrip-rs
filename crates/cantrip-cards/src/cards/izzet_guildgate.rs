use cantrip_api::*;
use cantrip_mechanics::mana;
use cantrip_oracle as oracle;

/// "This land enters tapped. {T}: Add {U} or {R}." (CR 614.1d, 605.1a): the shared enters_tapped and one Blue-or-Red tap.
pub static CARD: CardDef = CardDef {
    enters: Some(mana::enters_tapped),
    abilities: &[mana_ability(TAP, add_one_of(&[Color::Blue, Color::Red]), "{T}: Add {U} or {R}.")],
    ..oracle::IZZET_GUILDGATE
};
