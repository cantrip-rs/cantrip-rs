use cantrip_api::*;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[mana_ability(TAP, add(Mana::G), "{T}: Add {G}.")],
    ..oracle::NOXIOUS_NEWT
};
