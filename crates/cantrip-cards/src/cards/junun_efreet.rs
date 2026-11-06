use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "At the beginning of your upkeep, sacrifice this creature unless you pay {B}{B}." (CR 608.2, 701.21a, 118.1).
fn sacrifice_creature_unless_pay_b(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    effects::sacrifice_this_unless_pay(g, item, cost("{B}{B}"))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOUR_UPKEEP,
            sacrifice_creature_unless_pay_b,
            "At the beginning of your upkeep, sacrifice this creature unless you pay {B}{B}.",
        ),
    ],
    ..oracle::JUN_N_EFREET
};
