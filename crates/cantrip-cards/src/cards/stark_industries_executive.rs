use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{2}, {T}"), tokens::create_a_treasure, "{2}, {T}: Create a Treasure token.")],
    ..oracle::STARK_INDUSTRIES_EXECUTIVE
};

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];
