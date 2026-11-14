use cantrip_api::*;
use cantrip_mechanics::mana;
use cantrip_oracle as oracle;

/// "This land enters tapped. {T}: Add {G} or {W}." (CR 614.1d, 605.1a): the shared enters_tapped and one Green-or-White tap.
pub static CARD: CardDef = CardDef {
    enters: Some(mana::enters_tapped),
    abilities: &[mana_ability(TAP, add_one_of(&[Color::Green, Color::White]), "{T}: Add {G} or {W}.")],
    ..oracle::SELESNYA_GUILDGATE
};
