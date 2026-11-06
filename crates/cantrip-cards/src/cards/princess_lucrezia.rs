use cantrip_api::*;
use cantrip_oracle as oracle;

/// "{T}: Add {U}." (CR 605.1a).
pub static CARD: CardDef = CardDef {
    abilities: &[mana_ability(TAP, add(Mana::U), "{T}: Add {U}.")],
    ..oracle::PRINCESS_LUCREZIA
};
