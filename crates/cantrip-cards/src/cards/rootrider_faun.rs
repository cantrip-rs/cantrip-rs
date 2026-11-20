use cantrip_api::*;
use cantrip_mechanics::lands;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[mana_ability(TAP, add(Mana::G), "{T}: Add {G}."), lands::ANY_COLOR_FOR_ONE],
    ..oracle::ROOTRIDER_FAUN
};
