use cantrip_api::*;
use cantrip_oracle as oracle;

/// "{T}: Add {C}{C}." (CR 605.1a).
pub static CARD: CardDef = CardDef {
    abilities: &[mana_ability(TAP, add_n(Mana::C, 2), "{T}: Add {C}{C}.")],
    ..oracle::SUNASTIAN_FALCONER
};
