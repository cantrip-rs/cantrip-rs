use cantrip_api::*;
use cantrip_mechanics::sunburst;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enters: Some(sunburst::enters),
    ..oracle::SUNTOUCHED_MYR
};
