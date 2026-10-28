use cantrip_api::*;
use cantrip_mechanics::echo;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[echo!(cost("{4}{G}"), "Echo {4}{G} (At the beginning of your upkeep, if this came under your control since the beginning of your last upkeep, sacrifice it unless you pay its echo cost.)")],
    ..oracle::WINDING_WURM
};
