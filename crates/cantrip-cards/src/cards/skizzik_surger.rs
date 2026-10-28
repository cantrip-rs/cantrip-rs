use cantrip_api::*;
use cantrip_mechanics::echo;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[echo!(sacrifice(2, targets::filters::land), echo::SACRIFICE_TWO_LANDS, "Echo—Sacrifice two lands. (At the beginning of your upkeep, if this came under your control since the beginning of your last upkeep, sacrifice it unless you pay its echo cost.)")],
    ..oracle::SKIZZIK_SURGER
};
