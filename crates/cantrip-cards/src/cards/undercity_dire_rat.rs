use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_mechanics::triggers::THIS_DIES;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_DIES, tokens::create_a_treasure, "Rat Tail \u{2014} When this creature dies, create a Treasure token.")],
    ..oracle::UNDERCITY_DIRE_RAT
};

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];
