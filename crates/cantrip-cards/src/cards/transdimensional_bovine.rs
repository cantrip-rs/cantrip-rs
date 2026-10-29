use cantrip_api::*;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[mana_ability(TAP, add_any_color(2), "{T}: Add two mana of any one color.")],
    ..oracle::TRANSDIMENSIONAL_BOVINE
};
