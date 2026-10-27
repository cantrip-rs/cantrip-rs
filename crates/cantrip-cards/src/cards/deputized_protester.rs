use cantrip_api::*;
use cantrip_mechanics::melee;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        melee::TRIGGER,
    ],
    ..oracle::DEPUTIZED_PROTESTER
};
