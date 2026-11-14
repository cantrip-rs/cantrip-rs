use cantrip_api::*;
use cantrip_mechanics::mana;
use cantrip_oracle as oracle;

/// "This land enters tapped. {T}: Add {G} or {U}." (CR 614.1d, 605.1a): the shared enters_tapped and one Green-or-Blue tap.
pub static CARD: CardDef = CardDef {
    enters: Some(mana::enters_tapped),
    abilities: &[mana_ability(TAP, add_one_of(&[Color::Green, Color::Blue]), "{T}: Add {G} or {U}.")],
    ..oracle::SIMIC_GUILDGATE
};
