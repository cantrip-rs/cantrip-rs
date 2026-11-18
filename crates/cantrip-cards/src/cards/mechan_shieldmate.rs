use cantrip_api::*;
use cantrip_mechanics::statics;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    attacks_despite_defender: &[statics::itself_attacks_despite_defender_after_an_artifact_entered],
    ..oracle::MECHAN_SHIELDMATE
};
