use cantrip_api::*;
use cantrip_oracle as oracle;

/// "{T}: Add {R}." (CR 605.1a).
pub static CARD: CardDef = CardDef {
    abilities: &[mana_ability(TAP, add(Mana::R), "{T}: Add {R}.")],
    ..oracle::SISTERS_OF_THE_FLAME
};
