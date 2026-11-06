use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(TAP, effects::untap_targets, "{T}: Untap target land.")
            .targets(&[targets::LAND]),
    ],
    ..oracle::JUNIPER_ORDER_DRUID
};
