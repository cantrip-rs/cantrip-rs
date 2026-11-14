use cantrip_api::*;
use cantrip_mechanics::mana;
use cantrip_oracle as oracle;

/// "This land enters tapped. {T}: Add {W} or {U}." (CR 614.1d, 605.1a): the shared enters_tapped and one White-or-Blue tap.
pub static CARD: CardDef = CardDef {
    enters: Some(mana::enters_tapped),
    abilities: &[mana_ability(TAP, add_one_of(&[Color::White, Color::Blue]), "{T}: Add {W} or {U}.")],
    ..oracle::AZORIUS_GUILDGATE
};
