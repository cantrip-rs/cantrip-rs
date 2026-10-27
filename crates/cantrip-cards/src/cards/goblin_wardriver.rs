use cantrip_api::*;
use cantrip_mechanics::battle_cry;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        battle_cry::TRIGGER,
    ],
    ..oracle::GOBLIN_WARDRIVER
};
