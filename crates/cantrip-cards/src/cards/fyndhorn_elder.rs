use cantrip_api::*;
use cantrip_oracle as oracle;

/// "{T}: Add {G}{G}." (CR 605.1a).
pub static CARD: CardDef = CardDef {
    abilities: &[mana_ability(TAP, add_n(Mana::G, 2), "{T}: Add {G}{G}.")],
    ..oracle::FYNDHORN_ELDER
};
