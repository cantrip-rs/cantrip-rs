use cantrip_api::*;
use cantrip_mechanics::{exchange, targets};
use cantrip_oracle as oracle;

/// Exchange life totals with target opponent (CR 701.12c).
fn exchange_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(opponent) = targets::player(g, item, 0) {
        exchange::life_totals(g, item.controller, opponent);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(TAP.and(SACRIFICE_THIS), exchange_life,
        "{T}, Sacrifice this artifact: Exchange life totals with target opponent. Activate only during your upkeep.")
        .targets(&[targets::OPPONENT]).condition(cantrip_mechanics::your_upkeep)],
    ..oracle::MIRROR_UNIVERSE
};
