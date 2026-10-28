use cantrip_api::*;
use cantrip_mechanics::mana;
use cantrip_oracle as oracle;

// Improvise is the core's, read from the card's JSON.
pub static CARD: CardDef = CardDef {
    enters: Some(mana::enters_tapped),
    abilities: &[mana_ability(TAP, add_n(Mana::C, 3), "{T}: Add {C}{C}{C}.")],
    ..oracle::ARC_REACTOR
};
