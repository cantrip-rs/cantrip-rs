use cantrip_api::*;
use cantrip_mechanics::dethrone;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[dethrone::TRIGGER],
    ..oracle::ENRAGED_REVOLUTIONARY
};
