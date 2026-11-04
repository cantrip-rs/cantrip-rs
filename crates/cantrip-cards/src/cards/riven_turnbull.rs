use cantrip_api::*;
use cantrip_oracle as oracle;

/// "{T}: Add {B}." (CR 605.1a).
pub static CARD: CardDef = CardDef {
    abilities: &[mana_ability(TAP, add(Mana::B), "{T}: Add {B}.")],
    ..oracle::RIVEN_TURNBULL
};
