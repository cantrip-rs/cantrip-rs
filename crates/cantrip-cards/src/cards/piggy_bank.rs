use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_mechanics::triggers::THIS_DIES;
use cantrip_oracle as oracle;

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_DIES, tokens::create_a_treasure, "When this creature dies, create a Treasure token.")],
    ..oracle::PIGGY_BANK
};
