use cantrip_api::*;
use cantrip_oracle as oracle;

/// "{T}: Add {G}." (CR 605.1a).
pub static CARD: CardDef = CardDef {
    abilities: &[mana_ability(TAP, add(Mana::G), "{T}: Add {G}.")],
    ..oracle::FYNDHORN_ELVES
};
